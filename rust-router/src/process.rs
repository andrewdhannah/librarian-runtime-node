//! Backend process manager — launches, monitors, and stops llama.cpp backends.
//!
//! Each profile gets its own `BackendProcess` that manages the child
//! `llama-server.exe` process lifecycle. The router holds a map of these,
//! keyed by profile alias.

use crate::config::Profile;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{error, info, warn};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Windows: CREATE_NO_WINDOW = 0x08000000 to avoid console window for backend.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(not(target_os = "windows"))]
const CREATE_NO_WINDOW: u32 = 0;

/// Timeout for backend startup health check (matching Python router: 180s).
const BACKEND_START_TIMEOUT: Duration = Duration::from_secs(180);
/// Timeout for individual health poll requests.
const HEALTH_POLL_TIMEOUT: Duration = Duration::from_secs(3);
/// Timeout for backend chat requests.
const BACKEND_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendState {
    Stopped,
    Starting,
    Healthy,
    Degraded,
    Failed,
}

impl BackendState {
    pub fn is_healthy(&self) -> bool {
        matches!(self, BackendState::Healthy)
    }

    pub fn is_available_for_chat(&self) -> bool {
        matches!(self, BackendState::Healthy)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            BackendState::Stopped => "stopped",
            BackendState::Starting => "starting",
            BackendState::Healthy => "healthy",
            BackendState::Degraded => "degraded",
            BackendState::Failed => "failed",
        }
    }
}

/// Runtime status of a single backend process.
#[derive(Debug, Serialize)]
pub struct BackendStatus {
    pub alias: String,
    pub state: String,
    pub pid: Option<u32>,
    pub port: u16,
    pub uptime_seconds: Option<u64>,
    pub health_fail_count: u32,
    pub error: Option<String>,
}

/// Manages a single `llama-server.exe` child process.
pub struct BackendProcess {
    pub alias: String,
    pub profile: Profile,
    child: Arc<Mutex<Option<Child>>>,
    pub state: Arc<Mutex<BackendState>>,
    pub start_time: Arc<Mutex<Option<Instant>>>,
    pub health_fail_count: Arc<Mutex<u32>>,
    pub error_message: Arc<Mutex<Option<String>>>,
}

