use std::time::Duration;

use ashford_core::gmail::OAuthTokens;
use ashford_core::migrations::run_migrations;
use ashford_core::{
    AccountConfig, AccountRepository, Database, JOB_TYPE_HISTORY_SYNC_GMAIL, JOB_TYPE_INGEST_GMAIL,
    JobDispatcher, JobQueue, MessageError, MessageRepository, PubsubConfig, ThreadRepository,
    WorkerConfig,
    constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID},
    run_worker,
};
use base64::Engine;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::{sleep, timeout};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fast_worker_config() -> WorkerConfig {
    WorkerConfig {
        poll_interval: Duration::from_millis(5),
        heartbeat_interval: Duration::from_millis(10),
        drain_timeout: Duration::from_secs(5),
    }
}

fn build_message_response(message_id: &str, thread_id: &str) -> serde_json::Value {
    let plain = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("Hello world");
    let html = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("<p>Hello world</p>");

    json!({
        "id": message_id,
        "threadId": thread_id,
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

async fn setup_account() -> (Database, AccountRepository, JobQueue, TempDir, String) {
    let dir = TempDir::new().expect("temp dir");
    // Use a unique database filename to avoid any potential conflicts
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
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
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
    (db, repo, queue, dir, account.id)
}

#[tokio::test]
async fn worker_processes_history_and_ingests_message() {
    let (db, account_repo, queue, _dir, account_id) = setup_account().await;
    let msg_repo = MessageRepository::new(db.clone());
    let thread_repo = ThreadRepository::new(db.clone());

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());
    let dispatcher =
        JobDispatcher::new(db.clone(), reqwest::Client::new()).with_gmail_api_base(api_base);

    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/user@example.com/history"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "history": [
                {
                    "id": "11",
                    "messagesAdded": [
                        { "message": { "id": "msg-1", "threadId": "thr-1" } }
                    ]
                }
            ],
            "historyId": "20"
        })))
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/user@example.com/messages/msg-1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(build_message_response("msg-1", "thr-1")),
        )
        .mount(&server)
        .await;

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_HISTORY_SYNC_GMAIL,
            json!({"account_id": account_id.clone(), "history_id": "10"}),
            None,
            1,
        )
        .await
        .expect("enqueue history job");

    // Wait for ingest to create the message.
    let stored = timeout(Duration::from_secs(3), async {
        loop {
            match msg_repo
                .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "msg-1")
                .await
            {
                Ok(msg) => break msg,
                Err(MessageError::NotFound(_)) => sleep(Duration::from_millis(20)).await,
                Err(err) => panic!("unexpected message error: {err}"),
            }
        }
    })
    .await
    .expect("message should be ingested");

    assert_eq!(stored.subject.as_deref(), Some("Greetings"));
    assert_eq!(stored.body_plain.as_deref(), Some("Hello world"));
    assert_eq!(stored.body_html.as_deref(), Some("<p>Hello world</p>"));

    let thread = thread_repo
        .get_by_provider_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "thr-1")
        .await
        .expect("thread");
    assert_eq!(thread.provider_thread_id, "thr-1");

    let account = account_repo
        .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id)
        .await
        .expect("account");
    assert_eq!(account.state.history_id.as_deref(), Some("20"));

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_deduplicates_ingest_for_same_message() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let msg_repo = MessageRepository::new(db.clone());

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());
    let dispatcher =
        JobDispatcher::new(db.clone(), reqwest::Client::new()).with_gmail_api_base(api_base);

    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/user@example.com/history"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({
                "history": [
                    { "id": "11", "messagesAdded": [ { "message": { "id": "msg-dup", "threadId": "thr-dup" } } ] },
                    { "id": "12", "messagesAdded": [ { "message": { "id": "msg-dup", "threadId": "thr-dup" } } ] }
                ],
                "historyId": "25"
            })),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/user@example.com/messages/msg-dup"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(build_message_response("msg-dup", "thr-dup")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_HISTORY_SYNC_GMAIL,
            json!({"account_id": account_id.clone(), "history_id": "10"}),
            None,
            1,
        )
        .await
        .expect("enqueue history job");

    // Wait until the message exists and the ingest job finishes so we don't assert while it's
    // still running.
    timeout(Duration::from_secs(3), async {
        loop {
            match msg_repo
                .exists(DEFAULT_ORG_ID, DEFAULT_USER_ID, &account_id, "msg-dup")
                .await
            {
                Ok(true) => {
                    let conn = db.connection().await.expect("conn");
                    let mut rows = conn
                        .query(
                            "SELECT state FROM jobs WHERE type = ?1 ORDER BY created_at DESC LIMIT 1",
                            libsql::params![JOB_TYPE_INGEST_GMAIL],
                        )
                        .await
                        .expect("query ingest jobs");
                    let state: Option<String> =
                        rows.next().await.unwrap().map(|row| row.get(0).unwrap());

                    match state.as_deref() {
                        Some("completed") => break state,
                        Some("failed") | Some("canceled") => {
                            panic!("ingest job ended in unexpected state: {state:?}")
                        }
                        _ => sleep(Duration::from_millis(20)).await,
                    }
                }
                Ok(false) => sleep(Duration::from_millis(20)).await,
                Err(err) => panic!("message exists check failed: {err}"),
            }
        }
    })
    .await
    .expect("message should be persisted once and ingest should finish");

    let conn = db.connection().await.expect("conn");
    let mut rows = conn
        .query(
            "SELECT COUNT(*) FROM messages WHERE account_id = ?1 AND provider_message_id = ?2",
            libsql::params![account_id, "msg-dup"],
        )
        .await
        .expect("query count");
    let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(count, 1, "duplicate ingest should not insert another row");

    let mut jobs = conn
        .query(
            "SELECT state FROM jobs WHERE type = ?1",
            libsql::params![JOB_TYPE_INGEST_GMAIL],
        )
        .await
        .expect("query ingest jobs");
    let ingest_state: Option<String> = jobs.next().await.unwrap().map(|row| row.get(0).unwrap());
    assert!(
        matches!(ingest_state.as_deref(), Some(s) if s == "completed"),
        "ingest job should have completed once"
    );

    shutdown.cancel();
    let _ = worker.await;
}
