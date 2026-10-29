//! Local asset caching and acceleration layer for AQVN Workspace.
//!
//! Pre-caches heavy static assets (.ttf, .woff2, .css, .js, .svg, .png)
//! in memory and on disk under the app cache directory
//! (`$XDG_CACHE_HOME/net.aqvn.workspace/assets/`), serving them instantly
//! (<100ms FCP) without upstream network roundtrips.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

/// In-memory cached asset representation.
#[derive(Clone, Debug)]
pub struct CachedAsset {
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

/// Asset caching engine: dual-layer (RAM + persistent disk).
pub struct AssetCache {
    base_dir: PathBuf,
    memory: RwLock<HashMap<(String, String), CachedAsset>>,
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new(Self::default_base_dir())
    }
}

impl AssetCache {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            memory: RwLock::new(HashMap::new()),
        }
    }

    /// Resolves the default cache directory on the host.
    pub fn default_base_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("AQVN_CACHE_DIR") {
            if !dir.trim().is_empty() {
                return PathBuf::from(dir);
            }
        }
        if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
            if !xdg.trim().is_empty() {
                return PathBuf::from(xdg).join("net.aqvn.workspace").join("assets");
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            if !home.trim().is_empty() {
                return PathBuf::from(home)
                    .join(".cache")
                    .join("net.aqvn.workspace")
                    .join("assets");
            }
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            if !profile.trim().is_empty() {
                return PathBuf::from(profile)
                    .join(".cache")
                    .join("net.aqvn.workspace")
                    .join("assets");
            }
        }
        std::env::temp_dir()
            .join("net.aqvn.workspace")
            .join("assets")
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Determines whether a request path qualifies for static asset caching.
    pub fn is_cacheable(&self, path: &str) -> bool {
        is_cacheable(path)
    }

    /// Resolves a safe filesystem path for an asset under `base_dir/<origin>/<path>`.
    pub fn disk_path(&self, origin: &str, path: &str) -> Option<PathBuf> {
        let origin_dir = sanitize_origin(origin);
        let origin_base = self.base_dir.join(origin_dir);
        safe_path_under_root(&origin_base, path)
    }

    /// Retrieves an asset from memory or disk cache.
    pub async fn get(&self, origin: &str, path: &str) -> Option<CachedAsset> {
        let key = (origin.to_string(), path.to_string());

        // 1. Memory cache check
        {
            let mem = self.memory.read().await;
            if let Some(asset) = mem.get(&key) {
                return Some(asset.clone());
            }
        }

        // 2. Disk cache check
        let disk_path = self.disk_path(origin, path)?;
        if !disk_path.is_file() {
            return None;
        }

        let body = tokio::fs::read(&disk_path).await.ok()?;

        // Retrieve content type: first check companion .meta file, then derive from extension
        let meta_path = meta_file_path(&disk_path);
        let content_type = if meta_path.is_file() {
            tokio::fs::read_to_string(&meta_path)
                .await
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
        .or_else(|| Some(mime_type_for_path(path).to_string()));

        let asset = CachedAsset { content_type, body };

        // Populate in-memory cache
        {
            let mut mem = self.memory.write().await;
            mem.insert(key, asset.clone());
        }

        Some(asset)
    }

    /// Stores an asset into both memory and persistent disk cache.
    pub async fn put(&self, origin: &str, path: &str, content_type: Option<&str>, body: &[u8]) {
        let key = (origin.to_string(), path.to_string());
        let asset = CachedAsset {
            content_type: content_type.map(str::to_string),
            body: body.to_vec(),
        };

        // 1. Store in memory
        {
            let mut mem = self.memory.write().await;
            mem.insert(key, asset);
        }

        // 2. Store on disk
        if let Some(disk_path) = self.disk_path(origin, path) {
            if let Some(parent) = disk_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            if tokio::fs::write(&disk_path, body).await.is_ok() {
                if let Some(ct) = content_type {
                    let meta_path = meta_file_path(&disk_path);
                    let _ = tokio::fs::write(&meta_path, ct.as_bytes()).await;
                }
            }
        }
    }

    /// Clears both memory and disk caches.
    pub async fn clear(&self) {
        let mut mem = self.memory.write().await;
        mem.clear();
        let _ = tokio::fs::remove_dir_all(&self.base_dir).await;
    }
}

/// Checks whether an asset path has a static cacheable extension.
pub fn is_cacheable(path: &str) -> bool {
    let clean = path.split('?').next().unwrap_or(path);
    if clean == "/sw.js" || clean.ends_with("/sw.js") || clean.starts_with("/ui/asset-bundle") {
        return false;
    }
    let Some(dot_idx) = clean.rfind('.') else {
        return false;
    };
    let ext = &clean[dot_idx + 1..];
    matches!(
        ext.to_ascii_lowercase().as_str(),
        "ttf"
            | "woff2"
            | "woff"
            | "otf"
            | "eot"
            | "css"
            | "js"
            | "mjs"
            | "svg"
            | "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "ico"
            | "webp"
            | "wasm"
            | "map"
    )
}

/// Deduces MIME content type from path extension.
pub fn mime_type_for_path(path: &str) -> &'static str {
    let clean = path.split('?').next().unwrap_or(path);
    let ext = clean.rfind('.').map(|i| &clean[i + 1..]).unwrap_or("");
    match ext.to_ascii_lowercase().as_str() {
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "webp" => "image/webp",
        "wasm" => "application/wasm",
        "json" | "map" => "application/json",
        _ => "application/octet-stream",
    }
}

fn sanitize_origin(origin: &str) -> String {
    let mut s = origin
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .replace("://", "_")
        .replace([':', '/'], "_");
    if s.is_empty() {
        s = "default".to_string();
    }
    s
}

fn safe_path_under_root(base: &Path, rel: &str) -> Option<PathBuf> {
    let clean = rel.trim_start_matches('/');
    let mut resolved = base.to_path_buf();
    for part in clean.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return None;
        }
        if part.contains('\\') || part.contains('\0') {
            return None;
        }
        resolved.push(part);
    }
    if resolved.starts_with(base) {
        Some(resolved)
    } else {
        None
    }
}

fn meta_file_path(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".meta");
    PathBuf::from(os)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_cacheable_rules() {
        assert!(is_cacheable("/css/main.css"));
        assert!(is_cacheable("/js/app.js"));
        assert!(is_cacheable("/vendor/font.woff2"));
        assert!(is_cacheable("/components/button.js"));
        assert!(is_cacheable("/components/button.js?v=123"));

        // Uncacheable dynamic endpoints / service worker
        assert!(!is_cacheable("/sw.js"));
        assert!(!is_cacheable("/sw.js?version=2026.1"));
        assert!(!is_cacheable("/ui/index"));
        assert!(!is_cacheable("/ui/asset-bundle"));
        assert!(!is_cacheable("/ui/asset-bundle?v=1"));
        assert!(!is_cacheable("/ui/asset-bundle/bundle.js"));
        assert!(!is_cacheable("/some/future/page.html"));
    }
}
