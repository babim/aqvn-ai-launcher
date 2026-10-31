use aqvn_workspace_lib::config::{AppConfig, ConfigWatcher};
use aqvn_workspace_lib::proxy::server::{bind_ephemeral, watch_target};
use std::time::Duration;

#[tokio::test]
async fn test_proxy_retargets_on_server_url_change() {
    let initial_url = "https://a0.matmagoc.com";
    let (listener, state, _info) = bind_ephemeral(initial_url).await.expect("bind proxy");
    drop(listener);

    let initial_upstream = state.upstream().await;
    assert_eq!(
        initial_upstream.target().origin(),
        "https://a0.matmagoc.com"
    );

    let initial_config = AppConfig {
        server_url: initial_url.to_string(),
        ..Default::default()
    };
    let watcher = ConfigWatcher::new(initial_config);

    // Spawn watch_target in background
    let watcher_clone = watcher.clone();
    let state_clone = state.clone();
    let task = tokio::spawn(async move {
        watch_target(watcher_clone, state_clone).await;
    });

    // Let watch_target start and subscribe
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Update config with a new server_url and server_profiles
    let new_url = "https://custom-instance.example.com";
    let new_config = AppConfig {
        server_url: new_url.to_string(),
        server_profiles: vec!["https://a0.matmagoc.com".to_string(), new_url.to_string()],
        ..Default::default()
    };
    watcher.update(new_config);

    // Allow watch_target to receive notification and update state
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while state.upstream().await.target().origin() != "https://custom-instance.example.com"
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let updated_upstream = state.upstream().await;
    assert_eq!(
        updated_upstream.target().origin(),
        "https://custom-instance.example.com"
    );

    task.abort();
}

#[tokio::test]
async fn test_cache_invalidation_on_retarget() {
    let initial_url = "https://a0.matmagoc.com";
    let (listener, state, _info) = bind_ephemeral(initial_url).await.expect("bind proxy");
    drop(listener);

    let origin = "https://a0.matmagoc.com";
    state
        .cache()
        .put(
            origin,
            "/css/app.css",
            Some("text/css"),
            b"body { background: #fff; }",
        )
        .await;

    assert!(
        state
            .cache()
            .contains_in_memory(origin, "/css/app.css")
            .await
    );
    assert!(state.cache().current_memory_bytes() > 0);

    let initial_config = AppConfig {
        server_url: initial_url.to_string(),
        ..Default::default()
    };
    let watcher = ConfigWatcher::new(initial_config);

    let watcher_clone = watcher.clone();
    let state_clone = state.clone();
    let task = tokio::spawn(async move {
        watch_target(watcher_clone, state_clone).await;
    });

    // Let watch_target start and subscribe
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Retarget to a different server URL
    let new_url = "https://custom-instance.example.com";
    let new_config = AppConfig {
        server_url: new_url.to_string(),
        ..Default::default()
    };
    watcher.update(new_config);

    // Allow watch_target to receive notification and update state
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while state.upstream().await.target().origin() != "https://custom-instance.example.com"
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    assert_eq!(
        state.upstream().await.target().origin(),
        "https://custom-instance.example.com"
    );

    // Verify cache has been invalidated
    assert!(
        !state
            .cache()
            .contains_in_memory(origin, "/css/app.css")
            .await,
        "in-memory cache must be purged on retarget"
    );
    assert_eq!(
        state.cache().current_memory_bytes(),
        0,
        "current_memory_bytes must be 0 after cache invalidation"
    );

    task.abort();
}
