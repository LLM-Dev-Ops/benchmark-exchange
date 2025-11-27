//! Example gRPC server
//!
//! Run with:
//! cargo run --example server

use llm_benchmark_api_grpc::{GrpcServer, ServerConfig};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Configure server
    let config = ServerConfig {
        addr: "0.0.0.0:50051".parse()?,
        enable_tls: false,
        enable_reflection: true,
        enable_health: true,
        max_concurrent_streams: Some(1000),
        tcp_keepalive: Some(std::time::Duration::from_secs(60)),
        timeout: Some(std::time::Duration::from_secs(30)),
        ..Default::default()
    };

    tracing::info!("Starting gRPC server example on {}", config.addr);
    tracing::info!("gRPC reflection enabled - use grpcurl to explore");
    tracing::info!("Example: grpcurl -plaintext localhost:50051 list");

    // Start server
    let server = GrpcServer::new(config);
    server.serve().await?;

    Ok(())
}
