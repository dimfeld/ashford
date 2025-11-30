use std::sync::Arc;

use chrono::Utc;
use reqwest::StatusCode;
use serde::Deserialize;
use tracing::{debug, info};

use crate::accounts::{Account, AccountRepository, SyncStatus};
use crate::gmail::{GmailClient, GmailClientError, NoopTokenStore};
use crate::jobs::{
    JOB_TYPE_BACKFILL_GMAIL, JOB_TYPE_INGEST_GMAIL, JobDispatcher, map_account_error,
    map_gmail_error,
};
use crate::queue::{JobQueue, QueueError};
use crate::{Job, JobError};

#[derive(Debug, Deserialize)]
struct HistoryPayload {
    account_id: String,
    history_id: String,
}

pub async fn handle_history_sync_gmail(
    dispatcher: &JobDispatcher,
    job: Job,
) -> Result<(), JobError> {
    let payload: HistoryPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid history.sync.gmail payload: {err}")))?;

    let account_repo = AccountRepository::new(dispatcher.db.clone());
    let account = account_repo
        .refresh_tokens_if_needed(&payload.account_id, &dispatcher.http)
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

    let start_history_id = account
        .state
        .history_id
        .as_deref()
        .unwrap_or(&payload.history_id);

    let queue = JobQueue::new(dispatcher.db.clone());
    let mut page_token: Option<String> = None;
    let mut latest_history_id: Option<String> = None;

    // Pagination loop - process all pages of history
    loop {
        let response = match client
            .list_history(start_history_id, page_token.as_deref(), None)
            .await
        {
            Ok(resp) => resp,
            Err(err) if is_history_not_found(&err) => {
                info!(
                    account_id = %payload.account_id,
                    history_id = %start_history_id,
                    "history_id too old, triggering backfill"
                );
                return trigger_backfill(&account_repo, &queue, &account).await;
            }
            Err(err) => return Err(map_gmail_error("list_history", err)),
        };

        // Track the latest historyId from responses
        if response.history_id.is_some() {
            latest_history_id = response.history_id.clone();
        }

        for record in response.history.iter() {
            if let Some(messages_added) = &record.messages_added {
                for change in messages_added {
                    enqueue_ingest_job(&queue, &payload.account_id, &change.message.id).await?;
                }
            }
        }

        match response.next_page_token {
            Some(token) => page_token = Some(token),
            None => break,
        }
    }

    let mut new_state = account.state.clone();
    new_state.history_id = latest_history_id.or_else(|| Some(payload.history_id.clone()));
    new_state.last_sync_at = Some(Utc::now());

    account_repo
        .update_state(&account.id, &new_state)
        .await
        .map_err(|err| map_account_error("update account state", err))?;

    info!(
        account_id = %payload.account_id,
        start = %start_history_id,
        new_history_id = %new_state.history_id.clone().unwrap_or_default(),
        "history sync complete"
    );

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
        .enqueue(JOB_TYPE_INGEST_GMAIL, payload, Some(idempotency), 1)
        .await
    {
        Ok(_) => Ok(()),
        Err(QueueError::DuplicateIdempotency { .. }) => {
            debug!(account_id, message_id, "ingest job already enqueued");
            Ok(())
        }
        Err(err) => Err(JobError::Retryable(format!(
            "enqueue ingest job failed: {err}"
        ))),
    }
}

/// Check if the error indicates the history ID was not found (stale/expired)
fn is_history_not_found(err: &GmailClientError) -> bool {
    match err {
        GmailClientError::Http(http_err) => http_err.status() == Some(StatusCode::NOT_FOUND),
        _ => false,
    }
}

