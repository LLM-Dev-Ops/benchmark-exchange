//! Benchmark validation rules

use super::{Validatable, ValidationResult, ValidationRules};
use llm_benchmark_domain::benchmark::{BenchmarkCategory, BenchmarkMetadata, BenchmarkStatus};
use serde::{Deserialize, Serialize};

/// Create benchmark request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBenchmarkRequest {
    pub name: String,
    pub slug: String,
    pub description: String,
    pub category: BenchmarkCategory,
    pub tags: Vec<String>,
    pub version: String,
}

impl CreateBenchmarkRequest {
    pub const MAX_NAME_LENGTH: usize = 200;
    pub const MAX_DESCRIPTION_LENGTH: usize = 5000;
    pub const MAX_TAGS: usize = 20;
}

impl Validatable for CreateBenchmarkRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Name validation
        let name_result = ValidationRules::validate_length(
            &self.name,
            "name",
            Some(3),
            Some(Self::MAX_NAME_LENGTH),
        );
        result.merge(name_result);

        // Slug validation
        let slug_result = ValidationRules::validate_slug(&self.slug);
        result.merge(slug_result);

        // Description validation
        let desc_result = ValidationRules::validate_length(
            &self.description,
            "description",
            Some(10),
            Some(Self::MAX_DESCRIPTION_LENGTH),
        );
        result.merge(desc_result);

        // Tags validation
        let tags_result = ValidationRules::validate_list_size(
            &self.tags,
            "tags",
            None,
            Some(Self::MAX_TAGS),
        );
        result.merge(tags_result);

        // Validate individual tags
        for (i, tag) in self.tags.iter().enumerate() {
            if tag.len() > 50 {
                result.add_field_error(
                    format!("tags[{}]", i),
                    "Tag must be 50 characters or less",
                );
            }
        }

        // Version validation
        let version_result = ValidationRules::validate_semver(&self.version);
        result.merge(version_result);

        result
    }
}

/// Update benchmark request validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBenchmarkRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub long_description: Option<String>,
}

impl Validatable for UpdateBenchmarkRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        if let Some(ref name) = self.name {
            let name_result = ValidationRules::validate_length(
                name,
                "name",
                Some(3),
                Some(CreateBenchmarkRequest::MAX_NAME_LENGTH),
            );
            result.merge(name_result);
        }

        if let Some(ref description) = self.description {
            let desc_result = ValidationRules::validate_length(
                description,
                "description",
                Some(10),
                Some(CreateBenchmarkRequest::MAX_DESCRIPTION_LENGTH),
            );
            result.merge(desc_result);
        }

        if let Some(ref tags) = self.tags {
            let tags_result = ValidationRules::validate_list_size(
                tags,
                "tags",
                None,
                Some(CreateBenchmarkRequest::MAX_TAGS),
            );
            result.merge(tags_result);
        }

        if let Some(ref long_desc) = self.long_description {
            let long_desc_result = ValidationRules::validate_length(
                long_desc,
                "long_description",
                None,
                Some(50000),
            );
            result.merge(long_desc_result);
        }

        result
    }
}

/// Benchmark status transition validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusTransitionRequest {
    pub current_status: BenchmarkStatus,
    pub target_status: BenchmarkStatus,
    pub reason: Option<String>,
}

impl Validatable for StatusTransitionRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Validate the transition is allowed
        if !self.current_status.can_transition_to(self.target_status) {
            result.add_object_error(format!(
                "Cannot transition from {:?} to {:?}",
                self.current_status, self.target_status
            ));
        }

        // Require reason for certain transitions
        match (self.current_status, self.target_status) {
            (BenchmarkStatus::UnderReview, BenchmarkStatus::Draft) => {
                if self.reason.is_none() || self.reason.as_ref().map(|r| r.is_empty()).unwrap_or(true) {
                    result.add_field_error(
                        "reason",
                        "Reason is required when rejecting a benchmark",
                    );
                }
            }
            (BenchmarkStatus::Active, BenchmarkStatus::Deprecated) => {
                if self.reason.is_none() || self.reason.as_ref().map(|r| r.is_empty()).unwrap_or(true) {
                    result.add_field_error(
                        "reason",
                        "Reason is required when deprecating a benchmark",
                    );
                }
            }
            _ => {}
        }

        // Validate reason length if provided
        if let Some(ref reason) = self.reason {
            let reason_result = ValidationRules::validate_length(
                reason,
                "reason",
                Some(10),
                Some(1000),
            );
            result.merge(reason_result);
        }

        result
    }
}

/// Benchmark version creation validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVersionRequest {
    pub version: String,
    pub changelog: String,
    pub breaking_changes: bool,
    pub migration_notes: Option<String>,
}

impl Validatable for CreateVersionRequest {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Version validation
        let version_result = ValidationRules::validate_semver(&self.version);
        result.merge(version_result);

        // Changelog validation
        let changelog_result = ValidationRules::validate_length(
            &self.changelog,
            "changelog",
            Some(10),
            Some(10000),
        );
        result.merge(changelog_result);

