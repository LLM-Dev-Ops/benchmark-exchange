# Application Crate - LLM Benchmark Exchange

This crate contains the business logic layer for the LLM Benchmark Exchange platform.

## Overview

The application layer orchestrates domain logic and coordinates between different layers of the system. It implements use cases and business workflows using services that depend on repository interfaces (implemented in the infrastructure layer).

## Structure

```
application/
├── src/
│   ├── dto/              # Data Transfer Objects for API layer
│   │   └── mod.rs        # All request/response types
│   ├── services/         # Business logic services
│   │   ├── mod.rs        # Service exports
│   │   ├── benchmark_service.rs
│   │   ├── submission_service.rs
│   │   ├── verification_service.rs
│   │   ├── leaderboard_service.rs
│   │   ├── governance_service.rs
│   │   └── user_service.rs
│   ├── scoring/          # Scoring engine
│   │   ├── mod.rs        # Scoring engine implementation
│   │   └── evaluators/   # Evaluator implementations
│   │       ├── mod.rs
│   │       ├── exact_match.rs
│   │       ├── fuzzy_match.rs
│   │       └── regex_match.rs
│   ├── validation/       # Validation logic
│   │   ├── mod.rs
│   │   ├── benchmark_validator.rs
│   │   └── submission_validator.rs
│   └── lib.rs            # Main library file
└── Cargo.toml
```

## Services

### BenchmarkService

Handles all benchmark-related operations:

- **create_benchmark()** - Create new benchmark with validation
- **get_benchmark()** - Retrieve benchmark by ID (with caching)
- **update_benchmark()** - Update benchmark (requires ownership or moderator)
- **submit_for_review()** - Submit benchmark for moderator approval
- **approve_benchmark()** - Approve benchmark (moderator only)
- **reject_benchmark()** - Reject benchmark (moderator only)
- **deprecate_benchmark()** - Deprecate active benchmark (moderator only)
- **list_benchmarks()** - List benchmarks with filtering and pagination
- **validate_definition()** - Validate benchmark definition

**Features:**
- Authorization checks based on user roles
- Event publishing for all state changes
- Cache invalidation on updates
- Comprehensive validation

### SubmissionService

Manages benchmark result submissions:

- **submit_results()** - Submit benchmark results with validation
- **get_submission()** - Retrieve submission by ID
- **list_submissions()** - List submissions with filtering
- **request_verification()** - Request verification for a submission
- **update_visibility()** - Change submission visibility

**Features:**
- Duplicate detection
- Score calculation using scoring engine
- Leaderboard cache invalidation
- Access control based on visibility settings

### VerificationService

Handles submission verification workflows:

- **start_verification()** - Initiate verification process
- **get_verification_status()** - Check verification status
- **complete_verification()** - Mark verification complete (admin/moderator)
- **submit_community_verification()** - Submit community verification

**Features:**
- Automated verification integration
- Community verification support
- Official verification tracking
- Async verification engine integration

### LeaderboardService

Manages rankings and model comparisons:

- **get_leaderboard()** - Get ranked submissions for a benchmark
- **get_category_leaderboard()** - Get category-specific rankings
- **compare_models()** - Compare multiple model submissions
- **get_ranking_history()** - Get historical ranking data

**Features:**
- Caching with TTL (5 minutes for lists, longer for history)
- Score-based ranking
- Anonymous submission support
- Strengths/weaknesses analysis

### GovernanceService

Manages proposals and voting:

- **create_proposal()** - Create governance proposal
- **get_proposal()** - Get proposal with votes and comments
- **list_proposals()** - List proposals with filtering
- **cast_vote()** - Cast vote on active proposal
- **finalize_proposal()** - Finalize voting and execute result
- **add_review_comment()** - Add comment to proposal

**Features:**
- Quorum checking (configurable percentage)
- Approval threshold enforcement (default 66%)
- Voting period management (3-30 days)
- Vote tracking and result calculation

### UserService

Handles user management and authentication:

- **register()** - Register new user with password hashing
- **login()** - Authenticate and generate JWT token
- **get_user()** - Retrieve user profile
- **update_profile()** - Update user profile
- **change_role()** - Change user role (admin only)
- **validate_token()** - Validate JWT token

**Features:**
- Argon2 password hashing
- JWT token generation and validation
- Role-based access control
- User reputation tracking

## Scoring Engine

