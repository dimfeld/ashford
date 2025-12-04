use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{info, warn};

use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::gmail::{GmailClientError, NoopTokenStore};
use crate::labels::{LabelError, LabelRepository};
use crate::messages::{MessageError, MessageRepository};
use crate::{Job, JobError};

use super::action_gmail::create_gmail_client;
use super::{JobDispatcher, map_gmail_error};

pub const JOB_TYPE: &str = "unsnooze.gmail";

#[derive(Debug, Deserialize)]
struct UnsnoozeJobPayload {
    pub account_id: String,
    pub message_id: String,
    pub action_id: String,
    pub snooze_label_id: Option<String>,
}

pub async fn handle_unsnooze_gmail(dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    let payload: UnsnoozeJobPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid unsnooze.gmail payload: {err}")))?;

    let message_repo = MessageRepository::new(dispatcher.db.clone());
    let message = match message_repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &payload.message_id)
        .await
    {
        Ok(message) => message,
        Err(MessageError::NotFound(_)) => {
            warn!(
                account_id = %payload.account_id,
                message_id = %payload.message_id,
                action_id = %payload.action_id,
                "message missing locally during unsnooze; skipping"
            );
            return Ok(());
        }
        Err(err) => {
            return Err(JobError::retryable(format!(
                "failed to load message for unsnooze: {err}"
            )));
        }
    };

    if message.account_id != payload.account_id {
        return Err(JobError::Fatal(format!(
            "message {} does not belong to account {}",
            payload.message_id, payload.account_id
        )));
    }

    let gmail_client: crate::gmail::GmailClient<NoopTokenStore> =
        create_gmail_client(dispatcher, &payload.account_id).await?;

    let snooze_label_id = if let Some(id) = payload.snooze_label_id.clone() {
        Some(id)
    } else {
        let snooze_label = dispatcher.gmail_config.snooze_label.clone();
        let label_repo = LabelRepository::new(dispatcher.db.clone());
        match label_repo
            .get_by_name(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                &payload.account_id,
                &snooze_label,
            )
            .await
        {
            Ok(label) => Some(label.provider_label_id),
            Err(LabelError::NotFound(_)) => None,
            Err(err) => {
                return Err(JobError::retryable(format!(
                    "lookup snooze label failed: {err}"
                )));
            }
        }
    };

    let add_labels = vec!["INBOX".to_string()];
    let remove_labels = snooze_label_id.clone().map(|id| vec![id]);

    let mut result = gmail_client
        .modify_message(
            &message.provider_message_id,
            Some(add_labels.clone()),
            remove_labels.clone(),
        )
        .await;

    if let Err(GmailClientError::Http(err)) = &result {
        if err.status() == Some(StatusCode::BAD_REQUEST) && remove_labels.is_some() {
            warn!(
                account_id = %payload.account_id,
                message_id = %payload.message_id,
                action_id = %payload.action_id,
                "snooze label missing in Gmail during unsnooze; retrying without removal"
            );

            result = gmail_client
                .modify_message(&message.provider_message_id, Some(add_labels.clone()), None)
                .await;
        }
    }

    match result {
        Ok(_) => {
            info!(
                account_id = %payload.account_id,
                message_id = %payload.message_id,
                action_id = %payload.action_id,
                "unsnoozed message"
            );
            Ok(())
        }
        Err(GmailClientError::Http(err)) if err.status() == Some(StatusCode::NOT_FOUND) => {
            warn!(
                account_id = %payload.account_id,
                message_id = %payload.message_id,
                action_id = %payload.action_id,
                "message missing in Gmail during unsnooze; skipping"
            );
            Ok(())
        }
        Err(err) => Err(map_gmail_error("unsnooze message", err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountConfig, AccountRepository, PubsubConfig};
    use crate::gmail::OAuthTokens;
    use crate::labels::NewLabel;
    use crate::messages::{Mailbox, MessageRepository, NewMessage};
    use crate::migrations::run_migrations;
    use crate::queue::JobQueue;
    use crate::threads::ThreadRepository;
    use chrono::Utc;
    use serde_json::json;
    use std::sync::Arc;
    use tempfile::TempDir;
    use uuid::Uuid;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_account() -> (crate::Database, TempDir, String) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = crate::Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");

        let account_repo = AccountRepository::new(db.clone());
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

        (db, dir, account.id)
    }

    async fn setup_message(
        db: &crate::Database,
        account_id: &str,
        provider_message_id: &str,
    ) -> String {
        let thread_repo = ThreadRepository::new(db.clone());
        let thread = thread_repo
            .upsert(
                DEFAULT_ORG_ID,
                DEFAULT_USER_ID,
                account_id,
                "thread-123",
                Some("Test Subject".to_string()),
                Some("Test snippet".to_string()),
                Some(Utc::now()),
                json!({}),
            )
            .await
            .expect("create thread");

        let msg_repo = MessageRepository::new(db.clone());
        let msg = msg_repo
            .upsert(NewMessage {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.to_string(),
                thread_id: thread.id.clone(),
                provider_message_id: provider_message_id.to_string(),
                from_email: Some("sender@example.com".to_string()),
                from_name: Some("Sender".to_string()),
                to: vec![Mailbox {
                    email: "user@example.com".to_string(),
                    name: Some("User".to_string()),
                }],
                cc: vec![],
                bcc: vec![],
                subject: Some("Test Subject".to_string()),
                received_at: Some(Utc::now()),
                internal_date: Some(Utc::now()),
                labels: vec!["INBOX".to_string(), "UNREAD".to_string()],
                headers: vec![],
                body_plain: Some("Test body".to_string()),
                body_html: None,
                snippet: Some("Test snippet".to_string()),
                raw_json: json!({}),
            })
            .await
            .expect("create message");

        msg.id
    }

    fn dispatcher(db: crate::Database, api_base: String) -> JobDispatcher {
        JobDispatcher::new(
            db,
            reqwest::Client::new(),
            Arc::new(crate::llm::MockLLMClient::new()),
            crate::config::PolicyConfig::default(),
        )
        .with_gmail_api_base(api_base)
    }

    #[tokio::test]
    async fn unsnooze_adds_inbox_and_removes_snooze_label() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "gmail-msg-snoozed";
        let message_id = setup_message(&db, &account_id, provider_message_id).await;

        // Seed snooze label locally
        let label_repo = LabelRepository::new(db.clone());
        label_repo
            .upsert(NewLabel {
                org_id: DEFAULT_ORG_ID,
                user_id: DEFAULT_USER_ID,
                account_id: account_id.clone(),
                provider_label_id: "Label_Snoozed".to_string(),
                name: "Ashford/Snoozed".to_string(),
                label_type: "user".to_string(),
                description: None,
                available_to_classifier: true,
                message_list_visibility: None,
                label_list_visibility: None,
                background_color: None,
                text_color: None,
            })
            .await
            .expect("insert label");

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "message_id": message_id,
                    "action_id": "action-1",
                    "snooze_label_id": "Label_Snoozed"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        // Expect modify to add INBOX and remove snooze label
        Mock::given(method("POST"))
            .and(path(
                "/gmail/v1/users/user@example.com/messages/gmail-msg-snoozed/modify",
            ))
            .and(body_json(json!({
                "addLabelIds": ["INBOX"],
                "removeLabelIds": ["Label_Snoozed"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["INBOX"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Pre-image fetch is not needed for unsnooze, but create_gmail_client may call profile? It doesn't. No additional mocks required.
        let dispatcher = dispatcher(db.clone(), api_base);
        handle_unsnooze_gmail(&dispatcher, job)
            .await
            .expect("unsnooze succeeds");
    }

    #[tokio::test]
    async fn unsnooze_skips_when_message_missing_in_gmail() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "gmail-msg-deleted";
        let message_id = setup_message(&db, &account_id, provider_message_id).await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "message_id": message_id,
                    "action_id": "action-2",
                    "snooze_label_id": "Label_Snoozed"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        // Modify returns 404
        Mock::given(method("POST"))
            .and(path(
                "/gmail/v1/users/user@example.com/messages/gmail-msg-deleted/modify",
            ))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;

        let dispatcher = dispatcher(db.clone(), api_base);
        handle_unsnooze_gmail(&dispatcher, job)
            .await
            .expect("unsnooze should tolerate missing message");
    }

    #[tokio::test]
    async fn unsnooze_retries_when_snooze_label_missing_in_gmail() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "gmail-msg-snooze-label-missing";
        let message_id = setup_message(&db, &account_id, provider_message_id).await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "message_id": message_id,
                    "action_id": "action-missing-label",
                    "snooze_label_id": "Label_Snoozed"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        // First attempt fails because the snooze label no longer exists in Gmail
        Mock::given(method("POST"))
            .and(path(
                "/gmail/v1/users/user@example.com/messages/gmail-msg-snooze-label-missing/modify",
            ))
            .and(body_json(json!({
                "addLabelIds": ["INBOX"],
                "removeLabelIds": ["Label_Snoozed"]
            })))
            .respond_with(
                ResponseTemplate::new(400).set_body_json(json!({
                    "error": {
                        "code": 400,
                        "message": "Label not found: Label_Snoozed"
                    }
                })),
            )
            .expect(1)
            .mount(&server)
            .await;

        // Retry should drop removeLabelIds and still add INBOX
        Mock::given(method("POST"))
            .and(path(
                "/gmail/v1/users/user@example.com/messages/gmail-msg-snooze-label-missing/modify",
            ))
            .and(body_json(json!({
                "addLabelIds": ["INBOX"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["INBOX"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let dispatcher = dispatcher(db.clone(), api_base);
        handle_unsnooze_gmail(&dispatcher, job)
            .await
            .expect("unsnooze should succeed after retry without snooze label");

        let received = server.received_requests().await.expect("requests list");
        assert_eq!(received.len(), 2, "should attempt modify twice");
    }

    #[tokio::test]
    async fn unsnooze_skips_when_message_missing_locally() {
        let (db, _dir, account_id) = setup_account().await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "message_id": "missing-message",
                    "action_id": "action-absent",
                    "snooze_label_id": "Label_Snoozed"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        let dispatcher = dispatcher(db.clone(), api_base);
        handle_unsnooze_gmail(&dispatcher, job)
            .await
            .expect("unsnooze should ignore missing local message");

        let requests = server.received_requests().await.expect("requests list");
        assert!(
            requests.is_empty(),
            "no gmail calls should be made for missing message"
        );
    }

    #[tokio::test]
    async fn unsnooze_works_without_local_label() {
        let (db, _dir, account_id) = setup_account().await;
        let provider_message_id = "gmail-msg-no-label";
        let message_id = setup_message(&db, &account_id, provider_message_id).await;

        let queue = JobQueue::new(db.clone());
        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "message_id": message_id,
                    "action_id": "action-3",
                    "snooze_label_id": "Label_Snoozed"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());

        // Without snooze label in the local DB, we still remove the snooze label id carried on the job
        Mock::given(method("POST"))
            .and(path(
                "/gmail/v1/users/user@example.com/messages/gmail-msg-no-label/modify",
            ))
            .and(body_json(json!({
                "addLabelIds": ["INBOX"],
                "removeLabelIds": ["Label_Snoozed"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": provider_message_id,
                "labelIds": ["INBOX"]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let dispatcher = dispatcher(db.clone(), api_base);
        handle_unsnooze_gmail(&dispatcher, job)
            .await
            .expect("unsnooze succeeds without local label");
    }
}
