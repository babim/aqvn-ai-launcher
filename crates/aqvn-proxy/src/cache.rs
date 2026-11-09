//! Local asset caching and acceleration layer for AQVN Workspace.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::sync::RwLock;

pub const MAX_MEMORY_CACHE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_DISK_CACHE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct CachedAsset {
    pub content_type: Option<String>,
    pub body: Vec<u8>,
    pub etag: String,
}

impl CachedAsset {
    pub fn new(content_type: Option<String>, body: Vec<u8>) -> Self {
        let etag = compute_etag(&body);
        Self {
            content_type,
            body,
            etag,
        }
    }

    pub fn with_etag(content_type: Option<String>, body: Vec<u8>, etag: String) -> Self {
        Self {
            content_type,
            body,
            etag,
        }
    }

    pub fn etag(&self) -> &str {
        &self.etag
    }
}

#[derive(Debug)]
struct CacheEntry {
    asset: CachedAsset,
    last_accessed: AtomicU64,
}

pub struct AssetCache {
    base_dir: PathBuf,
    max_memory_bytes: usize,
    max_disk_cache_bytes: AtomicU64,
    memory: RwLock<HashMap<(String, String), CacheEntry>>,
    current_memory_bytes: AtomicUsize,
    access_counter: AtomicU64,
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::new(Self::default_base_dir())
    }
}

impl AssetCache {
    pub fn new(base_dir: PathBuf) -> Self {
        Self::with_limits(base_dir, MAX_MEMORY_CACHE_BYTES, MAX_DISK_CACHE_BYTES)
    }

    pub fn compute_etag(body: &[u8]) -> String {
        compute_etag(body)
    }

    pub fn with_max_memory(base_dir: PathBuf, max_memory_bytes: usize) -> Self {
        Self::with_limits(base_dir, max_memory_bytes, MAX_DISK_CACHE_BYTES)
    }

    pub fn with_max_disk(base_dir: PathBuf, max_disk_cache_bytes: u64) -> Self {
        Self::with_limits(base_dir, MAX_MEMORY_CACHE_BYTES, max_disk_cache_bytes)
    }

    pub fn with_max_disk_bytes(base_dir: PathBuf, max_disk_cache_bytes: u64) -> Self {
        Self::with_limits(base_dir, MAX_MEMORY_CACHE_BYTES, max_disk_cache_bytes)
    }

    pub fn with_limits(
        base_dir: PathBuf,
        max_memory_bytes: usize,
        max_disk_cache_bytes: u64,
    ) -> Self {
        Self {
            base_dir,
            max_memory_bytes,
            max_disk_cache_bytes: AtomicU64::new(max_disk_cache_bytes),
            memory: RwLock::new(HashMap::new()),
            current_memory_bytes: AtomicUsize::new(0),
            access_counter: AtomicU64::new(1),
        }
    }

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

    pub fn max_memory_bytes(&self) -> usize {
        self.max_memory_bytes
    }

    pub fn current_memory_bytes(&self) -> usize {
        self.current_memory_bytes.load(Ordering::Relaxed)
    }

    pub fn max_disk_cache_bytes(&self) -> u64 {
        self.max_disk_cache_bytes.load(Ordering::Relaxed)
    }

    pub fn set_max_disk_cache_bytes(&self, bytes: u64) {
        self.max_disk_cache_bytes.store(bytes, Ordering::Relaxed);
    }

    pub async fn current_disk_cache_bytes(&self) -> u64 {
        let base_dir = self.base_dir.clone();
        tokio::task::spawn_blocking(move || {
            let (_, total) = scan_disk_cache(&base_dir);
            total
        })
        .await
        .unwrap_or(0)
    }

    pub async fn contains_in_memory(&self, origin: &str, path: &str) -> bool {
        let key = (origin.to_string(), path.to_string());
        let mem = self.memory.read().await;
        mem.contains_key(&key)
    }

    pub async fn get_from_memory(&self, origin: &str, path: &str) -> Option<CachedAsset> {
        let key = (origin.to_string(), path.to_string());
        let mem = self.memory.read().await;
        if let Some(entry) = mem.get(&key) {
            let seq = self.access_counter.fetch_add(1, Ordering::Relaxed);
            entry.last_accessed.store(seq, Ordering::Relaxed);
            Some(entry.asset.clone())
        } else {
            None
        }
    }

    pub fn is_cacheable(&self, path: &str) -> bool {
        is_cacheable(path)
    }

    pub fn disk_path(&self, origin: &str, path: &str) -> Option<PathBuf> {
        let origin_dir = sanitize_origin(origin);
        let origin_base = self.base_dir.join(origin_dir);
        safe_path_under_root(&origin_base, path)
    }

