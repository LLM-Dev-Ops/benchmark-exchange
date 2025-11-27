//! User repository implementation.
//!
//! PostgreSQL-backed implementation for user persistence operations.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use tracing::{debug, instrument};
use uuid::Uuid;

use llm_benchmark_common::pagination::{PaginatedResult, PaginationParams, SortDirection, SortParams};
use llm_benchmark_domain::{
    identifiers::{OrganizationId, UserId},
    user::{OrganizationMembership, OrganizationRole, User, UserProfile, UserRole},
};

use crate::{Error, Result};

/// Query parameters for user searches.
#[derive(Debug, Clone, Default)]
pub struct UserQuery {
    pub role: Option<UserRole>,
    pub email_verified: Option<bool>,
    pub search_text: Option<String>,
    pub organization_id: Option<OrganizationId>,
    pub pagination: PaginationParams,
    pub sort: SortParams,
}

/// User credentials for authentication.
#[derive(Debug, Clone)]
pub struct UserCredentials {
    pub user_id: UserId,
    pub password_hash: String,
}

/// Repository trait for user operations.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Create a new user.
    async fn create(&self, user: &User, password_hash: &str) -> Result<UserId>;

    /// Get a user by their ID.
    async fn get_by_id(&self, id: UserId) -> Result<Option<User>>;

    /// Get a user by their email.
    async fn get_by_email(&self, email: &str) -> Result<Option<User>>;

    /// Get a user by their username.
    async fn get_by_username(&self, username: &str) -> Result<Option<User>>;

    /// List users with filtering and pagination.
    async fn list(&self, query: UserQuery) -> Result<PaginatedResult<User>>;

    /// Update a user's profile.
    async fn update_profile(&self, id: UserId, profile: &UserProfile) -> Result<()>;

    /// Update a user's role.
    async fn update_role(&self, id: UserId, role: UserRole) -> Result<()>;

    /// Update a user's email verification status.
    async fn update_email_verified(&self, id: UserId, verified: bool) -> Result<()>;

    /// Update a user's password hash.
    async fn update_password(&self, id: UserId, password_hash: &str) -> Result<()>;

    /// Get user credentials for authentication.
    async fn get_credentials(&self, email: &str) -> Result<Option<UserCredentials>>;

    /// Update user's last active timestamp.
    async fn update_last_active(&self, id: UserId) -> Result<()>;

    /// Delete a user (soft delete).
    async fn delete(&self, id: UserId) -> Result<bool>;

    /// Check if an email is already registered.
    async fn email_exists(&self, email: &str) -> Result<bool>;

    /// Check if a username is already taken.
    async fn username_exists(&self, username: &str) -> Result<bool>;

    /// Add a user to an organization.
    async fn add_to_organization(
        &self,
        user_id: UserId,
        organization_id: OrganizationId,
        role: OrganizationRole,
    ) -> Result<()>;

    /// Remove a user from an organization.
    async fn remove_from_organization(
        &self,
        user_id: UserId,
        organization_id: OrganizationId,
    ) -> Result<bool>;

    /// Update a user's role in an organization.
    async fn update_organization_role(
        &self,
        user_id: UserId,
        organization_id: OrganizationId,
        role: OrganizationRole,
    ) -> Result<()>;

    /// Get all organizations a user belongs to.
    async fn get_organizations(&self, user_id: UserId) -> Result<Vec<OrganizationMembership>>;

    /// Count total users matching optional filters.
    async fn count(&self, role: Option<UserRole>, email_verified: Option<bool>) -> Result<u64>;
}

/// PostgreSQL implementation of UserRepository.
pub struct PgUserRepository {
    pool: PgPool,
}

impl PgUserRepository {
    /// Create a new PostgreSQL user repository.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Convert a database row to a User.
    async fn row_to_user(&self, row: sqlx::postgres::PgRow) -> Result<User> {
        let id: Uuid = row.get("id");
        let role_str: String = row.get("role");
        let profile_json: serde_json::Value = row.get("profile");

        // Fetch organization memberships
        let memberships = self.get_organizations(UserId::from(id)).await?;

        Ok(User {
            id: UserId::from(id),
            email: row.get("email"),
            username: row.get("username"),
            display_name: row.get("display_name"),
            role: parse_role(&role_str)?,
            organizations: memberships,
            created_at: row.get("created_at"),
            last_active_at: row.get("last_active_at"),
            email_verified: row.get("email_verified"),
            profile: serde_json::from_value(profile_json).map_err(Error::Serialization)?,
        })
    }
}

