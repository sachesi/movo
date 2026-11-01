use movo_core::storage::cache::ImageCache;
use relm4::gtk::{self, gdk, glib, prelude::*};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::Semaphore;

const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";

/// Posters are requested in grid-sized batches; without a cap a single page
/// opens dozens of sockets at once and starves the API requests.
const MAX_PARALLEL_DOWNLOADS: usize = 6;

/// Far beyond any poster. The address comes from a provider's page, so its reply
/// is bounded before it can fill memory.
const MAX_DOWNLOAD_BYTES: usize = 16 * 1024 * 1024;

/// Decoding allocates by what the file header claims, not by how long the file is.
const MAX_DECODED_SIDE: u32 = 8192;
const MAX_DECODED_BYTES: u64 = 256 * 1024 * 1024;

/// About 170 KB each at the size a grid shows them, so some 40 MB. Enough that
/// scrolling back and returning to a list repaints at once.
const TEXTURES_KEPT: usize = 256;

/// Monotonic counter owned by a recycled widget.
///
/// [`GridView`](gtk::GridView) reuses the same [`gtk::Picture`] for different
/// items, so a load started for the previous item must not paint over the
/// current one. Every load bumps the token and drops its result if the token
/// moved on while it was in flight.
pub type ImageToken = Rc<Cell<u64>>;

/// A picture waiting for a poster that another picture already asked for.
struct Waiter {
    picture: glib::WeakRef<gtk::Picture>,
    token: ImageToken,
    id: u64,
}

impl Waiter {
    fn still_wanted(&self) -> bool {
        self.token.get() == self.id
    }
}

/// The most recently used textures, and who waits for the ones on their way.
#[derive(Default)]
struct Posters {
    textures: Recent<gdk::Texture>,
    /// Rails and grids show the same titles, so one poster is often asked for
    /// several times at once. Only the first request goes out.
    pending: HashMap<String, Vec<Waiter>>,
}

thread_local! {
    static POSTERS: RefCell<Posters> = RefCell::default();
}

pub fn load(
    picture: &gtk::Picture,
    url: Option<&str>,
    width: i32,
    height: i32,
    token: &ImageToken,
) {
    let id = token.get().wrapping_add(1);
    token.set(id);

    let Some(url) = url.filter(|url| !url.is_empty()).map(str::to_string) else {
        picture.set_paintable(None::<&gdk::Paintable>);
        return;
    };
    let key = format!("{url}|{width}x{height}");
    let waiter = Waiter {
        picture: picture.downgrade(),
        token: token.clone(),
        id,
    };

    let first_asker = POSTERS.with_borrow_mut(|posters| {
        if let Some(texture) = posters.textures.get(&key) {
            picture.set_paintable(Some(texture));
            return None;
        }
        picture.set_paintable(None::<&gdk::Paintable>);
        match posters.pending.get_mut(&key) {
            Some(waiters) => {
                waiters.push(waiter);
                None
            }
            None => {
                posters.pending.insert(key.clone(), vec![waiter]);
                Some(key.clone())
            }
        }
    });
    let Some(key) = first_asker else {
        return;
    };

    relm4::spawn_local(async move {
        let still_wanted = {
            let key = key.clone();
            move || {
                POSTERS.with_borrow(|posters| {
                    posters
                        .pending
                        .get(&key)
                        .is_some_and(|waiters| waiters.iter().any(Waiter::still_wanted))
                })
            }
        };
        let scaled = scaled_rgba(url, key.clone(), width, height, still_wanted).await;
        let texture = scaled.map(|(pixels, width, height)| {
            gdk::MemoryTexture::new(
                width,
                height,
                gdk::MemoryFormat::R8g8b8a8,
                &glib::Bytes::from_owned(pixels),
                width as usize * 4,
            )
            .upcast::<gdk::Texture>()
        });
        let waiters = POSTERS.with_borrow_mut(|posters| {
            if let Some(texture) = &texture {
                posters.textures.insert(key.clone(), texture.clone());
            }
            posters.pending.remove(&key).unwrap_or_default()
        });
        let Some(texture) = texture else {
            return;
        };
        for waiter in waiters.iter().filter(|waiter| waiter.still_wanted()) {
            if let Some(picture) = waiter.picture.upgrade() {
                picture.set_paintable(Some(&texture));
            }
        }
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
    key: String,
    width: i32,
    height: i32,
    still_wanted: impl Fn() -> bool,
) -> Option<(Vec<u8>, i32, i32)> {
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
        relm4::spawn(download(url)).await.ok()??
    };

    relm4::spawn_blocking(move || {
        let scaled = scale_to_png(&bytes, width, height)?;
        ImageCache::put(&key, &scaled);
        decode_rgba(&scaled)
    })
    .await
    .ok()?
}

async fn download(url: String) -> Option<Vec<u8>> {
    let mut response = client()
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.ok()? {
        if bytes.len() + chunk.len() > MAX_DOWNLOAD_BYTES {
            log::warn!("poster over {MAX_DOWNLOAD_BYTES} bytes dropped: {url}");
            return None;
        }
        bytes.extend_from_slice(&chunk);
    }
    Some(bytes)
}

fn decode(bytes: &[u8]) -> Option<image::DynamicImage> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DECODED_SIDE);
    limits.max_image_height = Some(MAX_DECODED_SIDE);
    limits.max_alloc = Some(MAX_DECODED_BYTES);
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    reader.limits(limits);
    reader.decode().ok()
}

