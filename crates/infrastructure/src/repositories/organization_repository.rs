//! Organization repository implementation.
//!
//! PostgreSQL-backed implementation for organization persistence operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use tracing::{debug, instrument};
use url::Url;
use uuid::Uuid;

use llm_benchmark_common::pagination::{PaginatedResult, PaginationParams, SortDirection, SortParams};
use llm_benchmark_domain::{
    identifiers::{OrganizationId, UserId},
    user::{Organization, OrganizationRole, OrganizationType},
};

use crate::{Error, Result};

/// Query parameters for organization searches.
#[derive(Debug, Clone, Default)]
pub struct OrganizationQuery {
    pub organization_type: Option<OrganizationType>,
    pub verified: Option<bool>,
    pub search_text: Option<String>,
    pub pagination: PaginationParams,
    pub sort: SortParams,
}

/// Organization member information.
#[derive(Debug, Clone)]
pub struct OrganizationMember {
    pub user_id: UserId,
    pub username: String,
    pub display_name: Option<String>,
    pub role: OrganizationRole,
    pub joined_at: DateTime<Utc>,
}

/// Repository trait for organization operations.
#[async_trait]
pub trait OrganizationRepository: Send + Sync {
    /// Create a new organization.
    async fn create(&self, organization: &Organization, owner_id: UserId) -> Result<OrganizationId>;

    /// Get an organization by its ID.
    async fn get_by_id(&self, id: OrganizationId) -> Result<Option<Organization>>;

    /// Get an organization by its slug.
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Organization>>;

    /// List organizations with filtering and pagination.
    async fn list(&self, query: OrganizationQuery) -> Result<PaginatedResult<Organization>>;

    /// Update an organization.
    async fn update(&self, organization: &Organization) -> Result<()>;

    /// Verify an organization.
    async fn verify(&self, id: OrganizationId) -> Result<()>;

    /// Unverify an organization.
    async fn unverify(&self, id: OrganizationId) -> Result<()>;

    /// Delete an organization (soft delete).
    async fn delete(&self, id: OrganizationId) -> Result<bool>;

    /// Check if a slug already exists.
    async fn slug_exists(&self, slug: &str) -> Result<bool>;

    /// Get all members of an organization.
    async fn get_members(&self, id: OrganizationId) -> Result<Vec<OrganizationMember>>;

    /// Get the count of members in an organization.
    async fn get_member_count(&self, id: OrganizationId) -> Result<u64>;

    /// Get organizations by type.
    async fn get_by_type(
        &self,
        org_type: OrganizationType,
        limit: usize,
    ) -> Result<Vec<Organization>>;

    /// Search organizations by name.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Organization>>;

    /// Count total organizations matching optional filters.
    async fn count(
        &self,
        org_type: Option<OrganizationType>,
        verified: Option<bool>,
    ) -> Result<u64>;
}

/// PostgreSQL implementation of OrganizationRepository.
pub struct PgOrganizationRepository {
    pool: PgPool,
}

impl PgOrganizationRepository {
    /// Create a new PostgreSQL organization repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Convert a database row to an Organization.
    fn row_to_organization(&self, row: &sqlx::postgres::PgRow) -> Result<Organization> {
        let org_type_str: String = row.get("organization_type");

        Ok(Organization {
            id: OrganizationId::from(row.get::<Uuid, _>("id")),
            name: row.get("name"),
            slug: row.get("slug"),
            description: row.get("description"),
            website: row
                .get::<Option<String>, _>("website")
                .and_then(|s| Url::parse(&s).ok()),
            logo_url: row
                .get::<Option<String>, _>("logo_url")
                .and_then(|s| Url::parse(&s).ok()),
            organization_type: parse_org_type(&org_type_str)?,
            verified: row.get("verified"),
            verification_date: row.get("verification_date"),
            created_at: row.get("created_at"),
        })
    }
}

