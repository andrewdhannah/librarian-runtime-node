//! ROUTER-RUST-CORE-1 — Minimal Rust router core for librarian-runtime-node.
//!
//! Preserves the Python router's HTTP contract and proves Windows process
//! lifecycle parity for llama.cpp backends.
//!
//! Usage:
//!     cargo run --release -- --port 9130
//!     cargo run --release -- --port 9130 --profiles <path-to-model-profiles.json>

mod config;
mod evidence;
mod process;
mod refusal;
mod server;

use crate::config::ProfileManager;
use crate::evidence::EvidenceWriter;
use crate::server::{build_router, AppState};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

/// Default port matching the Python router's convention.
const DEFAULT_PORT: u16 = 9130;

#[derive(Parser, Debug)]
#[command(name = "rust-router", version, about = "Minimal Rust router core for librarian-runtime-node")]
struct Args {
    /// Router HTTP port.
    #[arg(long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Path to model-profiles.json (overrides default sources).
    #[arg(long)]
    profiles: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .init();

    let args = Args::parse();

    // Define profile sources matching Python router
    let runtime_config = PathBuf::from(r"G:\openwork\librarian-runtime-node\config\model-profiles.json");
    let librarian_config = PathBuf::from(
        r"G:\openwork\thelibrarian\fixtures\windows-runtime-node\router\model-profiles.json",
    );

    let sources: Vec<&std::path::Path> = if let Some(ref custom) = args.profiles {
        vec![custom.as_path(), &runtime_config, &librarian_config]
    } else {
        vec![&runtime_config, &librarian_config]
    };

    // Load profiles
    let profile_manager = match ProfileManager::load_from_sources(&sources) {
        Ok(pm) => pm,
        Err(e) => {
            error!("Failed to load profiles: {}", e);
            std::process::exit(1);
        }
    };

    info!(
        "Loaded {} profiles: {}",
        profile_manager.len(),
        profile_manager.aliases().join(", ")
    );

    // Backends are created on-demand via /backend/select
    let backends = tokio::sync::Mutex::new(std::collections::HashMap::new());

    // Evidence writer
    let evidence_writer = EvidenceWriter::new();

    // Build app state
    let state = Arc::new(AppState {
        profile_manager,
        backends,
        evidence_writer,
        start_time: std::time::Instant::now(),
    });

    // Write startup evidence
    state.evidence_writer.write(
        "router-startup.json",
        &serde_json::json!({
            "status": "started",
            "port": args.port,
            "profiles_loaded": state.profile_manager.len(),
            "profiles": state.profile_manager.aliases(),
            "authority": "advisory_only",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    // Build router
    let app = build_router(state.clone());

    // Bind and serve
    let addr = format!("0.0.0.0:{}", args.port);
    let sep = "=".repeat(60);
    info!("{}", sep);
    info!("rust-router v0.1 (ROUTER-RUST-CORE-1)");
    info!("Listening on http://{}", addr);
    info!(
        "Profiles: {}",
        state.profile_manager.aliases().join(", ")
    );
    info!("Authority: advisory_only");
    info!("{}", sep);

    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    // Serve with graceful shutdown on ctrl-c
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(state))
        .await
        .unwrap_or_else(|e| {
            error!("Server error: {}", e);
        });
}

/// Handle graceful shutdown on Ctrl+C.
async fn shutdown_signal(state: Arc<AppState>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutting down... Cleaning up backends...");

    let backends = state.backends.lock().await;
    for (alias, bp) in backends.iter() {
        info!("Stopping backend '{}'...", alias);
        bp.stop().await;
    }

    info!("Shutdown complete.");
}