    pub async fn get(&self, origin: &str, path: &str) -> Option<CachedAsset> {
        let key = (origin.to_string(), path.to_string());

        {
            let mem = self.memory.read().await;
            if let Some(entry) = mem.get(&key) {
                let seq = self.access_counter.fetch_add(1, Ordering::Relaxed);
                entry.last_accessed.store(seq, Ordering::Relaxed);
                if let Some(disk_path) = self.disk_path(origin, path) {
                    if disk_path.is_file() {
                        touch_disk_file(&disk_path);
                    }
                }
                return Some(entry.asset.clone());
            }
        }

        let disk_path = self.disk_path(origin, path)?;
        if !disk_path.is_file() {
            return None;
        }

        let body = tokio::fs::read(&disk_path).await.ok()?;
        touch_disk_file(&disk_path);

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

        let etag_path = etag_file_path(&disk_path);
        let etag = if etag_path.is_file() {
            tokio::fs::read_to_string(&etag_path)
                .await
                .ok()
                .map(|s| s.trim().to_string())
        } else {
            None
        }
        .unwrap_or_else(|| compute_etag(&body));

        let asset = CachedAsset {
            content_type,
            body,
            etag,
        };

        self.put_memory(key, asset.clone()).await;

        Some(asset)
    }

    pub async fn put(&self, origin: &str, path: &str, content_type: Option<&str>, body: &[u8]) {
        let key = (origin.to_string(), path.to_string());
        let etag = compute_etag(body);
        let asset = CachedAsset {
            content_type: content_type.map(str::to_string),
            body: body.to_vec(),
            etag: etag.clone(),
        };

        self.put_memory(key, asset).await;

        let max_disk = self.max_disk_cache_bytes();

        if body.len() as u64 > max_disk {
            if let Some(disk_path) = self.disk_path(origin, path) {
                let _ = tokio::fs::remove_file(&disk_path).await;
                let _ = tokio::fs::remove_file(meta_file_path(&disk_path)).await;
                let _ = tokio::fs::remove_file(etag_file_path(&disk_path)).await;
            }
            return;
        }

        if let Some(disk_path) = self.disk_path(origin, path) {
            if let Some(parent) = disk_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            if tokio::fs::write(&disk_path, body).await.is_ok() {
                if let Some(ct) = content_type {
                    let meta_path = meta_file_path(&disk_path);
                    let _ = tokio::fs::write(&meta_path, ct.as_bytes()).await;
                }
                let etag_path = etag_file_path(&disk_path);
                let _ = tokio::fs::write(&etag_path, etag.as_bytes()).await;

                self.prune_disk_cache().await;
            }
        }
    }

    async fn put_memory(&self, key: (String, String), asset: CachedAsset) {
        let asset_size = asset.body.len();
        if asset_size > self.max_memory_bytes {
            let mut mem = self.memory.write().await;
            if let Some(removed) = mem.remove(&key) {
                let current = self.current_memory_bytes.load(Ordering::Relaxed);
                self.current_memory_bytes.store(
                    current.saturating_sub(removed.asset.body.len()),
                    Ordering::Relaxed,
                );
            }
            return;
        }

        let mut mem = self.memory.write().await;
        let mut current = self.current_memory_bytes.load(Ordering::Relaxed);

        if let Some(old) = mem.remove(&key) {
            current = current.saturating_sub(old.asset.body.len());
        }

        while current + asset_size > self.max_memory_bytes && !mem.is_empty() {
            let oldest_key = mem
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed.load(Ordering::Relaxed))
                .map(|(k, _)| k.clone());

            if let Some(k) = oldest_key {
                if let Some(evicted) = mem.remove(&k) {
                    current = current.saturating_sub(evicted.asset.body.len());
                }
            } else {
                break;
            }
        }

