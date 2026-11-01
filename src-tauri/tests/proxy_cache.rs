use aqvn_workspace_lib::proxy::cache::{AssetCache, MAX_DISK_CACHE_BYTES};
use std::path::PathBuf;

fn create_temp_cache_dir(label: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("aqvn-test-cache-{}-{}", label, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir
}

#[tokio::test]
async fn test_disk_cache_quota_prune() {
    let temp_dir = create_temp_cache_dir("quota-prune");
    let origin = "http://localhost:55088";

    // 1. Verify default quota constant is 256MB
    assert_eq!(
        MAX_DISK_CACHE_BYTES,
        256 * 1024 * 1024,
        "MAX_DISK_CACHE_BYTES must be 256MB"
    );

    // 2. Initialize AssetCache with bounded disk quota of 260 bytes
    // (Assets include body + .meta + .etag companion files, ~114 bytes each)
    let cache = AssetCache::with_max_disk(temp_dir.clone(), 260);
    assert_eq!(cache.max_disk_cache_bytes(), 260);

    // Initial disk consumption should be 0
    assert_eq!(cache.current_disk_cache_bytes().await, 0);

    // Insert asset A (40 bytes body)
    cache
        .put(origin, "/asset_a.css", Some("text/css"), &[1u8; 40])
        .await;
    let size_after_a = cache.current_disk_cache_bytes().await;
    assert!((40..=260).contains(&size_after_a));

    // Small delay to ensure distinct filesystem timestamps
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Insert asset B (40 bytes body)
    cache
        .put(origin, "/asset_b.css", Some("text/css"), &[2u8; 40])
        .await;
    let size_after_b = cache.current_disk_cache_bytes().await;
    assert!(
        size_after_b > size_after_a,
        "both A and B fit under 260 bytes quota"
    );
    assert!(size_after_b <= 260);

    // Small delay
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Access asset A via get() to refresh its LRU recency
    let hit_a = cache.get(origin, "/asset_a.css").await;
    assert!(hit_a.is_some(), "asset A must be retrievable");

    // Small delay
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Insert asset C (50 bytes body). Total (A + B + C) exceeds 260 bytes!
    // Since asset A was accessed recently, asset B is the oldest and must be pruned!
    cache
        .put(origin, "/asset_c.css", Some("text/css"), &[3u8; 50])
        .await;

    // Verify disk cache usage is strictly bounded within quota limit
    let size_after_c = cache.current_disk_cache_bytes().await;
    assert!(
        size_after_c <= 260,
        "disk cache size ({}) must not exceed quota (260)",
        size_after_c
    );

    // Clear RAM memory to force checking disk directly
    cache.clear_memory().await;

    // Asset B should be pruned from disk
    let disk_path_b = cache.disk_path(origin, "/asset_b.css").unwrap();
    assert!(
        !disk_path_b.is_file(),
        "oldest asset B must be pruned from disk"
    );

    // Asset A and C should remain on disk
    let disk_path_a = cache.disk_path(origin, "/asset_a.css").unwrap();
    assert!(
        disk_path_a.is_file(),
        "recently touched asset A must remain on disk"
    );

    let disk_path_c = cache.disk_path(origin, "/asset_c.css").unwrap();
    assert!(disk_path_c.is_file(), "newest asset C must remain on disk");

    // 3. Test manual clear cache handler
    cache.clear_disk_cache().await;
    assert_eq!(
        cache.current_disk_cache_bytes().await,
        0,
        "disk cache must be completely cleared after clear_disk_cache"
    );
    assert!(
        !disk_path_a.is_file(),
        "asset A must be deleted after clear"
    );
    assert!(
        !disk_path_c.is_file(),
        "asset C must be deleted after clear"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_disk_cache_oversized_asset_rejection() {
    let temp_dir = create_temp_cache_dir("oversized");
    let origin = "http://localhost:55088";

    let cache = AssetCache::with_max_disk(temp_dir.clone(), 100);

    // Asset of 150 bytes exceeds 100 bytes disk quota
    cache.put(origin, "/huge.dat", None, &[7u8; 150]).await;

    assert_eq!(
        cache.current_disk_cache_bytes().await,
        0,
        "oversized asset must not be stored on disk"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}