impl BackendProcess {
    pub fn new(profile: Profile) -> Self {
        BackendProcess {
            alias: profile.alias.clone(),
            profile,
            child: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(BackendState::Stopped)),
            start_time: Arc::new(Mutex::new(None)),
            health_fail_count: Arc::new(Mutex::new(0)),
            error_message: Arc::new(Mutex::new(None)),
        }
    }

    /// State accessors for lock-free reads in response handlers.
    pub async fn get_state(&self) -> BackendState {
        *self.state.lock().await
    }

    pub async fn get_status(&self) -> BackendStatus {
        let state = *self.state.lock().await;
        let pid = {
            let guard = self.child.lock().await;
            guard.as_ref().and_then(|c| c.id())
        };
        let start_time = *self.start_time.lock().await;
        let uptime = start_time.map(|t| t.elapsed().as_secs());
        let health_fail_count = *self.health_fail_count.lock().await;
        let error = self.error_message.lock().await.clone();

        BackendStatus {
            alias: self.alias.clone(),
            state: state.as_str().to_string(),
            pid,
            port: self.profile.port,
            uptime_seconds: uptime,
            health_fail_count,
            error,
        }
    }

    /// Start the backend process and wait for it to become healthy.
    pub async fn start(&self) -> Result<(), String> {
        let mut state = self.state.lock().await;
        let mut child_guard = self.child.lock().await;
        let mut start_time_guard = self.start_time.lock().await;

        // Check if already running
        if let Some(child) = child_guard.as_mut() {
            match child.try_wait() {
                Ok(Some(_)) => { /* exited, will restart */ }
                Ok(None) => {
                    info!("[{}] already running", self.alias);
                    return Ok(());
                }
                Err(_) => {}
            }
        }

        *state = BackendState::Starting;
        *self.error_message.lock().await = None;
        *self.health_fail_count.lock().await = 0;

        let binary = r"G:\openwork\librarian-runtime-node\runtime\llama.cpp\llama-server.exe";
        if !std::path::Path::new(binary).exists() {
            *state = BackendState::Failed;
            let msg = format!("Binary not found: {}", binary);
            *self.error_message.lock().await = Some(msg.clone());
            error!("[{}] {}", self.alias, msg);
            return Err(msg);
        }

        let cmd_str = format!(
            "{} -m \"{}\" -p {} -c {} -ngl {} -n 512 --alias \"{}\"",
            binary,
            self.profile.model_path,
            self.profile.port,
            self.profile.context,
            self.profile.ngl,
            self.alias,
        );
        info!("[{}] launching: {}", self.alias, cmd_str);

        let log_path = format!("backend_{}.log", self.alias);
        let log_file = std::fs::File::create(&log_path)
            .map_err(|e| format!("Failed to create log file '{}': {}", log_path, e))?;

        let child = Command::new(binary)
            .arg("-m")
            .arg(&self.profile.model_path)
            .arg("-p")
            .arg(self.profile.port.to_string())
            .arg("-c")
            .arg(self.profile.context.to_string())
            .arg("-ngl")
            .arg(self.profile.ngl.to_string())
            .arg("-n")
            .arg("512")
            .arg("--alias")
            .arg(&self.alias)
            .stdout(log_file)
            .stderr(std::process::Stdio::inherit())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map_err(|e| format!("Failed to spawn backend: {}", e))?;

        let pid = child.id().unwrap_or(0);
        info!("[{}] started (PID {}, port {})", self.alias, pid, self.profile.port);
        *child_guard = Some(child);
        *start_time_guard = Some(Instant::now());

        // Drop locks before the wait loop
        drop(child_guard);
        drop(start_time_guard);
        drop(state);

        // Wait for health
        let deadline = Instant::now() + BACKEND_START_TIMEOUT;
        let mut last_log = Instant::now();

        while Instant::now() < deadline {
            // Check if process exited
            {
                let mut guard = self.child.lock().await;
                if let Some(child) = guard.as_mut() {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            *self.state.lock().await = BackendState::Failed;
                            let msg = format!("Process exited (code {})", status);
                            *self.error_message.lock().await = Some(msg.clone());
                            error!("[{}] {}", self.alias, msg);
                            return Err(msg);
                        }
                        _ => {}
                    }
                }
            }

            if self.check_health().await {
                *self.state.lock().await = BackendState::Healthy;
                let elapsed = self.start_time.lock().await
                    .map(|t| t.elapsed().as_secs_f64())
                    .unwrap_or(0.0);
                info!("[{}] healthy after {:.1}s", self.alias, elapsed);
                return Ok(());
            }

            if last_log.elapsed() > Duration::from_secs(10) {
                info!("[{}] waiting for health...", self.alias);
                last_log = Instant::now();
            }

            sleep(Duration::from_secs(2)).await;
        }

        *self.state.lock().await = BackendState::Failed;
        let msg = format!("Timed out after {}s waiting for health", BACKEND_START_TIMEOUT.as_secs());
        *self.error_message.lock().await = Some(msg.clone());
        error!("[{}] {}", self.alias, msg);
        Err(msg)
    }

    /// Stop the backend process.
    pub async fn stop(&self) {
        let mut child_guard = self.child.lock().await;
        let mut state = self.state.lock().await;

        if let Some(child) = child_guard.as_mut() {
            let pid = child.id().unwrap_or(0);
            info!("[{}] stopping (PID {})", self.alias, pid);

            // Try graceful terminate first
            match child.start_kill() {
                Ok(()) => {
                    // Wait a bit for graceful shutdown
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                Err(e) => {
                    warn!("[{}] kill failed: {}", self.alias, e);
                }
            }
        }

        *state = BackendState::Stopped;
        *child_guard = None;
        info!("[{}] stopped", self.alias);
    }

    /// Poll the backend health endpoint. Returns true if healthy.
    pub async fn check_health(&self) -> bool {
        let url = format!("http://127.0.0.1:{}/health", self.profile.port);

        match reqwest::Client::builder()
            .timeout(HEALTH_POLL_TIMEOUT)
            .build()
            .ok()
        {
            Some(client) => {
                match client.get(&url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let body: serde_json::Value = resp.json().await.unwrap_or_default();
                        let status_str = body.get("status").and_then(|v| v.as_str()).unwrap_or("");
                        if status_str == "ok" {
                            *self.health_fail_count.lock().await = 0;
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            None => {}
        }

        // Increment fail count
        let mut count = self.health_fail_count.lock().await;
        *count += 1;
        if *count >= 3 {
            let mut state = self.state.lock().await;
            if *state == BackendState::Healthy {
                *state = BackendState::Degraded;
                *self.error_message.lock().await = Some("Consecutive health check failures".to_string());
                warn!("[{}] degraded after {} health failures", self.alias, *count);
            }
        }
        false
    }

    /// Proxy a chat request to the backend's OpenAI-compatible endpoint.
    pub async fn proxy_chat(
        &self,
        messages: &[serde_json::Value],
        max_tokens: u32,
        temperature: f64,
    ) -> Result<serde_json::Value, String> {
        let url = format!("http://127.0.0.1:{}/v1/chat/completions", self.profile.port);

        let body = serde_json::json!({
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": false,
        });

        let client = reqwest::Client::builder()
            .timeout(BACKEND_REQUEST_TIMEOUT)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Backend request failed: {}", e))?;

        let status = resp.status();
        let response_body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse backend response: {}", e))?;

        if !status.is_success() {
            return Err(format!("Backend returned {}: {}", status, response_body));
        }

        Ok(response_body)
    }
}
