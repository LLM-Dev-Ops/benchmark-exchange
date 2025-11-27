//! Organization Service
//!
//! Business logic for organization management including membership,
//! roles, and team operations.

use super::{
    Authorizer, EventPublisher, PaginatedResult, Pagination, ServiceConfig, ServiceContext,
    ServiceEvent,
};
use crate::validation::{
    AddMemberRequest, CreateOrganizationRequest, OrganizationRole, UpdateOrganizationRequest,
    Validatable,
};
use crate::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Organization data transfer object
#[derive(Debug, Clone)]
pub struct OrganizationDto {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
    pub logo_url: Option<String>,
    pub member_count: u64,
    pub is_verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Organization member data transfer object
#[derive(Debug, Clone)]
pub struct OrganizationMemberDto {
    pub user_id: String,
    pub username: String,
    pub display_name: String,
    pub role: OrganizationRole,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

/// Organization repository trait
#[async_trait]
pub trait OrganizationRepositoryPort: Send + Sync {
    async fn create(&self, org: &CreateOrganizationData) -> Result<String, ApplicationError>;
    async fn get_by_id(&self, id: &str) -> Result<Option<OrganizationDto>, ApplicationError>;
    async fn get_by_slug(&self, slug: &str) -> Result<Option<OrganizationDto>, ApplicationError>;
    async fn list(
        &self,
        pagination: &Pagination,
    ) -> Result<(Vec<OrganizationDto>, u64), ApplicationError>;
    async fn update(&self, id: &str, update: &UpdateOrganizationData) -> Result<(), ApplicationError>;
    async fn delete(&self, id: &str) -> Result<(), ApplicationError>;
    async fn slug_exists(&self, slug: &str) -> Result<bool, ApplicationError>;
    async fn add_member(
        &self,
        org_id: &str,
        user_id: &str,
        role: OrganizationRole,
    ) -> Result<(), ApplicationError>;
    async fn update_member_role(
        &self,
        org_id: &str,
        user_id: &str,
        role: OrganizationRole,
    ) -> Result<(), ApplicationError>;
    async fn remove_member(&self, org_id: &str, user_id: &str) -> Result<(), ApplicationError>;
    async fn get_members(&self, org_id: &str) -> Result<Vec<OrganizationMemberDto>, ApplicationError>;
    async fn get_member_role(
        &self,
        org_id: &str,
        user_id: &str,
    ) -> Result<Option<OrganizationRole>, ApplicationError>;
    async fn get_user_organizations(
        &self,
        user_id: &str,
    ) -> Result<Vec<(OrganizationDto, OrganizationRole)>, ApplicationError>;
}

/// Data for creating an organization
#[derive(Debug, Clone)]
pub struct CreateOrganizationData {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
    pub owner_id: String,
}

/// Data for updating an organization
#[derive(Debug, Clone)]
pub struct UpdateOrganizationData {
    pub name: Option<String>,
    pub description: Option<String>,
    pub website: Option<String>,
    pub contact_email: Option<String>,
    pub logo_url: Option<String>,
}

/// Organization service implementation
pub struct OrganizationService<R, A, E>
where
    R: OrganizationRepositoryPort,
    A: Authorizer,
    E: EventPublisher,
{
    repository: Arc<R>,
    authorizer: Arc<A>,
    event_publisher: Arc<E>,
    config: ServiceConfig,
}

impl<R, A, E> OrganizationService<R, A, E>
where
    R: OrganizationRepositoryPort,
    A: Authorizer,
    E: EventPublisher,
{
    pub fn new(
        repository: Arc<R>,
        authorizer: Arc<A>,
        event_publisher: Arc<E>,
        config: ServiceConfig,
    ) -> Self {
        Self {
            repository,
            authorizer,
            event_publisher,
            config,
        }
    }

    /// Create a new organization
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn create(
        &self,
        ctx: &ServiceContext,
        request: CreateOrganizationRequest,
    ) -> ApplicationResult<OrganizationDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Get authenticated user
        let user_id = ctx.require_authenticated()?;

        // Check slug uniqueness
        if self.repository.slug_exists(&request.slug).await? {
            return Err(ApplicationError::Conflict(format!(
                "Organization with slug '{}' already exists",
                request.slug
            )));
        }

        // Create organization
        let create_data = CreateOrganizationData {
            name: request.name,
            slug: request.slug,
            description: request.description,
            website: request.website,
            contact_email: request.contact_email,
            owner_id: user_id.to_string(),
        };

        let id = self.repository.create(&create_data).await?;

        // Add creator as owner
        self.repository
            .add_member(&id, user_id, OrganizationRole::Owner)
            .await?;

        info!(org_id = %id, "Organization created");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::OrganizationCreated {
                organization_id: id.clone(),
            })
            .await?;

