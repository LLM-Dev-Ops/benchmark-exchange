//! Submission repository implementation.
//!
//! PostgreSQL-backed implementation for submission persistence operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use tracing::{debug, instrument};
use uuid::Uuid;

use llm_benchmark_common::pagination::{PaginatedResult, PaginationParams, SortDirection, SortParams};
use llm_benchmark_domain::{
    identifiers::{BenchmarkId, BenchmarkVersionId, ModelId, OrganizationId, SubmissionId, UserId, VerificationId},
    submission::{
        ConfidenceInterval, ExecutionMetadata, MetricScore, ModelInfo, StatisticalSignificance,
        Submission, SubmissionResults, SubmissionVisibility, SubmitterInfo, TestCaseResult,
        VerificationLevel, VerificationStatus,
    },
};

use crate::{Error, Result};

/// Query parameters for submission searches.
#[derive(Debug, Clone, Default)]
pub struct SubmissionQuery {
    pub benchmark_id: Option<BenchmarkId>,
    pub model_provider: Option<String>,
    pub model_name: Option<String>,
    pub user_id: Option<UserId>,
    pub organization_id: Option<OrganizationId>,
    pub verification_level: Option<VerificationLevel>,
    pub visibility: Option<SubmissionVisibility>,
    pub min_score: Option<f64>,
    pub max_score: Option<f64>,
    pub pagination: PaginationParams,
    pub sort: SortParams,
}

/// Leaderboard entry for benchmark rankings.
#[derive(Debug, Clone)]
pub struct LeaderboardEntry {
    pub submission_id: SubmissionId,
    pub rank: u32,
    pub model_info: ModelInfo,
    pub aggregate_score: f64,
    pub verification_level: VerificationLevel,
    pub submitted_at: DateTime<Utc>,
    pub submitter_name: Option<String>,
    pub organization_name: Option<String>,
}

/// Repository trait for submission operations.
#[async_trait]
pub trait SubmissionRepository: Send + Sync {
    /// Create a new submission.
    async fn create(&self, submission: &Submission) -> Result<SubmissionId>;

    /// Get a submission by its ID.
    async fn get_by_id(&self, id: SubmissionId) -> Result<Option<Submission>>;

    /// List submissions with filtering and pagination.
    async fn list(&self, query: SubmissionQuery) -> Result<PaginatedResult<Submission>>;

