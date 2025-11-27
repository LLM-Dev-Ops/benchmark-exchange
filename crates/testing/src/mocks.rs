//! Mock implementations for repositories and external services.
//!
//! Provides in-memory mocks for testing without database dependencies.

use llm_benchmark_domain::{
    governance::Proposal,
    identifiers::*,
    submission::Submission,
    user::User,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Mock event publisher for testing domain events
pub struct MockEventPublisher {
    pub published_events: Arc<RwLock<Vec<String>>>,
}

impl MockEventPublisher {
    pub fn new() -> Self {
        Self {
            published_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn publish(&self, event_type: impl Into<String>) {
        self.published_events.write().push(event_type.into());
    }

    pub fn get_published_events(&self) -> Vec<String> {
        self.published_events.read().clone()
    }

    pub fn clear(&self) {
        self.published_events.write().clear();
    }

    pub fn event_count(&self) -> usize {
        self.published_events.read().len()
    }
}

impl Default for MockEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock cache provider for testing caching logic
pub struct MockCacheProvider {
    cache: Arc<RwLock<HashMap<String, String>>>,
}

impl MockCacheProvider {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        self.cache.read().get(key).cloned()
    }

    pub async fn set(&self, key: String, value: String) {
        self.cache.write().insert(key, value);
    }

    pub async fn delete(&self, key: &str) {
        self.cache.write().remove(key);
    }

    pub async fn clear(&self) {
        self.cache.write().clear();
    }

    pub fn entry_count(&self) -> usize {
        self.cache.read().len()
    }
}

impl Default for MockCacheProvider {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock user repository for testing
pub struct MockUserRepository {
    users: Arc<RwLock<HashMap<UserId, User>>>,
    users_by_email: Arc<RwLock<HashMap<String, UserId>>>,
    users_by_username: Arc<RwLock<HashMap<String, UserId>>>,
}

impl MockUserRepository {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            users_by_email: Arc::new(RwLock::new(HashMap::new())),
            users_by_username: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create(&self, user: User) -> anyhow::Result<User> {
        let id = user.id;
        let email = user.email.clone();
        let username = user.username.clone();

        // Check for duplicates
        if self.users_by_email.read().contains_key(&email) {
            anyhow::bail!("Email already exists");
        }
        if self.users_by_username.read().contains_key(&username) {
            anyhow::bail!("Username already exists");
        }

        self.users.write().insert(id, user.clone());
        self.users_by_email.write().insert(email, id);
        self.users_by_username.write().insert(username, id);

        Ok(user)
    }

    pub async fn find_by_id(&self, id: &UserId) -> anyhow::Result<Option<User>> {
        Ok(self.users.read().get(id).cloned())
    }

    pub async fn find_by_email(&self, email: &str) -> anyhow::Result<Option<User>> {
        let user_id = self.users_by_email.read().get(email).copied();
        match user_id {
            Some(id) => self.find_by_id(&id).await,
            None => Ok(None),
        }
    }

    pub async fn find_by_username(&self, username: &str) -> anyhow::Result<Option<User>> {
        let user_id = self.users_by_username.read().get(username).copied();
        match user_id {
            Some(id) => self.find_by_id(&id).await,
            None => Ok(None),
        }
    }

    pub async fn update(&self, user: User) -> anyhow::Result<User> {
        let id = user.id;
        if !self.users.read().contains_key(&id) {
            anyhow::bail!("User not found");
        }
        self.users.write().insert(id, user.clone());
        Ok(user)
    }

    pub async fn delete(&self, id: &UserId) -> anyhow::Result<()> {
        if let Some(user) = self.users.write().remove(id) {
            self.users_by_email.write().remove(&user.email);
            self.users_by_username.write().remove(&user.username);
        }
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.users.read().len()
    }

    pub fn clear(&self) {
        self.users.write().clear();
        self.users_by_email.write().clear();
        self.users_by_username.write().clear();
    }
}

impl Default for MockUserRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock submission repository for testing
pub struct MockSubmissionRepository {
    submissions: Arc<RwLock<HashMap<SubmissionId, Submission>>>,
}

impl MockSubmissionRepository {
    pub fn new() -> Self {
        Self {
            submissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create(&self, submission: Submission) -> anyhow::Result<Submission> {
        let id = submission.id;
        self.submissions.write().insert(id, submission.clone());
        Ok(submission)
    }

    pub async fn find_by_id(&self, id: &SubmissionId) -> anyhow::Result<Option<Submission>> {
        Ok(self.submissions.read().get(id).cloned())
    }

    pub async fn find_by_benchmark(&self, benchmark_id: &BenchmarkId) -> anyhow::Result<Vec<Submission>> {
        Ok(self
            .submissions
            .read()
            .values()
            .filter(|s| s.benchmark_id == *benchmark_id)
            .cloned()
            .collect())
    }

    pub async fn update(&self, submission: Submission) -> anyhow::Result<Submission> {
        let id = submission.id;
        if !self.submissions.read().contains_key(&id) {
            anyhow::bail!("Submission not found");
        }
        self.submissions.write().insert(id, submission.clone());
        Ok(submission)
    }

    pub async fn delete(&self, id: &SubmissionId) -> anyhow::Result<()> {
        self.submissions.write().remove(id);
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.submissions.read().len()
    }

    pub fn clear(&self) {
        self.submissions.write().clear();
    }
}

impl Default for MockSubmissionRepository {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock proposal repository for testing
pub struct MockProposalRepository {
    proposals: Arc<RwLock<HashMap<ProposalId, Proposal>>>,
}

impl MockProposalRepository {
    pub fn new() -> Self {
        Self {
            proposals: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create(&self, proposal: Proposal) -> anyhow::Result<Proposal> {
        let id = proposal.id;
        self.proposals.write().insert(id, proposal.clone());
        Ok(proposal)
    }

    pub async fn find_by_id(&self, id: &ProposalId) -> anyhow::Result<Option<Proposal>> {
        Ok(self.proposals.read().get(id).cloned())
    }

    pub async fn update(&self, proposal: Proposal) -> anyhow::Result<Proposal> {
        let id = proposal.id;
        if !self.proposals.read().contains_key(&id) {
            anyhow::bail!("Proposal not found");
        }
        self.proposals.write().insert(id, proposal.clone());
        Ok(proposal)
    }

    pub async fn delete(&self, id: &ProposalId) -> anyhow::Result<()> {
        self.proposals.write().remove(id);
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.proposals.read().len()
    }

    pub fn clear(&self) {
        self.proposals.write().clear();
    }
}

impl Default for MockProposalRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::*;

    #[tokio::test]
    async fn test_mock_user_repository() {
        let repo = MockUserRepository::new();
        let user = create_test_user();

        let created = repo.create(user.clone()).await.unwrap();
        assert_eq!(created.id, user.id);

        let found = repo.find_by_id(&user.id).await.unwrap();
        assert!(found.is_some());

        let found_by_email = repo.find_by_email(&user.email).await.unwrap();
        assert!(found_by_email.is_some());

        assert_eq!(repo.count(), 1);

        repo.delete(&user.id).await.unwrap();
        assert_eq!(repo.count(), 0);
    }

    #[tokio::test]
    async fn test_mock_submission_repository() {
        let repo = MockSubmissionRepository::new();
        let submission = create_test_submission();

        let created = repo.create(submission.clone()).await.unwrap();
        assert_eq!(created.id, submission.id);

        let found = repo.find_by_id(&submission.id).await.unwrap();
        assert!(found.is_some());

        let by_benchmark = repo
            .find_by_benchmark(&submission.benchmark_id)
            .await
            .unwrap();
        assert_eq!(by_benchmark.len(), 1);

        assert_eq!(repo.count(), 1);
    }

    #[tokio::test]
    async fn test_mock_cache_provider() {
        let cache = MockCacheProvider::new();

        cache.set("key1".to_string(), "value1".to_string()).await;
        let value = cache.get("key1").await;
        assert_eq!(value, Some("value1".to_string()));

        assert_eq!(cache.entry_count(), 1);

        cache.delete("key1").await;
        assert_eq!(cache.entry_count(), 0);
    }

    #[test]
    fn test_mock_event_publisher() {
        let publisher = MockEventPublisher::new();

        publisher.publish("event1");
        publisher.publish("event2");

        assert_eq!(publisher.event_count(), 2);

        let events = publisher.get_published_events();
        assert_eq!(events, vec!["event1", "event2"]);

        publisher.clear();
        assert_eq!(publisher.event_count(), 0);
    }
}
