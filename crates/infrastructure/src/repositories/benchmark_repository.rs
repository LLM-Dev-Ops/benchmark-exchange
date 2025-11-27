//! Benchmark repository implementation.
//!
//! PostgreSQL-backed implementation for benchmark persistence operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use tracing::{debug, instrument};
use uuid::Uuid;

use llm_benchmark_common::pagination::{PaginatedResult, PaginationParams, SortDirection, SortParams};
use llm_benchmark_domain::{
    benchmark::{BenchmarkCategory, BenchmarkMetadata, BenchmarkStatus, LicenseType},
    evaluation::{EvaluationCriteria, ExecutionConfig},
    identifiers::{BenchmarkId, BenchmarkVersionId, UserId},
    test_case::TestCase,
    version::SemanticVersion,
};

use crate::{Error, Result};

/// Query parameters for benchmark searches.
#[derive(Debug, Clone, Default)]
pub struct BenchmarkQuery {
    pub category: Option<BenchmarkCategory>,
    pub status: Option<BenchmarkStatus>,
    pub created_by: Option<UserId>,
    pub search_text: Option<String>,
    pub tags: Option<Vec<String>>,
    pub pagination: PaginationParams,
    pub sort: SortParams,
}

/// Benchmark definition as stored in the database.
#[derive(Debug, Clone)]
pub struct BenchmarkRecord {
    pub id: BenchmarkId,
    pub version_id: BenchmarkVersionId,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub long_description: Option<String>,
    pub category: BenchmarkCategory,
    pub status: BenchmarkStatus,
    pub version: SemanticVersion,
    pub tags: Vec<String>,
    pub license: LicenseType,
    pub created_by: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub evaluation_criteria: EvaluationCriteria,
    pub execution_config: ExecutionConfig,
    pub test_cases: Vec<TestCase>,
}

/// Version summary for benchmark history.
#[derive(Debug, Clone)]
pub struct BenchmarkVersionSummary {
    pub version_id: BenchmarkVersionId,
    pub version: SemanticVersion,
    pub created_at: DateTime<Utc>,
    pub changelog: String,
    pub breaking_changes: bool,
}

/// Repository trait for benchmark operations.
#[async_trait]
pub trait BenchmarkRepository: Send + Sync {
    /// Create a new benchmark with initial version.
    async fn create(&self, benchmark: &BenchmarkRecord) -> Result<BenchmarkId>;

    /// Get a benchmark by its ID (latest version).
    async fn get_by_id(&self, id: BenchmarkId) -> Result<Option<BenchmarkRecord>>;

    /// Get a specific version of a benchmark.
    async fn get_version(&self, id: BenchmarkId, version: &SemanticVersion) -> Result<Option<BenchmarkRecord>>;

    /// Get a benchmark by its slug (latest version).
    async fn get_by_slug(&self, slug: &str) -> Result<Option<BenchmarkRecord>>;

    /// List benchmarks with filtering and pagination.
    async fn list(&self, query: BenchmarkQuery) -> Result<PaginatedResult<BenchmarkRecord>>;

    /// Update a benchmark, creating a new version.
    async fn update(&self, benchmark: &BenchmarkRecord, changelog: &str, breaking: bool) -> Result<BenchmarkVersionId>;

    /// Update only the benchmark status.
    async fn update_status(&self, id: BenchmarkId, status: BenchmarkStatus) -> Result<()>;

    /// Get version history for a benchmark.
    async fn get_version_history(&self, id: BenchmarkId) -> Result<Vec<BenchmarkVersionSummary>>;

    /// Check if a slug already exists.
    async fn slug_exists(&self, slug: &str) -> Result<bool>;

    /// Full-text search across benchmarks.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<BenchmarkRecord>>;

    /// Delete a benchmark (soft delete by setting status to Archived).
    async fn delete(&self, id: BenchmarkId) -> Result<bool>;

    /// Get benchmarks by category.
    async fn get_by_category(&self, category: BenchmarkCategory, limit: usize) -> Result<Vec<BenchmarkRecord>>;