/// Crop and resize to the display bounds so a poster's natural size cannot
/// force its card wider than the cell it was allocated.
fn scale_to_png(bytes: &[u8], width: i32, height: i32) -> Option<Vec<u8>> {
    let scaled = decode(bytes)?.resize_to_fill(
        width.max(1) as u32,
        height.max(1) as u32,
        image::imageops::FilterType::Lanczos3,
    );
    let mut png = Cursor::new(Vec::new());
    scaled.write_to(&mut png, image::ImageFormat::Png).ok()?;
    Some(png.into_inner())
}

fn decode_rgba(bytes: &[u8]) -> Option<(Vec<u8>, i32, i32)> {
    let image = decode(bytes)?.into_rgba8();
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

/// A map keeping its [`TEXTURES_KEPT`] most recently used values.
struct Recent<T> {
    values: HashMap<String, T>,
    /// Oldest use first.
    order: VecDeque<String>,
}

impl<T> Default for Recent<T> {
    fn default() -> Self {
        Self {
            values: HashMap::new(),
            order: VecDeque::new(),
        }
    }
}

impl<T> Recent<T> {
    fn get(&mut self, key: &str) -> Option<&T> {
        let value = self.values.get(key)?;
        if let Some(at) = self.order.iter().position(|known| known == key) {
            if let Some(known) = self.order.remove(at) {
                self.order.push_back(known);
            }
        }
        Some(value)
    }

    fn insert(&mut self, key: String, value: T) {
        if self.values.insert(key.clone(), value).is_none() {
            self.order.push_back(key);
        }
        while self.order.len() > TEXTURES_KEPT {
            if let Some(oldest) = self.order.pop_front() {
                self.values.remove(&oldest);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_rgba, scale_to_png, Recent, TEXTURES_KEPT};

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut source = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(width, height)
            .write_to(&mut source, image::ImageFormat::Png)
            .unwrap();
        source.into_inner()
    }

    #[test]
    fn scales_posters_to_the_requested_bounds() {
        let scaled = scale_to_png(&png(20, 20), 170, 255).unwrap();
        let (pixels, width, height) = decode_rgba(&scaled).unwrap();

        assert_eq!((width, height), (170, 255));
        assert_eq!(pixels.len(), 170 * 255 * 4);
    }

    #[test]
    fn an_image_claiming_enormous_sides_is_refused_before_decoding() {
        let mut claimed = png(2, 2);
        // Width and height sit at fixed offsets in the header; the checksum
        // after them is not read before the limit applies.
        claimed[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        claimed[20..24].copy_from_slice(&100_000u32.to_be_bytes());

        assert!(decode_rgba(&claimed).is_none());
    }

    #[test]
    fn something_other_than_an_image_is_refused() {
        assert!(decode_rgba(b"<html>blocked</html>").is_none());
    }

    #[test]
    fn the_least_recently_used_value_goes_first() {
        let mut recent = Recent::default();
        for number in 0..TEXTURES_KEPT {
            recent.insert(number.to_string(), number);
        }
        assert_eq!(recent.get("0"), Some(&0));

        recent.insert("new".to_string(), 0);

        assert!(recent.get("0").is_some(), "it was just used");
        assert!(recent.get("1").is_none());
        assert_eq!(recent.values.len(), TEXTURES_KEPT);
    }
}
