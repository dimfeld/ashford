use std::sync::Arc;

use chrono::Utc;
use serde::Deserialize;
use tracing::{debug, info};

use crate::Job;
use crate::accounts::{AccountRepository, SyncStatus};
use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::gmail::{GmailClient, NoopTokenStore};
use crate::jobs::{JOB_TYPE_INGEST_GMAIL, JobDispatcher, map_account_error, map_gmail_error};
use crate::queue::{JobQueue, QueueError};
use crate::worker::JobError;

pub const JOB_TYPE: &str = "backfill.gmail";

/// Priority for backfill continuation jobs (lower than real-time)
const BACKFILL_PRIORITY: i64 = -10;

/// Priority for ingest jobs created during backfill (high priority)
const INGEST_PRIORITY: i64 = 1;

#[derive(Debug, Deserialize)]
struct BackfillPayload {
    account_id: String,
    query: String,
    #[serde(default)]
    page_token: Option<String>,
}

pub async fn handle_backfill_gmail(dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    let payload: BackfillPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid backfill.gmail payload: {err}")))?;

    let account_repo = AccountRepository::new(dispatcher.db.clone());
    let queue = JobQueue::new(dispatcher.db.clone());

    // 1. Refresh account tokens
    let account = account_repo
        .refresh_tokens_if_needed(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            &payload.account_id,
            &dispatcher.http,
        )
        .await
        .map_err(|err| map_account_error("refresh account tokens", err))?;

    // 2. Create Gmail client
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

    // 3. List messages with query and page_token
    let response = client
        .list_messages(
            Some(&payload.query),
            payload.page_token.as_deref(),
            false,
            None,
        )
        .await
        .map_err(|err| map_gmail_error("list_messages", err))?;

    // 4. Enqueue ingest.gmail job for each message
    for msg in response.messages.iter() {
        enqueue_ingest_job(&queue, &payload.account_id, &msg.id).await?;
    }

    info!(
        account_id = %payload.account_id,
        query = %payload.query,
        message_count = response.messages.len(),
        has_more = response.next_page_token.is_some(),
        "backfill page processed"
    );

    // 5. Handle pagination or finalize
    if let Some(next_token) = response.next_page_token {
        // More pages to process - enqueue next backfill job at low priority
        enqueue_next_page(&queue, &payload.account_id, &payload.query, &next_token).await?;
    } else {
        // Final page - get fresh historyId and update account state
        finalize_backfill(
            &client,
            &account_repo,
            account.org_id,
            account.user_id,
            &payload.account_id,
        )
        .await?;
    }

    Ok(())
}

async fn enqueue_ingest_job(
    queue: &JobQueue,
    account_id: &str,
    message_id: &str,
) -> Result<(), JobError> {
    let payload = serde_json::json!({
        "account_id": account_id,
        "message_id": message_id,
    });
    let idempotency = format!("{JOB_TYPE_INGEST_GMAIL}:{account_id}:{message_id}");

    match queue
        .enqueue(
            JOB_TYPE_INGEST_GMAIL,
            payload,
            Some(idempotency),
            INGEST_PRIORITY,
        )
        .await
    {
        Ok(_) => Ok(()),
        Err(QueueError::DuplicateIdempotency { .. }) => {
            debug!(account_id, message_id, "ingest job already enqueued");
            Ok(())
        }
        Err(err) => Err(JobError::retryable(format!(
            "enqueue ingest job failed: {err}"
        ))),
    }
}

async fn enqueue_next_page(
    queue: &JobQueue,
    account_id: &str,
    query: &str,
    page_token: &str,
) -> Result<(), JobError> {
    let payload = serde_json::json!({
        "account_id": account_id,
        "query": query,
        "page_token": page_token,
    });
    // Use unique idempotency key per page to allow processing same query with different pages
    let idempotency = format!("{JOB_TYPE}:{account_id}:{page_token}");

    match queue
        .enqueue(JOB_TYPE, payload, Some(idempotency), BACKFILL_PRIORITY)
        .await
    {
        Ok(_) => {
            debug!(account_id, page_token, "next backfill page enqueued");
            Ok(())
        }
        Err(QueueError::DuplicateIdempotency { .. }) => {
            debug!(account_id, page_token, "backfill page already enqueued");
            Ok(())
        }
        Err(err) => Err(JobError::retryable(format!(
            "enqueue next backfill page failed: {err}"
        ))),
    }
}

