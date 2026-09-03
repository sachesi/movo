use movo_core::storage::cache::ImageCache;
use relm4::gtk::{self, gdk, glib};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Semaphore;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";

/// Posters are requested in grid-sized batches; without a cap a single page
/// opens dozens of sockets at once and starves the API requests.
const MAX_PARALLEL_DOWNLOADS: usize = 6;

/// Monotonic counter owned by a recycled widget.
///
/// [`GridView`](gtk::GridView) reuses the same [`gtk::Picture`] for different
/// items, so a load started for the previous item must not paint over the
/// current one. Every load bumps the token and drops its result if the token
/// moved on while it was in flight.
pub type ImageToken = Rc<Cell<u64>>;

pub fn load(
    picture: &gtk::Picture,
    url: Option<&str>,
    width: i32,
    height: i32,
    token: &ImageToken,
) {
    let id = token.get().wrapping_add(1);
    token.set(id);
    picture.set_paintable(None::<&gdk::Paintable>);

    let Some(url) = url.filter(|url| !url.is_empty()).map(str::to_string) else {
        return;
    };

    let picture = picture.clone();
    let token = token.clone();
    relm4::spawn_local(async move {
        let still_wanted = {
            let token = token.clone();
            move || token.get() == id
        };
        let Some((pixels, width, height)) = scaled_rgba(url, width, height, still_wanted).await
        else {
            return;
        };
        if token.get() != id {
            return;
        }
        let texture = gdk::MemoryTexture::new(
            width,
            height,
            gdk::MemoryFormat::R8g8b8a8,
            &glib::Bytes::from_owned(pixels),
            width as usize * 4,
        );
        picture.set_paintable(Some(&texture));
    });
}

/// Fetch a poster and return it as RGBA pixels ready for a texture.
///
/// The disk cache holds the *scaled* PNG, so a cache hit skips the download
/// and the resize, and decoding never touches the main thread.
///
/// `still_wanted` is re-checked once the download slot frees up. Scrolling
/// recycles the widget while its load queues behind the permit, and without
/// this the poster for a cell that is long gone is still fetched in full.
async fn scaled_rgba(
    url: String,
    width: i32,
    height: i32,
    still_wanted: impl Fn() -> bool,
) -> Option<(Vec<u8>, i32, i32)> {
    let key = format!("{url}|{width}x{height}");

    let cache_key = key.clone();
    if let Ok(Some(pixels)) = relm4::spawn_blocking(move || {
        ImageCache::get(&cache_key).and_then(|bytes| decode_rgba(&bytes))
    })
    .await
    {
        return Some(pixels);
    }

    let bytes = {
        let _permit = permits().acquire().await.ok()?;
        if !still_wanted() {
            return None;
        }
        relm4::spawn(async move {
            let response = client()
                .get(&url)
                .header("User-Agent", USER_AGENT)
                .send()
                .await
                .ok()?;
            response.bytes().await.ok()
        })
        .await
        .ok()??
    };

    relm4::spawn_blocking(move || {
        let scaled = scale_to_png(&bytes, width, height)?;
        ImageCache::put(&key, &scaled);
        decode_rgba(&scaled)
    })
    .await
    .ok()?
}

/// Crop and resize to the display bounds so a poster's natural size cannot
/// force its card wider than the cell it was allocated.
fn scale_to_png(bytes: &[u8], width: i32, height: i32) -> Option<Vec<u8>> {
    let image = image::load_from_memory(bytes).ok()?;
    let scaled = image.resize_to_fill(
        width.max(1) as u32,
        height.max(1) as u32,
        image::imageops::FilterType::Lanczos3,
    );
    let mut png = std::io::Cursor::new(Vec::new());
    scaled.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
}

fn decode_rgba(bytes: &[u8]) -> Option<(Vec<u8>, i32, i32)> {
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = image.dimensions();
    Some((image.into_raw(), width as i32, height as i32))
}

fn permits() -> &'static Semaphore {
    static PERMITS: OnceLock<Semaphore> = OnceLock::new();
    PERMITS.get_or_init(|| Semaphore::new(MAX_PARALLEL_DOWNLOADS))
}

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::{decode_rgba, scale_to_png};

    #[test]
    fn scales_posters_to_the_requested_bounds() {
        let mut source = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(20, 20)
            .write_to(&mut source, image::ImageFormat::Png)
            .unwrap();

        let png = scale_to_png(&source.into_inner(), 170, 255).unwrap();
        let (pixels, width, height) = decode_rgba(&png).unwrap();

        assert_eq!((width, height), (170, 255));
        assert_eq!(pixels.len(), 170 * 255 * 4);
    }
}
