use serde_json::Value;
use thiserror::Error;

use crate::messages::Message;

use super::conditions::{
    ConditionError, EvaluationContext, evaluate, extract_domain, parse_condition,
};
use super::repositories::{DeterministicRuleError, DeterministicRuleRepository};
use super::types::{DeterministicRule, RuleScope, SafeMode};

#[derive(Debug, Clone)]
pub struct RuleMatch {
    pub rule: DeterministicRule,
    pub action_type: String,
    pub action_parameters: Value,
    pub safe_mode: SafeMode,
}

#[derive(Debug, Error)]
pub enum RuleLoaderError {
    #[error("failed to load rules: {0}")]
    Repository(#[from] DeterministicRuleError),
}

#[derive(Clone)]
pub struct RuleLoader {
    repo: DeterministicRuleRepository,
}

impl RuleLoader {
    pub fn new(repo: DeterministicRuleRepository) -> Self {
        Self { repo }
    }

    pub async fn load_applicable_rules(
        &self,
        org_id: i64,
        user_id: i64,
        account_id: &str,
        sender_email: Option<&str>,
    ) -> Result<Vec<DeterministicRule>, RuleLoaderError> {
        let mut rules = Vec::new();

        rules.extend(
            self.repo
                .list_enabled_by_scope(org_id, user_id, RuleScope::Global, None)
                .await?,
        );

        rules.extend(
            self.repo
                .list_enabled_by_scope(org_id, user_id, RuleScope::Account, Some(account_id))
                .await?,
        );

        if let Some(email) = sender_email {
            if let Some(domain) = extract_domain(email).map(str::to_lowercase) {
                rules.extend(
                    self.repo
                        .list_enabled_by_scope(org_id, user_id, RuleScope::Domain, Some(&domain))
                        .await?,
                );
            }

            rules.extend(
                self.repo
                    .list_enabled_by_scope(org_id, user_id, RuleScope::Sender, Some(email))
                    .await?,
            );
        }

        rules.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
                .then_with(|| a.id.cmp(&b.id))
        });

        Ok(rules)
    }
}