The scoring engine provides:

- **aggregate_scores()** - Calculate weighted average from test case results
- **normalize_score()** - Normalize scores to 0-100 range
- **compute_confidence_interval()** - Calculate statistical confidence intervals

### Evaluators

1. **ExactMatchEvaluator** - Exact JSON comparison with normalization
2. **FuzzyMatchEvaluator** - Fuzzy string matching using Levenshtein distance
3. **RegexMatchEvaluator** - Pattern matching using regular expressions

## Validation

### BenchmarkDefinitionValidator

Validates benchmark definitions:
- Test case completeness and uniqueness
- Weight validation
- Pass threshold ranges
- Time limit reasonableness
- Metadata structure

### SubmissionValidator

Validates submissions:
- Model name/version presence
- Complete test case coverage
- Score ranges (0.0-1.0)
- Execution time reasonableness
- Output consistency

## Data Transfer Objects (DTOs)

### Benchmark DTOs
- CreateBenchmarkRequest
- UpdateBenchmarkRequest
- BenchmarkResponse
- BenchmarkDetailResponse
- BenchmarkListResponse

### Submission DTOs
- SubmitResultsRequest
- SubmissionResponse
- SubmissionDetailResponse
- SubmissionListResponse

### Verification DTOs
- VerificationRequest
- VerificationResponse
- CommunityVerificationRequest

### Leaderboard DTOs
- LeaderboardRequest
- LeaderboardResponse
- CompareModelsRequest
- CompareModelsResponse

### Governance DTOs
- CreateProposalRequest
- ProposalResponse
- ProposalDetailResponse
- CastVoteRequest
- VoteResponse
- AddReviewCommentRequest

### User DTOs
- RegisterRequest
- LoginRequest
- LoginResponse
- UserResponse
- UpdateProfileRequest
- ChangeRoleRequest

## Error Handling

The application layer defines `ApplicationError` enum with variants:
- **NotFound** - Resource not found
- **Unauthorized** - Authentication required
- **Forbidden** - Insufficient permissions
- **InvalidInput** - Invalid request data
- **ValidationFailed** - Validation errors
- **Conflict** - Resource conflict (e.g., duplicate)
- **Internal** - Internal server error
- **ServiceUnavailable** - External service unavailable

## Authentication Context

All services accept `AuthenticatedUser` context containing:
- user_id
- username
- email
- role (Admin, Moderator, Contributor, User)

## Role-Based Permissions

- **Admin**: Full access, user management
- **Moderator**: Approve benchmarks, moderate content
- **Contributor**: Create benchmarks, vote on proposals
- **User**: Submit results, view public content

## Dependencies

Services depend on repository traits (defined but implemented in infrastructure):
- BenchmarkRepository
- SubmissionRepository
- VerificationRepository
- ProposalRepository
- VoteRepository
- CommentRepository
- UserRepository
- EventPublisher
- CacheService
- VerificationEngine
- PasswordHasher

## Testing

Each module includes comprehensive unit tests:
- Service method testing with mock repositories
- Validation logic testing
- Scoring engine accuracy tests
- Evaluator correctness tests

## Usage Example

```rust
use application::services::BenchmarkService;
use application::dto::CreateBenchmarkRequest;
use application::AuthenticatedUser;

async fn create_benchmark_example(
    service: Arc<dyn BenchmarkService>,
    user: AuthenticatedUser,
) -> Result<(), ApplicationError> {
    let request = CreateBenchmarkRequest {
        name: "My Benchmark".to_string(),
        description: "A test benchmark".to_string(),
        // ... other fields
    };

    let benchmark = service.create_benchmark(request, &user).await?;
    println!("Created benchmark: {}", benchmark.benchmark.id);

    Ok(())
}
```

## Configuration

Services accept configuration structs:
- **JwtConfig** - JWT token settings (secret, expiration, issuer)
- **GovernanceConfig** - Governance rules (quorum, threshold, voting periods)

## Event Publishing

All state changes publish domain events:
- BenchmarkCreated, BenchmarkUpdated, BenchmarkApproved, etc.
- SubmissionCreated, VerificationRequested, etc.
- ProposalCreated, VoteCast, ProposalFinalized
- UserRegistered, UserLoggedIn, UserRoleChanged

Events enable:
- Audit logging
- Real-time notifications
- Event sourcing
- Integration with external systems
