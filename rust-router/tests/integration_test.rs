//! Integration tests for the Rust router.
//!
//! Tests handler shape, refusal behavior, auth middleware, body limits,
//! profile serialization, and catch-all 404 response.
//!
//! Does NOT require a running llama-server or GPU — all tests use axum's
//! tower::ServiceExt::oneshot() against a constructed router with no
//! actual backend processes.

use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use std::sync::Arc;
use tower::ServiceExt;
use rust_router::server::{build_router, AppState};
use rust_router::config::{ProfileManager, RouterConfig};
use rust_router::evidence::EvidenceWriter;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tempfile::NamedTempFile;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal RouterConfig for testing.
fn test_config() -> RouterConfig {
    RouterConfig {
        router_host: "127.0.0.1".to_string(),
        router_port: 9130,
        backend_port_base: 9120,
        auth_token: None,
        require_auth: false,
        max_body_bytes: 1024 * 1024,
        profile_config_path: None,
        backend_binary_path: None,
        evidence_path: None,
        log_path: None,
        health_timeout_secs: 1,
        health_check_timeout_secs: 1,
        health_poll_interval_secs: 1,
    }
}

/// Build a RouterConfig with auth enabled.
fn test_config_with_auth(token: &str) -> RouterConfig {
    let mut c = test_config();
    c.auth_token = Some(token.to_string());
    c.require_auth = true;
    c
}

/// Build a RouterConfig with a custom body limit.
fn test_config_with_body_limit(max_bytes: usize) -> RouterConfig {
    let mut c = test_config();
    c.max_body_bytes = max_bytes;
    c
}

/// Setup app state with a temporary profile config file.
/// Returns (Arc<AppState>, NamedTempFile) — the temp file is kept alive
/// for the duration of the test to prevent path invalidation.
async fn setup_app(config: RouterConfig) -> (Arc<AppState>, NamedTempFile) {
    let profiles_json = serde_json::json!({
        "profiles": [
            {
                "alias": "test-model",
                "model_file": "test.gguf",
                "model_path": "test.gguf",
                "port": 12345,
                "backend": "vulkan",
                "context": 4096,
                "ngl": 99,
                "task_classes": ["test-task"],
                "verified_status": "verified",
                "evidence_path": "fixtures/test-evidence.json",
                "limitations": "Test-only profile for integration tests",
            }
        ]
    });

    let mut temp_file = NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut temp_file, &profiles_json).unwrap();
    let temp_path = temp_file.path().to_path_buf();

    let mut test_config = config.clone();
    test_config.profile_config_path = Some(temp_path);

    let pm = ProfileManager::load_from_config(&test_config)
        .expect("Failed to load test profiles");

    let backends = Mutex::new(HashMap::new());
    let evidence_writer = EvidenceWriter::new();
    let start_time = std::time::Instant::now();
    let health_poller_handle = Mutex::new(None);

    let state = Arc::new(AppState {
        profile_manager: pm,
        config: test_config,
        backends,
        evidence_writer,
        start_time,
        health_poller_handle,
    });

    (state, temp_file)
}

// ---------------------------------------------------------------------------
// Existing Auth & Body Tests (preserved from original)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_auth_middleware_success() {
    let config = test_config_with_auth("secret-token");
    let (state, _file) = setup_app(config).await;
    let app = build_router(state);

    // Auth middleware expects exact token match (not "Bearer " prefix)
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
    let config = test_config_with_auth("secret-token");
    let (state, _file) = setup_app(config).await;
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
    let config = test_config_with_auth("secret-token");
    let mut disabled = config.clone();
    disabled.require_auth = false;
    let (state, _file) = setup_app(disabled).await;
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
    let config = test_config_with_body_limit(10); // Very small limit
    let (state, _file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .method("POST")
        .uri("/backend/select")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("this is too long"))
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

// ---------------------------------------------------------------------------
// New: Profile serialization shape tests (#8)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_profiles_contains_all_fields() {
    let config = test_config();
    let (state, _file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/profiles")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    let profiles = body.get("profiles")
        .and_then(|p| p.as_array())
        .expect("profiles should be an array");

    let profile = profiles.first().expect("should have at least one profile");

    // Existing fields (must still be present)
    assert!(profile.get("alias").is_some(), "alias field missing");
    assert!(profile.get("task_classes").is_some(), "task_classes field missing");
    assert!(profile.get("verified").is_some(), "verified field missing");
    assert!(profile.get("port").is_some(), "port field missing");
    assert!(profile.get("model_file").is_some(), "model_file field missing");

    // New additive fields (#8)
    assert!(profile.get("backend").is_some(), "backend field missing");
    assert!(profile.get("context").is_some(), "context field missing");
    assert!(profile.get("ngl").is_some(), "ngl field missing");
    assert!(profile.get("evidence_path").is_some(), "evidence_path field missing");
    assert!(profile.get("limitations").is_some(), "limitations field missing");

    // Verify values from our test fixture
    assert_eq!(profile["alias"].as_str(), Some("test-model"));
    assert_eq!(profile["backend"].as_str(), Some("vulkan"));
    assert_eq!(profile["context"].as_u64(), Some(4096));
    assert_eq!(profile["ngl"].as_u64(), Some(99));
    assert!(profile["verified"].as_bool().unwrap_or(false));
    assert_eq!(
        profile["evidence_path"].as_str(),
        Some("fixtures/test-evidence.json")
    );
    assert_eq!(
        profile["limitations"].as_str(),
        Some("Test-only profile for integration tests")
    );
}