    /// Get submissions for a specific benchmark.
    async fn get_for_benchmark(
        &self,
        benchmark_id: BenchmarkId,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<Submission>>;

    /// Get submissions by a specific user.
    async fn get_by_user(
        &self,
        user_id: UserId,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<Submission>>;

    /// Update submission verification status.
    async fn update_verification(
        &self,
        id: SubmissionId,
        status: &VerificationStatus,
    ) -> Result<()>;

    /// Update submission visibility.
    async fn update_visibility(
        &self,
        id: SubmissionId,
        visibility: SubmissionVisibility,
    ) -> Result<()>;

    /// Delete a submission.
    async fn delete(&self, id: SubmissionId) -> Result<bool>;

    /// Get leaderboard for a benchmark.
    async fn get_leaderboard(
        &self,
        benchmark_id: BenchmarkId,
        version_id: Option<BenchmarkVersionId>,
        limit: usize,
    ) -> Result<Vec<LeaderboardEntry>>;

    /// Get the best submission for a model on a benchmark.
    async fn get_best_for_model(
        &self,
        benchmark_id: BenchmarkId,
        model_provider: &str,
        model_name: &str,
    ) -> Result<Option<Submission>>;

    /// Count submissions matching optional filters.
    async fn count(
        &self,
        benchmark_id: Option<BenchmarkId>,
        user_id: Option<UserId>,
    ) -> Result<u64>;

    /// Check if a similar submission already exists.
    async fn exists_for_model_version(
        &self,
        benchmark_id: BenchmarkId,
        benchmark_version_id: BenchmarkVersionId,
        model_provider: &str,
        model_name: &str,
        model_version: Option<&str>,
    ) -> Result<bool>;
}

/// PostgreSQL implementation of SubmissionRepository.
pub struct PgSubmissionRepository {
    pool: PgPool,
}

impl PgSubmissionRepository {
    /// Create a new PostgreSQL submission repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Convert a database row to a Submission.
    async fn row_to_submission(&self, row: sqlx::postgres::PgRow) -> Result<Submission> {
        let id: Uuid = row.get("id");
        let model_info_json: serde_json::Value = row.get("model_info");
        let submitter_info_json: serde_json::Value = row.get("submitter_info");
        let results_json: serde_json::Value = row.get("results");
        let execution_metadata_json: serde_json::Value = row.get("execution_metadata");
        let verification_status_json: serde_json::Value = row.get("verification_status");
        let visibility_str: String = row.get("visibility");

        Ok(Submission {
            id: SubmissionId::from(id),
            benchmark_id: BenchmarkId::from(row.get::<Uuid, _>("benchmark_id")),
            benchmark_version_id: BenchmarkVersionId::from(row.get::<Uuid, _>("benchmark_version_id")),
            model_info: serde_json::from_value(model_info_json).map_err(Error::Serialization)?,
            submitter: serde_json::from_value(submitter_info_json).map_err(Error::Serialization)?,
            results: serde_json::from_value(results_json).map_err(Error::Serialization)?,
            execution_metadata: serde_json::from_value(execution_metadata_json)
                .map_err(Error::Serialization)?,
            verification_status: serde_json::from_value(verification_status_json)
                .map_err(Error::Serialization)?,
            visibility: parse_visibility(&visibility_str)?,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

#[async_trait]
impl SubmissionRepository for PgSubmissionRepository {
    #[instrument(skip(self, submission), fields(benchmark_id = %submission.benchmark_id))]
    async fn create(&self, submission: &Submission) -> Result<SubmissionId> {
        let id = SubmissionId::new();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO submissions (
                id, benchmark_id, benchmark_version_id,
                model_info, submitter_info, results, execution_metadata,
                verification_status, visibility, aggregate_score,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(id.as_uuid())
        .bind(submission.benchmark_id.as_uuid())
        .bind(submission.benchmark_version_id.as_uuid())
        .bind(serde_json::to_value(&submission.model_info).map_err(Error::Serialization)?)
        .bind(serde_json::to_value(&submission.submitter).map_err(Error::Serialization)?)
        .bind(serde_json::to_value(&submission.results).map_err(Error::Serialization)?)
        .bind(serde_json::to_value(&submission.execution_metadata).map_err(Error::Serialization)?)
        .bind(serde_json::to_value(&submission.verification_status).map_err(Error::Serialization)?)
        .bind(visibility_to_str(&submission.visibility))
        .bind(submission.results.aggregate_score)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        debug!(submission_id = %id, "Submission created successfully");
        Ok(id)
    }

    #[instrument(skip(self))]
    async fn get_by_id(&self, id: SubmissionId) -> Result<Option<Submission>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, benchmark_id, benchmark_version_id,
                model_info, submitter_info, results, execution_metadata,
                verification_status, visibility, created_at, updated_at
            FROM submissions
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_submission(row).await?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self, query))]
    async fn list(&self, query: SubmissionQuery) -> Result<PaginatedResult<Submission>> {
        let offset = query.pagination.offset() as i64;
        let limit = query.pagination.limit() as i64;

        // Build dynamic WHERE clause
        let mut conditions = vec!["1=1".to_string()];
        let mut param_count = 0;

        if query.benchmark_id.is_some() {
            param_count += 1;
            conditions.push(format!("benchmark_id = ${}", param_count));
        }
        if query.user_id.is_some() {
            param_count += 1;
            conditions.push(format!("submitter_info->>'user_id' = ${}", param_count));
        }
        if query.visibility.is_some() {
            param_count += 1;
            conditions.push(format!("visibility = ${}", param_count));
        }
        if query.min_score.is_some() {
            param_count += 1;
            conditions.push(format!("aggregate_score >= ${}", param_count));
        }
        if query.max_score.is_some() {
            param_count += 1;
            conditions.push(format!("aggregate_score <= ${}", param_count));
        }

        let where_clause = conditions.join(" AND ");
        let order_column = match query.sort.field.as_str() {
            "score" | "aggregate_score" => "aggregate_score",
            "created_at" => "created_at",
            "updated_at" => "updated_at",
            _ => "created_at",
        };
        let order_direction = match query.sort.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        // Count total
        let count_sql = format!(
            "SELECT COUNT(*) FROM submissions WHERE {}",
            where_clause
        );

        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref benchmark_id) = query.benchmark_id {
            count_query = count_query.bind(benchmark_id.as_uuid());
        }
        if let Some(ref user_id) = query.user_id {
            count_query = count_query.bind(user_id.to_string());
        }
        if let Some(ref visibility) = query.visibility {
            count_query = count_query.bind(visibility_to_str(visibility));
        }
        if let Some(min_score) = query.min_score {
            count_query = count_query.bind(min_score);
        }
        if let Some(max_score) = query.max_score {
            count_query = count_query.bind(max_score);
        }

        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)?;

        // Fetch results
        let list_sql = format!(
            r#"
            SELECT
                id, benchmark_id, benchmark_version_id,
                model_info, submitter_info, results, execution_metadata,
                verification_status, visibility, created_at, updated_at
            FROM submissions
            WHERE {}
            ORDER BY {} {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_column, order_direction, limit, offset
        );

        let mut list_query = sqlx::query(&list_sql);
        if let Some(ref benchmark_id) = query.benchmark_id {
            list_query = list_query.bind(benchmark_id.as_uuid());
        }
        if let Some(ref user_id) = query.user_id {
            list_query = list_query.bind(user_id.to_string());
        }
        if let Some(ref visibility) = query.visibility {
            list_query = list_query.bind(visibility_to_str(visibility));
        }
        if let Some(min_score) = query.min_score {
            list_query = list_query.bind(min_score);
        }
        if let Some(max_score) = query.max_score {
            list_query = list_query.bind(max_score);
        }

        let rows = list_query
            .fetch_all(&self.pool)
            .await
            .map_err(Error::Database)?;

        let mut submissions = Vec::with_capacity(rows.len());
        for row in rows {
            submissions.push(self.row_to_submission(row).await?);
        }

        Ok(PaginatedResult::new(
            submissions,
            query.pagination.page,
            query.pagination.per_page,
            total as u64,
        ))
    }

    #[instrument(skip(self))]
    async fn get_for_benchmark(
        &self,
        benchmark_id: BenchmarkId,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<Submission>> {
        let query = SubmissionQuery {
            benchmark_id: Some(benchmark_id),
            visibility: Some(SubmissionVisibility::Public),
            pagination,
            sort: SortParams::desc("aggregate_score"),
            ..Default::default()
        };
        self.list(query).await
    }

    #[instrument(skip(self))]
    async fn get_by_user(
        &self,
        user_id: UserId,
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<Submission>> {
        let query = SubmissionQuery {
            user_id: Some(user_id),
            pagination,
            sort: SortParams::desc("created_at"),
            ..Default::default()
        };
        self.list(query).await
    }

    #[instrument(skip(self, status))]
    async fn update_verification(
        &self,
        id: SubmissionId,
        status: &VerificationStatus,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE submissions
            SET verification_status = $2, updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .bind(serde_json::to_value(status).map_err(Error::Serialization)?)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Submission {}", id)));
        }

        debug!(submission_id = %id, "Verification status updated");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn update_visibility(
        &self,
        id: SubmissionId,
        visibility: SubmissionVisibility,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE submissions
            SET visibility = $2, updated_at = $3
            WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .bind(visibility_to_str(&visibility))
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Submission {}", id)));
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: SubmissionId) -> Result<bool> {
        let result = sqlx::query("DELETE FROM submissions WHERE id = $1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await
            .map_err(Error::Database)?;

        Ok(result.rows_affected() > 0)
    }

    #[instrument(skip(self))]
    async fn get_leaderboard(
        &self,
        benchmark_id: BenchmarkId,
        version_id: Option<BenchmarkVersionId>,
        limit: usize,
    ) -> Result<Vec<LeaderboardEntry>> {
        let rows = match version_id {
            Some(vid) => {
                sqlx::query(
                    r#"
                    SELECT
                        s.id,
                        s.model_info,
                        s.aggregate_score,
                        s.verification_status,
                        s.created_at,
                        u.display_name as submitter_name,
                        o.name as organization_name,
                        ROW_NUMBER() OVER (ORDER BY s.aggregate_score DESC) as rank
                    FROM submissions s
                    LEFT JOIN users u ON u.id = (s.submitter_info->>'user_id')::uuid
                    LEFT JOIN organizations o ON o.id = (s.submitter_info->>'organization_id')::uuid
                    WHERE s.benchmark_id = $1
                      AND s.benchmark_version_id = $2
                      AND s.visibility = 'public'
                    ORDER BY s.aggregate_score DESC
                    LIMIT $3
                    "#,
                )
                .bind(benchmark_id.as_uuid())
                .bind(vid.as_uuid())
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            None => {
                sqlx::query(
                    r#"
                    SELECT
                        s.id,
                        s.model_info,
                        s.aggregate_score,
                        s.verification_status,
                        s.created_at,
                        u.display_name as submitter_name,
                        o.name as organization_name,
                        ROW_NUMBER() OVER (ORDER BY s.aggregate_score DESC) as rank
                    FROM submissions s
                    LEFT JOIN users u ON u.id = (s.submitter_info->>'user_id')::uuid
                    LEFT JOIN organizations o ON o.id = (s.submitter_info->>'organization_id')::uuid
                    WHERE s.benchmark_id = $1
                      AND s.visibility = 'public'
                    ORDER BY s.aggregate_score DESC
                    LIMIT $2
                    "#,
                )
                .bind(benchmark_id.as_uuid())
                .bind(limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(Error::Database)?
            }
        };

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let model_info_json: serde_json::Value = row.get("model_info");
            let verification_status_json: serde_json::Value = row.get("verification_status");
            let verification_status: VerificationStatus =
                serde_json::from_value(verification_status_json).map_err(Error::Serialization)?;

            entries.push(LeaderboardEntry {
                submission_id: SubmissionId::from(row.get::<Uuid, _>("id")),
                rank: row.get::<i64, _>("rank") as u32,
                model_info: serde_json::from_value(model_info_json).map_err(Error::Serialization)?,
                aggregate_score: row.get("aggregate_score"),
                verification_level: verification_status.level,
                submitted_at: row.get("created_at"),
                submitter_name: row.get("submitter_name"),
                organization_name: row.get("organization_name"),
            });
        }

        Ok(entries)
    }

    #[instrument(skip(self))]
    async fn get_best_for_model(
        &self,
        benchmark_id: BenchmarkId,
        model_provider: &str,
        model_name: &str,
    ) -> Result<Option<Submission>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, benchmark_id, benchmark_version_id,
                model_info, submitter_info, results, execution_metadata,
                verification_status, visibility, created_at, updated_at
            FROM submissions
            WHERE benchmark_id = $1
              AND model_info->>'provider' = $2
              AND model_info->>'model_name' = $3
              AND visibility = 'public'
            ORDER BY aggregate_score DESC
            LIMIT 1
            "#,
        )
        .bind(benchmark_id.as_uuid())
        .bind(model_provider)
        .bind(model_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_submission(row).await?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn count(
        &self,
        benchmark_id: Option<BenchmarkId>,
        user_id: Option<UserId>,
    ) -> Result<u64> {
        let count: i64 = match (benchmark_id, user_id) {
            (Some(bid), Some(uid)) => {
                sqlx::query_scalar(
                    r#"
                    SELECT COUNT(*) FROM submissions
                    WHERE benchmark_id = $1 AND submitter_info->>'user_id' = $2
                    "#,
                )
                .bind(bid.as_uuid())
                .bind(uid.to_string())
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            (Some(bid), None) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM submissions WHERE benchmark_id = $1")
                    .bind(bid.as_uuid())
                    .fetch_one(&self.pool)
                    .await
                    .map_err(Error::Database)?
            }
            (None, Some(uid)) => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM submissions WHERE submitter_info->>'user_id' = $1",
                )
                .bind(uid.to_string())
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            (None, None) => sqlx::query_scalar("SELECT COUNT(*) FROM submissions")
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?,
        };

        Ok(count as u64)
    }

    #[instrument(skip(self))]
    async fn exists_for_model_version(
        &self,
        benchmark_id: BenchmarkId,
        benchmark_version_id: BenchmarkVersionId,
        model_provider: &str,
        model_name: &str,
        model_version: Option<&str>,
    ) -> Result<bool> {
        let exists: bool = match model_version {
            Some(version) => {
                sqlx::query_scalar(
                    r#"
                    SELECT EXISTS(
                        SELECT 1 FROM submissions
                        WHERE benchmark_id = $1
                          AND benchmark_version_id = $2
                          AND model_info->>'provider' = $3
                          AND model_info->>'model_name' = $4
                          AND model_info->>'model_version' = $5
                    )
                    "#,
                )
                .bind(benchmark_id.as_uuid())
                .bind(benchmark_version_id.as_uuid())
                .bind(model_provider)
                .bind(model_name)
                .bind(version)
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            None => {
                sqlx::query_scalar(
                    r#"
                    SELECT EXISTS(
                        SELECT 1 FROM submissions
                        WHERE benchmark_id = $1
                          AND benchmark_version_id = $2
                          AND model_info->>'provider' = $3
                          AND model_info->>'model_name' = $4
                          AND model_info->>'model_version' IS NULL
                    )
                    "#,
                )
                .bind(benchmark_id.as_uuid())
                .bind(benchmark_version_id.as_uuid())
                .bind(model_provider)
                .bind(model_name)
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
        };

        Ok(exists)
    }
}

// Helper functions for visibility conversion

fn visibility_to_str(visibility: &SubmissionVisibility) -> &'static str {
    match visibility {
        SubmissionVisibility::Public => "public",
        SubmissionVisibility::Unlisted => "unlisted",
        SubmissionVisibility::Private => "private",
    }
}

fn parse_visibility(s: &str) -> Result<SubmissionVisibility> {
    match s.to_lowercase().as_str() {
        "public" => Ok(SubmissionVisibility::Public),
        "unlisted" => Ok(SubmissionVisibility::Unlisted),
        "private" => Ok(SubmissionVisibility::Private),
        _ => Err(Error::Configuration(format!("Unknown visibility: {}", s))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visibility_conversion() {
        assert_eq!(visibility_to_str(&SubmissionVisibility::Public), "public");
        assert!(parse_visibility("public").is_ok());
        assert!(parse_visibility("invalid").is_err());
    }
}
