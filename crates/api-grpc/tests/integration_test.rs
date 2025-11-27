//! Integration tests for gRPC API
//!
//! Note: These tests require a running server and are marked with #[ignore].
//! Run with: cargo test --test integration_test -- --ignored

use llm_benchmark_api_grpc::{GrpcServer, ServerConfig};
use std::net::SocketAddr;

/// Helper to start test server
async fn _start_test_server(addr: SocketAddr) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let config = ServerConfig {
            addr,
            enable_tls: false,
            enable_reflection: false,
            enable_health: true,
            ..Default::default()
        };

        let server = GrpcServer::new(config);
        let _ = server.serve().await;
    })
}

#[tokio::test]
#[ignore]
async fn test_list_benchmarks() {
    use llm_benchmark_api_grpc::proto::{
        benchmark_service_client::BenchmarkServiceClient, ListBenchmarksRequest,
    };

    // Start server
    let addr: SocketAddr = "127.0.0.1:50052".parse().unwrap();
    let _server_handle = _start_test_server(addr).await;

    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect client
    let mut client = BenchmarkServiceClient::connect(format!("http://{}", addr))
        .await
        .expect("Failed to connect to test server");

    // Make request
    let request = tonic::Request::new(ListBenchmarksRequest {
        page: 1,
        page_size: 10,
        ..Default::default()
    });

    let response = client
        .list_benchmarks(request)
        .await
        .expect("Failed to list benchmarks");

    let list_response = response.into_inner();
    assert_eq!(list_response.page, 1);
    assert_eq!(list_response.page_size, 10);
}

#[tokio::test]
#[ignore]
async fn test_health_check() {
    use tonic_health::pb::health_client::HealthClient;
    use tonic::transport::Channel;

    // Start server
    let addr: SocketAddr = "127.0.0.1:50053".parse().unwrap();
    let _server_handle = _start_test_server(addr).await;

    // Wait for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Connect health client using Channel
    let channel = Channel::from_shared(format!("http://{}", addr))
        .expect("Failed to parse URI")
        .connect()
        .await
        .expect("Failed to connect");

    let mut client = HealthClient::new(channel);

    // Check health
    let request = tonic::Request::new(tonic_health::pb::HealthCheckRequest {
        service: "".to_string(),
    });

    let response = client.check(request).await.expect("Health check failed");

    assert_eq!(
        response.into_inner().status,
        tonic_health::pb::health_check_response::ServingStatus::Serving as i32
    );
}

#[tokio::test]
#[ignore]
async fn test_server_config_defaults() {
    let config = ServerConfig::default();

    // Verify sensible defaults
    assert!(config.enable_health, "Health should be enabled by default");
    assert!(!config.enable_tls, "TLS should be disabled by default");
}

#[test]
fn test_config_creation() {
    let addr: SocketAddr = "127.0.0.1:50051".parse().unwrap();
    let config = ServerConfig {
        addr,
        enable_tls: false,
        enable_reflection: false,
        enable_health: true,
        ..Default::default()
    };

    assert_eq!(config.addr, addr);
    assert!(!config.enable_tls);
    assert!(!config.enable_reflection);
    assert!(config.enable_health);
}