        let seq = self.access_counter.fetch_add(1, Ordering::Relaxed);
        current += asset_size;
        mem.insert(
            key,
            CacheEntry {
                asset,
                last_accessed: AtomicU64::new(seq),
            },
        );
        self.current_memory_bytes.store(current, Ordering::Relaxed);
    }

    pub async fn prune_disk_cache(&self) -> u64 {
        let base_dir = self.base_dir.clone();
        let max_bytes = self.max_disk_cache_bytes();

        tokio::task::spawn_blocking(move || {
            let (mut assets, mut current_total) = scan_disk_cache(&base_dir);
            if current_total <= max_bytes {
                return 0;
            }

            assets.sort_by(|a, b| {
                a.last_accessed
                    .cmp(&b.last_accessed)
                    .then_with(|| a.primary_path.cmp(&b.primary_path))
            });

            let mut pruned_bytes: u64 = 0;
            for asset in assets {
                if current_total <= max_bytes {
                    break;
                }

                let _ = std::fs::remove_file(&asset.primary_path);
                let _ = std::fs::remove_file(&asset.meta_path);
                let _ = std::fs::remove_file(&asset.etag_path);

                current_total = current_total.saturating_sub(asset.total_size);
                pruned_bytes = pruned_bytes.saturating_add(asset.total_size);
            }

            cleanup_empty_dirs(&base_dir);

            pruned_bytes
        })
        .await
        .unwrap_or(0)
    }

    pub async fn clear(&self) {
        self.clear_disk_cache().await;
    }

    pub async fn clear_disk_cache(&self) {
        self.clear_memory().await;
        let base_dir = self.base_dir.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let _ = std::fs::remove_dir_all(&base_dir);
            let _ = std::fs::create_dir_all(&base_dir);
        })
        .await;
    }

    pub async fn clear_memory(&self) {
        let mut mem = self.memory.write().await;
        mem.clear();
        self.current_memory_bytes.store(0, Ordering::Relaxed);
    }
}

#[derive(Debug)]
struct ScannedAsset {
    primary_path: PathBuf,
    meta_path: PathBuf,
    etag_path: PathBuf,
    total_size: u64,
    last_accessed: std::time::SystemTime,
}

fn scan_disk_cache(base_dir: &Path) -> (Vec<ScannedAsset>, u64) {
    if !base_dir.is_dir() {
        return (Vec::new(), 0);
    }
    let mut files = Vec::new();
    collect_files_recursive(base_dir, &mut files);

    let mut file_map: HashMap<PathBuf, (u64, std::time::SystemTime)> = HashMap::new();
    for path in files {
        if let Ok(meta) = std::fs::metadata(&path) {
            let size = meta.len();
            let ts = match (meta.modified(), meta.accessed()) {
                (Ok(m), Ok(a)) => std::cmp::max(m, a),
                (Ok(m), _) => m,
                (_, Ok(a)) => a,
                _ => std::time::SystemTime::UNIX_EPOCH,
            };
            file_map.insert(path, (size, ts));
        }
    }

    let mut total_disk_bytes: u64 = file_map.values().map(|(size, _)| *size).sum();
    let mut assets = Vec::new();

    for (path, (size, ts)) in &file_map {
        let path_str = path.to_string_lossy();
        if path_str.ends_with(".meta") || path_str.ends_with(".etag") {
            continue;
        }

        let meta_p = meta_file_path(path);
        let etag_p = etag_file_path(path);

        let meta_size = file_map.get(&meta_p).map(|(s, _)| *s).unwrap_or(0);
        let etag_size = file_map.get(&etag_p).map(|(s, _)| *s).unwrap_or(0);

        let total_size = size + meta_size + etag_size;

        assets.push(ScannedAsset {
            primary_path: path.clone(),
            meta_path: meta_p,
            etag_path: etag_p,
            total_size,
            last_accessed: *ts,
        });
    }

    for (path, (size, _)) in &file_map {
        let path_str = path.to_string_lossy();
        if path_str.ends_with(".meta") {
            let primary = PathBuf::from(path_str.trim_end_matches(".meta"));
            if !file_map.contains_key(&primary) {
                let _ = std::fs::remove_file(path);
                total_disk_bytes = total_disk_bytes.saturating_sub(*size);
            }
        } else if path_str.ends_with(".etag") {
            let primary = PathBuf::from(path_str.trim_end_matches(".etag"));
            if !file_map.contains_key(&primary) {
                let _ = std::fs::remove_file(path);
                total_disk_bytes = total_disk_bytes.saturating_sub(*size);
            }
        }
    }

    (assets, total_disk_bytes)
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursive(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

fn cleanup_empty_dirs(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            cleanup_empty_dirs(&path);
            let _ = std::fs::remove_dir(&path);
        }
    }
}

fn touch_disk_file(path: &Path) {
    if let Ok(file) = std::fs::File::options().write(true).open(path) {
        let now = std::fs::FileTimes::new().set_modified(std::time::SystemTime::now());
        let _ = file.set_times(now);
    }
}

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

fn etag_file_path(path: &Path) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(".etag");
    PathBuf::from(os)
}

pub fn compute_etag(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    format!("\"{:x}\"", hasher.finalize())
}