        // Migration notes required for breaking changes
        if self.breaking_changes {
            match &self.migration_notes {
                None => {
                    result.add_field_error(
                        "migration_notes",
                        "Migration notes are required for breaking changes",
                    );
                }
                Some(notes) if notes.is_empty() => {
                    result.add_field_error(
                        "migration_notes",
                        "Migration notes are required for breaking changes",
                    );
                }
                Some(notes) => {
                    let notes_result = ValidationRules::validate_length(
                        notes,
                        "migration_notes",
                        Some(20),
                        Some(10000),
                    );
                    result.merge(notes_result);
                }
            }
        }

        result
    }
}

/// Benchmark metadata validation
impl Validatable for BenchmarkMetadata {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Name validation
        let name_result = ValidationRules::validate_length(
            &self.name,
            "name",
            Some(3),
            Some(200),
        );
        result.merge(name_result);

        // Slug validation
        let slug_result = ValidationRules::validate_slug(&self.slug);
        result.merge(slug_result);

        // Description validation
        let desc_result = ValidationRules::validate_length(
            &self.description,
            "description",
            Some(10),
            Some(5000),
        );
        result.merge(desc_result);

        // Documentation URL validation
        if let Some(ref url) = self.documentation_url {
            let url_result = ValidationRules::validate_url(url.as_str());
            result.merge(url_result);
        }

        // Source URL validation
        if let Some(ref url) = self.source_url {
            let url_result = ValidationRules::validate_url(url.as_str());
            result.merge(url_result);
        }

        // Maintainers validation
        if self.maintainers.is_empty() {
            result.add_field_error("maintainers", "At least one maintainer is required");
        }

        result
    }
}

/// Benchmark query filters validation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BenchmarkQueryFilters {
    pub category: Option<BenchmarkCategory>,
    pub status: Option<BenchmarkStatus>,
    pub tags: Option<Vec<String>>,
    pub search: Option<String>,
    pub maintainer_id: Option<String>,
}

impl Validatable for BenchmarkQueryFilters {
    fn validate_all(&self) -> ValidationResult {
        let mut result = ValidationResult::success();

        // Validate search query if provided
        if let Some(ref search) = self.search {
            if search.len() < 2 {
                result.add_field_error("search", "Search query must be at least 2 characters");
            }
            if search.len() > 200 {
                result.add_field_error("search", "Search query must be 200 characters or less");
            }
        }

        // Validate maintainer ID if provided
        if let Some(ref maintainer_id) = self.maintainer_id {
            let id_result = ValidationRules::validate_uuid(maintainer_id, "maintainer_id");
            result.merge(id_result);
        }

        // Validate tags if provided
        if let Some(ref tags) = self.tags {
            if tags.len() > 10 {
                result.add_field_error("tags", "Maximum 10 tags for filtering");
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_benchmark_validation() {
        let valid = CreateBenchmarkRequest {
            name: "Test Benchmark".to_string(),
            slug: "test-benchmark".to_string(),
            description: "A test benchmark for validation".to_string(),
            category: BenchmarkCategory::Accuracy,
            tags: vec!["test".to_string()],
            version: "1.0.0".to_string(),
        };
        assert!(valid.validate_all().valid);

        let invalid_slug = CreateBenchmarkRequest {
            name: "Test Benchmark".to_string(),
            slug: "Invalid Slug!".to_string(),
            description: "A test benchmark for validation".to_string(),
            category: BenchmarkCategory::Accuracy,
            tags: vec!["test".to_string()],
            version: "1.0.0".to_string(),
        };
        assert!(!invalid_slug.validate_all().valid);

        let invalid_version = CreateBenchmarkRequest {
            name: "Test Benchmark".to_string(),
            slug: "test-benchmark".to_string(),
            description: "A test benchmark for validation".to_string(),
            category: BenchmarkCategory::Accuracy,
            tags: vec!["test".to_string()],
            version: "invalid".to_string(),
        };
        assert!(!invalid_version.validate_all().valid);
    }

    #[test]
    fn test_status_transition_validation() {
        let valid = StatusTransitionRequest {
            current_status: BenchmarkStatus::Draft,
            target_status: BenchmarkStatus::UnderReview,
            reason: None,
        };
        assert!(valid.validate_all().valid);

        let invalid_transition = StatusTransitionRequest {
            current_status: BenchmarkStatus::Draft,
            target_status: BenchmarkStatus::Active,
            reason: None,
        };
        assert!(!invalid_transition.validate_all().valid);

        let missing_reason = StatusTransitionRequest {
            current_status: BenchmarkStatus::Active,
            target_status: BenchmarkStatus::Deprecated,
            reason: None,
        };
        assert!(!missing_reason.validate_all().valid);
    }

    #[test]
    fn test_create_version_validation() {
        let valid = CreateVersionRequest {
            version: "2.0.0".to_string(),
            changelog: "Major changes to the benchmark methodology".to_string(),
            breaking_changes: false,
            migration_notes: None,
        };
        assert!(valid.validate_all().valid);

        let breaking_without_notes = CreateVersionRequest {
            version: "2.0.0".to_string(),
            changelog: "Breaking changes to the API".to_string(),
            breaking_changes: true,
            migration_notes: None,
        };
        assert!(!breaking_without_notes.validate_all().valid);

        let breaking_with_notes = CreateVersionRequest {
            version: "2.0.0".to_string(),
            changelog: "Breaking changes to the API".to_string(),
            breaking_changes: true,
            migration_notes: Some("Migrate by updating your test cases to use the new format".to_string()),
        };
        assert!(breaking_with_notes.validate_all().valid);
    }
}
