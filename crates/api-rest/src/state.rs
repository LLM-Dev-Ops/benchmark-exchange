//! Application state and dependency injection.
//!
//! This module defines the shared application state that is passed
//! to all route handlers via Axum's state extraction.

use crate::config::ApiConfig;
use async_trait::async_trait;
use llm_benchmark_application::{
    services::{
        Authorizer, AuthorizationResult, BenchmarkDto, BenchmarkFilters, BenchmarkRepositoryPort,
        BenchmarkService, BenchmarkVersionDto, CreateBenchmarkData, CreateVersionData,
        CreateSubmissionData, DefaultAuthorizer, EventPublisher, LeaderboardEntryDto,
        NoOpEventPublisher, Pagination, PaginatedResult, ServiceConfig, ServiceContext,
        ServiceEvent, SubmissionDto, SubmissionRepositoryPort, SubmissionService,
        UpdateBenchmarkData, UpdateSubmissionData, UserDto, UserProfileDto, UserRepositoryPort,
        UserService, ApiKeyDto, ApiKeyWithSecretDto, CreateApiKeyData, CreateUserData,
        UpdateUserData, VerificationData, PasswordHasher, Argon2PasswordHasher,
    },
    validation::SubmissionQueryFilters,
    ApplicationError,
};
use llm_benchmark_domain::benchmark::BenchmarkStatus;
use llm_benchmark_domain::submission::{SubmissionResults, VerificationLevel};
use std::sync::Arc;

/// Application state shared across all requests
#[derive(Clone)]
pub struct AppState {
    /// API configuration
    pub config: Arc<ApiConfig>,

    /// JWT encoding/decoding key
    pub jwt_secret: Arc<String>,

    /// Benchmark service (type-erased)
    pub benchmark_service: Arc<dyn BenchmarkServiceTrait>,

    /// Submission service (type-erased)
    pub submission_service: Arc<dyn SubmissionServiceTrait>,

    /// User service (type-erased)
    pub user_service: Arc<dyn UserServiceTrait>,
}

impl AppState {
    /// Create a new application state with default in-memory implementations
    /// Suitable for development and testing
    pub fn new(config: ApiConfig) -> Self {
        let jwt_secret = config.jwt_secret.clone();
        let service_config = ServiceConfig::default();

        // Create default implementations
        let benchmark_repo = Arc::new(InMemoryBenchmarkRepository::new());
        let submission_repo = Arc::new(InMemorySubmissionRepository::new());
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let authorizer = Arc::new(DefaultAuthorizer);
        let event_publisher = Arc::new(NoOpEventPublisher);
        let password_hasher = Arc::new(Argon2PasswordHasher);

        let benchmark_service = Arc::new(BenchmarkService::new(
            benchmark_repo,
            Arc::clone(&authorizer),
            Arc::clone(&event_publisher),
            service_config.clone(),
        ));

        let submission_service = Arc::new(SubmissionService::new(
            submission_repo,
            Arc::clone(&authorizer),
            Arc::clone(&event_publisher),
            service_config.clone(),
        ));

        let user_service = Arc::new(UserService::new(
            user_repo,
            Arc::clone(&event_publisher),
            password_hasher,
            service_config,
        ));

        Self {
            config: Arc::new(config),
            jwt_secret: Arc::new(jwt_secret),
            benchmark_service,
            submission_service,
            user_service,
        }
    }

    /// Create application state with custom service implementations
    pub fn with_services<B, S, U>(
        config: ApiConfig,
        benchmark_service: B,
        submission_service: S,
        user_service: U,
    ) -> Self
    where
        B: BenchmarkServiceTrait + 'static,
        S: SubmissionServiceTrait + 'static,
        U: UserServiceTrait + 'static,
    {
        let jwt_secret = config.jwt_secret.clone();

        Self {
            config: Arc::new(config),
            jwt_secret: Arc::new(jwt_secret),
            benchmark_service: Arc::new(benchmark_service),
            submission_service: Arc::new(submission_service),
            user_service: Arc::new(user_service),
        }
    }

