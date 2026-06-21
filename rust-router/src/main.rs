//! ROUTER-RUST-HARDEN-1 — Hardened Rust router core for librarian-runtime-node.
//!
//! Preserves the Python router's HTTP contract and adds operational hardening.
//!
//! Usage:
//!     cargo run --release -- --port 9130
//!     cargo run --release -- --port 9130 --profiles <path-to-model-profiles.json>
//!     ROUTER_PORT=9130 cargo run --release
//! 

use rust_router::config::{ProfileManager, RouterConfig};
use rust_router::evidence::EvidenceWriter;
use rust_router::server::{build_router, AppState, start_health_poller, stop_health_poller};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::EnvFilter;

/// Default port matching the Python router's convention.
const DEFAULT_PORT: u16 = 9130;

#[derive(Parser, Debug)]
#[command(name = "rust-router", version, about = "Hardened Rust router core for librarian-runtime-node")]
struct Args {
    /// Router host.
    #[arg(long)]
    host: Option<String>,

    /// Router HTTP port.
    #[arg(long)]
    port: Option<u16>,

    /// Path to model-profiles.json (overrides default sources).
    #[arg(long)]
    profiles: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    // Load config first to initialize logging
    let config = RouterConfig::from_env();

    // Initialize logging with optional file output
    let writer: BoxMakeWriter = if let Some(ref log_path) = config.log_path {
        let file = std::fs::File::create(log_path).expect("Failed to create log file");
        BoxMakeWriter::new(file)
    } else {
        BoxMakeWriter::new(std::io::stdout)
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_writer(writer)
        .init();

    let args = Args::parse();

    // Load profiles
    let profile_manager = match ProfileManager::load_from_config(&config) {
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
        config: config.clone(),
        backends,
        evidence_writer,
        start_time: std::time::Instant::now(),
        health_poller_handle: tokio::sync::Mutex::new(None),
    });

    // Write startup evidence
    state.evidence_writer.write(
        "router-startup.json",
        &serde_json::json!({
            "status": "started",
            "port": args.port.unwrap_or(DEFAULT_PORT),
            "profiles_loaded": state.profile_manager.len(),
            "profiles": state.profile_manager.aliases(),
            "authority": "advisory_only",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );

    // Start background health poller
    start_health_poller(state.clone(), config.health_poll_interval_secs).await;

    // Build router
    let app = build_router(state.clone());

    // Bind and serve
    let host = args.host.unwrap_or_else(|| config.router_host.clone());
    let port = args.port.unwrap_or(config.router_port);
    let addr = format!("{}:{}", host, port);
    let sep = "=".repeat(60);
    info!("{}", sep);
    info!("rust-router v0.1 (ROUTER-RUST-HARDEN-1)");
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
        .with_graceful_shutdown(shutdown_signal(state.clone()))
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

    info!("Shutting down... Cleaning up backends and health poller...");

    // Stop health poller
    stop_health_poller(&state).await;

    let backends = state.backends.lock().await;
    for (alias, bp) in backends.iter() {
        info!("Stopping backend '{}'...", alias);
        bp.stop().await;
    }

    info!("Shutdown complete.");
}