    /// Get benchmarks by tag.
    async fn get_by_tag(&self, tag: &str, limit: usize) -> Result<Vec<BenchmarkRecord>>;

    /// Count total benchmarks matching optional filters.
    async fn count(&self, status: Option<BenchmarkStatus>, category: Option<BenchmarkCategory>) -> Result<u64>;
}

/// PostgreSQL implementation of BenchmarkRepository.
pub struct PgBenchmarkRepository {
    pool: PgPool,
}

impl PgBenchmarkRepository {
    /// Create a new PostgreSQL benchmark repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Convert a database row to a BenchmarkRecord.
    async fn row_to_benchmark(&self, row: sqlx::postgres::PgRow) -> Result<BenchmarkRecord> {
        let id: Uuid = row.get("id");
        let version_id: Uuid = row.get("version_id");
        let category_str: String = row.get("category");
        let status_str: String = row.get("status");
        let license_json: serde_json::Value = row.get("license");
        let evaluation_criteria_json: serde_json::Value = row.get("evaluation_criteria");
        let execution_config_json: serde_json::Value = row.get("execution_config");
        let tags: Vec<String> = row.try_get("tags").unwrap_or_default();

        // Fetch test cases for this version
        let test_cases = self.fetch_test_cases(version_id).await?;

        Ok(BenchmarkRecord {
            id: BenchmarkId::from(id),
            version_id: BenchmarkVersionId::from(version_id),
            slug: row.get("slug"),
            name: row.get("name"),
            description: row.get("description"),
            long_description: row.get("long_description"),
            category: parse_category(&category_str)?,
            status: parse_status(&status_str)?,
            version: SemanticVersion::new(
                row.get::<i32, _>("version_major") as u32,
                row.get::<i32, _>("version_minor") as u32,
                row.get::<i32, _>("version_patch") as u32,
            ),
            tags,
            license: serde_json::from_value(license_json).map_err(Error::Serialization)?,
            created_by: UserId::from(row.get::<Uuid, _>("created_by")),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            evaluation_criteria: serde_json::from_value(evaluation_criteria_json)
                .map_err(Error::Serialization)?,
            execution_config: serde_json::from_value(execution_config_json)
                .map_err(Error::Serialization)?,
            test_cases,
        })
    }