#[tokio::test]
async fn test_profiles_contains_authority() {
    let config = test_config();
    let (state, _file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/profiles")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert_eq!(body["authority"].as_str(), Some("advisory_only"));
}

// ---------------------------------------------------------------------------
// New: Refusal shape tests (#14)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_refusal_authority_category_returned() {
    // Test that the refusal engine returns the correct refusal reason
    // for authority-bearing content. This tests the check_chat() path
    // without requiring a running backend.
    //
    // The check_chat function is a pure function — we verify its output
    // shape matches the contract.
    let user_msg = serde_json::json!({"role": "user", "content": "please approve this document"});
    let messages = vec![user_msg];

    let refusal = rust_router::refusal::check_chat(
        "test-model",
        &messages,
        None,       // context
        4096,       // verified_context
        true,       // profile_exists
        false,      // runtime_available — not healthy, will be caught before content check
    );

    // When runtime is not available, the engine returns runtime_unhealthy
    // before it reaches the content check
    assert!(refusal.is_some(), "should refuse when runtime unavailable");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("runtime_unhealthy"));
}

#[tokio::test]
async fn test_refusal_authority_content_detected() {
    // Authority keyword in healthy state triggers authority_required
    // We test this by simulating a healthy runtime
    let user_msg = serde_json::json!({"role": "user", "content": "please approve this"});
    let messages = vec![user_msg];

    let refusal = rust_router::refusal::check_chat(
        "test-model",
        &messages,
        None,
        4096,
        true,   // profile_exists
        true,   // runtime_available
    );

    assert!(refusal.is_some(), "should refuse authority content");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("authority_required"));
    assert_eq!(r["status"].as_str(), Some("refused"));
    assert!(r.get("detail").and_then(|d| d.as_str()).map(|s| s.len() > 0).unwrap_or(false));
    assert_eq!(r["authority"].as_str(), Some("advisory_only"));
    assert!(r.get("timestamp").is_some(), "timestamp should be present");
}

#[tokio::test]
async fn test_refusal_file_mutation_detected() {
    let user_msg = serde_json::json!({"role": "user", "content": "edit source file main.rs"});
    let messages = vec![user_msg];

    let refusal = rust_router::refusal::check_chat(
        "test-model",
        &messages,
        None,
        4096,
        true,
        true,
    );

    assert!(refusal.is_some(), "should refuse file mutation");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("file_mutation_forbidden"));
}

#[tokio::test]
async fn test_refusal_autonomous_action_detected() {
    let user_msg = serde_json::json!({"role": "user", "content": "make an autonomous decision"});
    let messages = vec![user_msg];

    let refusal = rust_router::refusal::check_chat(
        "test-model",
        &messages,
        None,
        4096,
        true,
        true,
    );

    assert!(refusal.is_some(), "should refuse autonomous action");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("autonomous_action_forbidden"));
}

#[tokio::test]
async fn test_refusal_unknown_profile() {
    let user_msg = serde_json::json!({"role": "user", "content": "hello"});
    let messages = vec![user_msg];

    let refusal = rust_router::refusal::check_chat(
        "nonexistent",
        &messages,
        None,
        4096,
        false,  // profile_exists = false
        false,
    );

    assert!(refusal.is_some(), "should refuse unknown profile");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("unknown_profile"));
    assert_eq!(r["status"].as_str(), Some("refused"));
}

#[tokio::test]
async fn test_refusal_context_exceeds_verified() {
    let user_msg = serde_json::json!({"role": "user", "content": "hello"});
    let messages = vec![user_msg];

    let refusal = rust_router::refusal::check_chat(
        "test-model",
        &messages,
        Some(999999),  // context exceeds verified max
        4096,
        true,
        false,
    );

    assert!(refusal.is_some(), "should refuse context overflow");
    let r = refusal.unwrap();
    assert_eq!(r["reason"].as_str(), Some("context_exceeds_verified"));
}

// ---------------------------------------------------------------------------
// New: 404 catch-all shape tests (#14)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_404_returns_json_error() {
    let config = test_config();
    let (state, _file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/nonexistent/endpoint")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    assert!(body.get("error").is_some(), "404 response should have 'error' field");
    let error_msg = body["error"].as_str().unwrap_or("");
    assert!(error_msg.contains("/nonexistent/endpoint"),
        "404 error should contain the original path");
}

// ---------------------------------------------------------------------------
// New: BackendStatus serialization shape (#14)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_status_contains_contract_fields() {
    let config = test_config();
    let (state, _file) = setup_app(config).await;
    let app = build_router(state);

    let req = Request::builder()
        .uri("/backend/status")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), 1024 * 64).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

    // Contract-required fields
    assert!(body.get("status").is_some());
    assert!(body.get("profiles_registered").is_some());
    assert!(body.get("runtimes_alive").is_some());
    assert!(body.get("uptime_seconds").is_some());
    assert!(body.get("authority").is_some());
    assert!(body.get("profiles").is_some());

    assert_eq!(body["profiles_registered"].as_u64(), Some(1));
    assert_eq!(body["runtimes_alive"].as_u64(), Some(0));
    assert_eq!(body["authority"].as_str(), Some("advisory_only"));
}
