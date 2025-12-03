use std::sync::Arc;
use std::time::Duration;

use ashford_core::gmail::OAuthTokens;
use ashford_core::messages::{Mailbox, NewMessage};
use ashford_core::migrations::run_migrations;
use ashford_core::threads::ThreadRepository;
use ashford_core::{
    AccountConfig, AccountRepository, ActionRepository, ActionStatus, Database,
    JOB_TYPE_ACTION_GMAIL, JobDispatcher, JobQueue, MessageRepository, MockLLMClient, NewAction,
    PolicyConfig, PubsubConfig, WorkerConfig,
    constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID},
    run_worker,
};
use chrono::{Duration as ChronoDuration, Utc};
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

async fn setup_account() -> (Database, AccountRepository, JobQueue, TempDir, String) {
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
            expires_at: Utc::now() + ChronoDuration::hours(1),
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

async fn setup_message(db: &Database, account_id: &str, provider_message_id: &str) -> String {
    let thread_repo = ThreadRepository::new(db.clone());
    let thread = thread_repo
        .upsert(
            DEFAULT_ORG_ID,
            DEFAULT_USER_ID,
            account_id,
            "thread-action-test",
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

async fn create_action(
    db: &Database,
    account_id: &str,
    message_id: &str,
    action_type: &str,
    parameters: serde_json::Value,
) -> String {
    let action_repo = ActionRepository::new(db.clone());
    let action = action_repo
        .create(NewAction {
            org_id: DEFAULT_ORG_ID,
            user_id: DEFAULT_USER_ID,
            account_id: account_id.to_string(),
            message_id: message_id.to_string(),
            decision_id: None,
            action_type: action_type.to_string(),
            parameters_json: parameters,
            status: ActionStatus::Queued,
            error_message: None,
            executed_at: None,
            undo_hint_json: json!({}),
            trace_id: None,
        })
        .await
        .expect("create action");

    action.id
}

#[tokio::test]
async fn worker_executes_archive_action_and_populates_undo_hint() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-archive-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    // Create the action record
    let action_id =
        create_action(&db, &account_id, &internal_message_id, "archive", json!({})).await;

    // Set up mock Gmail API
    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    // Mock for getting message state (pre-image capture)
    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["INBOX", "UNREAD", "IMPORTANT"],
            "snippet": "Test",
            "internalDate": "1730000000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Mock for modifying the message (archive = remove INBOX label)
    Mock::given(method("POST"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}/modify",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["UNREAD", "IMPORTANT"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    // Enqueue the action job
    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    // Wait for action to complete
    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    // Verify the action completed successfully
    assert_eq!(completed_action.status, ActionStatus::Completed);
    assert!(completed_action.executed_at.is_some());

    // Verify undo_hint is populated with pre-image state
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "archive");
    assert_eq!(undo_hint["inverse_action"], "apply_label");
    assert_eq!(undo_hint["inverse_parameters"]["label"], "INBOX");
    assert_eq!(undo_hint["pre_in_inbox"], true);
    assert_eq!(undo_hint["pre_unread"], true);
    assert!(
        undo_hint["pre_labels"]
            .as_array()
            .unwrap()
            .contains(&json!("IMPORTANT"))
    );

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_executes_mark_read_action_successfully() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-mark-read-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id = create_action(
        &db,
        &account_id,
        &internal_message_id,
        "mark_read",
        json!({}),
    )
    .await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    // Pre-image capture
    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["INBOX", "UNREAD"],
            "snippet": "Test",
            "internalDate": "1730000000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Modify message (remove UNREAD)
    Mock::given(method("POST"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}/modify",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "labelIds": ["INBOX"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    assert_eq!(completed_action.status, ActionStatus::Completed);
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "mark_read");
    assert_eq!(undo_hint["inverse_action"], "mark_unread");
    assert_eq!(undo_hint["pre_unread"], true);

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_executes_mark_unread_action_successfully() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-mark-unread-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id = create_action(
        &db,
        &account_id,
        &internal_message_id,
        "mark_unread",
        json!({}),
    )
    .await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    // Pre-image capture (message currently read)
    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["INBOX"],
            "snippet": "Test",
            "internalDate": "1730000000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Modify message (add UNREAD)
    Mock::given(method("POST"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}/modify",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "labelIds": ["INBOX", "UNREAD"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    assert_eq!(completed_action.status, ActionStatus::Completed);
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "mark_unread");
    assert_eq!(undo_hint["inverse_action"], "mark_read");
    assert_eq!(undo_hint["pre_unread"], false);

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_executes_apply_label_action_successfully() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-apply-label-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id = create_action(
        &db,
        &account_id,
        &internal_message_id,
        "apply_label",
        json!({"label": "IMPORTANT"}),
    )
    .await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    // Pre-image capture
    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["INBOX"],
            "snippet": "Test",
            "internalDate": "1730000000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Modify message (add IMPORTANT)
    Mock::given(method("POST"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}/modify",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "labelIds": ["INBOX", "IMPORTANT"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    assert_eq!(completed_action.status, ActionStatus::Completed);
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "apply_label");
    assert_eq!(undo_hint["inverse_action"], "remove_label");
    assert_eq!(undo_hint["inverse_parameters"]["label"], "IMPORTANT");

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_executes_remove_label_action_successfully() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-remove-label-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id = create_action(
        &db,
        &account_id,
        &internal_message_id,
        "remove_label",
        json!({"label": "IMPORTANT"}),
    )
    .await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    // Pre-image capture
    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["INBOX", "IMPORTANT"],
            "snippet": "Test",
            "internalDate": "1730000000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Modify message (remove IMPORTANT)
    Mock::given(method("POST"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}/modify",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "labelIds": ["INBOX"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    assert_eq!(completed_action.status, ActionStatus::Completed);
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "remove_label");
    assert_eq!(undo_hint["inverse_action"], "apply_label");
    assert_eq!(undo_hint["inverse_parameters"]["label"], "IMPORTANT");
    assert!(
        undo_hint["pre_labels"]
            .as_array()
            .unwrap()
            .contains(&json!("IMPORTANT"))
    );

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_executes_star_action_successfully() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-star-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id = create_action(&db, &account_id, &internal_message_id, "star", json!({})).await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["INBOX"],
            "snippet": "Test",
            "internalDate": "1730000000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}/modify",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "labelIds": ["INBOX", "STARRED"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    assert_eq!(completed_action.status, ActionStatus::Completed);
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "star");
    assert_eq!(undo_hint["inverse_action"], "unstar");
    assert_eq!(undo_hint["pre_starred"], false);

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_executes_unstar_action_successfully() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-unstar-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id =
        create_action(&db, &account_id, &internal_message_id, "unstar", json!({})).await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["INBOX", "STARRED"],
            "snippet": "Test",
            "internalDate": "1730000000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}/modify",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "labelIds": ["INBOX"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    assert_eq!(completed_action.status, ActionStatus::Completed);
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "unstar");
    assert_eq!(undo_hint["inverse_action"], "star");
    assert_eq!(undo_hint["pre_starred"], true);

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_executes_trash_action_successfully() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-trash-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id = create_action(&db, &account_id, &internal_message_id, "trash", json!({})).await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    // Pre-image capture
    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["INBOX", "IMPORTANT"],
            "snippet": "Test",
            "internalDate": "1730000000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Trash message
    Mock::given(method("POST"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}/trash",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "labelIds": ["TRASH"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    assert_eq!(completed_action.status, ActionStatus::Completed);
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "trash");
    assert_eq!(undo_hint["inverse_action"], "restore");
    // Pre-image shows it was in INBOX before trash
    assert_eq!(undo_hint["pre_in_inbox"], true);
    assert_eq!(undo_hint["pre_in_trash"], false);

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_executes_restore_action_successfully() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-restore-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id =
        create_action(&db, &account_id, &internal_message_id, "restore", json!({})).await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    // Pre-image capture showing message in trash
    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "threadId": "thread-action-test",
            "labelIds": ["TRASH"],
            "snippet": "Test",
            "internalDate": "1730000000000"
        })))
        .expect(1)
        .mount(&server)
        .await;

    // Untrash message
    Mock::given(method("POST"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}/untrash",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": provider_message_id,
            "labelIds": ["INBOX"],
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    assert_eq!(completed_action.status, ActionStatus::Completed);
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "restore");
    assert_eq!(undo_hint["inverse_action"], "trash");
    assert_eq!(undo_hint["pre_in_trash"], true);

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_executes_delete_action_with_irreversible_undo_hint() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    let provider_message_id = "gmail-msg-delete-test";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id =
        create_action(&db, &account_id, &internal_message_id, "delete", json!({})).await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    // Delete message (no pre-image needed, it's irreversible)
    Mock::given(method("DELETE"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let completed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Completed => break action,
                ActionStatus::Failed | ActionStatus::Canceled | ActionStatus::Rejected => {
                    panic!(
                        "action ended in unexpected state: {:?}, error: {:?}",
                        action.status, action.error_message
                    );
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should complete");

    assert_eq!(completed_action.status, ActionStatus::Completed);
    let undo_hint = &completed_action.undo_hint_json;
    assert_eq!(undo_hint["action"], "delete");
    assert_eq!(undo_hint["inverse_action"], "none");
    assert_eq!(undo_hint["irreversible"], true);

    shutdown.cancel();
    let _ = worker.await;
}

#[tokio::test]
async fn worker_marks_action_failed_on_gmail_404() {
    let (db, _account_repo, queue, _dir, account_id) = setup_account().await;
    // Use a provider message ID that will 404
    let provider_message_id = "gmail-msg-not-found";
    let internal_message_id = setup_message(&db, &account_id, provider_message_id).await;

    let action_id =
        create_action(&db, &account_id, &internal_message_id, "archive", json!({})).await;

    let server = MockServer::start().await;
    let api_base = format!("{}/gmail/v1/users", &server.uri());

    // Pre-image fetch returns 404 (message doesn't exist in Gmail)
    Mock::given(method("GET"))
        .and(path(format!(
            "/gmail/v1/users/user@example.com/messages/{}",
            provider_message_id
        )))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "error": {
                "code": 404,
                "message": "Requested entity was not found."
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let dispatcher = JobDispatcher::new(
        db.clone(),
        reqwest::Client::new(),
        Arc::new(MockLLMClient::new()),
        PolicyConfig::default(),
    )
    .with_gmail_api_base(api_base);

    let shutdown = CancellationToken::new();
    let worker = tokio::spawn(run_worker(
        queue.clone(),
        dispatcher.clone(),
        fast_worker_config(),
        shutdown.clone(),
    ));

    queue
        .enqueue(
            JOB_TYPE_ACTION_GMAIL,
            json!({"account_id": account_id.clone(), "action_id": action_id.clone()}),
            None,
            1,
        )
        .await
        .expect("enqueue action job");

    let action_repo = ActionRepository::new(db.clone());
    let failed_action = timeout(Duration::from_secs(5), async {
        loop {
            let action = action_repo
                .get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id)
                .await
                .expect("get action");

            match action.status {
                ActionStatus::Failed => break action,
                ActionStatus::Completed => {
                    panic!("action should have failed, but completed");
                }
                _ => sleep(Duration::from_millis(20)).await,
            }
        }
    })
    .await
    .expect("action should fail");

    assert_eq!(failed_action.status, ActionStatus::Failed);
    assert!(failed_action.error_message.is_some());

    shutdown.cancel();
    let _ = worker.await;
}