    /// Fetch test cases for a benchmark version.
    async fn fetch_test_cases(&self, version_id: Uuid) -> Result<Vec<TestCase>> {
        let rows = sqlx::query(
            r#"
            SELECT case_id, name, description, input, expected_output,
                   evaluation_method, weight, tags
            FROM test_cases
            WHERE benchmark_version_id = $1
            ORDER BY case_id
            "#,
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut test_cases = Vec::with_capacity(rows.len());
        for row in rows {
            let input_json: serde_json::Value = row.get("input");
            let expected_output_json: Option<serde_json::Value> = row.get("expected_output");
            let evaluation_method_json: serde_json::Value = row.get("evaluation_method");

            test_cases.push(TestCase {
                id: row.get("case_id"),
                name: row.get("name"),
                description: row.get("description"),
                input: serde_json::from_value(input_json).map_err(Error::Serialization)?,
                expected_output: expected_output_json
                    .map(|v| serde_json::from_value(v))
                    .transpose()
                    .map_err(Error::Serialization)?,
                evaluation_method: serde_json::from_value(evaluation_method_json)
                    .map_err(Error::Serialization)?,
                weight: row.get("weight"),
                tags: row.try_get("tags").unwrap_or_default(),
                difficulty: None,
            });
        }

        Ok(test_cases)
    }

    /// Insert test cases for a benchmark version.
    async fn insert_test_cases(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        version_id: Uuid,
        test_cases: &[TestCase],
    ) -> Result<()> {
        for test_case in test_cases {
            sqlx::query(
                r#"
                INSERT INTO test_cases (
                    id, benchmark_version_id, case_id, name, description,
                    input, expected_output, evaluation_method, weight, tags
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                "#,
            )
            .bind(Uuid::now_v7())
            .bind(version_id)
            .bind(&test_case.id)
            .bind(&test_case.name)
            .bind(&test_case.description)
            .bind(serde_json::to_value(&test_case.input).map_err(Error::Serialization)?)
            .bind(
                test_case
                    .expected_output
                    .as_ref()
                    .map(|o| serde_json::to_value(o))
                    .transpose()
                    .map_err(Error::Serialization)?,
            )
            .bind(serde_json::to_value(&test_case.evaluation_method).map_err(Error::Serialization)?)
            .bind(test_case.weight)
            .bind(&test_case.tags)
            .execute(&mut **tx)
            .await
            .map_err(Error::Database)?;
        }
        Ok(())
    }

    /// Insert tags for a benchmark.
    async fn insert_tags(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        benchmark_id: Uuid,
        tags: &[String],
    ) -> Result<()> {
        for tag in tags {
            sqlx::query(
                r#"
                INSERT INTO benchmark_tags (benchmark_id, tag)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(benchmark_id)
            .bind(tag)
            .execute(&mut **tx)
            .await
            .map_err(Error::Database)?;
        }
        Ok(())
    }
}

#[async_trait]
impl BenchmarkRepository for PgBenchmarkRepository {
    #[instrument(skip(self, benchmark), fields(benchmark_name = %benchmark.name))]
    async fn create(&self, benchmark: &BenchmarkRecord) -> Result<BenchmarkId> {
        let id = BenchmarkId::new();
        let version_id = BenchmarkVersionId::new();
        let now = Utc::now();

        let mut tx = self.pool.begin().await.map_err(Error::Database)?;

        // Insert benchmark
        sqlx::query(
            r#"
            INSERT INTO benchmarks (
                id, slug, name, description, long_description, category, status,
                license, created_by, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(id.as_uuid())
        .bind(&benchmark.slug)
        .bind(&benchmark.name)
        .bind(&benchmark.description)
        .bind(&benchmark.long_description)
        .bind(category_to_str(&benchmark.category))
        .bind(status_to_str(&benchmark.status))
        .bind(serde_json::to_value(&benchmark.license).map_err(Error::Serialization)?)
        .bind(benchmark.created_by.as_uuid())
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(Error::Database)?;

        // Insert initial version
        sqlx::query(
            r#"
            INSERT INTO benchmark_versions (
                id, benchmark_id, version_major, version_minor, version_patch,
                evaluation_criteria, execution_config, changelog, breaking_changes,
                created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(version_id.as_uuid())
        .bind(id.as_uuid())
        .bind(benchmark.version.major as i32)
        .bind(benchmark.version.minor as i32)
        .bind(benchmark.version.patch as i32)
        .bind(serde_json::to_value(&benchmark.evaluation_criteria).map_err(Error::Serialization)?)
        .bind(serde_json::to_value(&benchmark.execution_config).map_err(Error::Serialization)?)
        .bind("Initial version")
        .bind(false)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(Error::Database)?;

        // Insert test cases
        self.insert_test_cases(&mut tx, *version_id.as_uuid(), &benchmark.test_cases)
            .await?;

        // Insert tags
        self.insert_tags(&mut tx, *id.as_uuid(), &benchmark.tags)
            .await?;

        tx.commit().await.map_err(Error::Database)?;

        debug!(benchmark_id = %id, "Benchmark created successfully");
        Ok(id)
    }

    #[instrument(skip(self))]
    async fn get_by_id(&self, id: BenchmarkId) -> Result<Option<BenchmarkRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                b.id, b.slug, b.name, b.description, b.long_description,
                b.category, b.status, b.license, b.created_by, b.created_at, b.updated_at,
                bv.id as version_id, bv.version_major, bv.version_minor, bv.version_patch,
                bv.evaluation_criteria, bv.execution_config,
                COALESCE(
                    (SELECT array_agg(tag) FROM benchmark_tags WHERE benchmark_id = b.id),
                    ARRAY[]::text[]
                ) as tags
            FROM benchmarks b
            JOIN benchmark_versions bv ON bv.benchmark_id = b.id
            WHERE b.id = $1
            ORDER BY bv.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_benchmark(row).await?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn get_version(
        &self,
        id: BenchmarkId,
        version: &SemanticVersion,
    ) -> Result<Option<BenchmarkRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                b.id, b.slug, b.name, b.description, b.long_description,
                b.category, b.status, b.license, b.created_by, b.created_at, b.updated_at,
                bv.id as version_id, bv.version_major, bv.version_minor, bv.version_patch,
                bv.evaluation_criteria, bv.execution_config,
                COALESCE(
                    (SELECT array_agg(tag) FROM benchmark_tags WHERE benchmark_id = b.id),
                    ARRAY[]::text[]
                ) as tags
            FROM benchmarks b
            JOIN benchmark_versions bv ON bv.benchmark_id = b.id
            WHERE b.id = $1
              AND bv.version_major = $2
              AND bv.version_minor = $3
              AND bv.version_patch = $4
            "#,
        )
        .bind(id.as_uuid())
        .bind(version.major as i32)
        .bind(version.minor as i32)
        .bind(version.patch as i32)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_benchmark(row).await?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn get_by_slug(&self, slug: &str) -> Result<Option<BenchmarkRecord>> {
        let row = sqlx::query(
            r#"
            SELECT
                b.id, b.slug, b.name, b.description, b.long_description,
                b.category, b.status, b.license, b.created_by, b.created_at, b.updated_at,
                bv.id as version_id, bv.version_major, bv.version_minor, bv.version_patch,
                bv.evaluation_criteria, bv.execution_config,
                COALESCE(
                    (SELECT array_agg(tag) FROM benchmark_tags WHERE benchmark_id = b.id),
                    ARRAY[]::text[]
                ) as tags
            FROM benchmarks b
            JOIN benchmark_versions bv ON bv.benchmark_id = b.id
            WHERE b.slug = $1
            ORDER BY bv.created_at DESC
            LIMIT 1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_benchmark(row).await?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self, query))]
    async fn list(&self, query: BenchmarkQuery) -> Result<PaginatedResult<BenchmarkRecord>> {
        let offset = query.pagination.offset() as i64;
        let limit = query.pagination.limit() as i64;

        // Build dynamic WHERE clause
        let mut conditions = vec!["1=1".to_string()];
        let mut bind_values: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();

        if let Some(ref category) = query.category {
            conditions.push(format!("b.category = ${}", conditions.len() + 1));
        }
        if let Some(ref status) = query.status {
            conditions.push(format!("b.status = ${}", conditions.len() + 1));
        }
        if let Some(ref search_text) = query.search_text {
            conditions.push(format!(
                "(b.name ILIKE ${0} OR b.description ILIKE ${0})",
                conditions.len() + 1
            ));
        }

        let where_clause = conditions.join(" AND ");
        let order_column = match query.sort.field.as_str() {
            "name" => "b.name",
            "created_at" => "b.created_at",
            "updated_at" => "b.updated_at",
            "category" => "b.category",
            _ => "b.created_at",
        };
        let order_direction = match query.sort.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        // Count total matching records
        let count_sql = format!(
            r#"SELECT COUNT(*) as count FROM benchmarks b WHERE {}"#,
            where_clause
        );

        // Build count query with bindings
        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref category) = query.category {
            count_query = count_query.bind(category_to_str(category));
        }
        if let Some(ref status) = query.status {
            count_query = count_query.bind(status_to_str(status));
        }
        if let Some(ref search_text) = query.search_text {
            count_query = count_query.bind(format!("%{}%", search_text));
        }

        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)?;

        // Fetch page of results
        let list_sql = format!(
            r#"
            SELECT
                b.id, b.slug, b.name, b.description, b.long_description,
                b.category, b.status, b.license, b.created_by, b.created_at, b.updated_at,
                bv.id as version_id, bv.version_major, bv.version_minor, bv.version_patch,
                bv.evaluation_criteria, bv.execution_config,
                COALESCE(
                    (SELECT array_agg(tag) FROM benchmark_tags WHERE benchmark_id = b.id),
                    ARRAY[]::text[]
                ) as tags
            FROM benchmarks b
            JOIN LATERAL (
                SELECT * FROM benchmark_versions
                WHERE benchmark_id = b.id
                ORDER BY created_at DESC
                LIMIT 1
            ) bv ON true
            WHERE {}
            ORDER BY {} {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_column, order_direction, limit, offset
        );

        let mut list_query = sqlx::query(&list_sql);
        if let Some(ref category) = query.category {
            list_query = list_query.bind(category_to_str(category));
        }
        if let Some(ref status) = query.status {
            list_query = list_query.bind(status_to_str(status));
        }
        if let Some(ref search_text) = query.search_text {
            list_query = list_query.bind(format!("%{}%", search_text));
        }

        let rows = list_query
            .fetch_all(&self.pool)
            .await
            .map_err(Error::Database)?;

        let mut benchmarks = Vec::with_capacity(rows.len());
        for row in rows {
            benchmarks.push(self.row_to_benchmark(row).await?);
        }

        Ok(PaginatedResult::new(
            benchmarks,
            query.pagination.page,
            query.pagination.per_page,
            total as u64,
        ))
    }

    #[instrument(skip(self, benchmark))]
    async fn update(
        &self,
        benchmark: &BenchmarkRecord,
        changelog: &str,
        breaking: bool,
    ) -> Result<BenchmarkVersionId> {
        let version_id = BenchmarkVersionId::new();
        let now = Utc::now();

        let mut tx = self.pool.begin().await.map_err(Error::Database)?;

        // Update benchmark metadata
        sqlx::query(
            r#"
            UPDATE benchmarks SET
                name = $2,
                description = $3,
                long_description = $4,
                license = $5,
                updated_at = $6
            WHERE id = $1
            "#,
        )
        .bind(benchmark.id.as_uuid())
        .bind(&benchmark.name)
        .bind(&benchmark.description)
        .bind(&benchmark.long_description)
        .bind(serde_json::to_value(&benchmark.license).map_err(Error::Serialization)?)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(Error::Database)?;

        // Insert new version
        sqlx::query(
            r#"
            INSERT INTO benchmark_versions (
                id, benchmark_id, version_major, version_minor, version_patch,
                evaluation_criteria, execution_config, changelog, breaking_changes,
                created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(version_id.as_uuid())
        .bind(benchmark.id.as_uuid())
        .bind(benchmark.version.major as i32)
        .bind(benchmark.version.minor as i32)
        .bind(benchmark.version.patch as i32)
        .bind(serde_json::to_value(&benchmark.evaluation_criteria).map_err(Error::Serialization)?)
        .bind(serde_json::to_value(&benchmark.execution_config).map_err(Error::Serialization)?)
        .bind(changelog)
        .bind(breaking)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(Error::Database)?;

        // Insert test cases for new version
        self.insert_test_cases(&mut tx, *version_id.as_uuid(), &benchmark.test_cases)
            .await?;

        // Update tags (delete existing and insert new)
        sqlx::query("DELETE FROM benchmark_tags WHERE benchmark_id = $1")
            .bind(benchmark.id.as_uuid())
            .execute(&mut *tx)
            .await
            .map_err(Error::Database)?;

        self.insert_tags(&mut tx, *benchmark.id.as_uuid(), &benchmark.tags)
            .await?;

        tx.commit().await.map_err(Error::Database)?;

        debug!(benchmark_id = %benchmark.id, version_id = %version_id, "Benchmark updated");
        Ok(version_id)
    }

    #[instrument(skip(self))]
    async fn update_status(&self, id: BenchmarkId, status: BenchmarkStatus) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE benchmarks SET status = $2, updated_at = $3 WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .bind(status_to_str(&status))
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Benchmark {}", id)));
        }
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_version_history(&self, id: BenchmarkId) -> Result<Vec<BenchmarkVersionSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, version_major, version_minor, version_patch,
                changelog, breaking_changes, created_at
            FROM benchmark_versions
            WHERE benchmark_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let summaries = rows
            .into_iter()
            .map(|row| BenchmarkVersionSummary {
                version_id: BenchmarkVersionId::from(row.get::<Uuid, _>("id")),
                version: SemanticVersion::new(
                    row.get::<i32, _>("version_major") as u32,
                    row.get::<i32, _>("version_minor") as u32,
                    row.get::<i32, _>("version_patch") as u32,
                ),
                created_at: row.get("created_at"),
                changelog: row.get("changelog"),
                breaking_changes: row.get("breaking_changes"),
            })
            .collect();

        Ok(summaries)
    }

    #[instrument(skip(self))]
    async fn slug_exists(&self, slug: &str) -> Result<bool> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM benchmarks WHERE slug = $1)")
                .bind(slug)
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?;

        Ok(exists)
    }

    #[instrument(skip(self))]
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<BenchmarkRecord>> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query(
            r#"
            SELECT
                b.id, b.slug, b.name, b.description, b.long_description,
                b.category, b.status, b.license, b.created_by, b.created_at, b.updated_at,
                bv.id as version_id, bv.version_major, bv.version_minor, bv.version_patch,
                bv.evaluation_criteria, bv.execution_config,
                COALESCE(
                    (SELECT array_agg(tag) FROM benchmark_tags WHERE benchmark_id = b.id),
                    ARRAY[]::text[]
                ) as tags,
                ts_rank(
                    to_tsvector('english', b.name || ' ' || COALESCE(b.description, '')),
                    plainto_tsquery('english', $1)
                ) as rank
            FROM benchmarks b
            JOIN LATERAL (
                SELECT * FROM benchmark_versions
                WHERE benchmark_id = b.id
                ORDER BY created_at DESC
                LIMIT 1
            ) bv ON true
            WHERE b.name ILIKE $2
               OR b.description ILIKE $2
               OR EXISTS (
                   SELECT 1 FROM benchmark_tags bt
                   WHERE bt.benchmark_id = b.id AND bt.tag ILIKE $2
               )
            ORDER BY rank DESC, b.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(query)
        .bind(&search_pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut benchmarks = Vec::with_capacity(rows.len());
        for row in rows {
            benchmarks.push(self.row_to_benchmark(row).await?);
        }

        Ok(benchmarks)
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: BenchmarkId) -> Result<bool> {
        // Soft delete by setting status to Archived
        let result = sqlx::query(
            r#"
            UPDATE benchmarks SET status = $2, updated_at = $3 WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .bind(status_to_str(&BenchmarkStatus::Archived))
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(result.rows_affected() > 0)
    }

    #[instrument(skip(self))]
    async fn get_by_category(
        &self,
        category: BenchmarkCategory,
        limit: usize,
    ) -> Result<Vec<BenchmarkRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                b.id, b.slug, b.name, b.description, b.long_description,
                b.category, b.status, b.license, b.created_by, b.created_at, b.updated_at,
                bv.id as version_id, bv.version_major, bv.version_minor, bv.version_patch,
                bv.evaluation_criteria, bv.execution_config,
                COALESCE(
                    (SELECT array_agg(tag) FROM benchmark_tags WHERE benchmark_id = b.id),
                    ARRAY[]::text[]
                ) as tags
            FROM benchmarks b
            JOIN LATERAL (
                SELECT * FROM benchmark_versions
                WHERE benchmark_id = b.id
                ORDER BY created_at DESC
                LIMIT 1
            ) bv ON true
            WHERE b.category = $1 AND b.status = 'active'
            ORDER BY b.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(category_to_str(&category))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut benchmarks = Vec::with_capacity(rows.len());
        for row in rows {
            benchmarks.push(self.row_to_benchmark(row).await?);
        }

        Ok(benchmarks)
    }

    #[instrument(skip(self))]
    async fn get_by_tag(&self, tag: &str, limit: usize) -> Result<Vec<BenchmarkRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT
                b.id, b.slug, b.name, b.description, b.long_description,
                b.category, b.status, b.license, b.created_by, b.created_at, b.updated_at,
                bv.id as version_id, bv.version_major, bv.version_minor, bv.version_patch,
                bv.evaluation_criteria, bv.execution_config,
                COALESCE(
                    (SELECT array_agg(tag) FROM benchmark_tags WHERE benchmark_id = b.id),
                    ARRAY[]::text[]
                ) as tags
            FROM benchmarks b
            JOIN benchmark_tags bt ON bt.benchmark_id = b.id
            JOIN LATERAL (
                SELECT * FROM benchmark_versions
                WHERE benchmark_id = b.id
                ORDER BY created_at DESC
                LIMIT 1
            ) bv ON true
            WHERE bt.tag = $1 AND b.status = 'active'
            ORDER BY b.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(tag)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut benchmarks = Vec::with_capacity(rows.len());
        for row in rows {
            benchmarks.push(self.row_to_benchmark(row).await?);
        }

        Ok(benchmarks)
    }

    #[instrument(skip(self))]
    async fn count(
        &self,
        status: Option<BenchmarkStatus>,
        category: Option<BenchmarkCategory>,
    ) -> Result<u64> {
        let (query, count) = match (status, category) {
            (Some(s), Some(c)) => {
                let count: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM benchmarks WHERE status = $1 AND category = $2",
                )
                .bind(status_to_str(&s))
                .bind(category_to_str(&c))
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?;
                ("both", count)
            }
            (Some(s), None) => {
                let count: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM benchmarks WHERE status = $1")
                        .bind(status_to_str(&s))
                        .fetch_one(&self.pool)
                        .await
                        .map_err(Error::Database)?;
                ("status", count)
            }
            (None, Some(c)) => {
                let count: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM benchmarks WHERE category = $1")
                        .bind(category_to_str(&c))
                        .fetch_one(&self.pool)
                        .await
                        .map_err(Error::Database)?;
                ("category", count)
            }
            (None, None) => {
                let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM benchmarks")
                    .fetch_one(&self.pool)
                    .await
                    .map_err(Error::Database)?;
                ("all", count)
            }
        };

        Ok(count as u64)
    }
}

// Helper functions for converting between domain types and database strings

fn category_to_str(category: &BenchmarkCategory) -> &'static str {
    match category {
        BenchmarkCategory::Performance => "performance",
        BenchmarkCategory::Accuracy => "accuracy",
        BenchmarkCategory::Reliability => "reliability",
        BenchmarkCategory::Safety => "safety",
        BenchmarkCategory::Cost => "cost",
        BenchmarkCategory::Capability => "capability",
    }
}