    /// Get JWT secret
    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }
}

// ============================================================================
// SERVICE TRAITS (Type-erased interfaces for route handlers)
// ============================================================================

/// Type-erased benchmark service trait
#[async_trait]
pub trait BenchmarkServiceTrait: Send + Sync {
    async fn create(
        &self,
        ctx: &ServiceContext,
        request: llm_benchmark_application::validation::CreateBenchmarkRequest,
    ) -> Result<BenchmarkDto, ApplicationError>;

    async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<BenchmarkDto>, ApplicationError>;

    async fn get_by_slug(
        &self,
        ctx: &ServiceContext,
        slug: &str,
    ) -> Result<Option<BenchmarkDto>, ApplicationError>;

    async fn list(
        &self,
        ctx: &ServiceContext,
        filters: BenchmarkFilters,
        pagination: Pagination,
    ) -> Result<PaginatedResult<BenchmarkDto>, ApplicationError>;

    async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::UpdateBenchmarkRequest,
    ) -> Result<BenchmarkDto, ApplicationError>;

    async fn transition_status(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::StatusTransitionRequest,
    ) -> Result<BenchmarkDto, ApplicationError>;

    async fn create_version(
        &self,
        ctx: &ServiceContext,
        benchmark_id: &str,
        request: llm_benchmark_application::validation::CreateVersionRequest,
    ) -> Result<BenchmarkVersionDto, ApplicationError>;

    async fn get_versions(
        &self,
        ctx: &ServiceContext,
        benchmark_id: &str,
    ) -> Result<Vec<BenchmarkVersionDto>, ApplicationError>;

    async fn delete(&self, ctx: &ServiceContext, id: &str) -> Result<(), ApplicationError>;

    async fn search(
        &self,
        ctx: &ServiceContext,
        query: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<BenchmarkDto>, ApplicationError>;
}

/// Type-erased submission service trait
#[async_trait]
pub trait SubmissionServiceTrait: Send + Sync {
    async fn create(
        &self,
        ctx: &ServiceContext,
        request: llm_benchmark_application::validation::CreateSubmissionRequest,
    ) -> Result<SubmissionDto, ApplicationError>;

    async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<SubmissionDto>, ApplicationError>;

    async fn get_results(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<SubmissionResults>, ApplicationError>;

    async fn list(
        &self,
        ctx: &ServiceContext,
        filters: SubmissionQueryFilters,
        pagination: Pagination,
    ) -> Result<PaginatedResult<SubmissionDto>, ApplicationError>;

    async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::UpdateSubmissionRequest,
    ) -> Result<SubmissionDto, ApplicationError>;

    async fn verify(
        &self,
        ctx: &ServiceContext,
        request: llm_benchmark_application::validation::VerificationRequest,
    ) -> Result<SubmissionDto, ApplicationError>;

    async fn get_leaderboard(
        &self,
        ctx: &ServiceContext,
        query: llm_benchmark_application::validation::LeaderboardQuery,
    ) -> Result<Vec<LeaderboardEntryDto>, ApplicationError>;

    async fn get_user_submissions(
        &self,
        ctx: &ServiceContext,
        user_id: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<SubmissionDto>, ApplicationError>;

    async fn delete(&self, ctx: &ServiceContext, id: &str) -> Result<(), ApplicationError>;
}

/// Type-erased user service trait
#[async_trait]
pub trait UserServiceTrait: Send + Sync {
    async fn register(
        &self,
        request: llm_benchmark_application::validation::CreateUserRequest,
    ) -> Result<UserDto, ApplicationError>;

    async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<UserDto>, ApplicationError>;

    async fn get_by_email(&self, email: &str) -> Result<Option<UserDto>, ApplicationError>;

    async fn get_by_username(&self, username: &str) -> Result<Option<UserDto>, ApplicationError>;

