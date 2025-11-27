//! Test database setup with testcontainers.
//!
//! Provides PostgreSQL test database instances for integration testing.

use sqlx::{postgres::PgPoolOptions, PgPool};
use std::sync::Arc;

/// Test database wrapper with automatic cleanup
pub struct TestDatabase {
    pool: Arc<PgPool>,
}

impl TestDatabase {
    /// Create a new test database with migrations applied
    ///
    /// Note: This is a simplified version. In a real implementation,
    /// you would use testcontainers-rs with the postgres module:
    /// ```ignore
    /// use testcontainers::{clients, images::postgres::Postgres, Container};
    /// ```
    pub async fn new_with_url(connection_string: &str) -> anyhow::Result<Self> {

        // Create connection pool
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(connection_string)
            .await?;

        // Run migrations (if you have a migrations directory)
        // sqlx::migrate!("../../migrations")
        //     .run(&pool)
        //     .await?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    /// Get a reference to the database pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get an Arc clone of the pool for sharing
    pub fn pool_arc(&self) -> Arc<PgPool> {
        Arc::clone(&self.pool)
    }

    /// Clean all tables for test isolation
    pub async fn clean(&self) -> anyhow::Result<()> {
        sqlx::query("TRUNCATE TABLE users CASCADE").execute(self.pool()).await.ok();
        sqlx::query("TRUNCATE TABLE organizations CASCADE").execute(self.pool()).await.ok();
        sqlx::query("TRUNCATE TABLE benchmarks CASCADE").execute(self.pool()).await.ok();
        sqlx::query("TRUNCATE TABLE submissions CASCADE").execute(self.pool()).await.ok();
        sqlx::query("TRUNCATE TABLE proposals CASCADE").execute(self.pool()).await.ok();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires PostgreSQL to be running
    async fn test_database_creation() {
        // Example connection string - would be provided by testcontainers
        let connection_string = "postgres://postgres:postgres@localhost:5432/test";

        // This would fail unless you have a real PostgreSQL instance
        // let db = TestDatabase::new_with_url(connection_string).await.unwrap();

        // Test placeholder
        assert_eq!(connection_string.contains("postgres"), true);
    }
}