        // Fetch and return created organization
        self.repository
            .get_by_id(&id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch created organization".to_string()))
    }

    /// Get organization by ID
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> ApplicationResult<Option<OrganizationDto>> {
        self.repository.get_by_id(id).await
    }

    /// Get organization by slug
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_by_slug(
        &self,
        ctx: &ServiceContext,
        slug: &str,
    ) -> ApplicationResult<Option<OrganizationDto>> {
        self.repository.get_by_slug(slug).await
    }

    /// List organizations
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn list(
        &self,
        ctx: &ServiceContext,
        pagination: Pagination,
    ) -> ApplicationResult<PaginatedResult<OrganizationDto>> {
        let pagination = Pagination::new(
            pagination.page.max(1),
            pagination.page_size.min(self.config.max_page_size),
        );

        let (items, total) = self.repository.list(&pagination).await?;
        Ok(PaginatedResult::new(items, total, &pagination))
    }

    /// Update an organization
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: UpdateOrganizationRequest,
    ) -> ApplicationResult<OrganizationDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        let auth = self.authorizer.can_manage_organization(ctx, id).await;
        auth.ensure_allowed()?;

        // Check user has admin or owner role
        self.require_org_admin(ctx, id).await?;

        // Check organization exists
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Organization not found: {}", id)))?;

        // Update organization
        let update_data = UpdateOrganizationData {
            name: request.name,
            description: request.description,
            website: request.website,
            contact_email: request.contact_email,
            logo_url: request.logo_url,
        };

        self.repository.update(id, &update_data).await?;

        info!(org_id = %id, "Organization updated");

