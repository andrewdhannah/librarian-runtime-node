//! HTTP server — contract endpoints for the Rust router.
//!
//! Preserves the same endpoint names and response shapes as the Python router:
//! - GET  /backend/status
//! - GET  /backend/profiles
//! - GET  /backend/health
//! - GET  /health
//! - POST /backend/select
//! - POST /backend/stop
//! - POST /backend/chat (internal) / POST /v1/chat/completions (OpenAI-compatible)
//!
//! Authority: advisory_only

use crate::config::ProfileManager;
use crate::evidence::EvidenceWriter;
use crate::process::{BackendProcess, BackendState};
use crate::refusal;
use axum::{
    extract::{State, DefaultBodyLimit},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::Json,
    routing::{get, post},
    Router,
    extract::Request,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};
// info used in evidence writes

/// Shared application state.
pub struct AppState {
    pub profile_manager: ProfileManager,
    pub config: crate::config::RouterConfig,
    pub backends: Mutex<HashMap<String, Arc<BackendProcess>>>,
    pub evidence_writer: EvidenceWriter,
    pub start_time: std::time::Instant,
    /// Background health poller handle (for graceful shutdown)
    pub health_poller_handle: Mutex<Option<JoinHandle<()>>>,
}

// ============================================================================
// Request/Response types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SelectRequest {
    pub profile: String,
    pub task_class: Option<String>,
    pub context: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct StopRequest {
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RestartRequest {
    pub profile: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub profile: String,
    pub messages: Option<Vec<Value>>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
    pub context: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct V1ChatRequest {
    pub model: Option<String>,
    pub messages: Option<Vec<Value>>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f64>,
}

// ============================================================================
// Middleware
// ============================================================================

async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    if state.config.require_auth {
        let auth_header = req.headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        match auth_header {
            Some(token) if Some(token) == state.config.auth_token.as_ref().map(|x| x.as_str()) => {
                Ok(next.run(req).await)
            }
            _ => {
                warn!("Unauthorized request attempt");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    } else {
        Ok(next.run(req).await)
    }
}

// ============================================================================
// GET /backend/status
// ============================================================================

async fn handle_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let backends = state.backends.lock().await;
    let mut profiles_status = json!({});
    let mut active_profile: Option<String> = None;
    let mut healthy_count = 0u32;

    for (alias, bp) in backends.iter() {
        let status = bp.get_status().await;
        if status.state == "healthy" {
            healthy_count += 1;
            if active_profile.is_none() {
                active_profile = Some(alias.clone());
            }
        }
        profiles_status[alias] = serde_json::to_value(&status).unwrap_or_default();
    }

    let overall = if healthy_count > 0 { "ok" } else { "degraded" };

    let response = json!({
        "status": overall,
        "active_profile": active_profile,
        "profiles_registered": state.profile_manager.len(),
        "runtimes_alive": healthy_count,
        "uptime_seconds": state.start_time.elapsed().as_secs(),
        "authority": "advisory_only",
        "profiles": profiles_status,
    });

    state.evidence_writer.write("status.json", &response);
    Json(response)
}

// ============================================================================
// GET /backend/profiles
// ============================================================================

async fn handle_profiles(State(state): State<Arc<AppState>>) -> Json<Value> {
    let response = json!({
        "profiles": state.profile_manager.list_all(),
        "authority": "advisory_only",
    });
    state.evidence_writer.write("profiles.json", &response);
    Json(response)
}

// ============================================================================
// GET /backend/health
// ============================================================================

async fn handle_health(State(state): State<Arc<AppState>>) -> Json<Value> {
    let backends = state.backends.lock().await;
    let mut profiles_health = json!({});
    let mut all_healthy = true;
    let mut active_profile: Option<String> = None;

    for (alias, bp) in backends.iter() {
        let s = bp.get_state().await;
        let h = bp.check_health().await;
        let health_status = if s == BackendState::Healthy { "ok" } else { "degraded" };
        if !h { all_healthy = false; }
        if s == BackendState::Healthy && active_profile.is_none() {
            active_profile = Some(alias.clone());
        }
        profiles_health[alias] = json!({
            "status": health_status,
            "state": s.as_str(),
            "identity_verified": s == BackendState::Healthy,
            "port": bp.profile.port,
        });
    }

    let response = json!({
        "status": if all_healthy { "ok" } else { "degraded" },
        "active_profile": active_profile,
        "profiles": profiles_health,
        "authority": "advisory_only",
    });
    state.evidence_writer.write("health.json", &response);
    Json(response)
}

// ============================================================================
// GET /health (legacy)
// ============================================================================

async fn handle_health_legacy(State(state): State<Arc<AppState>>) -> Json<Value> {
    // Get backend aliases without holding the lock across await
    let aliases: Vec<String> = {
        let backends = state.backends.lock().await;
        backends.keys().cloned().collect()
    };

    // Check each backend with brief health poll
    let mut active: Option<String> = None;
    for alias in &aliases {
        let bp = {
            let backends = state.backends.lock().await;
            backends.get(alias).cloned()
        };
        if let Some(bp) = bp {
            if bp.check_health().await && bp.get_state().await.is_healthy() {
                active = Some(alias.clone());
                break;
            }
        }
    }

    let response = json!({
        "status": if active.is_some() { "ok" } else { "degraded" },
        "router": "ok",
        "active_profile": active,
        "authority": "advisory_only",
    });
    Json(response)
}

// ============================================================================
// POST /backend/select
// ============================================================================

async fn handle_select(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<SelectRequest>,
) -> (StatusCode, Json<Value>) {
    let alias = &body.profile;
    let profile = state.profile_manager.get(alias);

    // Check refusal conditions
    if let Some(refusal) = refusal::check_select(
        alias,
        body.task_class.as_deref(),
        profile.is_some(),
        profile.map(|p| p.verified_status == "verified").unwrap_or(false),
        false, // runtime_failed is checked below
    ) {
        // Handle task_class case specially
        if refusal.get("reason") == Some(&json!("task_class_check_needed")) {
            // Check task classes from the profile
            if let Some(p) = profile {
                if let Some(ref tc) = body.task_class {
                    if !p.task_classes.contains(tc) {
                        let resp = json!({
                            "status": "refused",
                            "reason": "unknown_profile",
                            "detail": format!(
                                "Task class '{}' not declared for profile '{}'. Declared: {:?}",
                                tc, alias, p.task_classes
                            ),
                            "authority": "advisory_only",
                            "timestamp": Utc::now().to_rfc3339(),
                        });
                        state.evidence_writer.write("select-invalid.json", &resp);
                        return (StatusCode::FORBIDDEN, Json(resp));
                    }
                }
            }
        } else {
            state.evidence_writer.write("select-invalid.json", &refusal);
            return (StatusCode::FORBIDDEN, Json(refusal));
        }
    }

    // Get or create backend process
    let bp = {
        let mut backends = state.backends.lock().await;
        if let Some(existing) = backends.get(alias) {
            existing.clone()
        } else if let Some(profile_data) = state.profile_manager.get(alias) {
            let bp = Arc::new(BackendProcess::new(profile_data.clone(), state.config.clone()));
            backends.insert(alias.clone(), bp.clone());
            bp
        } else {
            let resp = json!({
                "status": "refused",
                "reason": "unknown_profile",
                "detail": format!("No profile registered with alias '{}'", alias),
                "authority": "advisory_only",
                "timestamp": Utc::now().to_rfc3339(),
            });
            state.evidence_writer.write("select-invalid.json", &resp);
            return (StatusCode::FORBIDDEN, Json(resp));
        }
    };

    // Start the backend if not running
    let current_state = bp.get_state().await;
    if current_state == BackendState::Stopped || current_state == BackendState::Failed {
        if let Err(e) = bp.start().await {
            let resp = json!({
                "status": "refused",
                "reason": "runtime_unhealthy",
                "detail": format!("Backend launch failed for '{}': {}", alias, e),
                "authority": "advisory_only",
            });
            state.evidence_writer.write("select-invalid.json", &resp);
            return (StatusCode::SERVICE_UNAVAILABLE, Json(resp));
        }
    }

    // Brief wait for health if still starting
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while std::time::Instant::now() < deadline && !bp.get_state().await.is_healthy() {
        bp.check_health().await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    let port = state.profile_manager.get(alias).map(|p| p.port).unwrap_or(0);
    let response = json!({
        "status": "selected",
        "profile": alias,
        "port": port,
        "authority": "advisory_only",
        "task_class": body.task_class,
    });
    state.evidence_writer.write("select-valid.json", &response);
    (StatusCode::OK, Json(response))
}

// ============================================================================
// POST /backend/stop
// ============================================================================

async fn handle_stop(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<StopRequest>,
) -> (StatusCode, Json<Value>) {
    let backends = state.backends.lock().await;

    let aliases_to_stop: Vec<String> = if let Some(ref profile) = body.profile {
        vec![profile.clone()]
    } else {
        backends.keys().cloned().collect()
    };

    if aliases_to_stop.is_empty() {
        let resp = json!({
            "status": "error",
            "detail": "No backends running",
        });
        return (StatusCode::BAD_REQUEST, Json(resp));
    }

    let mut stopped = Vec::new();
    let mut not_found = Vec::new();

    for alias in &aliases_to_stop {
        if let Some(bp) = backends.get(alias) {
            bp.stop().await;
            stopped.push(alias.clone());
        } else {
            not_found.push(alias.clone());
        }
    }

    let response = json!({
        "status": "stopped",
        "stopped": stopped,
        "not_found": not_found,
        "authority": "advisory_only",
    });
    state.evidence_writer.write("stop-result.json", &response);
    (StatusCode::OK, Json(response))
}

// ============================================================================
// POST /backend/restart
// ============================================================================

async fn handle_restart(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<RestartRequest>,
) -> (StatusCode, Json<Value>) {
    let alias = &body.profile;

    // Check if profile exists
    let profile = state.profile_manager.get(alias);
    if profile.is_none() {
        let resp = json!({
            "status": "refused",
            "reason": "unknown_profile",
            "detail": format!("No profile registered with alias '{}'", alias),
            "authority": "advisory_only",
            "timestamp": Utc::now().to_rfc3339(),
        });
        state.evidence_writer.write("restart-invalid.json", &resp);
        return (StatusCode::FORBIDDEN, Json(resp));
    }

    // Get the backend process
    let bp = {
        let backends = state.backends.lock().await;
        backends.get(alias).cloned()
    };

    let bp = match bp {
        Some(bp) => bp,
        None => {
            let resp = json!({
                "status": "refused",
                "reason": "runtime_unhealthy",
                "detail": format!("No runtime for profile '{}'. Select it first.", alias),
                "authority": "advisory_only",
                "timestamp": Utc::now().to_rfc3339(),
            });
            state.evidence_writer.write("restart-invalid.json", &resp);
            return (StatusCode::SERVICE_UNAVAILABLE, Json(resp));
        }
    };

    // Get old PID before restart
    let old_pid = bp.get_status().await.pid;

    // Perform restart (stop -> start -> wait healthy)
    let result = bp.restart().await;

    let new_pid = bp.get_status().await.pid;

    // Write process-before-after.txt (matching Python router)
    let timestamp = Utc::now().to_rfc3339();
    state.evidence_writer.write_text(
        "process-before-after.txt",
        &format!(
            "Before: PID={}\nAfter: PID={}\nProfile: {}\nTimestamp: {}\n",
            old_pid.unwrap_or(0),
            new_pid.unwrap_or(0),
            alias,
            timestamp,
        ),
    );

    match result {
        Ok(()) => {
            let response = json!({
                "status": "restarted",
                "profile": alias,
                "old_pid": old_pid,
                "new_pid": new_pid,
                "authority": "advisory_only",
            });
            state.evidence_writer.write("restart-result.json", &response);
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let resp = json!({
                "status": "failed",
                "profile": alias,
                "old_pid": old_pid,
                "new_pid": new_pid,
                "error": e,
                    "authority": "advisory_only",
            });
            state.evidence_writer.write("restart-result.json", &resp);
            (StatusCode::SERVICE_UNAVAILABLE, Json(resp))
        }
    }
}

// ============================================================================
// POST /backend/chat (internal router endpoint)
// ============================================================================

async fn handle_chat(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<ChatRequest>,
) -> (StatusCode, Json<Value>) {
    let alias = &body.profile;
    let messages = body.messages.unwrap_or_default();

    if alias.is_empty() {
        let resp = json!({"status": "error", "error": "Missing 'profile' field"});
        return (StatusCode::BAD_REQUEST, Json(resp));
    }

    if messages.is_empty() {
        let resp = json!({"status": "error", "error": "Missing 'messages' field"});
        return (StatusCode::BAD_REQUEST, Json(resp));
    }

    let profile = state.profile_manager.get(alias);
    let verified_context = profile.map(|p| p.context).unwrap_or(1024);

    // Check refusal
    let (is_healthy, bp_for_identity) = {
        let backends = state.backends.lock().await;
        match backends.get(alias) {
            Some(bp) => (bp.get_state().await.is_healthy(), Some(bp.clone())),
            None => (false, None),
        }
    };

    if let Some(refusal) = refusal::check_chat(
        alias,
        &messages,
        body.context,
        verified_context,
        profile.is_some(),
        is_healthy,
    ) {
        state.evidence_writer.write("chat-refusal-authority.json", &refusal);
        return (StatusCode::FORBIDDEN, Json(refusal));
    }

    // Identity verification: if backend is healthy, verify model identity
    // before proxying (matches Python router's verify_identity flow)
    if let Some(ref bp) = bp_for_identity {
        if is_healthy {
            let (identity_ok, identity_detail) = bp.verify_identity().await;
            if !identity_ok {
                let resp = json!({
                    "status": "refused",
                    "reason": "identity_mismatch",
                    "detail": format!("Identity verification failed: {}", identity_detail),
                    "profile": alias,
                    "authority": "advisory_only",
                    "timestamp": Utc::now().to_rfc3339(),
                });
                state.evidence_writer.write("chat-refusal-authority.json", &resp);
                return (StatusCode::FORBIDDEN, Json(resp));
            }
        }
    }

    // Proxy to backend
    let max_tokens = body.max_tokens.unwrap_or(256);
    let temperature = body.temperature.unwrap_or(0.7);

    let bp = {
        let backends = state.backends.lock().await;
        backends.get(alias).cloned()
    };

    match bp {
        Some(process) => {
            match process.proxy_chat(&messages, max_tokens, temperature).await {
                Ok(backend_response) => {
                    // Extract content from OpenAI-compatible response
                    let choices = backend_response.get("choices").and_then(|c| c.as_array());
                    let content = choices
                        .and_then(|c| c.first())
                        .and_then(|c| c.get("message"))
                        .and_then(|m| m.get("content"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("");
                    let finish_reason = choices
                        .and_then(|c| c.first())
                        .and_then(|c| c.get("finish_reason"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("stop");

                    let response = json!({
                        "status": "ok",
                        "content": content,
                        "finish_reason": finish_reason,
                        "profile": alias,
                        "authority": "advisory_only",
                    });
                    state.evidence_writer.write("chat-valid.json", &response);
                    (StatusCode::OK, Json(response))
                }
                Err(e) => {
                    let resp = json!({
                        "status": "error",
                        "error": e,
                        "profile": alias,
                        "authority": "advisory_only",
                    });
                    (StatusCode::BAD_GATEWAY, Json(resp))
                }
            }
        }
        None => {
            let resp = json!({
                "status": "refused",
                "reason": "runtime_unhealthy",
                "detail": format!("No runtime for profile '{}'. Select it first.", alias),
                "authority": "advisory_only",
            });
            (StatusCode::SERVICE_UNAVAILABLE, Json(resp))
        }
    }
}

// ============================================================================
// POST /v1/chat/completions (OpenAI-compatible endpoint)
// ============================================================================

async fn handle_v1_chat(
    State(state): State<Arc<AppState>>,
    axum::Json(body): axum::Json<V1ChatRequest>,
) -> (StatusCode, Json<Value>) {
    let model = body.model.clone().unwrap_or_default();

    // Find the target backend process
    let target_bp: Option<Arc<BackendProcess>> = {
        let backends = state.backends.lock().await;

        if !model.is_empty() {
            // Try exact match with model alias
            backends.get(&model).cloned()
        } else {
            // Return the first backend (any)
            backends.values().next().cloned()
        }
    };

    let process = match target_bp {
        Some(p) => p,
        None => {
            let resp = json!({
                "error": "No active backend. Use /backend/select first.",
                "authority": "advisory_only",
            });
            return (StatusCode::SERVICE_UNAVAILABLE, Json(resp));
        }
    };

    // Identity verification before proxying
    let backend_state = process.get_state().await;
    if backend_state.is_healthy() {
        let (identity_ok, identity_detail) = process.verify_identity().await;
        if !identity_ok {
            let resp = json!({
                "error": format!("Identity verification failed: {}", identity_detail),
                "authority": "advisory_only",
            });
            return (StatusCode::FORBIDDEN, Json(resp));
        }
    }

    let messages = body.messages.unwrap_or_default();
    if messages.is_empty() {
        let resp = json!({"error": "Missing 'messages' field"});
        return (StatusCode::BAD_REQUEST, Json(resp));
    }

    let max_tokens = body.max_tokens.unwrap_or(256);
    let temperature = body.temperature.unwrap_or(0.7);

    match process.proxy_chat(&messages, max_tokens, temperature).await {
        Ok(backend_response) => (StatusCode::OK, Json(backend_response)),
        Err(e) => {
            let resp = json!({"error": e});
            (StatusCode::BAD_GATEWAY, Json(resp))
        }
    }
}

// ============================================================================
// GET /v1/models (OpenAI-compatible identity endpoint)
// ============================================================================

async fn handle_v1_models(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    // Return configured available profiles as OpenAI-compatible models list
    // Does not expose local model file paths
    let models: Vec<Value> = state.profile_manager.iter().map(|p| {
        json!({
            "id": p.alias,
            "object": "model",
            "created": chrono::Utc::now().timestamp(),
            "owned_by": "librarian-runtime-node",
            "permission": [],
            "root": p.alias,
            "parent": null,
        })
    }).collect();

    let response = json!({
        "object": "list",
        "data": models,
        "authority": "advisory_only",
    });
    state.evidence_writer.write("v1-models.json", &response);
    Json(response)
}

// ============================================================================
// Router construction
// ============================================================================

/// Build the axum Router with all contract endpoints.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/backend/status", get(handle_status))
        .route("/backend/profiles", get(handle_profiles))
        .route("/backend/health", get(handle_health))
        .route("/health", get(handle_health_legacy))
        .route("/backend/select", post(handle_select))
        .route("/backend/stop", post(handle_stop))
        .route("/backend/restart", post(handle_restart))
        .route("/backend/chat", post(handle_chat))
        .route("/v1/chat/completions", post(handle_v1_chat))
        .route("/v1/models", get(handle_v1_models))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(DefaultBodyLimit::max(state.config.max_body_bytes))
        .layer(
            tower_http::cors::CorsLayer::permissive()
        )
        .fallback(handle_404)
        .with_state(state)
}

/// 404 catch-all handler matching Python router's JSON error response shape.
async fn handle_404(req: axum::extract::Request) -> (StatusCode, Json<Value>) {
    let path = req.uri().path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or(req.uri().path());
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": format!("Not found: {}", path),
        })),
    )
}

/// Start the background health poller.
/// Polls all running backends at the given interval and updates their state.
/// Does NOT auto-restart backends - only updates state to degraded/failed.
pub async fn start_health_poller(state: Arc<AppState>, interval_secs: u64) {
    let state_for_spawn = state.clone();
    let handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        info!("Health poller started (interval: {}s)", interval_secs);

        loop {
            interval.tick().await;

            // Get list of backend aliases to check
            let aliases: Vec<String> = {
                let backends = state_for_spawn.backends.lock().await;
                backends.keys().cloned().collect()
            };

            for alias in aliases {
                let bp = {
                    let backends = state_for_spawn.backends.lock().await;
                    backends.get(&alias).cloned()
                };

                if let Some(bp) = bp {
                    let state_val = bp.get_state().await;
                    // Only poll if not stopped/failed
                    if state_val != BackendState::Stopped && state_val != BackendState::Failed {
                        let healthy = bp.check_health().await;
                        if !healthy {
                            let new_state = bp.get_state().await;
                            if new_state == BackendState::Degraded {
                                warn!("[{}] health poller: backend degraded", alias);
                            } else if new_state == BackendState::Failed {
                                warn!("[{}] health poller: backend failed", alias);
                            }
                        }
                    }
                }
            }
        }
    });

    // Store the handle for graceful shutdown
    let mut handle_guard = state.health_poller_handle.lock().await;
    *handle_guard = Some(handle);
}

/// Stop the background health poller.
pub async fn stop_health_poller(state: &Arc<AppState>) {
    let mut handle_guard = state.health_poller_handle.lock().await;
    if let Some(handle) = handle_guard.take() {
        handle.abort();
        info!("Health poller stopped");
    }
}
