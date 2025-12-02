use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::accounts::AccountRepository;
use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::gmail::{GmailClient, NoopTokenStore, parse_message};
use crate::jobs::{JOB_TYPE_CLASSIFY, JobDispatcher, map_account_error, map_gmail_error};
use crate::messages::{Mailbox, MessageRepository, NewMessage};
use crate::queue::{JobQueue, QueueError};
use crate::threads::ThreadRepository;
use crate::{Job, JobError};

#[derive(Debug, Deserialize)]
struct IngestPayload {
    account_id: String,
    message_id: String,
}

pub async fn handle_ingest_gmail(dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    let payload: IngestPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid ingest.gmail payload: {err}")))?;

    let account_repo = AccountRepository::new(dispatcher.db.clone());
    let account = account_repo
        .refresh_tokens_if_needed(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            &payload.account_id,
            &dispatcher.http,
        )
        .await
        .map_err(|err| map_account_error("refresh account tokens", err))?;

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

    let message = client
        .get_message(&payload.message_id)
        .await
        .map_err(|err| map_gmail_error("get_message", err))?;

    let parsed = parse_message(&message);
    let thread_id = message
        .thread_id
        .clone()
        .ok_or_else(|| JobError::Fatal("message missing thread_id".into()))?;
    let last_message_at = parse_internal_date(&message.internal_date)?;

    let raw_json = serde_json::to_value(&message)
        .map_err(|err| JobError::Fatal(format!("serialize message: {err}")))?;
    let headers = message
        .payload
        .as_ref()
        .map(|p| p.headers.clone())
        .unwrap_or_default();

    let thread_repo = ThreadRepository::new(dispatcher.db.clone());
    let thread = thread_repo
        .upsert(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            &payload.account_id,
            &thread_id,
            parsed.subject.clone(),
            message.snippet.clone(),
            last_message_at,
            raw_json.clone(),
        )
        .await
        .map_err(|err| JobError::retryable(format!("upsert thread failed: {err}")))?;

    let msg_repo = MessageRepository::new(dispatcher.db.clone());
    let new_msg = NewMessage {
        org_id: DEFAULT_ORG_ID,
        user_id: DEFAULT_USER_ID,
        account_id: payload.account_id.clone(),
        thread_id: thread.id,
        provider_message_id: message.id.clone(),
        from_email: parsed.from_email,
        from_name: parsed.from_name,
        to: parsed
            .to
            .into_iter()
            .map(|r| Mailbox {
                email: r.email,
                name: r.name,
            })
            .collect(),
        cc: parsed
            .cc
            .into_iter()
            .map(|r| Mailbox {
                email: r.email,
                name: r.name,
            })
            .collect(),
        bcc: parsed
            .bcc
            .into_iter()
            .map(|r| Mailbox {
                email: r.email,
                name: r.name,
            })
            .collect(),
        subject: parsed.subject,
        snippet: message.snippet.clone(),
        received_at: last_message_at,
        internal_date: last_message_at,
        labels: message.label_ids.clone(),
        headers,
        body_plain: parsed.body_plain,
        body_html: parsed.body_html,
        raw_json,
    };

    let persisted_msg = msg_repo
        .upsert(new_msg)
        .await
        .map_err(|err| JobError::retryable(format!("upsert message failed: {err}")))?;

    // Enqueue classify job for the persisted message
    enqueue_classify_job(dispatcher, &payload.account_id, &persisted_msg.id).await?;

    info!(
        account_id = %payload.account_id,
        message_id = %payload.message_id,
        thread_id = %thread_id,
        "ingested gmail message"
    );

    Ok(())
}

fn parse_internal_date(internal_date: &Option<String>) -> Result<Option<DateTime<Utc>>, JobError> {
    let Some(raw) = internal_date else {
        return Ok(None);
    };

    match raw.parse::<i64>() {
        Ok(ms) => match Utc.timestamp_millis_opt(ms).single() {
            Some(dt) => Ok(Some(dt)),
            None => Err(JobError::Fatal(format!(
                "invalid internalDate millis: {raw}"
            ))),
        },
        Err(err) => {
            warn!(value = %raw, error = %err, "failed to parse internalDate");
            Ok(None)
        }
    }
}