pub fn etag_matches(if_none_match: &str, etag: &str) -> bool {
    let inm = if_none_match.trim();
    if inm == "*" {
        return true;
    }
    let target = etag.trim().trim_matches('"');
    for part in inm.split(',') {
        let candidate = part.trim();
        if candidate == "*" {
            return true;
        }
        let candidate_clean = candidate.trim_start_matches("W/").trim_matches('"');
        if candidate_clean == target {
            return true;
        }
    }
    false
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

        assert!(!is_cacheable("/sw.js"));
        assert!(!is_cacheable("/sw.js?version=2026.1"));
        assert!(!is_cacheable("/ui/index"));
        assert!(!is_cacheable("/ui/asset-bundle"));
        assert!(!is_cacheable("/ui/asset-bundle?v=1"));
        assert!(!is_cacheable("/ui/asset-bundle/bundle.js"));
        assert!(!is_cacheable("/some/future/page.html"));
    }

    #[tokio::test]
    async fn test_asset_cache_memory_lru_eviction() {
        let temp_dir = std::env::temp_dir().join(format!("aqvn-unit-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let cache = AssetCache::with_max_memory(temp_dir.clone(), 100);
        let origin = "http://localhost:55088";

        cache.put(origin, "/a.css", None, &[1u8; 40]).await;
        assert_eq!(cache.current_memory_bytes(), 40);
        assert!(cache.contains_in_memory(origin, "/a.css").await);

        cache.put(origin, "/b.css", None, &[2u8; 40]).await;
        assert_eq!(cache.current_memory_bytes(), 80);
        assert!(cache.contains_in_memory(origin, "/b.css").await);

        cache.put(origin, "/c.css", None, &[3u8; 30]).await;
        assert_eq!(cache.current_memory_bytes(), 70);
        assert!(!cache.contains_in_memory(origin, "/a.css").await);
        assert!(cache.contains_in_memory(origin, "/b.css").await);
        assert!(cache.contains_in_memory(origin, "/c.css").await);

        assert!(cache.get(origin, "/b.css").await.is_some());

        cache.put(origin, "/d.css", None, &[4u8; 30]).await;
        assert_eq!(cache.current_memory_bytes(), 100);
        assert!(cache.contains_in_memory(origin, "/b.css").await);
        assert!(cache.contains_in_memory(origin, "/c.css").await);
        assert!(cache.contains_in_memory(origin, "/d.css").await);

        cache.put(origin, "/e.css", None, &[5u8; 10]).await;
        assert_eq!(cache.current_memory_bytes(), 80);
        assert!(!cache.contains_in_memory(origin, "/c.css").await);
        assert!(cache.contains_in_memory(origin, "/b.css").await);
        assert!(cache.contains_in_memory(origin, "/d.css").await);
        assert!(cache.contains_in_memory(origin, "/e.css").await);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_asset_cache_update_existing_and_oversized() {
        let temp_dir =
            std::env::temp_dir().join(format!("aqvn-unit-cache-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let cache = AssetCache::with_max_memory(temp_dir.clone(), 100);
        let origin = "http://localhost:55088";

        cache.put(origin, "/a.css", None, &[1u8; 40]).await;
        assert_eq!(cache.current_memory_bytes(), 40);

        cache.put(origin, "/a.css", None, &[1u8; 60]).await;
        assert_eq!(cache.current_memory_bytes(), 60);

        cache.put(origin, "/oversized.bin", None, &[9u8; 105]).await;
        assert_eq!(cache.current_memory_bytes(), 60);
        assert!(!cache.contains_in_memory(origin, "/oversized.bin").await);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_compute_etag_and_matching() {
        let body = b"hello world cache asset content";
        let etag = compute_etag(body);
        assert!(etag.starts_with('"') && etag.ends_with('"'));
        assert_eq!(AssetCache::compute_etag(body), etag);

        assert!(etag_matches(&etag, &etag));
        let raw = etag.trim_matches('"');
        assert!(etag_matches(raw, &etag));
        assert!(etag_matches(&format!("W/{etag}"), &etag));
        assert!(etag_matches("*", &etag));
        assert!(etag_matches(
            &format!("\"other\", {etag}, \"something\""),
            &etag
        ));
        assert!(!etag_matches("\"mismatched\"", &etag));
    }

    #[tokio::test]
    async fn test_asset_cache_stores_and_returns_etag() {
        let temp_dir =
            std::env::temp_dir().join(format!("aqvn-unit-cache-etag-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);

        let cache = AssetCache::new(temp_dir.clone());
        let origin = "http://localhost:55088";
        let path = "/css/test.css";
        let body = b"body { color: red; }";

        cache.put(origin, path, Some("text/css"), body).await;

        let asset = cache.get(origin, path).await.expect("cached asset");
        assert_eq!(asset.etag, compute_etag(body));
        assert_eq!(asset.etag(), compute_etag(body));

        cache.clear_memory().await;
        let asset_from_disk = cache.get(origin, path).await.expect("asset from disk");
        assert_eq!(asset_from_disk.etag, compute_etag(body));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
