use std::sync::Arc;

use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::accounts::AccountRepository;
use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::gmail::{GmailClient, NoopTokenStore};
use crate::jobs::{JobDispatcher, map_account_error, map_gmail_error};
use crate::labels::{LabelRepository, NewLabel};
use crate::rules::repositories::DeterministicRuleRepository;
use crate::{Job, JobError};

pub const JOB_TYPE: &str = "labels.sync.gmail";

#[derive(Debug, Deserialize)]
struct LabelsSyncPayload {
    account_id: String,
}

/// Handle the labels.sync.gmail job.
/// This job:
/// 1. Loads the account and creates a GmailClient
/// 2. Calls list_labels() to get all labels from Gmail
/// 3. Upserts all labels to the local database
/// 4. Detects deleted labels (labels in DB but not in API response)
/// 5. For deleted labels, finds and disables dependent rules
/// 6. Removes deleted labels from the database
pub async fn handle_labels_sync_gmail(
    dispatcher: &JobDispatcher,
    job: Job,
) -> Result<(), JobError> {
    let payload: LabelsSyncPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid labels.sync.gmail payload: {err}")))?;

    let account_repo = AccountRepository::new(dispatcher.db.clone());
    let label_repo = LabelRepository::new(dispatcher.db.clone());
    let rule_repo = DeterministicRuleRepository::new(dispatcher.db.clone());

    // Load and refresh account tokens
    let account = account_repo
        .refresh_tokens_if_needed(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            &payload.account_id,
            &dispatcher.http,
        )
        .await
        .map_err(|err| map_account_error("refresh account tokens", err))?;

    // Create Gmail client
    let client = GmailClient::new(
        dispatcher.http.clone(),
        account.email.clone(),
        account.config.client_id.clone(),
        account.config.client_secret.clone(),
        account.config.oauth.clone(),
        Arc::new(NoopTokenStore),
    )
    .with_api_base(
        dispatcher
            .gmail_api_base
            .clone()
            .unwrap_or_else(|| "https://gmail.googleapis.com/gmail/v1/users".to_string()),
    );

    // Fetch labels from Gmail API
    let response = client
        .list_labels()
        .await
        .map_err(|err| map_gmail_error("list_labels", err))?;

    info!(
        account_id = %payload.account_id,
        label_count = response.labels.len(),
        "fetched labels from Gmail"
    );

    // Collect provider label IDs from API response
    let api_provider_ids: Vec<&str> = response.labels.iter().map(|l| l.id.as_str()).collect();

    // Find labels that exist locally but were deleted on Gmail
    let deleted_label_ids = label_repo
        .find_deleted_label_ids(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            &payload.account_id,
            &api_provider_ids,
        )
        .await
        .map_err(|err| JobError::retryable(format!("find deleted labels: {err}")))?;

    // For each deleted label, disable dependent rules
    for deleted_provider_id in &deleted_label_ids {
        // Get the label name before it's deleted
        let label_name = label_repo
            .get_by_provider_id(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &payload.account_id,
                deleted_provider_id,
            )
            .await
            .ok()
            .map(|l| l.name.clone())
            .unwrap_or_else(|| deleted_provider_id.clone());

        // Find rules that reference this label
        let affected_rules = rule_repo
            .find_rules_referencing_label(DEFAULT_ORG_ID, DEFAULT_USER_ID, deleted_provider_id)
            .await
            .map_err(|err| JobError::retryable(format!("find affected rules: {err}")))?;

        // Disable each affected rule with a reason
        for rule in affected_rules {
            let reason = format!("Label '{}' was deleted from Gmail", label_name);
            match rule_repo
                .disable_rule_with_reason(DEFAULT_ORG_ID, DEFAULT_USER_ID, &rule.id, &reason)
                .await
            {
                Ok(_) => {
                    info!(
                        rule_id = %rule.id,
                        rule_name = %rule.name,
                        label_name = %label_name,
                        "disabled rule due to deleted label"
                    );
                }
                Err(err) => {
                    warn!(
                        rule_id = %rule.id,
                        error = %err,
                        "failed to disable rule for deleted label"
                    );
                }
            }
        }
    }

    // Delete labels that no longer exist on Gmail
    if !deleted_label_ids.is_empty() {
        let deleted_count = label_repo
            .delete_not_in_provider_ids(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &payload.account_id,
                &api_provider_ids,
            )
            .await
            .map_err(|err| JobError::retryable(format!("delete labels: {err}")))?;

        info!(
            account_id = %payload.account_id,
            deleted_count,
            "removed deleted labels from database"
        );
    }

    // Upsert all labels from the API response
    let mut upserted_count = 0;
    for api_label in response.labels {
        let new_label = NewLabel {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
            account_id: payload.account_id.clone(),
            provider_label_id: api_label.id.clone(),
            name: api_label.name.clone(),
            label_type: api_label.label_type.unwrap_or_else(|| "user".to_string()),
            description: None,             // Preserved by upsert if already set
            available_to_classifier: true, // Preserved by upsert if already set
            message_list_visibility: api_label.message_list_visibility,
            label_list_visibility: api_label.label_list_visibility,
            background_color: api_label
                .color
                .as_ref()
                .and_then(|c| c.background_color.clone()),
            text_color: api_label.color.as_ref().and_then(|c| c.text_color.clone()),
        };

        match label_repo.upsert(new_label).await {
            Ok(_) => {
                upserted_count += 1;
                debug!(
                    account_id = %payload.account_id,
                    label_id = %api_label.id,
                    label_name = %api_label.name,
                    "upserted label"
                );
            }
            Err(err) => {
                warn!(
                    account_id = %payload.account_id,
                    label_id = %api_label.id,
                    error = %err,
                    "failed to upsert label"
                );
            }
        }
    }

    info!(
        account_id = %payload.account_id,
        upserted = upserted_count,
        deleted = deleted_label_ids.len(),
        "label sync complete"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountConfig, PubsubConfig};
    use crate::config::PolicyConfig;
    use crate::gmail::OAuthTokens;
    use crate::llm::MockLLMClient;
    use crate::migrations::run_migrations;
    use crate::queue::JobQueue;
    use crate::rules::types::{NewDeterministicRule, RuleScope, SafeMode};
    use chrono::Utc;
    use serde_json::json;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_account() -> (
        AccountRepository,
        LabelRepository,
        JobDispatcher,
        TempDir,
        String,
    ) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = crate::Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");

        let account_repo = AccountRepository::new(db.clone());
        let label_repo = LabelRepository::new(db.clone());

        let config = AccountConfig {
            client_id: "client".into(),
            client_secret: "secret".into(),
            oauth: OAuthTokens {
                access_token: "access".into(),
                refresh_token: "refresh".into(),
                expires_at: Utc::now() + chrono::Duration::hours(1),
            },
            pubsub: PubsubConfig::default(),
        };
        let account = account_repo
            .create(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                "user@example.com",
                Some("User".into()),
                config,
            )
            .await
            .expect("create account");

        let dispatcher = JobDispatcher::new(
            db.clone(),
            reqwest::Client::new(),
            Arc::new(MockLLMClient::new()),
            PolicyConfig::default(),
        );
        (account_repo, label_repo, dispatcher, dir, account.id)
    }

    fn sample_labels_response() -> serde_json::Value {
        json!({
            "labels": [
                {
                    "id": "INBOX",
                    "name": "INBOX",
                    "type": "system",
                    "messageListVisibility": "show",
                    "labelListVisibility": "labelShow"
                },
                {
                    "id": "Label_123",
                    "name": "Work",
                    "type": "user",
                    "messageListVisibility": "show",
                    "labelListVisibility": "labelShow",
                    "color": {
                        "backgroundColor": "#ff0000",
                        "textColor": "#ffffff"
                    }
                },
                {
                    "id": "Label_456",
                    "name": "Personal",
                    "type": "user"
                }
            ]
        })
    }

    #[tokio::test]
    async fn labels_sync_inserts_new_labels() {
        let (_account_repo, label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_labels_response()))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect("labels sync");

        // Verify labels were inserted
        let labels = label_repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get labels");

        assert_eq!(labels.len(), 3);

        let inbox = labels.iter().find(|l| l.name == "INBOX").expect("INBOX");
        assert_eq!(inbox.provider_label_id, "INBOX");
        assert_eq!(inbox.label_type, "system");

        let work = labels.iter().find(|l| l.name == "Work").expect("Work");
        assert_eq!(work.provider_label_id, "Label_123");
        assert_eq!(work.label_type, "user");
        assert_eq!(work.background_color.as_deref(), Some("#ff0000"));
        assert_eq!(work.text_color.as_deref(), Some("#ffffff"));

        let personal = labels
            .iter()
            .find(|l| l.name == "Personal")
            .expect("Personal");
        assert_eq!(personal.provider_label_id, "Label_456");
        assert!(personal.background_color.is_none());
    }

    #[tokio::test]
    async fn labels_sync_updates_existing_labels() {
        let (_account_repo, label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        // Pre-create a label with old name
        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                provider_label_id: "Label_123".to_string(),
                name: "Old Work Name".to_string(),
                label_type: "user".to_string(),
                description: Some("My work label".to_string()),
                available_to_classifier: false, // User set this to false
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("pre-create label");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_labels_response()))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect("labels sync");

        // Verify label was updated but user fields preserved
        let work = label_repo
            .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "Label_123")
            .await
            .expect("get label");

        assert_eq!(work.name, "Work"); // Updated from Gmail
        assert_eq!(work.description.as_deref(), Some("My work label")); // Preserved
        assert!(!work.available_to_classifier); // Preserved
        assert_eq!(work.background_color.as_deref(), Some("#ff0000")); // Updated from Gmail
    }

    #[tokio::test]
    async fn labels_sync_deletes_removed_labels() {
        let (_account_repo, label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        // Pre-create a label that will be "deleted" on Gmail
        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                provider_label_id: "Label_OLD".to_string(),
                name: "Deleted Label".to_string(),
                label_type: "user".to_string(),
                description: None,
                available_to_classifier: true,
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("pre-create label");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // API response doesn't include Label_OLD
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_labels_response()))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect("labels sync");

        // Verify deleted label is gone
        let result = label_repo
            .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "Label_OLD")
            .await;
        assert!(result.is_err());

        // Verify other labels still exist
        let labels = label_repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get labels");
        assert_eq!(labels.len(), 3);
    }

    #[tokio::test]
    async fn labels_sync_disables_rules_referencing_deleted_labels() {
        let (_account_repo, label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());
        let rule_repo = DeterministicRuleRepository::new(dispatcher.db.clone());

        // Pre-create a label that will be "deleted" on Gmail
        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                provider_label_id: "Label_DELETED".to_string(),
                name: "ToBeDeleted".to_string(),
                label_type: "user".to_string(),
                description: None,
                available_to_classifier: true,
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("pre-create label");

        // Create a rule that references this label
        let rule = rule_repo
            .create(NewDeterministicRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Apply deleted label".to_string(),
                description: None,
                scope: RuleScope::Global,
                scope_ref: None,
                priority: 10,
                enabled: true,
                disabled_reason: None,
                conditions_json: json!({"all": true}),
                action_type: "apply_label".to_string(),
                action_parameters_json: json!({"label_id": "Label_DELETED"}),
                safe_mode: SafeMode::Default,
            })
            .await
            .expect("create rule");

        assert!(rule.enabled);
        assert!(rule.disabled_reason.is_none());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // API response doesn't include Label_DELETED
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_labels_response()))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect("labels sync");

        // Verify the rule was disabled with a reason
        let updated_rule = rule_repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &rule.id)
            .await
            .expect("get rule");

        assert!(!updated_rule.enabled);
        assert_eq!(
            updated_rule.disabled_reason.as_deref(),
            Some("Label 'ToBeDeleted' was deleted from Gmail")
        );
    }

    #[tokio::test]
    async fn labels_sync_handles_empty_labels_response() {
        let (_account_repo, label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "labels": [] })))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect("labels sync");

        // Verify no labels exist
        let labels = label_repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get labels");
        assert!(labels.is_empty());
    }

    #[tokio::test]
    async fn labels_sync_retries_on_rate_limit() {
        let (_account_repo, _label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect_err("should retry on rate limit");

        match err {
            JobError::Retryable { message, .. } => {
                assert!(message.contains("429") || message.contains("rate"))
            }
            other => panic!("expected retryable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn labels_sync_fails_for_invalid_payload() {
        let (_account_repo, _label_repo, dispatcher, _dir, _account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"invalid": "payload"}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect_err("should fail for invalid payload");

        match err {
            JobError::Fatal(msg) => assert!(msg.contains("invalid")),
            other => panic!("expected fatal error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn labels_sync_fails_for_nonexistent_account() {
        let (_account_repo, _label_repo, dispatcher, _dir, _account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({"account_id": "nonexistent-account-id"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect_err("should fail for nonexistent account");

        match err {
            JobError::Fatal(msg) => assert!(msg.contains("not found")),
            other => panic!("expected fatal error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn labels_sync_disables_multiple_rules_for_same_deleted_label() {
        let (_account_repo, label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());
        let rule_repo = DeterministicRuleRepository::new(dispatcher.db.clone());

        // Pre-create a label that will be "deleted" on Gmail
        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                provider_label_id: "Label_SHARED".to_string(),
                name: "SharedLabel".to_string(),
                label_type: "user".to_string(),
                description: None,
                available_to_classifier: true,
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("pre-create label");

        // Create multiple rules referencing the same label
        let rule1 = rule_repo
            .create(NewDeterministicRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Rule 1 - condition".to_string(),
                description: None,
                scope: RuleScope::Global,
                scope_ref: None,
                priority: 10,
                enabled: true,
                disabled_reason: None,
                conditions_json: json!({"type": "LabelPresent", "value": "Label_SHARED"}),
                action_type: "archive".to_string(),
                action_parameters_json: json!({}),
                safe_mode: SafeMode::Default,
            })
            .await
            .expect("create rule1");

        let rule2 = rule_repo
            .create(NewDeterministicRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Rule 2 - action".to_string(),
                description: None,
                scope: RuleScope::Global,
                scope_ref: None,
                priority: 20,
                enabled: true,
                disabled_reason: None,
                conditions_json: json!({"all": true}),
                action_type: "apply_label".to_string(),
                action_parameters_json: json!({"label_id": "Label_SHARED"}),
                safe_mode: SafeMode::Default,
            })
            .await
            .expect("create rule2");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // API response doesn't include Label_SHARED
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_labels_response()))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect("labels sync");

        // Verify both rules were disabled
        let updated_rule1 = rule_repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &rule1.id)
            .await
            .expect("get rule1");
        let updated_rule2 = rule_repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &rule2.id)
            .await
            .expect("get rule2");

        assert!(!updated_rule1.enabled);
        assert!(!updated_rule2.enabled);
        assert!(updated_rule1.disabled_reason.is_some());
        assert!(updated_rule2.disabled_reason.is_some());
        assert!(
            updated_rule1
                .disabled_reason
                .as_ref()
                .unwrap()
                .contains("SharedLabel")
        );
        assert!(
            updated_rule2
                .disabled_reason
                .as_ref()
                .unwrap()
                .contains("SharedLabel")
        );
    }

    #[tokio::test]
    async fn labels_sync_is_idempotent() {
        let (_account_repo, label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_labels_response()))
            .expect(2) // Expect to be called twice
            .mount(&server)
            .await;

        // First sync
        let job_id1 = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job1");
        let job1 = queue.fetch_job(&job_id1).await.expect("fetch job1");
        handle_labels_sync_gmail(&dispatcher, job1)
            .await
            .expect("first sync");

        let labels_after_first = label_repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get labels after first");
        assert_eq!(labels_after_first.len(), 3);

        // Second sync with same API response
        let job_id2 = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job2");
        let job2 = queue.fetch_job(&job_id2).await.expect("fetch job2");
        handle_labels_sync_gmail(&dispatcher, job2)
            .await
            .expect("second sync");

        let labels_after_second = label_repo
            .get_by_account(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("get labels after second");

        // Same count and same content
        assert_eq!(labels_after_second.len(), 3);

        // Verify IDs are preserved (same rows, not recreated)
        let first_ids: Vec<&str> = labels_after_first.iter().map(|l| l.id.as_str()).collect();
        let second_ids: Vec<&str> = labels_after_second.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(first_ids, second_ids);
    }

    #[tokio::test]
    async fn labels_sync_handles_rule_referencing_label_in_both_condition_and_action() {
        let (_account_repo, label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());
        let rule_repo = DeterministicRuleRepository::new(dispatcher.db.clone());

        // Pre-create a label that will be "deleted" on Gmail
        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                provider_label_id: "Label_DUAL".to_string(),
                name: "DualLabel".to_string(),
                label_type: "user".to_string(),
                description: None,
                available_to_classifier: true,
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("pre-create label");

        // Create a rule that references this label in BOTH condition AND action
        let rule = rule_repo
            .create(NewDeterministicRule {
                org_id: DEFAULT_ORG_ID,
                user_id: Some(DEFAULT_USER_ID),
                name: "Dual reference rule".to_string(),
                description: None,
                scope: RuleScope::Global,
                scope_ref: None,
                priority: 10,
                enabled: true,
                disabled_reason: None,
                conditions_json: json!({"type": "LabelPresent", "value": "Label_DUAL"}),
                action_type: "apply_label".to_string(),
                action_parameters_json: json!({"label_id": "Label_DUAL"}),
                safe_mode: SafeMode::Default,
            })
            .await
            .expect("create rule");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_labels_response()))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect("labels sync");

        // Verify the rule was disabled only once (not double-disabled)
        let updated_rule = rule_repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &rule.id)
            .await
            .expect("get rule");

        assert!(!updated_rule.enabled);
        assert_eq!(
            updated_rule.disabled_reason.as_deref(),
            Some("Label 'DualLabel' was deleted from Gmail")
        );
    }

    #[tokio::test]
    async fn labels_sync_preserves_user_set_description() {
        let (_account_repo, label_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        // Pre-create a label with a user-set description
        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                provider_label_id: "Label_123".to_string(),
                name: "Work".to_string(),
                label_type: "user".to_string(),
                description: Some("My important work emails".to_string()),
                available_to_classifier: false, // User disabled classifier access
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("pre-create label");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // API returns updated metadata for same label
        let response = json!({
            "labels": [
                {
                    "id": "Label_123",
                    "name": "Work Renamed",  // Name changed on Gmail
                    "type": "user",
                    "color": {
                        "backgroundColor": "#0000ff",
                        "textColor": "#ffffff"
                    }
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/labels"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"account_id": account_id.clone()}), None, 1)
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_labels_sync_gmail(&dispatcher, job)
            .await
            .expect("labels sync");

        let label = label_repo
            .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "Label_123")
            .await
            .expect("get label");

        // Name should be updated from Gmail
        assert_eq!(label.name, "Work Renamed");
        // Color should be updated from Gmail
        assert_eq!(label.background_color.as_deref(), Some("#0000ff"));
        // User-set fields should be preserved
        assert_eq!(
            label.description.as_deref(),
            Some("My important work emails")
        );
        assert!(!label.available_to_classifier);
    }
}