async fn enqueue_classify_job(
    dispatcher: &JobDispatcher,
    account_id: &str,
    message_id: &str,
) -> Result<(), JobError> {
    let queue = JobQueue::new(dispatcher.db.clone());
    let payload = serde_json::json!({
        "account_id": account_id,
        "message_id": message_id,
    });
    let idempotency_key = format!("{JOB_TYPE_CLASSIFY}:{account_id}:{message_id}");

    match queue
        .enqueue(JOB_TYPE_CLASSIFY, payload, Some(idempotency_key), 0)
        .await
    {
        Ok(_) => Ok(()),
        Err(QueueError::DuplicateIdempotency { .. }) => {
            debug!(account_id, message_id, "classify job already enqueued");
            Ok(())
        }
        Err(err) => Err(JobError::retryable(format!(
            "enqueue classify job failed: {err}"
        ))),
    }
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
    use base64::Engine;
    use serde_json::json;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn setup_account() -> (AccountRepository, JobDispatcher, TempDir, String) {
        let dir = TempDir::new().expect("temp dir");
        // Use a unique database filename to avoid any potential conflicts
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = crate::Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");

        let repo = AccountRepository::new(db.clone());
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
        let account = repo
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
        (repo, dispatcher, dir, account.id)
    }

    fn build_message_response() -> serde_json::Value {
        let plain = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("Hello world");
        let html = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("<p>Hello world</p>");

        json!({
            "id": "msg-1",
            "threadId": "thr-1",
            "labelIds": ["INBOX"],
            "snippet": "Hello world",
            "internalDate": "1730000000000",
            "payload": {
                "mimeType": "multipart/alternative",
                "headers": [
                    {"name": "From", "value": "Alice <alice@example.com>"},
                    {"name": "To", "value": "Bob <bob@example.com>"},
                    {"name": "Subject", "value": "Greetings"}
                ],
                "parts": [
                    {
                        "mimeType": "text/plain",
                        "headers": [],
                        "body": {"size": 11, "data": plain}
                    },
                    {
                        "mimeType": "text/html",
                        "headers": [],
                        "body": {"size": 19, "data": html}
                    }
                ]
            }
        })
    }

    #[tokio::test]
    async fn ingest_fetches_and_persists_message() {
        let (_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages/msg-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(build_message_response()))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_INGEST_GMAIL,
                json!({"account_id": account_id, "message_id": "msg-1"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_ingest_gmail(&dispatcher, job).await.expect("ingest");

        let thread_repo = ThreadRepository::new(dispatcher.db.clone());
        let thread = thread_repo
            .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "thr-1")
            .await
            .expect("thread");
        assert_eq!(thread.provider_thread_id, "thr-1");
        assert_eq!(thread.subject.as_deref(), Some("Greetings"));

        let msg_repo = MessageRepository::new(dispatcher.db.clone());
        let stored = msg_repo
            .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "msg-1")
            .await
            .expect("message");
        assert_eq!(stored.subject.as_deref(), Some("Greetings"));
        assert_eq!(stored.to.len(), 1);
        assert_eq!(stored.body_plain.as_deref(), Some("Hello world"));
        assert_eq!(stored.body_html.as_deref(), Some("<p>Hello world</p>"));

        // Verify classify job was enqueued
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT payload_json, priority FROM jobs WHERE type = ?1",
                libsql::params![crate::jobs::JOB_TYPE_CLASSIFY],
            )
            .await
            .expect("query");
        let row = rows
            .next()
            .await
            .expect("row")
            .expect("classify job exists");
        let payload: String = row.get(0).expect("payload");
        let priority: i64 = row.get(1).expect("priority");

        // Verify payload contains correct account_id and message_id (internal UUID)
        let payload_value: serde_json::Value =
            serde_json::from_str(&payload).expect("parse payload");
        assert_eq!(
            payload_value["account_id"].as_str(),
            Some(account_id.as_str())
        );
        assert_eq!(
            payload_value["message_id"].as_str(),
            Some(stored.id.as_str())
        );
        assert_eq!(priority, 0, "classify job should have priority 0");
    }

    #[tokio::test]
    async fn ingest_returns_fatal_on_not_found() {
        let (_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_INGEST_GMAIL,
                json!({"account_id": account_id, "message_id": "missing"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_ingest_gmail(&dispatcher, job)
            .await
            .expect_err("ingest should surface fatal error");

        match err {
            JobError::Fatal(msg) => assert!(msg.contains("404")),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn ingest_retries_on_rate_limit() {
        let (_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages/msg-rl"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_INGEST_GMAIL,
                json!({"account_id": account_id, "message_id": "msg-rl"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_ingest_gmail(&dispatcher, job)
            .await
            .expect_err("ingest should retry on rate limit");

        match err {
            JobError::Retryable { message, .. } => {
                assert!(message.contains("429") || message.contains("rate"))
            }
            other => panic!("expected retryable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn ingest_errors_when_thread_id_missing() {
        let (_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        let mut body = build_message_response();
        if let Some(obj) = body.as_object_mut() {
            obj.remove("threadId");
        }

        Mock::given(method("GET"))
            .and(path(
                "/gmail/v1/users/user@example.com/messages/msg-no-thread",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_INGEST_GMAIL,
                json!({"account_id": account_id, "message_id": "msg-no-thread"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_ingest_gmail(&dispatcher, job)
            .await
            .expect_err("ingest should fail without thread id");

        match err {
            JobError::Fatal(msg) => assert!(msg.contains("thread_id")),
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn ingest_does_not_enqueue_classify_on_failure() {
        let (_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // Return 404 to simulate Gmail API failure
        Mock::given(method("GET"))
            .and(path(
                "/gmail/v1/users/user@example.com/messages/missing-msg",
            ))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_INGEST_GMAIL,
                json!({"account_id": account_id, "message_id": "missing-msg"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        // Ingest should fail
        let err = handle_ingest_gmail(&dispatcher, job)
            .await
            .expect_err("ingest should fail on 404");
        assert!(matches!(err, JobError::Fatal(_)));

        // Verify no classify job was enqueued
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM jobs WHERE type = ?1",
                libsql::params![crate::jobs::JOB_TYPE_CLASSIFY],
            )
            .await
            .expect("query");
        let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(count, 0, "no classify job should be enqueued on failure");
    }

    #[tokio::test]
    async fn ingest_handles_duplicate_classify_idempotency() {
        let (_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages/msg-dup"))
            .respond_with(ResponseTemplate::new(200).set_body_json({
                let mut resp = build_message_response();
                resp["id"] = json!("msg-dup");
                resp
            }))
            .expect(2) // Called twice
            .mount(&server)
            .await;

        // First ingest
        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_INGEST_GMAIL,
                json!({"account_id": account_id.clone(), "message_id": "msg-dup"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");
        handle_ingest_gmail(&dispatcher, job)
            .await
            .expect("first ingest succeeds");

        // Second ingest of the same message (simulating re-processing)
        let job_id2 = queue
            .enqueue(
                crate::jobs::JOB_TYPE_INGEST_GMAIL,
                json!({"account_id": account_id, "message_id": "msg-dup"}),
                None,
                1,
            )
            .await
            .expect("enqueue job 2");
        let job2 = queue.fetch_job(&job_id2).await.expect("fetch job 2");

        // Second ingest should also succeed (duplicate idempotency handled gracefully)
        handle_ingest_gmail(&dispatcher, job2)
            .await
            .expect("second ingest succeeds");

        // Verify only one classify job exists (due to idempotency)
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM jobs WHERE type = ?1",
                libsql::params![crate::jobs::JOB_TYPE_CLASSIFY],
            )
            .await
            .expect("query");
        let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(
            count, 1,
            "only one classify job should exist due to idempotency"
        );
    }
}