#[async_trait]
impl OrganizationRepository for PgOrganizationRepository {
    #[instrument(skip(self, organization), fields(org_name = %organization.name))]
    async fn create(&self, organization: &Organization, owner_id: UserId) -> Result<OrganizationId> {
        let id = OrganizationId::new();
        let now = Utc::now();

        let mut tx = self.pool.begin().await.map_err(Error::Database)?;

        // Create organization
        sqlx::query(
            r#"
            INSERT INTO organizations (
                id, name, slug, description, website, logo_url,
                organization_type, verified, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(id.as_uuid())
        .bind(&organization.name)
        .bind(&organization.slug)
        .bind(&organization.description)
        .bind(organization.website.as_ref().map(|u| u.to_string()))
        .bind(organization.logo_url.as_ref().map(|u| u.to_string()))
        .bind(org_type_to_str(&organization.organization_type))
        .bind(false)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(Error::Database)?;

        // Add owner as a member
        sqlx::query(
            r#"
            INSERT INTO organization_members (user_id, organization_id, role, joined_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(owner_id.as_uuid())
        .bind(id.as_uuid())
        .bind("owner")
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(Error::Database)?;

        tx.commit().await.map_err(Error::Database)?;

        debug!(organization_id = %id, "Organization created successfully");
        Ok(id)
    }

    #[instrument(skip(self))]
    async fn get_by_id(&self, id: OrganizationId) -> Result<Option<Organization>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, name, slug, description, website, logo_url,
                organization_type, verified, verification_date, created_at
            FROM organizations
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_organization(&row)?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn get_by_slug(&self, slug: &str) -> Result<Option<Organization>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, name, slug, description, website, logo_url,
                organization_type, verified, verification_date, created_at
            FROM organizations
            WHERE LOWER(slug) = LOWER($1) AND deleted_at IS NULL
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_organization(&row)?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self, query))]
    async fn list(&self, query: OrganizationQuery) -> Result<PaginatedResult<Organization>> {
        let offset = query.pagination.offset() as i64;
        let limit = query.pagination.limit() as i64;

        // Build dynamic WHERE clause
        let mut conditions = vec!["deleted_at IS NULL".to_string()];
        let mut param_count = 0;

        if query.organization_type.is_some() {
            param_count += 1;
            conditions.push(format!("organization_type = ${}", param_count));
        }
        if query.verified.is_some() {
            param_count += 1;
            conditions.push(format!("verified = ${}", param_count));
        }
        if query.search_text.is_some() {
            param_count += 1;
            conditions.push(format!(
                "(name ILIKE ${0} OR description ILIKE ${0})",
                param_count
            ));
        }

        let where_clause = conditions.join(" AND ");
        let order_column = match query.sort.field.as_str() {
            "name" => "name",
            "created_at" => "created_at",
            _ => "created_at",
        };
        let order_direction = match query.sort.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM organizations WHERE {}", where_clause);

        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref org_type) = query.organization_type {
            count_query = count_query.bind(org_type_to_str(org_type));
        }
        if let Some(verified) = query.verified {
            count_query = count_query.bind(verified);
        }
        if let Some(ref search) = query.search_text {
            count_query = count_query.bind(format!("%{}%", search));
        }

        let total: i64 = count_query
            .fetch_one(&self.pool)
            .await
            .map_err(Error::Database)?;

        // Fetch results
        let list_sql = format!(
            r#"
            SELECT
                id, name, slug, description, website, logo_url,
                organization_type, verified, verification_date, created_at
            FROM organizations
            WHERE {}
            ORDER BY {} {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_column, order_direction, limit, offset
        );

        let mut list_query = sqlx::query(&list_sql);
        if let Some(ref org_type) = query.organization_type {
            list_query = list_query.bind(org_type_to_str(org_type));
        }
        if let Some(verified) = query.verified {
            list_query = list_query.bind(verified);
        }
        if let Some(ref search) = query.search_text {
            list_query = list_query.bind(format!("%{}%", search));
        }

        let rows = list_query
            .fetch_all(&self.pool)
            .await
            .map_err(Error::Database)?;

        let mut organizations = Vec::with_capacity(rows.len());
        for row in rows {
            organizations.push(self.row_to_organization(&row)?);
        }

        Ok(PaginatedResult::new(
            organizations,
            query.pagination.page,
            query.pagination.per_page,
            total as u64,
        ))
    }

    #[instrument(skip(self, organization))]
    async fn update(&self, organization: &Organization) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE organizations SET
                name = $2,
                description = $3,
                website = $4,
                logo_url = $5,
                updated_at = $6
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(organization.id.as_uuid())
        .bind(&organization.name)
        .bind(&organization.description)
        .bind(organization.website.as_ref().map(|u| u.to_string()))
        .bind(organization.logo_url.as_ref().map(|u| u.to_string()))
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Organization {}", organization.id)));
        }

