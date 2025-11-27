# API Usage Examples

This document provides curl examples for interacting with the LLM Benchmark Exchange REST API.

## Prerequisites

Start the server:

```bash
cargo run --example simple_server
```

## Environment Variables

```bash
export API_URL="http://localhost:8080"
export TOKEN="" # Will be set after login
```

## Health Checks

### Health Check

```bash
curl -X GET "$API_URL/health"
```

Response:
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "version": "0.1.0"
  }
}
```

### Readiness Check

```bash
curl -X GET "$API_URL/ready"
```

## Authentication

### Register

```bash
curl -X POST "$API_URL/api/v1/auth/register" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "username": "testuser",
    "password": "securepassword123"
  }'
```

Response:
```json
{
  "token": "eyJ0eXAiOiJKV1QiLCJhbGc...",
  "user": {
    "id": "...",
    "email": "user@example.com",
    "username": "testuser",
    "role": "registered",
    "created_at": "2024-01-01T00:00:00Z",
    "email_verified": false
  },
  "expires_at": "2024-01-02T00:00:00Z"
}
```

Save the token:
```bash
export TOKEN="<token-from-response>"
```

### Login

```bash
curl -X POST "$API_URL/api/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "securepassword123"
  }'
```

## User Management

### Get Current User

```bash
curl -X GET "$API_URL/api/v1/users/me" \
  -H "Authorization: Bearer $TOKEN"
```

### Update Profile

```bash
curl -X PUT "$API_URL/api/v1/users/me" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "Test User",
    "bio": "AI/ML enthusiast",
    "affiliation": "Example University",
    "website": "https://example.com"
  }'
```

### Get User by ID

```bash
curl -X GET "$API_URL/api/v1/users/{user_id}"
```

## Benchmarks

### List Benchmarks

```bash
curl -X GET "$API_URL/api/v1/benchmarks?page=1&per_page=20&sort_by=created_at&sort_dir=desc"
```

### Create Benchmark

```bash
curl -X POST "$API_URL/api/v1/benchmarks" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Benchmark",
    "slug": "my-benchmark",
    "category": "Performance",
    "description": "A comprehensive performance benchmark for LLMs",
    "version": "1.0.0"
  }'
```

### Get Benchmark

```bash
curl -X GET "$API_URL/api/v1/benchmarks/{benchmark_id}"
```

### Get Benchmark by Slug

```bash
curl -X GET "$API_URL/api/v1/benchmarks/slug/my-benchmark"
```

### Update Benchmark

```bash
curl -X PUT "$API_URL/api/v1/benchmarks/{benchmark_id}" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Updated Benchmark Name",
    "description": "Updated description"
  }'
```

### Submit for Review

```bash
curl -X POST "$API_URL/api/v1/benchmarks/{benchmark_id}/submit-for-review" \
  -H "Authorization: Bearer $TOKEN"
```

### Approve Benchmark (Reviewer role required)

```bash
curl -X POST "$API_URL/api/v1/benchmarks/{benchmark_id}/approve" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Meets all quality criteria"
  }'
```

### Reject Benchmark (Reviewer role required)

```bash
curl -X POST "$API_URL/api/v1/benchmarks/{benchmark_id}/reject" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Needs more test cases"
  }'
```

## Submissions

### Submit Results

```bash
curl -X POST "$API_URL/api/v1/benchmarks/{benchmark_id}/submissions" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model_name": "GPT-4",
    "model_version": "1.0.0",
    "results": {
      "scores": {
        "accuracy": 0.95,
        "latency_ms": 150
      }
    },
    "metadata": {
      "hardware": "A100",
      "batch_size": 32
    }
  }'
```

### Get Submission

```bash
curl -X GET "$API_URL/api/v1/submissions/{submission_id}"
```

### List Benchmark Submissions

```bash
curl -X GET "$API_URL/api/v1/benchmarks/{benchmark_id}/submissions?page=1&per_page=20"
```

### Request Verification

```bash
curl -X POST "$API_URL/api/v1/submissions/{submission_id}/request-verification" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "notes": "Please verify these results"
  }'
```

### Update Visibility

```bash
curl -X PATCH "$API_URL/api/v1/submissions/{submission_id}/visibility" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "visibility": "Public"
  }'
```

## Leaderboards

### Get Benchmark Leaderboard

```bash
curl -X GET "$API_URL/api/v1/benchmarks/{benchmark_id}/leaderboard?page=1&per_page=20"
```

### Get Category Leaderboard

```bash
curl -X GET "$API_URL/api/v1/categories/Performance/leaderboard?page=1&per_page=20"
```

### Compare Models

```bash
curl -X GET "$API_URL/api/v1/models/compare?models={model_id_1},{model_id_2},{model_id_3}"
```

### Get Model History

```bash
curl -X GET "$API_URL/api/v1/models/{model_id}/history?page=1&per_page=20"
```

## Governance

### List Proposals

```bash
curl -X GET "$API_URL/api/v1/proposals?page=1&per_page=20&status=Active"
```

### Create Proposal

```bash
curl -X POST "$API_URL/api/v1/proposals" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Add new benchmark category",
    "description": "Propose adding a new category for multilingual benchmarks",
    "proposal_type": "BenchmarkChange",
    "voting_duration_days": 7
  }'
```

### Get Proposal

```bash
curl -X GET "$API_URL/api/v1/proposals/{proposal_id}"
```

### Vote on Proposal

```bash
curl -X POST "$API_URL/api/v1/proposals/{proposal_id}/vote" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "vote": "For",
    "comment": "Great idea, fully support this"
  }'
```

### Add Comment

```bash
curl -X POST "$API_URL/api/v1/proposals/{proposal_id}/comments" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "This is a well-thought-out proposal"
  }'
```

## Error Handling

All errors follow the same format:

```json
{
  "error": "ERROR_CODE",
  "message": "Human-readable error message",
  "request_id": "abc-123"
}
```

Common error codes:
- `UNAUTHORIZED` - Authentication required or invalid token
- `VALIDATION_ERROR` - Request validation failed
- `NOT_FOUND` - Resource not found
- `BAD_REQUEST` - Invalid request
- `RATE_LIMIT_EXCEEDED` - Too many requests

## Pagination

All list endpoints support pagination with the following query parameters:

- `page` - Page number (1-indexed, default: 1)
- `per_page` - Items per page (default: 20, max: 100)
- `sort_by` - Field to sort by
- `sort_dir` - Sort direction (`asc` or `desc`)

Example response:
```json
{
  "items": [...],
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total": 150,
    "total_pages": 8,
    "has_next": true,
    "has_prev": false
  }
}
```

## Rate Limiting

The API implements rate limiting:
- Default: 60 requests per minute per IP
- Rate limit headers are included in responses
- Exceeding the limit returns HTTP 429

## Testing with jq

For prettier JSON output, pipe curl responses through jq:

```bash
curl -X GET "$API_URL/health" | jq
```

## Postman Collection

Import the endpoints into Postman:
1. Create a new collection
2. Set base URL variable: `{{api_url}}` = `http://localhost:8080`
3. Set authorization token variable: `{{token}}`
4. Add requests from examples above
