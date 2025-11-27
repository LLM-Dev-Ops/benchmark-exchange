//! Simple server example for the LLM Benchmark Exchange REST API.
//!
//! This example demonstrates how to start the API server with default configuration.
//!
//! Run with:
//! ```bash
//! cargo run --example simple_server
//! ```

use llm_benchmark_api_rest::{create_app, ApiConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration from environment or use defaults
    let config = ApiConfig::from_env().unwrap_or_default();

    println!("Starting LLM Benchmark Exchange API server...");
    println!("Configuration:");
    println!("  Server: {}", config.server_address());
    println!("  CORS: {:?}", config.cors_allowed_origins);
    println!("  Rate limit: {} req/min", config.rate_limit_per_minute);
    println!("  Swagger UI: {}", config.enable_swagger);

    // Create the application
    let app = create_app(config.clone()).await?;

    // Bind to address
    let listener = tokio::net::TcpListener::bind(&config.server_address()).await?;

    println!("\nServer ready!");
    println!("  Health: http://{}/health", config.server_address());

    if config.enable_swagger {
        println!(
            "  Swagger UI: http://{}/swagger-ui",
            config.server_address()
        );
    }

    println!("\nPress Ctrl+C to stop");

    // Start serving
    axum::serve(listener, app).await?;

    Ok(())
}