    async fn get_profile(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<UserProfileDto>, ApplicationError>;

    async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::UpdateUserRequest,
    ) -> Result<UserDto, ApplicationError>;

    async fn change_password(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::ChangePasswordRequest,
    ) -> Result<(), ApplicationError>;

    async fn authenticate(
        &self,
        email: &str,
        password: &str,
    ) -> Result<UserDto, ApplicationError>;

    async fn create_api_key(
        &self,
        ctx: &ServiceContext,
        request: llm_benchmark_application::validation::CreateApiKeyRequest,
    ) -> Result<ApiKeyWithSecretDto, ApplicationError>;

    async fn list_api_keys(&self, ctx: &ServiceContext) -> Result<Vec<ApiKeyDto>, ApplicationError>;

    async fn revoke_api_key(
        &self,
        ctx: &ServiceContext,
        key_id: &str,
    ) -> Result<(), ApplicationError>;

    async fn verify_api_key(
        &self,
        key_secret: &str,
    ) -> Result<Option<(String, Vec<String>)>, ApplicationError>;

    async fn delete(&self, ctx: &ServiceContext, id: &str) -> Result<(), ApplicationError>;
}

// ============================================================================
// TRAIT IMPLEMENTATIONS FOR CONCRETE SERVICES
// ============================================================================

#[async_trait]
impl<R, A, E> BenchmarkServiceTrait for BenchmarkService<R, A, E>
where
    R: BenchmarkRepositoryPort + 'static,
    A: Authorizer + 'static,
    E: EventPublisher + 'static,
{
    async fn create(
        &self,
        ctx: &ServiceContext,
        request: llm_benchmark_application::validation::CreateBenchmarkRequest,
    ) -> Result<BenchmarkDto, ApplicationError> {
        BenchmarkService::create(self, ctx, request).await
    }

    async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<BenchmarkDto>, ApplicationError> {
        BenchmarkService::get_by_id(self, ctx, id).await
    }

    async fn get_by_slug(
        &self,
        ctx: &ServiceContext,
        slug: &str,
    ) -> Result<Option<BenchmarkDto>, ApplicationError> {
        BenchmarkService::get_by_slug(self, ctx, slug).await
    }

    async fn list(
        &self,
        ctx: &ServiceContext,
        filters: BenchmarkFilters,
        pagination: Pagination,
    ) -> Result<PaginatedResult<BenchmarkDto>, ApplicationError> {
        BenchmarkService::list(self, ctx, filters, pagination).await
    }

    async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::UpdateBenchmarkRequest,
    ) -> Result<BenchmarkDto, ApplicationError> {
        BenchmarkService::update(self, ctx, id, request).await
    }

    async fn transition_status(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::StatusTransitionRequest,
    ) -> Result<BenchmarkDto, ApplicationError> {
        BenchmarkService::transition_status(self, ctx, id, request).await
    }

    async fn create_version(
        &self,
        ctx: &ServiceContext,
        benchmark_id: &str,
        request: llm_benchmark_application::validation::CreateVersionRequest,
    ) -> Result<BenchmarkVersionDto, ApplicationError> {
        BenchmarkService::create_version(self, ctx, benchmark_id, request).await
    }

    async fn get_versions(
        &self,
        ctx: &ServiceContext,
        benchmark_id: &str,
    ) -> Result<Vec<BenchmarkVersionDto>, ApplicationError> {
        BenchmarkService::get_versions(self, ctx, benchmark_id).await
    }

    async fn delete(&self, ctx: &ServiceContext, id: &str) -> Result<(), ApplicationError> {
        BenchmarkService::delete(self, ctx, id).await
    }

