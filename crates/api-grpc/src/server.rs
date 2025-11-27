//! gRPC Server implementation

use crate::error::GrpcResult;
use crate::interceptors::{AuthInterceptor, LoggingInterceptor, MetricsInterceptor};
use crate::proto::{
    benchmark_service_server::BenchmarkServiceServer,
    governance_service_server::GovernanceServiceServer,
    leaderboard_service_server::LeaderboardServiceServer,
    submission_service_server::SubmissionServiceServer,
    user_service_server::UserServiceServer,
};
use crate::services::{
    BenchmarkServiceImpl, GovernanceServiceImpl, LeaderboardServiceImpl, SubmissionServiceImpl,
    UserServiceImpl,
};
use std::net::SocketAddr;
use tonic::transport::{Server, ServerTlsConfig};
use tracing::{info, warn};

/// gRPC server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server bind address
    pub addr: SocketAddr,
    /// Enable TLS
    pub enable_tls: bool,
    /// TLS certificate path
    pub tls_cert_path: Option<String>,
    /// TLS key path
    pub tls_key_path: Option<String>,
    /// Enable gRPC reflection
    pub enable_reflection: bool,
    /// Enable health service
    pub enable_health: bool,
    /// Maximum concurrent streams
    pub max_concurrent_streams: Option<u32>,
    /// TCP keepalive duration
    pub tcp_keepalive: Option<std::time::Duration>,
    /// Request timeout
    pub timeout: Option<std::time::Duration>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:50051".parse().unwrap(),
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            enable_reflection: true,
            enable_health: true,
            max_concurrent_streams: Some(1000),
            tcp_keepalive: Some(std::time::Duration::from_secs(60)),
            timeout: Some(std::time::Duration::from_secs(30)),
        }
    }
}

/// gRPC server
pub struct GrpcServer {
    config: ServerConfig,
}

impl GrpcServer {
    /// Create a new gRPC server with the given configuration
    pub fn new(config: ServerConfig) -> Self {
        Self { config }
    }

    /// Start the gRPC server
    pub async fn serve(self) -> GrpcResult<()> {
        info!("Starting gRPC server on {}", self.config.addr);

        // Build server
        let mut server = Server::builder();

        // Configure TLS if enabled
        if self.config.enable_tls {
            if let (Some(cert_path), Some(key_path)) =
                (&self.config.tls_cert_path, &self.config.tls_key_path)
            {
                info!("Configuring TLS with cert: {}, key: {}", cert_path, key_path);
                let cert = std::fs::read_to_string(cert_path)
                    .map_err(|e| crate::error::GrpcError::Internal(format!("Failed to read TLS cert: {}", e)))?;
                let key = std::fs::read_to_string(key_path)
                    .map_err(|e| crate::error::GrpcError::Internal(format!("Failed to read TLS key: {}", e)))?;

                let tls_config = ServerTlsConfig::new()
                    .identity(tonic::transport::Identity::from_pem(&cert, &key));

                server = server
                    .tls_config(tls_config)
                    .map_err(|e| crate::error::GrpcError::Internal(format!("TLS config error: {}", e)))?;
            } else {
                warn!("TLS enabled but cert/key paths not provided, running without TLS");
            }
        }

        // Configure server settings
        if let Some(max_streams) = self.config.max_concurrent_streams {
            server = server.max_concurrent_streams(max_streams);
        }

        if let Some(keepalive) = self.config.tcp_keepalive {
            server = server.tcp_keepalive(Some(keepalive));
        }

        if let Some(timeout) = self.config.timeout {
            server = server.timeout(timeout);
        }

        // Create service interceptors
        let auth_interceptor = AuthInterceptor::new();
        let logging_interceptor = LoggingInterceptor::new();
        let metrics_interceptor = MetricsInterceptor::new();

        // Create service implementations
        let benchmark_service = BenchmarkServiceImpl::new();
        let submission_service = SubmissionServiceImpl::new();
        let leaderboard_service = LeaderboardServiceImpl::new();
        let governance_service = GovernanceServiceImpl::new();
        let user_service = UserServiceImpl::new();

        // Add services with interceptors
        let mut router = server
            .add_service(
                BenchmarkServiceServer::new(benchmark_service)
                    .max_decoding_message_size(64 * 1024 * 1024) // 64MB
                    .max_encoding_message_size(64 * 1024 * 1024),
            )
            .add_service(
                SubmissionServiceServer::new(submission_service)
                    .max_decoding_message_size(128 * 1024 * 1024) // 128MB for large submissions
                    .max_encoding_message_size(128 * 1024 * 1024),
            )
            .add_service(LeaderboardServiceServer::new(leaderboard_service))
            .add_service(GovernanceServiceServer::new(governance_service))
            .add_service(UserServiceServer::new(user_service));

        // Add reflection service if enabled
        #[cfg(feature = "reflection")]
        if self.config.enable_reflection {
            info!("Enabling gRPC reflection");
            let reflection_service = tonic_reflection::server::Builder::configure()
                .register_encoded_file_descriptor_set(include_bytes!("generated/descriptor.bin"))
                .build()
                .map_err(|e| crate::error::GrpcError::Internal(format!("Reflection service error: {}", e)))?;
            router = router.add_service(reflection_service);
        }

        // Add health service if enabled
        if self.config.enable_health {
            info!("Enabling health service");
            let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
            health_reporter
                .set_serving::<BenchmarkServiceServer<BenchmarkServiceImpl>>()
                .await;
            health_reporter
                .set_serving::<SubmissionServiceServer<SubmissionServiceImpl>>()
                .await;
            health_reporter
                .set_serving::<LeaderboardServiceServer<LeaderboardServiceImpl>>()
                .await;
            health_reporter
                .set_serving::<GovernanceServiceServer<GovernanceServiceImpl>>()
                .await;
            health_reporter
                .set_serving::<UserServiceServer<UserServiceImpl>>()
                .await;

            router = router.add_service(health_service);
        }

        info!("gRPC server listening on {}", self.config.addr);

        // Start server
        router
            .serve(self.config.addr)
            .await
            .map_err(|e| crate::error::GrpcError::Internal(format!("Server error: {}", e)))?;

        Ok(())
    }
}

impl Default for GrpcServer {
    fn default() -> Self {
        Self::new(ServerConfig::default())
    }
}
