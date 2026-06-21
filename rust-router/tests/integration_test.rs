use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;
use rust_router::server::{build_router, AppState};
use rust_router::config::{ProfileManager, RouterConfig};
use rust_router::evidence::EvidenceWriter;
use rust_router::process::BackendState;
use std::collections::HashMap;
use tokio::sync::Mutex;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use std::io::Write;

async fn setup_app(config: RouterConfig) -> Arc<AppState> {
    // We need a profile manager. Since we can't easily mock it, 
    // we'll create a temporary profile config file.
    let mut temp_profiles = NamedTempFile::new().unwrap();
    let profiles_json = serde_json::json!({
        "profiles": [
            {
                "alias": "test-model",
                "model_file": "test.gguf",
                "model_path": "test.gguf",
                "port": 12345,
                "task_classes": ["test-task"]
            }
        ]
    });
    serde_json::to_writer(&mut temp_profiles, &profiles_json).unwrap();
    let temp_path = temp_profiles.path().to_path_buf();

    // For the purpose of this test, we'll manually construct a ProfileManager 
    // if possible, but since it's private, we'll use the load_from_config.
    // However, load_from_config uses hardcoded paths in the current implementation.
    // This is a problem for testing.
    
    // Let's see if we can bypass it.
    // Actually, let's just use the existing implementation and hope for the best, 
    // or better, let's modify RouterConfig to allow passing the path more easily.
    // But I shouldn't change the implementation unless necessary.
    
    // Wait, RouterConfig::load_from_config uses config.profile_config_path if provided.
    // So we can set it!
    
    let mut test_config = config.clone();
    test_config.profile_config_path = Some(temp_path);

    let pm = ProfileManager::load_from_config(&test_config).expect("Failed to load profiles");

    let backends = Mutex::new(HashMap::new());
    let evidence_writer = EvidenceWriter::new();
    let start_time = std::time::Instant::now();
    let health_poller_handle = Mutex::new(None);

    Arc::new(AppState {
        profile_manager: pm,
        config: test_config,
        backends,
        evidence_writer,
        start_time,
        health_poller_handle,
    })
}

#[tokio::test]
async fn test_auth_middleware_success() {
    let config = RouterConfig {
        router_host: "127.0.0.1".to_string(),
        router_port: 9130,
        backend_port_base: 9120,
        auth_token: Some("secret-token".to_string()),
        require_auth: true,
        max_body_bytes: 1024 * 1024,
        profile_config_path: None,
        backend_binary_path: None,
        evidence_path: None,
        log_path: None,
        health_timeout_secs: 1,
        health_poll_interval_secs: 1,
    };

    let state = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/status")
        .header(header::AUTHORIZATION, "Bearer secret-token") // Wait, the middleware expects exact match, not "Bearer " prefix
        .body(Body::empty())
        .unwrap();
    
    // Re-checking middleware implementation:
    // match auth_header {
    //     Some(token) if Some(token) == state.config.auth_token.as_ref().map(|x| x.as_str()) => {
    //         Ok(next.run(req).await)
    //     }
    // ...
    // So it expects the exact token.

    let req = Request::builder()
        .uri("/backend/status")
        .header(header::AUTHORIZATION, "secret-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_auth_middleware_failure() {
    let config = RouterConfig {
        router_host: "127.0.0.1".to_string(),
        router_port: 9130,
        backend_port_base: 9120,
        auth_token: Some("secret-token".to_string()),
        require_auth: true,
        max_body_bytes: 1024 * 1024,
        profile_config_path: None,
        backend_binary_path: None,
        evidence_path: None,
        log_path: None,
        health_timeout_secs: 1,
        health_poll_interval_secs: 1,
    };

    let state = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/status")
        .header(header::AUTHORIZATION, "wrong-token")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_middleware_disabled() {
    let config = RouterConfig {
        router_host: "127.0.0.1".to_string(),
        router_port: 9130,
        backend_port_base: 9120,
        auth_token: Some("secret-token".to_string()),
        require_auth: false,
        max_body_bytes: 1024 * 1024,
        profile_config_path: None,
        backend_binary_path: None,
        evidence_path: None,
        log_path: None,
        health_timeout_secs: 1,
        health_poll_interval_secs: 1,
    };

    let state = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_max_body_bytes() {
    let config = RouterConfig {
        router_host: "127.0.0.1".to_string(),
        router_port: 9130,
        backend_port_base: 9120,
        auth_token: None,
        require_auth: false,
        max_body_bytes: 10, // Very small limit
        profile_config_path: None,
        backend_binary_path: None,
        evidence_path: None,
        log_path: None,
        health_timeout_secs: 1,
        health_poll_interval_secs: 1,
    };

    let state = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .method("POST")
        .uri("/backend/select")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("this is too long"))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    // Axum's DefaultBodyLimit returns 413 Payload Too Large
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}
