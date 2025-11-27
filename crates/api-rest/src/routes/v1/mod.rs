//! API v1 routes.

use crate::state::AppState;
use axum::Router;

pub mod benchmarks;
pub mod governance;
pub mod leaderboards;
pub mod submissions;
pub mod users;

/// Create all v1 API routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(benchmarks::routes())
        .merge(submissions::routes())
        .merge(leaderboards::routes())
        .merge(governance::routes())
        .merge(users::routes())
}
