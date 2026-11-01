use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::SystemTime;

/// Some ten thousand posters at the size a grid shows them.
const MAX_CACHE_BYTES: u64 = 512 * 1024 * 1024;

pub struct ImageCache;

impl ImageCache {
    fn cache_dir() -> PathBuf {
        let dir = match super::project_dirs() {
            Some(dirs) => dirs.cache_dir().join("posters"),
            None => PathBuf::from(".cache/posters"),
        };
        // Which posters are here says what the user browsed and watched.
        let _ = super::create_private_dir(&dir);
        dir
    }

    /// The cache directory, resolved and created once.
    ///
    /// [`cache_dir`](Self::cache_dir) creates the directory on every call, and
    /// every poster shown asks for its path at least once.
    fn resolved_dir() -> &'static PathBuf {
        static DIR: OnceLock<PathBuf> = OnceLock::new();
        DIR.get_or_init(Self::cache_dir)
    }

    fn key_path(url: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = hex::encode(hasher.finalize());
        Self::resolved_dir().join(format!("{}.img", hash))
    }

    pub fn get(url: &str) -> Option<Vec<u8>> {
        let path = Self::key_path(url);
        if path.exists() {
            fs::read(path).ok()
        } else {
            None
        }
    }

    /// Writes beside the final name and renames, so a reader never sees half
    /// a poster.
    pub fn put(url: &str, bytes: &[u8]) {
        let path = Self::key_path(url);
        let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
        if fs::write(&temporary, bytes).is_err() || fs::rename(&temporary, path).is_err() {
            let _ = fs::remove_file(&temporary);
        }
    }

    /// Removes the oldest posters until the cache fits its limit. Nothing else
    /// ever removes one, so without this it grows for as long as the app is used.
    pub fn trim() {
        trim_dir(Self::resolved_dir(), MAX_CACHE_BYTES);
    }

    pub fn clear() {
        let dir = Self::cache_dir();
        let _ = fs::remove_dir_all(&dir);
        let _ = super::create_private_dir(&dir);
    }
}

fn trim_dir(dir: &Path, max_bytes: u64) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(SystemTime, u64, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            metadata
                .is_file()
                .then(|| Some((metadata.modified().ok()?, metadata.len(), entry.path())))?
        })
        .collect();
    let mut total: u64 = files.iter().map(|(_, size, _)| size).sum();
    if total <= max_bytes {
        return;
    }
    files.sort_unstable();
    for (_, size, path) in files {
        if total <= max_bytes {
            break;
        }
        if fs::remove_file(path).is_ok() {
            total -= size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::trim_dir;
    use std::fs;
    use std::time::{Duration, SystemTime};

    #[test]
    fn trimming_removes_the_oldest_until_it_fits() {
        let dir = std::env::temp_dir().join(format!("movo-cache-trim-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let now = SystemTime::now();
        for (name, age) in [("oldest", 300), ("middle", 200), ("newest", 100)] {
            let path = dir.join(name);
            fs::write(&path, [0; 10]).unwrap();
            let file = fs::File::options().write(true).open(&path).unwrap();
            file.set_modified(now - Duration::from_secs(age)).unwrap();
        }

        trim_dir(&dir, 25);

        assert!(!dir.join("oldest").exists());
        assert!(dir.join("middle").exists());
        assert!(dir.join("newest").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