#[async_trait]
impl UserRepository for PgUserRepository {
    #[instrument(skip(self, user, password_hash), fields(email = %user.email))]
    async fn create(&self, user: &User, password_hash: &str) -> Result<UserId> {
        let id = UserId::new();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO users (
                id, email, username, display_name, role,
                profile, password_hash, email_verified,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(id.as_uuid())
        .bind(&user.email)
        .bind(&user.username)
        .bind(&user.display_name)
        .bind(role_to_str(&user.role))
        .bind(serde_json::to_value(&user.profile).map_err(Error::Serialization)?)
        .bind(password_hash)
        .bind(user.email_verified)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        debug!(user_id = %id, "User created successfully");
        Ok(id)
    }

    #[instrument(skip(self))]
    async fn get_by_id(&self, id: UserId) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, email, username, display_name, role,
                profile, email_verified, created_at, last_active_at
            FROM users
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row).await?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn get_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, email, username, display_name, role,
                profile, email_verified, created_at, last_active_at
            FROM users
            WHERE LOWER(email) = LOWER($1) AND deleted_at IS NULL
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row).await?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn get_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, email, username, display_name, role,
                profile, email_verified, created_at, last_active_at
            FROM users
            WHERE LOWER(username) = LOWER($1) AND deleted_at IS NULL
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        match row {
            Some(row) => Ok(Some(self.row_to_user(row).await?)),
            None => Ok(None),
        }
    }

    #[instrument(skip(self, query))]
    async fn list(&self, query: UserQuery) -> Result<PaginatedResult<User>> {
        let offset = query.pagination.offset() as i64;
        let limit = query.pagination.limit() as i64;

        // Build dynamic WHERE clause
        let mut conditions = vec!["deleted_at IS NULL".to_string()];
        let mut param_count = 0;

        if query.role.is_some() {
            param_count += 1;
            conditions.push(format!("role = ${}", param_count));
        }
        if query.email_verified.is_some() {
            param_count += 1;
            conditions.push(format!("email_verified = ${}", param_count));
        }
        if query.search_text.is_some() {
            param_count += 1;
            conditions.push(format!(
                "(username ILIKE ${0} OR email ILIKE ${0} OR display_name ILIKE ${0})",
                param_count
            ));
        }

        let where_clause = conditions.join(" AND ");
        let order_column = match query.sort.field.as_str() {
            "username" => "username",
            "email" => "email",
            "created_at" => "created_at",
            "last_active_at" => "last_active_at",
            _ => "created_at",
        };
        let order_direction = match query.sort.direction {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        };

        // Count total
        let count_sql = format!("SELECT COUNT(*) FROM users WHERE {}", where_clause);

        let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
        if let Some(ref role) = query.role {
            count_query = count_query.bind(role_to_str(role));
        }
        if let Some(verified) = query.email_verified {
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
                id, email, username, display_name, role,
                profile, email_verified, created_at, last_active_at
            FROM users
            WHERE {}
            ORDER BY {} {}
            LIMIT {} OFFSET {}
            "#,
            where_clause, order_column, order_direction, limit, offset
        );

        let mut list_query = sqlx::query(&list_sql);
        if let Some(ref role) = query.role {
            list_query = list_query.bind(role_to_str(role));
        }
        if let Some(verified) = query.email_verified {
            list_query = list_query.bind(verified);
        }
        if let Some(ref search) = query.search_text {
            list_query = list_query.bind(format!("%{}%", search));
        }

        let rows = list_query
            .fetch_all(&self.pool)
            .await
            .map_err(Error::Database)?;

        let mut users = Vec::with_capacity(rows.len());
        for row in rows {
            users.push(self.row_to_user(row).await?);
        }

        Ok(PaginatedResult::new(
            users,
            query.pagination.page,
            query.pagination.per_page,
            total as u64,
        ))
    }

    #[instrument(skip(self, profile))]
    async fn update_profile(&self, id: UserId, profile: &UserProfile) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET profile = $2, updated_at = $3
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .bind(serde_json::to_value(profile).map_err(Error::Serialization)?)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("User {}", id)));
        }

        debug!(user_id = %id, "Profile updated");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn update_role(&self, id: UserId, role: UserRole) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET role = $2, updated_at = $3
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .bind(role_to_str(&role))
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("User {}", id)));
        }

        debug!(user_id = %id, role = ?role, "Role updated");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn update_email_verified(&self, id: UserId, verified: bool) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET email_verified = $2, updated_at = $3
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .bind(verified)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("User {}", id)));
        }

        Ok(())
    }

    #[instrument(skip(self, password_hash))]
    async fn update_password(&self, id: UserId, password_hash: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE users
            SET password_hash = $2, updated_at = $3
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .bind(password_hash)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!("User {}", id)));
        }

        debug!(user_id = %id, "Password updated");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_credentials(&self, email: &str) -> Result<Option<UserCredentials>> {
        let row = sqlx::query(
            r#"
            SELECT id, password_hash
            FROM users
            WHERE LOWER(email) = LOWER($1) AND deleted_at IS NULL
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(row.map(|r| UserCredentials {
            user_id: UserId::from(r.get::<Uuid, _>("id")),
            password_hash: r.get("password_hash"),
        }))
    }

    #[instrument(skip(self))]
    async fn update_last_active(&self, id: UserId) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET last_active_at = $2
            WHERE id = $1 AND deleted_at IS NULL
            "#,
        )
        .bind(id.as_uuid())
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn delete(&self, id: UserId) -> Result<bool> {
        // Soft delete
        let result = sqlx::query(
            r#"
            UPDATE users
            SET deleted_at = $2, updated_at = $2
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
    async fn email_exists(&self, email: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(email) = LOWER($1) AND deleted_at IS NULL)",
        )
        .bind(email)
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(exists)
    }

    #[instrument(skip(self))]
    async fn username_exists(&self, username: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM users WHERE LOWER(username) = LOWER($1) AND deleted_at IS NULL)",
        )
        .bind(username)
        .fetch_one(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(exists)
    }

    #[instrument(skip(self))]
    async fn add_to_organization(
        &self,
        user_id: UserId,
        organization_id: OrganizationId,
        role: OrganizationRole,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO organization_members (user_id, organization_id, role, joined_at)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, organization_id) DO UPDATE SET role = $3
            "#,
        )
        .bind(user_id.as_uuid())
        .bind(organization_id.as_uuid())
        .bind(org_role_to_str(&role))
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        debug!(user_id = %user_id, organization_id = %organization_id, "User added to organization");
        Ok(())
    }

    #[instrument(skip(self))]
    async fn remove_from_organization(
        &self,
        user_id: UserId,
        organization_id: OrganizationId,
    ) -> Result<bool> {
        let result = sqlx::query(
            "DELETE FROM organization_members WHERE user_id = $1 AND organization_id = $2",
        )
        .bind(user_id.as_uuid())
        .bind(organization_id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        Ok(result.rows_affected() > 0)
    }

    #[instrument(skip(self))]
    async fn update_organization_role(
        &self,
        user_id: UserId,
        organization_id: OrganizationId,
        role: OrganizationRole,
    ) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE organization_members
            SET role = $3
            WHERE user_id = $1 AND organization_id = $2
            "#,
        )
        .bind(user_id.as_uuid())
        .bind(organization_id.as_uuid())
        .bind(org_role_to_str(&role))
        .execute(&self.pool)
        .await
        .map_err(Error::Database)?;

        if result.rows_affected() == 0 {
            return Err(Error::NotFound(format!(
                "Membership for user {} in organization {}",
                user_id, organization_id
            )));
        }

        Ok(())
    }

    #[instrument(skip(self))]
    async fn get_organizations(&self, user_id: UserId) -> Result<Vec<OrganizationMembership>> {
        let rows = sqlx::query(
            r#"
            SELECT organization_id, role, joined_at
            FROM organization_members
            WHERE user_id = $1
            ORDER BY joined_at
            "#,
        )
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(Error::Database)?;

        let memberships = rows
            .into_iter()
            .map(|row| {
                let role_str: String = row.get("role");
                OrganizationMembership {
                    organization_id: OrganizationId::from(row.get::<Uuid, _>("organization_id")),
                    role: parse_org_role(&role_str).unwrap_or(OrganizationRole::Member),
                    joined_at: row.get("joined_at"),
                }
            })
            .collect();

        Ok(memberships)
    }

    #[instrument(skip(self))]
    async fn count(&self, role: Option<UserRole>, email_verified: Option<bool>) -> Result<u64> {
        let count: i64 = match (role, email_verified) {
            (Some(r), Some(v)) => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM users WHERE role = $1 AND email_verified = $2 AND deleted_at IS NULL",
                )
                .bind(role_to_str(&r))
                .bind(v)
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            (Some(r), None) => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM users WHERE role = $1 AND deleted_at IS NULL",
                )
                .bind(role_to_str(&r))
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            (None, Some(v)) => {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM users WHERE email_verified = $1 AND deleted_at IS NULL",
                )
                .bind(v)
                .fetch_one(&self.pool)
                .await
                .map_err(Error::Database)?
            }
            (None, None) => {
                sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE deleted_at IS NULL")
                    .fetch_one(&self.pool)
                    .await
                    .map_err(Error::Database)?
            }
        };

        Ok(count as u64)
    }
}