        // Fetch and return updated organization
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch updated organization".to_string()))
    }

    /// Add a member to an organization
    #[instrument(skip(self, ctx, request), fields(correlation_id = %ctx.correlation_id))]
    pub async fn add_member(
        &self,
        ctx: &ServiceContext,
        org_id: &str,
        request: AddMemberRequest,
    ) -> ApplicationResult<OrganizationMemberDto> {
        // Validate request
        let validation = request.validate_all();
        validation.ensure_valid()?;

        // Check authorization
        self.require_org_admin(ctx, org_id).await?;

        // Check organization exists
        self.repository
            .get_by_id(org_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Organization not found: {}", org_id)))?;

        // Check user is not already a member
        let existing_role = self
            .repository
            .get_member_role(org_id, &request.user_id)
            .await?;

        if existing_role.is_some() {
            return Err(ApplicationError::Conflict(
                "User is already a member of this organization".to_string(),
            ));
        }

        // Add member
        self.repository
            .add_member(org_id, &request.user_id, request.role)
            .await?;

        info!(org_id = %org_id, user_id = %request.user_id, role = ?request.role, "Member added to organization");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::OrganizationMemberAdded {
                organization_id: org_id.to_string(),
                user_id: request.user_id.clone(),
            })
            .await?;

        // Get members and return the new one
        let members = self.repository.get_members(org_id).await?;
        members
            .into_iter()
            .find(|m| m.user_id == request.user_id)
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch added member".to_string()))
    }

    /// Update a member's role
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn update_member_role(
        &self,
        ctx: &ServiceContext,
        org_id: &str,
        user_id: &str,
        role: OrganizationRole,
    ) -> ApplicationResult<OrganizationMemberDto> {
        // Check authorization - only owners can change roles
        self.require_org_owner(ctx, org_id).await?;

        // Cannot change own role
        let current_user_id = ctx.require_authenticated()?;
        if current_user_id == user_id {
            return Err(ApplicationError::Forbidden(
                "Cannot change your own role".to_string(),
            ));
        }

        // Check member exists
        let existing_role = self
            .repository
            .get_member_role(org_id, user_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::NotFound("User is not a member of this organization".to_string())
            })?;

        // Cannot demote another owner unless you're the only owner
        if existing_role == OrganizationRole::Owner && role != OrganizationRole::Owner {
            let members = self.repository.get_members(org_id).await?;
            let owner_count = members
                .iter()
                .filter(|m| m.role == OrganizationRole::Owner)
                .count();

            if owner_count <= 1 {
                return Err(ApplicationError::Forbidden(
                    "Cannot demote the last owner".to_string(),
                ));
            }
        }

        // Update role
        self.repository
            .update_member_role(org_id, user_id, role)
            .await?;

        info!(org_id = %org_id, user_id = %user_id, new_role = ?role, "Member role updated");

        // Get members and return the updated one
        let members = self.repository.get_members(org_id).await?;
        members
            .into_iter()
            .find(|m| m.user_id == user_id)
            .ok_or_else(|| ApplicationError::Internal("Failed to fetch updated member".to_string()))
    }

    /// Remove a member from an organization
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn remove_member(
        &self,
        ctx: &ServiceContext,
        org_id: &str,
        user_id: &str,
    ) -> ApplicationResult<()> {
        let current_user_id = ctx.require_authenticated()?;

        // Users can leave on their own, or admins/owners can remove others
        if current_user_id != user_id {
            self.require_org_admin(ctx, org_id).await?;
        }

        // Check member exists
        let existing_role = self
            .repository
            .get_member_role(org_id, user_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::NotFound("User is not a member of this organization".to_string())
            })?;

        // Cannot remove the last owner
        if existing_role == OrganizationRole::Owner {
            let members = self.repository.get_members(org_id).await?;
            let owner_count = members
                .iter()
                .filter(|m| m.role == OrganizationRole::Owner)
                .count();

            if owner_count <= 1 {
                return Err(ApplicationError::Forbidden(
                    "Cannot remove the last owner. Transfer ownership first.".to_string(),
                ));
            }
        }

        // Remove member
        self.repository.remove_member(org_id, user_id).await?;

        info!(org_id = %org_id, user_id = %user_id, "Member removed from organization");

        // Publish event
        self.event_publisher
            .publish(ServiceEvent::OrganizationMemberRemoved {
                organization_id: org_id.to_string(),
                user_id: user_id.to_string(),
            })
            .await?;

        Ok(())
    }

    /// Get all members of an organization
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_members(
        &self,
        ctx: &ServiceContext,
        org_id: &str,
    ) -> ApplicationResult<Vec<OrganizationMemberDto>> {
        // Check organization exists
        self.repository
            .get_by_id(org_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Organization not found: {}", org_id)))?;

        self.repository.get_members(org_id).await
    }

    /// Get organizations for a user
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn get_user_organizations(
        &self,
        ctx: &ServiceContext,
        user_id: &str,
    ) -> ApplicationResult<Vec<(OrganizationDto, OrganizationRole)>> {
        self.repository.get_user_organizations(user_id).await
    }

    /// Delete an organization
    #[instrument(skip(self, ctx), fields(correlation_id = %ctx.correlation_id))]
    pub async fn delete(&self, ctx: &ServiceContext, id: &str) -> ApplicationResult<()> {
        // Only owners can delete
        self.require_org_owner(ctx, id).await?;

        // Check organization exists
        self.repository
            .get_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound(format!("Organization not found: {}", id)))?;

        // Delete organization
        self.repository.delete(id).await?;

        info!(org_id = %id, "Organization deleted");

        Ok(())
    }

    /// Check if user has admin or owner role in organization
    async fn require_org_admin(&self, ctx: &ServiceContext, org_id: &str) -> ApplicationResult<()> {
        let user_id = ctx.require_authenticated()?;

        // Platform admins can always manage
        if ctx.is_admin {
            return Ok(());
        }

        let role = self
            .repository
            .get_member_role(org_id, user_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::Forbidden("You are not a member of this organization".to_string())
            })?;

        if role != OrganizationRole::Owner && role != OrganizationRole::Admin {
            return Err(ApplicationError::Forbidden(
                "Admin or owner role required".to_string(),
            ));
        }

        Ok(())
    }

    /// Check if user has owner role in organization
    async fn require_org_owner(&self, ctx: &ServiceContext, org_id: &str) -> ApplicationResult<()> {
        let user_id = ctx.require_authenticated()?;

        // Platform admins can always manage
        if ctx.is_admin {
            return Ok(());
        }

        let role = self
            .repository
            .get_member_role(org_id, user_id)
            .await?
            .ok_or_else(|| {
                ApplicationError::Forbidden("You are not a member of this organization".to_string())
            })?;

        if role != OrganizationRole::Owner {
            return Err(ApplicationError::Forbidden("Owner role required".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here with mock implementations
}