        debug!(organization_id = %organization.id, "Organization updated");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn verify(&self, id: OrganizationId) -> Result<()> {
        let now = Utc::now();
        let result = sqlx::query(
            r#"
            UPDATE organizations SET
                verified = true,
                verification_date = $2,
                updated_at = $3
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Organization {}", id)));
        }

        debug!(organization_id = %id, "Organization verified");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn unverify(&self, id: OrganizationId) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE organizations SET
                verified = false,
                verification_date = NULL,
                updated_at = $2
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("Organization {}", id)));
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: OrganizationId) -> Result<bool> {
        // Soft delete
        let result = sqlx::query(
            r#"
            UPDATE organizations SET
                deleted_at = $2,
                updated_at = $2
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(result.rows_affected() > 0)
    }

    #[instrument(skip(self))]
    async fn slug_exists(&self, slug: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM organizations WHERE LOWER(slug) = LOWER($1) AND deleted_at IS NULL)",
        )
        .bind(slug)
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(exists)
    }

    #[instrument(skip(self))]
    async fn get_members(&self, id: OrganizationId) -> Result<Vec<OrganizationMember>> {
        let rows = sqlx::query(
            r#"
            SELECT
                u.id as user_id,
                u.username,
                u.display_name,
                om.role,
                om.joined_at
            FROM organization_members om
            JOIN users u ON u.id = om.user_id
            WHERE om.organization_id = $1 AND u.deleted_at IS NULL
            ORDER BY om.joined_at
            "#,
        )
        .bind(id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let members = rows
            .into_iter()
            .map(|row| {
                let role_str: String = row.get("role");
                OrganizationMember {
                    user_id: UserId::from(row.get::<Uuid, _>("user_id")),
                    username: row.get("username"),
                    display_name: row.get("display_name"),
                    role: parse_org_role(&role_str).unwrap_or(OrganizationRole::Member),
                    joined_at: row.get("joined_at"),
                }
            })
            .collect();

        Ok(members)
    }

    #[instrument(skip(self))]
    async fn get_member_count(&self, id: OrganizationId) -> Result<u64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM organization_members WHERE organization_id = $1")
                .bind(id.as_uuid())
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?;

        Ok(count as u64)
    }

    #[instrument(skip(self))]
    async fn get_by_type(
        &self,
        org_type: OrganizationType,
        limit: usize,
    ) -> Result<Vec<Organization>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, name, slug, description, website, logo_url,
                organization_type, verified, verification_date, created_at
            FROM organizations
            WHERE organization_type = $1 AND deleted_at IS NULL
            ORDER BY verified DESC, created_at DESC
            LIMIT $2
            "#,
        )
        .bind(org_type_to_str(&org_type))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut organizations = Vec::with_capacity(rows.len());
        for row in rows {
            organizations.push(self.row_to_organization(&row)?);
        }

        Ok(organizations)
    }

    #[instrument(skip(self))]
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Organization>> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query(
            r#"
            SELECT
                id, name, slug, description, website, logo_url,
                organization_type, verified, verification_date, created_at
            FROM organizations
            WHERE (name ILIKE $1 OR description ILIKE $1) AND deleted_at IS NULL
            ORDER BY verified DESC, created_at DESC
            LIMIT $2
            "#,
        )
        .bind(&search_pattern)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let mut organizations = Vec::with_capacity(rows.len());
        for row in rows {
            organizations.push(self.row_to_organization(&row)?);
        }

        Ok(organizations)
    }

    #[instrument(skip(self))]
    async fn count(
        &self,
        org_type: Option<OrganizationType>,
        verified: Option<bool>,
    ) -> Result<u64> {
        let count: i64 = match (org_type, verified) {
            (Some(t), Some(v)) => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM organizations WHERE organization_type = $1 AND verified = $2 AND deleted_at IS NULL",
                )
                .bind(org_type_to_str(&t))
                .bind(v)
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            (Some(t), None) => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM organizations WHERE organization_type = $1 AND deleted_at IS NULL",
                )
                .bind(org_type_to_str(&t))
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            (None, Some(v)) => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM organizations WHERE verified = $1 AND deleted_at IS NULL",
                )
                .bind(v)
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            (None, None) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM organizations WHERE deleted_at IS NULL")
                    .fetch_one(&self.pool)
                    .await
                    .map_err(Error::Database)?
            }
        };

        Ok(count as u64)
    }
}

// Helper functions for organization type conversion

fn org_type_to_str(org_type: &OrganizationType) -> &'static str {
    match org_type {
        OrganizationType::LlmProvider => "llm_provider",
        OrganizationType::ResearchInstitution => "research_institution",
        OrganizationType::Enterprise => "enterprise",
        OrganizationType::OpenSource => "open_source",
        OrganizationType::Individual => "individual",
    }
}

fn parse_org_type(s: &str) -> Result<OrganizationType> {
    match s.to_lowercase().as_str() {
        "llm_provider" => Ok(OrganizationType::LlmProvider),
        "research_institution" => Ok(OrganizationType::ResearchInstitution),
        "enterprise" => Ok(OrganizationType::Enterprise),
        "open_source" => Ok(OrganizationType::OpenSource),
        "individual" => Ok(OrganizationType::Individual),
        _ => Err(Error::Configuration(format!("Unknown org type: {}", s))),
    }
}

fn parse_org_role(s: &str) -> Result<OrganizationRole> {
    match s.to_lowercase().as_str() {
        "member" => Ok(OrganizationRole::Member),
        "admin" => Ok(OrganizationRole::Admin),
        "owner" => Ok(OrganizationRole::Owner),
        _ => Err(Error::Configuration(format!("Unknown org role: {}", s))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_org_type_conversion() {
        assert_eq!(org_type_to_str(&OrganizationType::LlmProvider), "llm_provider");
        assert!(parse_org_type("llm_provider").is_ok());
        assert!(parse_org_type("invalid").is_err());
    }
}