// Helper functions for role conversion

fn role_to_str(role: &UserRole) -> &'static str {
    match role {
        UserRole::Anonymous => "anonymous",
        UserRole::Registered => "registered",
        UserRole::Contributor => "contributor",
        UserRole::Reviewer => "reviewer",
        UserRole::Admin => "admin",
    }
}

fn parse_role(s: &str) -> Result<UserRole> {
    match s.to_lowercase().as_str() {
        "anonymous" => Ok(UserRole::Anonymous),
        "registered" => Ok(UserRole::Registered),
        "contributor" => Ok(UserRole::Contributor),
        "reviewer" => Ok(UserRole::Reviewer),
        "admin" => Ok(UserRole::Admin),
        _ => Err(Error::Configuration(format!("Unknown role: {}", s))),
    }
}

fn org_role_to_str(role: &OrganizationRole) -> &'static str {
    match role {
        OrganizationRole::Member => "member",
        OrganizationRole::Admin => "admin",
        OrganizationRole::Owner => "owner",
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
    fn test_role_conversion() {
        assert_eq!(role_to_str(&UserRole::Admin), "admin");
        assert!(parse_role("admin").is_ok());
        assert!(parse_role("invalid").is_err());
    }

    #[test]
    fn test_org_role_conversion() {
        assert_eq!(org_role_to_str(&OrganizationRole::Owner), "owner");
        assert!(parse_org_role("owner").is_ok());
        assert!(parse_org_role("invalid").is_err());
    }
}
