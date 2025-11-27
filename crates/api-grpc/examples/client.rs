//! Example gRPC client
//!
//! Run with:
//! cargo run --example client

use llm_benchmark_api_grpc::proto::{
    benchmark_service_client::BenchmarkServiceClient, BenchmarkCategory, BenchmarkMetadata,
    BenchmarkStatus, CreateBenchmarkRequest, ListBenchmarksRequest,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to server
    let mut client = BenchmarkServiceClient::connect("http://localhost:50051").await?;

    println!("Connected to gRPC server\n");

    // Example 1: Create a benchmark
    println!("=== Creating Benchmark ===");
    let create_request = tonic::Request::new(CreateBenchmarkRequest {
        metadata: Some(BenchmarkMetadata {
            name: "Example Benchmark".to_string(),
            slug: "example-benchmark".to_string(),
            description: "An example benchmark created via gRPC".to_string(),
            long_description: None,
            tags: vec!["example".to_string(), "test".to_string()],
            license: 1, // MIT
            custom_license: None,
            citation: None,
            documentation_url: None,
            source_url: None,
            maintainer_ids: vec![],
        }),
        category: BenchmarkCategory::Performance as i32,
        version: "1.0.0".to_string(),
    });

    match client.create_benchmark(create_request).await {
        Ok(response) => {
            let benchmark = response.into_inner().benchmark.unwrap();
            println!("Created benchmark: {} (ID: {})", benchmark.metadata.as_ref().map(|m| &m.name).unwrap_or(&"".to_string()), benchmark.id);
        }
        Err(e) => println!("Error creating benchmark: {}", e),
    }

    println!();

    // Example 2: List benchmarks
    println!("=== Listing Benchmarks ===");
    let list_request = tonic::Request::new(ListBenchmarksRequest {
        category: BenchmarkCategory::Unspecified as i32,
        status: BenchmarkStatus::Unspecified as i32,
        tags: vec![],
        search_query: None,
        page: 1,
        page_size: 10,
        sort_by: "created_at".to_string(),
        sort_desc: true,
    });

    match client.list_benchmarks(list_request).await {
        Ok(response) => {
            let list_response = response.into_inner();
            println!(
                "Found {} benchmarks (showing {} on page {})",
                list_response.total_count, list_response.benchmarks.len(), list_response.page
            );
            for benchmark in list_response.benchmarks {
                println!(
                    "  - {} (v{}) - {:?}",
                    benchmark.metadata.as_ref().map(|m| &m.name).unwrap_or(&"".to_string()),
                    benchmark.version,
                    benchmark.status
                );
            }
        }
        Err(e) => println!("Error listing benchmarks: {}", e),
    }

    Ok(())
}