#[derive(Debug, Error)]
pub enum ExecutorError {
    #[error("rule loading failed: {0}")]
    RuleLoader(#[from] RuleLoaderError),
    #[error("condition evaluation failed: {0}")]
    Condition(#[from] ConditionError),
}

#[derive(Clone)]
pub struct RuleExecutor {
    loader: RuleLoader,
}

impl RuleExecutor {
    pub fn new(repo: DeterministicRuleRepository) -> Self {
        Self {
            loader: RuleLoader::new(repo),
        }
    }

    pub fn with_loader(loader: RuleLoader) -> Self {
        Self { loader }
    }

    pub async fn evaluate(
        &self,
        org_id: i64,
        user_id: i64,
        message: &Message,
    ) -> Result<Option<RuleMatch>, ExecutorError> {
        let mut ctx = EvaluationContext::new();
        let rules = self
            .loader
            .load_applicable_rules(
                org_id,
                user_id,
                &message.account_id,
                message.from_email.as_deref(),
            )
            .await?;

        for rule in rules {
            let condition = parse_condition(&rule.conditions_json)?;
            if evaluate(&condition, message, &mut ctx)? {
                return Ok(Some(RuleMatch {
                    action_type: rule.action_type.clone(),
                    action_parameters: rule.action_parameters_json.clone(),
                    safe_mode: rule.safe_mode.clone(),
                    rule,
                }));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
    use crate::db::Database;
    use crate::gmail::types::Header;
    use crate::messages::{Mailbox, Message};
    use crate::migrations::run_migrations;
    use crate::rules::types::NewDeterministicRule;
    use libsql::params;
    use tempfile::TempDir;
    use uuid::Uuid;

    async fn setup_executor() -> (RuleExecutor, DeterministicRuleRepository, Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");

        let repo = DeterministicRuleRepository::new(db.clone());
        let executor = RuleExecutor::new(repo.clone());

        (executor, repo, db, dir)
    }

    fn sample_message(account_id: &str, from_email: &str) -> Message {
        Message {
            id: "msg1".into(),
            account_id: account_id.to_string(),
            thread_id: "thread1".into(),
            provider_message_id: "provider1".into(),
            from_email: Some(from_email.to_string()),
            from_name: Some("Sender".into()),
            to: vec![Mailbox {
                email: "to@example.com".into(),
                name: None,
            }],
            cc: vec![],
            bcc: vec![],
            subject: Some("Your package has shipped".into()),
            snippet: None,
            received_at: None,
            internal_date: None,
            labels: vec!["INBOX".into()],
            headers: vec![Header {
                name: "Subject".into(),
                value: "Your package has shipped".into(),
            }],
            body_plain: None,
            body_html: None,
            raw_json: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
        }
    }

    fn new_rule(
        scope: RuleScope,
        scope_ref: Option<&str>,
        priority: i64,
        enabled: bool,
        conditions_json: Value,
    ) -> NewDeterministicRule {
        NewDeterministicRule {
            org_id: DEFAULT_ORG_ID,
            user_id: Some(DEFAULT_USER_ID),
            name: "rule".into(),
            description: None,
            scope,
            scope_ref: scope_ref.map(|s| s.to_string()),
            priority,
            enabled,
            conditions_json,
            action_type: "label".into(),
            action_parameters_json: serde_json::json!({"label": "Applied"}),
            safe_mode: SafeMode::Default,
        }
    }

    #[tokio::test]
    async fn executor_no_rules_returns_none() {
        let (executor, _repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@example.com");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn executor_single_matching_rule() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        repo.create(new_rule(
            RuleScope::Global,
            None,
            10,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        let matched = result.expect("should match");
        assert_eq!(matched.rule.scope, RuleScope::Global);
        assert_eq!(matched.action_type, "label");
        assert_eq!(matched.safe_mode, SafeMode::Default);
    }

    #[tokio::test]
    async fn executor_single_non_matching_rule() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        repo.create(new_rule(
            RuleScope::Global,
            None,
            10,
            true,
            serde_json::json!({"type": "subject_contains", "value": "invoice"}),
        ))
        .await
        .expect("create rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn executor_priority_ordering() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        let high_priority = repo
            .create(new_rule(
                RuleScope::Global,
                None,
                5,
                true,
                serde_json::json!({"type": "subject_contains", "value": "package"}),
            ))
            .await
            .expect("create high priority");

        repo.create(new_rule(
            RuleScope::Global,
            None,
            20,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create low priority");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        let matched = result.expect("should match");
        assert_eq!(matched.rule.id, high_priority.id);
    }

    #[tokio::test]
    async fn executor_first_match_stops() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        repo.create(new_rule(
            RuleScope::Global,
            None,
            1,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create first rule");

        repo.create(new_rule(
            RuleScope::Global,
            None,
            2,
            true,
            serde_json::json!({"type": "subject_regex", "value": "("}),
        ))
        .await
        .expect("create invalid regex rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");

        assert!(result.is_some(), "first rule should short-circuit");
    }

    #[tokio::test]
    async fn executor_global_scope_matches() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@example.com");

        repo.create(new_rule(
            RuleScope::Global,
            None,
            10,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn executor_account_scope_matches() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct-special", "alice@example.com");

        repo.create(new_rule(
            RuleScope::Account,
            Some("acct-special"),
            5,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create account rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn executor_domain_scope_matches() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        repo.create(new_rule(
            RuleScope::Domain,
            Some("amazon.com"),
            5,
            true,
            serde_json::json!({"type": "sender_domain", "value": "amazon.com"}),
        ))
        .await
        .expect("create domain rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn executor_domain_scope_matches_case_insensitive() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "Alice@Amazon.COM");

        repo.create(new_rule(
            RuleScope::Domain,
            Some("amazon.com"),
            5,
            true,
            serde_json::json!({"type": "sender_domain", "value": "amazon.com"}),
        ))
        .await
        .expect("create domain rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn executor_domain_scope_matches_with_uppercase_scope_ref_in_db() {
        let (executor, repo, db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        let rule = repo
            .create(new_rule(
                RuleScope::Domain,
                Some("amazon.com"),
                5,
                true,
                serde_json::json!({"type": "sender_domain", "value": "amazon.com"}),
            ))
            .await
            .expect("create domain rule");

        // Simulate legacy data stored with mixed-case scope_ref.
        let conn = db.connection().await.expect("connection");
        conn.execute(
            "UPDATE deterministic_rules SET scope_ref = 'Amazon.COM' WHERE id = ?1",
            params![rule.id],
        )
        .await
        .expect("uppercase scope_ref");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn executor_sender_scope_matches() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        repo.create(new_rule(
            RuleScope::Sender,
            Some("alice@amazon.com"),
            5,
            true,
            serde_json::json!({"type": "sender_email", "value": "alice@amazon.com"}),
        ))
        .await
        .expect("create sender rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn executor_sender_scope_matches_mixed_case_email() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "Alice@Amazon.com");

        repo.create(new_rule(
            RuleScope::Sender,
            Some("alice@amazon.com"),
            5,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create sender rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn executor_sender_scope_matches_with_uppercase_scope_ref_in_db() {
        let (executor, repo, db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        let rule = repo
            .create(new_rule(
                RuleScope::Sender,
                Some("alice@amazon.com"),
                5,
                true,
                serde_json::json!({"type": "subject_contains", "value": "package"}),
            ))
            .await
            .expect("create sender rule");

        // Simulate legacy data stored with mixed-case scope_ref.
        let conn = db.connection().await.expect("connection");
        conn.execute(
            "UPDATE deterministic_rules SET scope_ref = 'Alice@Amazon.COM' WHERE id = ?1",
            params![rule.id],
        )
        .await
        .expect("uppercase scope_ref");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        assert!(result.is_some());
    }

    #[tokio::test]
    async fn executor_scope_aggregation_prefers_specific() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        repo.create(new_rule(
            RuleScope::Global,
            None,
            50,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create global rule");

        repo.create(new_rule(
            RuleScope::Account,
            Some("acct1"),
            30,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create account rule");

        repo.create(new_rule(
            RuleScope::Domain,
            Some("amazon.com"),
            20,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create domain rule");

        let sender_rule = repo
            .create(new_rule(
                RuleScope::Sender,
                Some("alice@amazon.com"),
                10,
                true,
                serde_json::json!({"type": "subject_contains", "value": "package"}),
            ))
            .await
            .expect("create sender rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        let matched = result.expect("should match");
        assert_eq!(matched.rule.id, sender_rule.id);
    }

    #[tokio::test]
    async fn executor_disabled_rules_skipped() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        repo.create(new_rule(
            RuleScope::Global,
            None,
            5,
            false,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create disabled rule");

        let enabled_rule = repo
            .create(new_rule(
                RuleScope::Global,
                None,
                10,
                true,
                serde_json::json!({"type": "subject_contains", "value": "package"}),
            ))
            .await
            .expect("create enabled rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");
        let matched = result.expect("should match enabled rule");
        assert_eq!(matched.rule.id, enabled_rule.id);
    }

    #[tokio::test]
    async fn executor_invalid_condition_returns_error() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let message = sample_message("acct1", "alice@amazon.com");

        repo.create(new_rule(
            RuleScope::Global,
            None,
            1,
            true,
            serde_json::json!({"type": "subject_regex", "value": "("}),
        ))
        .await
        .expect("create rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await;

        assert!(matches!(
            result,
            Err(ExecutorError::Condition(
                ConditionError::InvalidRegex { .. }
            ))
        ));
    }

    #[tokio::test]
    async fn executor_matches_without_sender_email() {
        let (executor, repo, _db, _dir) = setup_executor().await;
        let mut message = sample_message("acct1", "alice@amazon.com");
        message.from_email = None;

        repo.create(new_rule(
            RuleScope::Account,
            Some("acct1"),
            5,
            true,
            serde_json::json!({"type": "subject_contains", "value": "package"}),
        ))
        .await
        .expect("create account rule");

        let result = executor
            .evaluate(DEFAULT_ORG_ID, DEFAULT_USER_ID, &message)
            .await
            .expect("execute");

        let matched = result.expect("should match account rule");
        assert_eq!(matched.rule.scope, RuleScope::Account);
    }
}
