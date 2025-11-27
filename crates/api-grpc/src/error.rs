//! Error module

use llm_benchmark_application::ApplicationError;
use thiserror::Error;
use tonic::{Code, Status};

#[derive(Debug, Error)]
pub enum GrpcError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Application error: {0}")]
    Application(#[from] ApplicationError),
}

impl From<GrpcError> for Status {
    fn from(err: GrpcError) -> Self {
        match err {
            GrpcError::NotFound(msg) => Status::new(Code::NotFound, msg),
            GrpcError::InvalidArgument(msg) => Status::new(Code::InvalidArgument, msg),
            GrpcError::Unauthorized(msg) => Status::new(Code::Unauthenticated, msg),
            GrpcError::PermissionDenied(msg) => Status::new(Code::PermissionDenied, msg),
            GrpcError::AlreadyExists(msg) => Status::new(Code::AlreadyExists, msg),
            GrpcError::Internal(msg) => Status::new(Code::Internal, msg),
            GrpcError::ServiceUnavailable(msg) => Status::new(Code::Unavailable, msg),
            GrpcError::Application(app_err) => match app_err {
                ApplicationError::NotFound(msg) => Status::new(Code::NotFound, msg),
                ApplicationError::Unauthorized(msg) => Status::new(Code::Unauthenticated, msg),
                ApplicationError::Forbidden(msg) => Status::new(Code::PermissionDenied, msg),
                ApplicationError::InvalidInput(msg) => Status::new(Code::InvalidArgument, msg),
                ApplicationError::ValidationFailed(msg) => Status::new(Code::InvalidArgument, msg),
                ApplicationError::Conflict(msg) => Status::new(Code::AlreadyExists, msg),
                ApplicationError::Internal(msg) => Status::new(Code::Internal, msg),
                ApplicationError::ServiceUnavailable(msg) => Status::new(Code::Unavailable, msg),
                ApplicationError::RateLimitExceeded(msg) => Status::new(Code::ResourceExhausted, msg),
                ApplicationError::Timeout(msg) => Status::new(Code::DeadlineExceeded, msg),
            },
        }
    }
}

pub type GrpcResult<T> = Result<T, GrpcError>;
