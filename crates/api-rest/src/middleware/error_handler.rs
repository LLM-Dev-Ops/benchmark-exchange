//! Error handling middleware.

use crate::error::{ApiError, ErrorResponse};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use std::panic::Location;
use tracing::error;

/// Handle errors and convert them to proper HTTP responses
pub async fn handle_error(error: ApiError) -> Response {
    let status = error.status_code();
    let error_code = error.error_code();
    let message = error.to_string();

    // Log the error
    error!(
        error_code = error_code,
        message = %message,
        "Request error"
    );

    let body = ErrorResponse::new(error_code, message);

    (status, Json(body)).into_response()
}

/// Handle panics and convert to 500 errors
pub fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown panic".to_string()
    };

    error!(details = %details, "Handler panicked");

    let body = ErrorResponse::new(
        "INTERNAL_ERROR",
        "An internal error occurred",
    );

    (StatusCode::INTERNAL_SERVER_ERROR, Json(body)).into_response()
}