fn parse_category(s: &str) -> Result<BenchmarkCategory> {
    match s.to_lowercase().as_str() {
        "performance" => Ok(BenchmarkCategory::Performance),
        "accuracy" => Ok(BenchmarkCategory::Accuracy),
        "reliability" => Ok(BenchmarkCategory::Reliability),
        "safety" => Ok(BenchmarkCategory::Safety),
        "cost" => Ok(BenchmarkCategory::Cost),
        "capability" => Ok(BenchmarkCategory::Capability),
        _ => Err(Error::Configuration(format!("Unknown category: {}", s))),
    }
}

fn status_to_str(status: &BenchmarkStatus) -> &'static str {
    match status {
        BenchmarkStatus::Draft => "draft",
        BenchmarkStatus::UnderReview => "under_review",
        BenchmarkStatus::Active => "active",
        BenchmarkStatus::Deprecated => "deprecated",
        BenchmarkStatus::Archived => "archived",
    }
}

fn parse_status(s: &str) -> Result<BenchmarkStatus> {
    match s.to_lowercase().as_str() {
        "draft" => Ok(BenchmarkStatus::Draft),
        "under_review" => Ok(BenchmarkStatus::UnderReview),
        "active" => Ok(BenchmarkStatus::Active),
        "deprecated" => Ok(BenchmarkStatus::Deprecated),
        "archived" => Ok(BenchmarkStatus::Archived),
        _ => Err(Error::Configuration(format!("Unknown status: {}", s))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_conversion() {
        assert_eq!(category_to_str(&BenchmarkCategory::Performance), "performance");
        assert!(parse_category("performance").is_ok());
        assert!(parse_category("invalid").is_err());
    }

    #[test]
    fn test_status_conversion() {
        assert_eq!(status_to_str(&BenchmarkStatus::Active), "active");
        assert!(parse_status("active").is_ok());
        assert!(parse_status("invalid").is_err());
    }
}
