use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct ImageCache;

impl ImageCache {
    pub fn cache_dir() -> PathBuf {
        if let Some(proj_dirs) = super::project_dirs() {
            let dir = proj_dirs.cache_dir().join("posters");
            let _ = fs::create_dir_all(&dir);
            dir
        } else {
            let dir = PathBuf::from(".cache/posters");
            let _ = fs::create_dir_all(&dir);
            dir
        }
    }

    /// The cache directory, resolved and created once.
    ///
    /// [`cache_dir`](Self::cache_dir) creates the directory on every call, and
    /// every poster shown asks for its path at least once.
    fn resolved_dir() -> &'static PathBuf {
        static DIR: OnceLock<PathBuf> = OnceLock::new();
        DIR.get_or_init(Self::cache_dir)
    }

    pub fn key_path(url: &str) -> PathBuf {
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

    pub fn put(url: &str, bytes: &[u8]) {
        let path = Self::key_path(url);
        let _ = fs::write(path, bytes);
    }

    pub fn clear() {
        let dir = Self::cache_dir();
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::create_dir_all(&dir);
    }
}
