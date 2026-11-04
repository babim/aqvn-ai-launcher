//! Loopback proxy CORS port isolation integration tests (VAL-SEC-001).

use aqvn_workspace_lib::proxy::server::{self, allowed_origin};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

async fn start_mock_chat_server() -> (String, tokio::task::JoinHandle<()>) {
    let app = Router::new().route(
        "/api/csrf_token",
        get(|| async {
            Json(json!({
                "ok": true,
                "runtime_id": "test_rt",
                "token": "test_token"
            }))
            .into_response()
        }),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind mock chat server");
    let addr = listener.local_addr().expect("mock addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve mock chat");
    });
    (format!("http://{addr}"), handle)
}

#[tokio::test]
async fn test_proxy_cors_port_isolation() {
    let (chat_url, _chat_handle) = start_mock_chat_server().await;
    let (listener, state, info) = server::bind_ephemeral(&chat_url).await.expect("bind proxy");

    let proxy_port = info.port;
    let proxy_url = info.url.clone();

    let proxy_handle = tokio::spawn(async move {
        server::serve(listener, state).await.expect("serve proxy");
    });

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("client");

    // 1. Authorized webview origins: tauri://localhost
    let res = client
        .get(format!("{proxy_url}/health"))
        .header(header::ORIGIN, "tauri://localhost")
        .send()
        .await
        .expect("request tauri://localhost");
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some("tauri://localhost"),
        "CORS must echo tauri://localhost"
    );

    // 2. Authorized webview origins: http://tauri.localhost
    let res = client
        .get(format!("{proxy_url}/health"))
        .header(header::ORIGIN, "http://tauri.localhost")
        .send()
        .await
        .expect("request http://tauri.localhost");
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some("http://tauri.localhost"),
        "CORS must echo http://tauri.localhost"
    );

    // 3. Authorized matching loopback IP & proxy port: 127.0.0.1:<proxy_port>
    let self_ip_origin = format!("http://127.0.0.1:{proxy_port}");
    let res = client
        .get(format!("{proxy_url}/health"))
        .header(header::ORIGIN, &self_ip_origin)
        .send()
        .await
        .expect("request matching proxy port");
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some(self_ip_origin.as_str()),
        "CORS must echo matching loopback IP with proxy port"
    );

    // 4. Authorized matching localhost & proxy port: localhost:<proxy_port>
    let self_lh_origin = format!("http://localhost:{proxy_port}");
    let res = client
        .get(format!("{proxy_url}/health"))
        .header(header::ORIGIN, &self_lh_origin)
        .send()
        .await
        .expect("request matching localhost port");
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some(self_lh_origin.as_str()),
        "CORS must echo matching localhost with proxy port"
    );

    // 5. Unauthorized foreign localhost port: http://localhost:3000 -> 403 Forbidden
    let res = client
        .get(format!("{proxy_url}/health"))
        .header(header::ORIGIN, "http://localhost:3000")
        .send()
        .await
        .expect("request foreign localhost:3000");
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "foreign localhost port must be rejected with 403 Forbidden"
    );
    assert_eq!(
        res.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        None,
        "rejected requests must not receive Access-Control-Allow-Origin header"
    );

    // 6. Unauthorized foreign localhost port: http://localhost:3155 -> 403 Forbidden
    let res = client
        .get(format!("{proxy_url}/health"))
        .header(header::ORIGIN, "http://localhost:3155")
        .send()
        .await
        .expect("request foreign localhost:3155");
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "foreign localhost:3155 must be rejected with 403 Forbidden"
    );
    assert_eq!(res.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN), None);

    // 7. Unauthorized foreign 127.0.0.1 port: http://127.0.0.1:8080 -> 403 Forbidden
    let res = client
        .get(format!("{proxy_url}/health"))
        .header(header::ORIGIN, "http://127.0.0.1:8080")
        .send()
        .await
        .expect("request foreign 127.0.0.1:8080");
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "foreign 127.0.0.1 port must be rejected with 403 Forbidden"
    );
    assert_eq!(res.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN), None);

    // 8. Foreign domain origin: https://evil.example -> 403 Forbidden
    let res = client
        .get(format!("{proxy_url}/health"))
        .header(header::ORIGIN, "https://evil.example")
        .send()
        .await
        .expect("request foreign origin");
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "foreign evil origin must be rejected with 403 Forbidden"
    );
    assert_eq!(res.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN), None);

    // 9. Preflight OPTIONS check on /proxy/csrf with foreign port -> 403 Forbidden
    let res = client
        .request(reqwest::Method::OPTIONS, format!("{proxy_url}/proxy/csrf"))
        .header(header::ORIGIN, "http://localhost:3000")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .send()
        .await
        .expect("preflight foreign port");
    assert_eq!(
        res.status(),
        StatusCode::FORBIDDEN,
        "preflight from foreign port must be rejected with 403"
    );

    // 10. Preflight OPTIONS check on /proxy/csrf with matching proxy port -> 204 No Content
    let res = client
        .request(reqwest::Method::OPTIONS, format!("{proxy_url}/proxy/csrf"))
        .header(header::ORIGIN, &self_ip_origin)
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
        .send()
        .await
        .expect("preflight matching port");
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        res.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some(self_ip_origin.as_str())
    );

    // 11. Unit test logic for allowed_origin helper
    assert!(allowed_origin("tauri://localhost", proxy_port));
    assert!(allowed_origin("http://tauri.localhost", proxy_port));
    assert!(allowed_origin("https://tauri.localhost", proxy_port));
    assert!(allowed_origin(
        &format!("http://127.0.0.1:{proxy_port}"),
        proxy_port
    ));
    assert!(allowed_origin(
        &format!("http://localhost:{proxy_port}"),
        proxy_port
    ));
    assert!(!allowed_origin("http://localhost:3000", proxy_port));
    assert!(!allowed_origin("http://127.0.0.1:3000", proxy_port));
    assert!(!allowed_origin("http://localhost:3155", proxy_port));
    assert!(!allowed_origin("https://evil.example", proxy_port));
    assert!(!allowed_origin("null", proxy_port));
    assert!(!allowed_origin("not a url", proxy_port));

    proxy_handle.abort();
}