/// Trigger a backfill job when history ID is stale
async fn trigger_backfill(
    account_repo: &AccountRepository,
    queue: &JobQueue,
    account: &Account,
) -> Result<(), JobError> {
    // Update account state to indicate backfill is needed
    let mut new_state = account.state.clone();
    new_state.sync_status = SyncStatus::NeedsBackfill;
    new_state.history_id = None; // Clear stale historyId

    account_repo
        .update_state(&account.id, &new_state)
        .await
        .map_err(|err| map_account_error("update state for backfill", err))?;

    // Determine backfill query based on last_sync_at
    let query = match new_state.last_sync_at {
        Some(dt) => {
            let days = (Utc::now() - dt).num_days().max(1).min(30);
            format!("newer_than:{}d", days)
        }
        None => "newer_than:7d".to_string(),
    };

    // Enqueue backfill job at lower priority
    let payload = serde_json::json!({
        "account_id": account.id,
        "query": query,
    });
    let idempotency = format!("{JOB_TYPE_BACKFILL_GMAIL}:{}:fallback", account.id);

    match queue
        .enqueue(JOB_TYPE_BACKFILL_GMAIL, payload, Some(idempotency), -10)
        .await
    {
        Ok(_) => info!(account_id = %account.id, "backfill job enqueued"),
        Err(QueueError::DuplicateIdempotency { .. }) => {
            debug!(account_id = %account.id, "backfill job already enqueued");
        }
        Err(err) => {
            return Err(JobError::Retryable(format!("enqueue backfill: {err}")));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountConfig, PubsubConfig};
    use crate::gmail::OAuthTokens;
    use crate::migrations::run_migrations;
    use serde_json::json;
    use tempfile::TempDir;
    use wiremock::matchers::{method, path, query_param};
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
            .create("user@example.com", Some("User".into()), config)
            .await
            .expect("create account");

        let dispatcher = JobDispatcher::new(db.clone(), reqwest::Client::new());
        (repo, dispatcher, dir, account.id)
    }

    #[tokio::test]
    async fn history_sync_enqueues_ingest_jobs_and_updates_state() {
        let (repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/history"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "history": [
                    {
                        "id": "11",
                        "messagesAdded": [
                            { "message": { "id": "msg-1", "threadId": "thr-1" } },
                            { "message": { "id": "msg-2", "threadId": "thr-2" } }
                        ]
                    }
                ],
                "historyId": "20"
            })))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_HISTORY_SYNC_GMAIL,
                json!({"account_id": account_id.clone(), "history_id": "10"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_history_sync_gmail(&dispatcher, job)
            .await
            .expect("history sync");

        // Two ingest jobs should be queued.
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT type, payload_json, idempotency_key, state FROM jobs WHERE type = ?1 ORDER BY payload_json",
                libsql::params!["ingest.gmail"],
            )
            .await
            .expect("query jobs");
        let first = rows.next().await.expect("first row").expect("row");
        let first_payload: String = first.get(1).expect("payload");
        assert!(first_payload.contains("msg-1"));
        let first_state: String = first.get(3).expect("state");
        assert_eq!(first_state, "queued");

        let second = rows.next().await.expect("second row").expect("row");
        let second_payload: String = second.get(1).expect("payload");
        assert!(second_payload.contains("msg-2"));

        let account = repo.get_by_id(&account_id).await.expect("fetch account");
        assert_eq!(account.state.history_id.as_deref(), Some("20"));
        assert!(account.state.last_sync_at.is_some());
    }

    #[tokio::test]
    async fn history_sync_retries_on_rate_limit() {
        let (_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/history"))
            .respond_with(ResponseTemplate::new(429))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_HISTORY_SYNC_GMAIL,
                json!({"account_id": account_id, "history_id": "10"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_history_sync_gmail(&dispatcher, job)
            .await
            .expect_err("history sync should retry on rate limit");

        match err {
            JobError::Retryable(msg) => assert!(msg.contains("429") || msg.contains("rate")),
            other => panic!("expected retryable, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn history_sync_triggers_backfill_on_not_found() {
        let (repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/history"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_HISTORY_SYNC_GMAIL,
                json!({"account_id": account_id.clone(), "history_id": "10"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        // Should succeed (not error) because it triggers backfill
        handle_history_sync_gmail(&dispatcher, job)
            .await
            .expect("history sync should succeed by triggering backfill");

        // Verify backfill job was enqueued
        let conn = dispatcher.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT type, payload_json FROM jobs WHERE type = ?1",
                libsql::params![crate::jobs::JOB_TYPE_BACKFILL_GMAIL],
            )
            .await
            .expect("query");
        let row = rows
            .next()
            .await
            .expect("row")
            .expect("backfill job exists");
        let job_type: String = row.get(0).expect("type");
        assert_eq!(job_type, "backfill.gmail");
        let payload: String = row.get(1).expect("payload");
        assert!(payload.contains(&account_id));

        // Verify account state was updated
        let account = repo.get_by_id(&account_id).await.expect("account");
        assert_eq!(account.state.sync_status, SyncStatus::NeedsBackfill);
        assert!(account.state.history_id.is_none()); // Cleared stale historyId
    }

    #[tokio::test]
    async fn history_sync_prefers_account_state_history_id() {
        let (repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        // Persist a newer history id in account state to ensure it overrides payload.
        let mut account = repo.get_by_id(&account_id).await.expect("account");
        account.state.history_id = Some("50".into());
        repo.update_state(&account.id, &account.state)
            .await
            .expect("update state");

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/history"))
            .and(query_param("startHistoryId", "50"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "history": [],
                "historyId": "60"
            })))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_HISTORY_SYNC_GMAIL,
                json!({"account_id": account_id.clone(), "history_id": "10"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_history_sync_gmail(&dispatcher, job)
            .await
            .expect("history sync");

        let account = repo.get_by_id(&account_id).await.expect("account");
        assert_eq!(account.state.history_id.as_deref(), Some("60"));
    }

    #[tokio::test]
    async fn history_sync_is_idempotent_for_ingest_jobs() {
        let (_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/history"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "history": [
                    { "id": "11", "messagesAdded": [ { "message": { "id": "msg-dup", "threadId": "thr-1" } } ] }
                ],
                "historyId": "20"
            })))
            .expect(2)
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_HISTORY_SYNC_GMAIL,
                json!({"account_id": account_id.clone(), "history_id": "10"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        // First run creates the ingest job.
        handle_history_sync_gmail(&dispatcher, job.clone())
            .await
            .expect("first history sync");
        // Second run should treat ingest job as duplicate and still succeed.
        handle_history_sync_gmail(&dispatcher, job)
            .await
            .expect("second history sync");

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
    }

    #[tokio::test]
    async fn history_sync_handles_pagination() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use wiremock::{Request, Respond, ResponseTemplate};

        let (repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        // Use a counter to track which response to return
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        struct PaginatedResponder {
            call_count: Arc<AtomicUsize>,
        }

        impl Respond for PaginatedResponder {
            fn respond(&self, _request: &Request) -> ResponseTemplate {
                let count = self.call_count.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    // First call - return page with nextPageToken
                    ResponseTemplate::new(200).set_body_json(json!({
                        "history": [
                            { "id": "11", "messagesAdded": [ { "message": { "id": "msg-page1", "threadId": "thr-1" } } ] }
                        ],
                        "historyId": "15",
                        "nextPageToken": "page2token"
                    }))
                } else {
                    // Second call - return final page
                    ResponseTemplate::new(200).set_body_json(json!({
                        "history": [
                            { "id": "16", "messagesAdded": [ { "message": { "id": "msg-page2", "threadId": "thr-2" } } ] }
                        ],
                        "historyId": "20"
                    }))
                }
            }
        }

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/history"))
            .respond_with(PaginatedResponder {
                call_count: call_count_clone,
            })
            .expect(2)
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_HISTORY_SYNC_GMAIL,
                json!({"account_id": account_id.clone(), "history_id": "10"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        handle_history_sync_gmail(&dispatcher, job)
            .await
            .expect("history sync with pagination");

        // Verify both calls were made
        assert_eq!(call_count.load(Ordering::SeqCst), 2);

        // Verify both messages from both pages were enqueued
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
        assert!(first_payload.contains("msg-page1"));

        let second = rows.next().await.expect("row").expect("second job");
        let second_payload: String = second.get(0).expect("payload");
        assert!(second_payload.contains("msg-page2"));

        // Verify historyId was updated to the last page's value
        let account = repo.get_by_id(&account_id).await.expect("account");
        assert_eq!(account.state.history_id.as_deref(), Some("20"));
    }

    #[tokio::test]
    async fn history_sync_retries_on_403_rate_limit() {
        let (_repo, dispatcher, _dir, account_id) = setup_account().await;
        let queue = JobQueue::new(dispatcher.db.clone());

        let server = MockServer::start().await;
        let api_base = format!("{}/gmail/v1/users", &server.uri());
        let dispatcher = dispatcher.with_gmail_api_base(api_base);

        Mock::given(method("GET"))
            .and(path("/gmail/v1/users/user@example.com/history"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let job_id = queue
            .enqueue(
                crate::jobs::JOB_TYPE_HISTORY_SYNC_GMAIL,
                json!({"account_id": account_id, "history_id": "10"}),
                None,
                1,
            )
            .await
            .expect("enqueue job");
        let job = queue.fetch_job(&job_id).await.expect("fetch job");

        let err = handle_history_sync_gmail(&dispatcher, job)
            .await
            .expect_err("history sync should retry on 403 rate limit");

        match err {
            JobError::Retryable(msg) => assert!(msg.contains("403") || msg.contains("rate")),
            other => panic!("expected retryable, got {:?}", other),
        }
    }
}