async fn finalize_backfill(
    client: &GmailClient<NoopTokenStore>,
    account_repo: &AccountRepository,
    org_id: i64,
    user_id: i64,
    account_id: &str,
) -> Result<(), JobError> {
    // Fetch fresh historyId from profile
    let profile = client
        .get_profile()
        .await
        .map_err(|err| map_gmail_error("get_profile", err))?;

    // Update account state to Normal with new historyId
    let account = account_repo
        .get_by_id(org_id, user_id, account_id)
        .await
        .map_err(|err| map_account_error("get account", err))?;

    let mut new_state = account.state.clone();
    new_state.history_id = Some(profile.history_id.clone());
    new_state.last_sync_at = Some(Utc::now());
    new_state.sync_status = SyncStatus::Normal;

    account_repo
        .update_state(org_id, user_id, account_id, &new_state)
        .await
        .map_err(|err| map_account_error("update account state", err))?;

    info!(
        account_id = %account_id,
        history_id = %profile.history_id,
        "backfill complete, resuming normal sync"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Database;
    use crate::accounts::{AccountConfig, PubsubConfig};
    use crate::gmail::OAuthTokens;
    use crate::migrations::run_migrations;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tempfile::TempDir;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    async fn setup_account() -> (AccountRepository, JobDispatcher, JobQueue, TempDir, String) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(&db_path).await.expect("create db");
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

        let queue = JobQueue::new(db.clone());
        let dispatcher = JobDispatcher::new(db.clone(), reqwest::Client::new());
        (repo, dispatcher, queue, dir, account.id)
    }

    #[tokio::test]
    async fn backfill_lists_messages_and_enqueues_ingest_jobs() {
        let (repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // Mock list messages response
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .and(query_param("q", "newer_than:7d"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "messages": [
                    { "id": "msg-1", "threadId": "thr-1" },
                    { "id": "msg-2", "threadId": "thr-2" }
                ],
                "resultSizeEstimate": 2
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Mock get profile response (called when no more pages)
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "emailAddress": "user@example.com",
                "historyId": "12345"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id.clone(),
                    "query": "newer_than:7d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_backfill_gmail(&dispatcher, job)
            .await
            .expect("backfill succeeds");

        // Verify ingest jobs were enqueued
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT payload_json FROM jobs WHERE type = ?1 ORDER BY payload_json",
                libsql::params![JOB_TYPE_INGEST_GMAIL],
            )
            .await
            .expect("query");

        let first = rows.next().await.expect("row").expect("first job");
        let first_payload: String = first.get(0).expect("payload");
        assert!(first_payload.contains("msg-1"));

        let second = rows.next().await.expect("row").expect("second job");
        let second_payload: String = second.get(0).expect("payload");
        assert!(second_payload.contains("msg-2"));

        // Verify account state was updated
        let account = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("account");
        assert_eq!(account.state.history_id.as_deref(), Some("12345"));
        assert_eq!(account.state.sync_status, SyncStatus::Normal);
        assert!(account.state.last_sync_at.is_some());
    }

    #[tokio::test]
    async fn backfill_handles_pagination_by_enqueueing_next_page() {
        let (_repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // Mock list messages with next page token
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "messages": [
                    { "id": "msg-1", "threadId": "thr-1" }
                ],
                "nextPageToken": "page2token",
                "resultSizeEstimate": 100
            })))
            .expect(1)
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id.clone(),
                    "query": "newer_than:7d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_backfill_gmail(&dispatcher, job)
            .await
            .expect("backfill succeeds");

        // Verify next backfill job was enqueued
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT payload_json, priority FROM jobs WHERE type = ?1 AND idempotency_key LIKE '%page2token%'",
                libsql::params![JOB_TYPE],
            )
            .await
            .expect("query");

        let row = rows.next().await.expect("row").expect("next page job");
        let payload: String = row.get(0).expect("payload");
        let priority: i64 = row.get(1).expect("priority");

        assert!(payload.contains("page2token"));
        assert!(payload.contains(&account_id));
        assert_eq!(priority, BACKFILL_PRIORITY);
    }

    #[tokio::test]
    async fn backfill_with_page_token_uses_token() {
        let (repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // Mock list messages with page token
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .and(query_param("pageToken", "page2token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "messages": [
                    { "id": "msg-2", "threadId": "thr-2" }
                ],
                "resultSizeEstimate": 1
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Mock get profile
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "emailAddress": "user@example.com",
                "historyId": "99999"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id.clone(),
                    "query": "newer_than:7d",
                    "page_token": "page2token"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_backfill_gmail(&dispatcher, job)
            .await
            .expect("backfill succeeds");

        // Verify account state was updated on final page
        let account = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("account");
        assert_eq!(account.state.history_id.as_deref(), Some("99999"));
        assert_eq!(account.state.sync_status, SyncStatus::Normal);
    }

    #[tokio::test]
    async fn backfill_handles_empty_results() {
        let (repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // Mock empty list messages response
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "messages": [],
                "resultSizeEstimate": 0
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Mock get profile
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "emailAddress": "user@example.com",
                "historyId": "55555"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id.clone(),
                    "query": "newer_than:1d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_backfill_gmail(&dispatcher, job)
            .await
            .expect("backfill succeeds with empty results");

        // Verify no ingest jobs were enqueued
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM jobs WHERE type = ?1",
                libsql::params![JOB_TYPE_INGEST_GMAIL],
            )
            .await
            .expect("query");
        let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(count, 0);

        // Verify account state was still updated
        let account = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("account");
        assert_eq!(account.state.history_id.as_deref(), Some("55555"));
        assert_eq!(account.state.sync_status, SyncStatus::Normal);
    }

    #[tokio::test]
    async fn backfill_retries_on_429_rate_limit() {
        let (_repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "query": "newer_than:7d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_backfill_gmail(&dispatcher, job)
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
    async fn backfill_retries_on_403_rate_limit() {
        let (_repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "query": "newer_than:7d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_backfill_gmail(&dispatcher, job)
            .await
            .expect_err("should retry on 403");

        match err {
            JobError::Retryable { message, .. } => {
                assert!(message.contains("403") || message.contains("rate"))
            }
            other => panic!("expected retryable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn backfill_fails_fatally_on_404() {
        let (_repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "query": "newer_than:7d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_backfill_gmail(&dispatcher, job)
            .await
            .expect_err("should fail on 404");

        match err {
            JobError::Fatal(msg) => assert!(msg.contains("404") || msg.contains("not found")),
            other => panic!("expected fatal, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn backfill_is_idempotent_for_ingest_jobs() {
        let (_repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        struct CountingResponder {
            call_count: Arc<AtomicUsize>,
        }

        impl Respond for CountingResponder {
            fn respond(&self, _request: &Request) -> ResponseTemplate {
                self.call_count.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({
                    "messages": [
                        { "id": "msg-dup", "threadId": "thr-1" }
                    ],
                    "resultSizeEstimate": 1
                }))
            }
        }

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .respond_with(CountingResponder {
                call_count: call_count_clone,
            })
            .expect(2)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "emailAddress": "user@example.com",
                "historyId": "12345"
            })))
            .expect(2)
            .mount(&server)
            .await;

        // Run backfill twice
        for _ in 0..2 {
            let job_id = queue
                .enqueue(
                    JOB_TYPE,
                    json!({
                        "account_id": account_id.clone(),
                        "query": "newer_than:7d"
                    }),
                    None,
                    0,
                )
                .await
                .expect("enqueue");
            let job = queue.fetch_job(&job_id).await.expect("fetch job");

            handle_backfill_gmail(&dispatcher, job)
                .await
                .expect("backfill succeeds");
        }

        // Verify only one ingest job was created despite two backfill runs
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT COUNT(*) FROM jobs WHERE type = ?1",
                libsql::params![JOB_TYPE_INGEST_GMAIL],
            )
            .await
            .expect("query");
        let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
        assert_eq!(count, 1, "duplicate ingest job should not be inserted");

        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn backfill_updates_from_needs_backfill_to_normal() {
        let (repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        // Set account to NeedsBackfill state
        let account = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("account");
        let mut state = account.state.clone();
        state.sync_status = SyncStatus::NeedsBackfill;
        state.history_id = None;
        repo.update_state(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, &state)
            .await
            .expect("update state");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "messages": [],
                "resultSizeEstimate": 0
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "emailAddress": "user@example.com",
                "historyId": "77777"
            })))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id.clone(),
                    "query": "newer_than:7d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_backfill_gmail(&dispatcher, job)
            .await
            .expect("backfill succeeds");

        // Verify state transition
        let account = repo
            .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
            .await
            .expect("account");
        assert_eq!(account.state.sync_status, SyncStatus::Normal);
        assert_eq!(account.state.history_id.as_deref(), Some("77777"));
    }

    #[tokio::test]
    async fn invalid_payload_returns_fatal_error() {
        let (_, dispatcher, queue, _dir, _) = setup_account().await;

        let job_id = queue
            .enqueue(JOB_TYPE, json!({"invalid": "payload"}), None, 0)
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let result = handle_backfill_gmail(&dispatcher, job).await;

        match result {
            Err(JobError::Fatal(msg)) => {
                assert!(msg.contains("invalid backfill.gmail payload"));
            }
            other => panic!("expected fatal error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn backfill_retries_on_get_profile_429() {
        let (_repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // Mock successful list messages (no next page token = final page)
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "messages": [
                    { "id": "msg-1", "threadId": "thr-1" }
                ],
                "resultSizeEstimate": 1
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Mock get_profile returning 429 rate limit error
        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/profile"))
            .respond_with(ResponseTemplate::new(429))
            .expect(1)
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id,
                    "query": "newer_than:7d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_backfill_gmail(&dispatcher, job)
            .await
            .expect_err("should retry when get_profile fails");

        match err {
            JobError::Retryable { message, .. } => {
                assert!(
                    message.contains("429")
                        || message.contains("rate")
                        || message.contains("get_profile"),
                    "error message should indicate rate limit or get_profile failure: {message}"
                );
            }
            other => panic!("expected retryable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn backfill_enqueues_ingest_jobs_at_high_priority() {
        let (_repo, dispatcher, queue, _dir, account_id) = setup_account().await;

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "messages": [
                    { "id": "msg-priority", "threadId": "thr-1" }
                ],
                "resultSizeEstimate": 1
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/profile"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "emailAddress": "user@example.com",
                "historyId": "12345"
            })))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                JOB_TYPE,
                json!({
                    "account_id": account_id.clone(),
                    "query": "newer_than:7d"
                }),
                None,
                0,
            )
            .await
            .expect("enqueue");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_backfill_gmail(&dispatcher, job)
            .await
            .expect("backfill succeeds");

        // Verify ingest job was enqueued with high priority
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT priority FROM jobs WHERE type = ?1",
                libsql::params![JOB_TYPE_INGEST_GMAIL],
            )
            .await
            .expect("query");

        let row = rows.next().await.expect("row").expect("ingest job");
        let priority: i64 = row.get(0).expect("priority");
        assert_eq!(priority, INGEST_PRIORITY);
    }
}