    async fn search(
        &self,
        ctx: &ServiceContext,
        query: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<BenchmarkDto>, ApplicationError> {
        BenchmarkService::search(self, ctx, query, pagination).await
    }
}

#[async_trait]
impl<R, A, E> SubmissionServiceTrait for SubmissionService<R, A, E>
where
    R: SubmissionRepositoryPort + 'static,
    A: Authorizer + 'static,
    E: EventPublisher + 'static,
{
    async fn create(
        &self,
        ctx: &ServiceContext,
        request: llm_benchmark_application::validation::CreateSubmissionRequest,
    ) -> Result<SubmissionDto, ApplicationError> {
        SubmissionService::create(self, ctx, request).await
    }

    async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<SubmissionDto>, ApplicationError> {
        SubmissionService::get_by_id(self, ctx, id).await
    }

    async fn get_results(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<SubmissionResults>, ApplicationError> {
        SubmissionService::get_results(self, ctx, id).await
    }

    async fn list(
        &self,
        ctx: &ServiceContext,
        filters: SubmissionQueryFilters,
        pagination: Pagination,
    ) -> Result<PaginatedResult<SubmissionDto>, ApplicationError> {
        SubmissionService::list(self, ctx, filters, pagination).await
    }

    async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::UpdateSubmissionRequest,
    ) -> Result<SubmissionDto, ApplicationError> {
        SubmissionService::update(self, ctx, id, request).await
    }

    async fn verify(
        &self,
        ctx: &ServiceContext,
        request: llm_benchmark_application::validation::VerificationRequest,
    ) -> Result<SubmissionDto, ApplicationError> {
        SubmissionService::verify(self, ctx, request).await
    }

    async fn get_leaderboard(
        &self,
        ctx: &ServiceContext,
        query: llm_benchmark_application::validation::LeaderboardQuery,
    ) -> Result<Vec<LeaderboardEntryDto>, ApplicationError> {
        SubmissionService::get_leaderboard(self, ctx, query).await
    }

    async fn get_user_submissions(
        &self,
        ctx: &ServiceContext,
        user_id: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<SubmissionDto>, ApplicationError> {
        SubmissionService::get_user_submissions(self, ctx, user_id, pagination).await
    }

    async fn delete(&self, ctx: &ServiceContext, id: &str) -> Result<(), ApplicationError> {
        SubmissionService::delete(self, ctx, id).await
    }
}

#[async_trait]
impl<R, E, H> UserServiceTrait for UserService<R, E, H>
where
    R: UserRepositoryPort + 'static,
    E: EventPublisher + 'static,
    H: PasswordHasher + 'static,
{
    async fn register(
        &self,
        request: llm_benchmark_application::validation::CreateUserRequest,
    ) -> Result<UserDto, ApplicationError> {
        UserService::register(self, request).await
    }

    async fn get_by_id(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<UserDto>, ApplicationError> {
        UserService::get_by_id(self, ctx, id).await
    }

    async fn get_by_email(&self, email: &str) -> Result<Option<UserDto>, ApplicationError> {
        UserService::get_by_email(self, email).await
    }

    async fn get_by_username(&self, username: &str) -> Result<Option<UserDto>, ApplicationError> {
        UserService::get_by_username(self, username).await
    }

    async fn get_profile(
        &self,
        ctx: &ServiceContext,
        id: &str,
    ) -> Result<Option<UserProfileDto>, ApplicationError> {
        UserService::get_profile(self, ctx, id).await
    }

    async fn update(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::UpdateUserRequest,
    ) -> Result<UserDto, ApplicationError> {
        UserService::update(self, ctx, id, request).await
    }

    async fn change_password(
        &self,
        ctx: &ServiceContext,
        id: &str,
        request: llm_benchmark_application::validation::ChangePasswordRequest,
    ) -> Result<(), ApplicationError> {
        UserService::change_password(self, ctx, id, request).await
    }

    async fn authenticate(
        &self,
        email: &str,
        password: &str,
    ) -> Result<UserDto, ApplicationError> {
        UserService::authenticate(self, email, password).await
    }

    async fn create_api_key(
        &self,
        ctx: &ServiceContext,
        request: llm_benchmark_application::validation::CreateApiKeyRequest,
    ) -> Result<ApiKeyWithSecretDto, ApplicationError> {
        UserService::create_api_key(self, ctx, request).await
    }

    async fn list_api_keys(&self, ctx: &ServiceContext) -> Result<Vec<ApiKeyDto>, ApplicationError> {
        UserService::list_api_keys(self, ctx).await
    }

    async fn revoke_api_key(
        &self,
        ctx: &ServiceContext,
        key_id: &str,
    ) -> Result<(), ApplicationError> {
        UserService::revoke_api_key(self, ctx, key_id).await
    }

    async fn verify_api_key(
        &self,
        key_secret: &str,
    ) -> Result<Option<(String, Vec<String>)>, ApplicationError> {
        UserService::verify_api_key(self, key_secret).await
    }

    async fn delete(&self, ctx: &ServiceContext, id: &str) -> Result<(), ApplicationError> {
        UserService::delete(self, ctx, id).await
    }
}

// ============================================================================
// IN-MEMORY IMPLEMENTATIONS (for development/testing)
// ============================================================================

use parking_lot::RwLock;
use std::collections::HashMap;

/// In-memory benchmark repository for development
pub struct InMemoryBenchmarkRepository {
    benchmarks: RwLock<HashMap<String, BenchmarkDto>>,
    versions: RwLock<HashMap<String, Vec<BenchmarkVersionDto>>>,
}

impl InMemoryBenchmarkRepository {
    pub fn new() -> Self {
        Self {
            benchmarks: RwLock::new(HashMap::new()),
            versions: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryBenchmarkRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BenchmarkRepositoryPort for InMemoryBenchmarkRepository {
    async fn create(&self, data: &CreateBenchmarkData) -> Result<String, ApplicationError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let benchmark = BenchmarkDto {
            id: id.clone(),
            name: data.name.clone(),
            slug: data.slug.clone(),
            description: data.description.clone(),
            category: data.category.clone(),
            status: BenchmarkStatus::Draft,
            tags: data.tags.clone(),
            current_version: Some(data.version.clone()),
            submission_count: 0,
            created_at: now,
            updated_at: now,
        };

        self.benchmarks.write().insert(id.clone(), benchmark);
        Ok(id)
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<BenchmarkDto>, ApplicationError> {
        Ok(self.benchmarks.read().get(id).cloned())
    }

    async fn get_by_slug(&self, slug: &str) -> Result<Option<BenchmarkDto>, ApplicationError> {
        Ok(self.benchmarks.read().values().find(|b| b.slug == slug).cloned())
    }

    async fn list(
        &self,
        filters: &BenchmarkFilters,
        pagination: &Pagination,
    ) -> Result<(Vec<BenchmarkDto>, u64), ApplicationError> {
        let benchmarks: Vec<_> = self.benchmarks.read()
            .values()
            .filter(|b| {
                if let Some(ref cat) = filters.category {
                    if b.category != *cat {
                        return false;
                    }
                }
                if let Some(ref status) = filters.status {
                    if b.status != *status {
                        return false;
                    }
                }
                if let Some(ref search) = filters.search {
                    if !b.name.to_lowercase().contains(&search.to_lowercase())
                        && !b.description.to_lowercase().contains(&search.to_lowercase()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let total = benchmarks.len() as u64;
        let offset = pagination.offset() as usize;
        let limit = pagination.limit() as usize;

        let items = benchmarks
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok((items, total))
    }

    async fn update(&self, id: &str, update: &UpdateBenchmarkData) -> Result<(), ApplicationError> {
        let mut benchmarks = self.benchmarks.write();
        if let Some(benchmark) = benchmarks.get_mut(id) {
            if let Some(ref name) = update.name {
                benchmark.name = name.clone();
            }
            if let Some(ref desc) = update.description {
                benchmark.description = desc.clone();
            }
            if let Some(ref tags) = update.tags {
                benchmark.tags = tags.clone();
            }
            benchmark.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ApplicationError::NotFound(format!("Benchmark not found: {}", id)))
        }
    }

    async fn update_status(&self, id: &str, status: BenchmarkStatus) -> Result<(), ApplicationError> {
        let mut benchmarks = self.benchmarks.write();
        if let Some(benchmark) = benchmarks.get_mut(id) {
            benchmark.status = status;
            benchmark.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ApplicationError::NotFound(format!("Benchmark not found: {}", id)))
        }
    }

    async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        self.benchmarks.write().remove(id);
        self.versions.write().remove(id);
        Ok(())
    }

    async fn slug_exists(&self, slug: &str) -> Result<bool, ApplicationError> {
        Ok(self.benchmarks.read().values().any(|b| b.slug == slug))
    }

    async fn create_version(&self, data: &CreateVersionData) -> Result<String, ApplicationError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let version = BenchmarkVersionDto {
            id: id.clone(),
            benchmark_id: data.benchmark_id.clone(),
            version: data.version.clone(),
            changelog: data.changelog.clone(),
            breaking_changes: data.breaking_changes,
            created_at: now,
        };

        self.versions
            .write()
            .entry(data.benchmark_id.clone())
            .or_default()
            .push(version);

        Ok(id)
    }

    async fn get_versions(&self, benchmark_id: &str) -> Result<Vec<BenchmarkVersionDto>, ApplicationError> {
        Ok(self.versions.read().get(benchmark_id).cloned().unwrap_or_default())
    }
}

/// In-memory submission repository for development
pub struct InMemorySubmissionRepository {
    submissions: RwLock<HashMap<String, SubmissionDto>>,
    results: RwLock<HashMap<String, SubmissionResults>>,
}

impl InMemorySubmissionRepository {
    pub fn new() -> Self {
        Self {
            submissions: RwLock::new(HashMap::new()),
            results: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySubmissionRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SubmissionRepositoryPort for InMemorySubmissionRepository {
    async fn create(&self, data: &CreateSubmissionData) -> Result<String, ApplicationError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let submission = SubmissionDto {
            id: id.clone(),
            benchmark_id: data.benchmark_id.clone(),
            benchmark_version_id: data.benchmark_version_id.clone(),
            model_provider: data.model_provider.clone(),
            model_name: data.model_name.clone(),
            model_version: data.model_version.clone(),
            submitter_id: data.submitter_id.clone(),
            organization_id: data.organization_id.clone(),
            aggregate_score: data.aggregate_score,
            verification_level: VerificationLevel::Unverified,
            visibility: data.visibility.clone(),
            created_at: now,
            updated_at: now,
        };

        self.submissions.write().insert(id.clone(), submission);
        Ok(id)
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<SubmissionDto>, ApplicationError> {
        Ok(self.submissions.read().get(id).cloned())
    }

    async fn list(
        &self,
        filters: &SubmissionQueryFilters,
        pagination: &Pagination,
    ) -> Result<(Vec<SubmissionDto>, u64), ApplicationError> {
        let submissions: Vec<_> = self.submissions.read()
            .values()
            .filter(|s| {
                if let Some(ref bid) = filters.benchmark_id {
                    if s.benchmark_id != *bid {
                        return false;
                    }
                }
                if let Some(ref provider) = filters.model_provider {
                    if s.model_provider != *provider {
                        return false;
                    }
                }
                if let Some(ref level) = filters.verification_level {
                    if (s.verification_level as u8) < (*level as u8) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        let total = submissions.len() as u64;
        let offset = pagination.offset() as usize;
        let limit = pagination.limit() as usize;

        let items = submissions
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok((items, total))
    }

    async fn update(&self, id: &str, update: &UpdateSubmissionData) -> Result<(), ApplicationError> {
        let mut submissions = self.submissions.write();
        if let Some(submission) = submissions.get_mut(id) {
            if let Some(ref vis) = update.visibility {
                submission.visibility = vis.clone();
            }
            submission.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ApplicationError::NotFound(format!("Submission not found: {}", id)))
        }
    }

    async fn update_verification(
        &self,
        id: &str,
        verification: &VerificationData,
    ) -> Result<(), ApplicationError> {
        let mut submissions = self.submissions.write();
        if let Some(submission) = submissions.get_mut(id) {
            submission.verification_level = verification.level.clone();
            submission.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ApplicationError::NotFound(format!("Submission not found: {}", id)))
        }
    }

    async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        self.submissions.write().remove(id);
        self.results.write().remove(id);
        Ok(())
    }

    async fn get_leaderboard(
        &self,
        benchmark_id: &str,
        _version_id: Option<&str>,
        limit: u32,
        _min_verification: Option<VerificationLevel>,
    ) -> Result<Vec<LeaderboardEntryDto>, ApplicationError> {
        let mut entries: Vec<_> = self.submissions.read()
            .values()
            .filter(|s| s.benchmark_id == benchmark_id)
            .cloned()
            .collect();

        entries.sort_by(|a, b| b.aggregate_score.partial_cmp(&a.aggregate_score).unwrap_or(std::cmp::Ordering::Equal));

        let entries = entries
            .into_iter()
            .take(limit as usize)
            .enumerate()
            .map(|(i, s)| LeaderboardEntryDto {
                rank: (i + 1) as u32,
                submission_id: s.id,
                model_provider: s.model_provider,
                model_name: s.model_name,
                model_version: s.model_version,
                aggregate_score: s.aggregate_score,
                verification_level: s.verification_level,
                submitter_name: s.submitter_id,
                submitted_at: s.created_at,
            })
            .collect();

        Ok(entries)
    }

    async fn get_user_submissions(
        &self,
        user_id: &str,
        pagination: &Pagination,
    ) -> Result<(Vec<SubmissionDto>, u64), ApplicationError> {
        let submissions: Vec<_> = self.submissions.read()
            .values()
            .filter(|s| s.submitter_id == user_id)
            .cloned()
            .collect();

        let total = submissions.len() as u64;
        let offset = pagination.offset() as usize;
        let limit = pagination.limit() as usize;

        let items = submissions
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        Ok((items, total))
    }

    async fn get_results(&self, id: &str) -> Result<Option<SubmissionResults>, ApplicationError> {
        Ok(self.results.read().get(id).cloned())
    }

    async fn save_results(&self, id: &str, results: &SubmissionResults) -> Result<(), ApplicationError> {
        self.results.write().insert(id.to_string(), results.clone());
        Ok(())
    }
}

/// In-memory user repository for development
pub struct InMemoryUserRepository {
    users: RwLock<HashMap<String, UserDto>>,
    passwords: RwLock<HashMap<String, String>>,
    api_keys: RwLock<HashMap<String, Vec<ApiKeyDto>>>,
    api_key_secrets: RwLock<HashMap<String, (String, Vec<String>)>>,
}

impl InMemoryUserRepository {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::new()),
            passwords: RwLock::new(HashMap::new()),
            api_keys: RwLock::new(HashMap::new()),
            api_key_secrets: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserRepositoryPort for InMemoryUserRepository {
    async fn create(&self, data: &CreateUserData) -> Result<String, ApplicationError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now();

        let user = UserDto {
            id: id.clone(),
            email: data.email.clone(),
            username: data.username.clone(),
            display_name: data.display_name.clone(),
            bio: None,
            website: None,
            avatar_url: None,
            is_verified: false,
            is_admin: false,
            created_at: now,
            updated_at: now,
        };

        self.users.write().insert(id.clone(), user);
        if let Some(ref hash) = data.password_hash {
            self.passwords.write().insert(id.clone(), hash.clone());
        }

        Ok(id)
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<UserDto>, ApplicationError> {
        Ok(self.users.read().get(id).cloned())
    }

    async fn get_by_email(&self, email: &str) -> Result<Option<UserDto>, ApplicationError> {
        Ok(self.users.read().values().find(|u| u.email == email).cloned())
    }

    async fn get_by_username(&self, username: &str) -> Result<Option<UserDto>, ApplicationError> {
        Ok(self.users.read().values().find(|u| u.username == username).cloned())
    }

    async fn update(&self, id: &str, update: &UpdateUserData) -> Result<(), ApplicationError> {
        let mut users = self.users.write();
        if let Some(user) = users.get_mut(id) {
            if let Some(ref name) = update.display_name {
                user.display_name = name.clone();
            }
            if let Some(ref bio) = update.bio {
                user.bio = Some(bio.clone());
            }
            if let Some(ref website) = update.website {
                user.website = Some(website.clone());
            }
            if let Some(ref avatar) = update.avatar_url {
                user.avatar_url = Some(avatar.clone());
            }
            user.updated_at = chrono::Utc::now();
            Ok(())
        } else {
            Err(ApplicationError::NotFound(format!("User not found: {}", id)))
        }
    }

    async fn update_password(&self, id: &str, password_hash: &str) -> Result<(), ApplicationError> {
        self.passwords.write().insert(id.to_string(), password_hash.to_string());
        Ok(())
    }

    async fn verify_password(&self, id: &str, password: &str) -> Result<bool, ApplicationError> {
        let passwords = self.passwords.read();
        if let Some(hash) = passwords.get(id) {
            // Simple comparison for in-memory (real impl would use argon2)
            Ok(hash == &format!("argon2:${}", password))
        } else {
            Ok(false)
        }
    }

    async fn delete(&self, id: &str) -> Result<(), ApplicationError> {
        self.users.write().remove(id);
        self.passwords.write().remove(id);
        self.api_keys.write().remove(id);
        Ok(())
    }

    async fn get_profile(&self, id: &str) -> Result<Option<UserProfileDto>, ApplicationError> {
        Ok(self.users.read().get(id).map(|u| UserProfileDto {
            id: u.id.clone(),
            username: u.username.clone(),
            display_name: u.display_name.clone(),
            bio: u.bio.clone(),
            website: u.website.clone(),
            avatar_url: u.avatar_url.clone(),
            submission_count: 0,
            benchmark_count: 0,
            joined_at: u.created_at,
        }))
    }

    async fn email_exists(&self, email: &str) -> Result<bool, ApplicationError> {
        Ok(self.users.read().values().any(|u| u.email == email))
    }

    async fn username_exists(&self, username: &str) -> Result<bool, ApplicationError> {
        Ok(self.users.read().values().any(|u| u.username == username))
    }

    async fn create_api_key(&self, user_id: &str, data: &CreateApiKeyData) -> Result<ApiKeyWithSecretDto, ApplicationError> {
        let id = uuid::Uuid::new_v4().to_string();
        let secret = format!("llm_bm_{}_{}", user_id, uuid::Uuid::new_v4());
        let now = chrono::Utc::now();

        let key = ApiKeyDto {
            id: id.clone(),
            name: data.name.clone(),
            description: data.description.clone(),
            scopes: data.scopes.clone(),
            last_used_at: None,
            expires_at: data.expires_in_days.map(|d| now + chrono::Duration::days(d as i64)),
            created_at: now,
        };

        self.api_keys
            .write()
            .entry(user_id.to_string())
            .or_default()
            .push(key.clone());

        self.api_key_secrets
            .write()
            .insert(secret.clone(), (user_id.to_string(), data.scopes.clone()));

        Ok(ApiKeyWithSecretDto { key, secret })
    }

    async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKeyDto>, ApplicationError> {
        Ok(self.api_keys.read().get(user_id).cloned().unwrap_or_default())
    }

    async fn revoke_api_key(&self, user_id: &str, key_id: &str) -> Result<(), ApplicationError> {
        let mut keys = self.api_keys.write();
        if let Some(user_keys) = keys.get_mut(user_id) {
            user_keys.retain(|k| k.id != key_id);
        }
        Ok(())
    }

    async fn verify_api_key(&self, key_secret: &str) -> Result<Option<(String, Vec<String>)>, ApplicationError> {
        Ok(self.api_key_secrets.read().get(key_secret).cloned())
    }
}
