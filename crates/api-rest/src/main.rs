//! LLM Benchmark Gateway - REST API Server
//!
//! This is the main entrypoint for the llm-benchmark-api binary.
//! It creates and runs the Axum-based REST API server.

use llm_benchmark_api_rest::{create_app, ApiConfig};
use std::net::SocketAddr;
use tracing::{info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,llm_benchmark=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().json())
        .init();

    info!("Starting LLM Benchmark Gateway");
    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Service version"
    );

    // Load configuration from environment
    let config = ApiConfig::from_env().map_err(|e| {
        warn!(error = %e, "Failed to load configuration from environment");
        e
    })?;

    info!(
        host = %config.host,
        port = %config.port,
        "Configuration loaded"
    );

    // Create the application
    let app = create_app(config.clone()).await.map_err(|e| {
        warn!(error = %e, "Failed to create application");
        e
    })?;

    // Bind to address
    let addr = SocketAddr::new(
        config.host.parse().unwrap_or_else(|_| "0.0.0.0".parse().unwrap()),
        config.port,
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "Server listening");

    // Graceful shutdown handling
    let shutdown_signal = async {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C, initiating graceful shutdown");
            }
            _ = terminate => {
                info!("Received SIGTERM, initiating graceful shutdown");
            }
        }
    };

    // Run the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    info!("Server shutdown complete");
    Ok(())
}
