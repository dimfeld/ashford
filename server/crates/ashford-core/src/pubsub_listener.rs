use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use futures::StreamExt;
use google_cloud_pubsub::subscriber::ReceivedMessage;
use serde_json::json;
use tokio::task::JoinHandle;
use tokio::time::{Interval, MissedTickBehavior, sleep};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::accounts::AccountRepository;
use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
use crate::pubsub::{
    GmailNotification, PubsubError, parse_gmail_notification,
    subscriber_client_from_service_account,
};
use crate::queue::QueueError;
use crate::{Database, JobQueue, jobs::JOB_TYPE_HISTORY_SYNC_GMAIL};

const SUPERVISOR_POLL_SECS: u64 = 30;
const STREAM_RECONNECT_MAX_BACKOFF: Duration = Duration::from_secs(60);

pub async fn run_account_listener(
    account_id: String,
    subscription_name: String,
    service_account_json: String,
    queue: JobQueue,
    shutdown: CancellationToken,
) -> Result<(), PubsubError> {
    let client = subscriber_client_from_service_account(&service_account_json).await?;
    let subscription = client.subscription(&subscription_name);
    let mut backoff = Duration::from_secs(1);

    info!(account_id, subscription = %subscription_name, "starting pubsub listener");

    while !shutdown.is_cancelled() {
        let mut stream = match subscription.subscribe(None).await {
            Ok(stream) => {
                backoff = Duration::from_secs(1);
                stream
            }
            Err(err) => {
                warn!(account_id, error = %err, "failed to open streaming pull; backing off");
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = sleep(backoff) => {},
                }
                backoff = (backoff * 2).min(STREAM_RECONNECT_MAX_BACKOFF);
                continue;
            }
        };

        let cancel = stream.cancellable();
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => {
                    cancel.cancel();
                    let _ = stream.dispose().await;
                    info!(account_id, "listener shutdown requested");
                    return Ok(());
                }
                maybe_msg = stream.next() => {
                    let Some(message) = maybe_msg else {
                        warn!(account_id, "stream ended; will reconnect");
                        let _ = stream.dispose().await;
                        break;
                    };

                    let result = process_message(&account_id, &message, &queue).await;

                    match result {
                        Ok(()) => {
                            if let Err(err) = message.ack().await {
                                warn!(account_id, error = %err, "failed to ack pubsub message");
                            }
                        }
                        Err(err) => {
                            warn!(account_id, error = %err, "failed to process pubsub message; nacking");
                            if let Err(nack_err) = message.nack().await {
                                warn!(account_id, error = %nack_err, "failed to nack message after processing error");
                            }
                        }
                    }
                }
            }
        }

        tokio::select! {
            _ = shutdown.cancelled() => break,
            _ = sleep(backoff) => {}
        }
        backoff = (backoff * 2).min(STREAM_RECONNECT_MAX_BACKOFF);
    }

    Ok(())
}

async fn process_message(
    account_id: &str,
    message: &ReceivedMessage,
    queue: &JobQueue,
) -> Result<(), PubsubError> {
    let gmail = parse_gmail_notification(&message.message)?;

    enqueue_history_job(account_id, &gmail, queue).await
}

async fn enqueue_history_job(
    account_id: &str,
    notification: &GmailNotification,
    queue: &JobQueue,
) -> Result<(), PubsubError> {
    let payload = json!({
        "account_id": account_id,
        "history_id": notification.history_id,
    });

    let idempotency = format!(
        "{}:{}:{}",
        JOB_TYPE_HISTORY_SYNC_GMAIL, account_id, notification.history_id
    );

    match queue
        .enqueue(JOB_TYPE_HISTORY_SYNC_GMAIL, payload, Some(idempotency), 1)
        .await
    {
        Ok(_) => Ok(()),
        Err(QueueError::DuplicateIdempotency { .. }) => {
            debug!(account_id, history_id = %notification.history_id, "history job already enqueued");
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn run_pubsub_supervisor(
    db: Database,
    queue: JobQueue,
    shutdown: CancellationToken,
) -> Result<(), PubsubError> {
    let repo = AccountRepository::new(db);
    let mut interval = build_poll_interval();
    let mut listeners: HashMap<String, ListenerHandle> = HashMap::new();

    reconcile_listeners(&repo, &queue, &shutdown, &mut listeners).await?;

    loop {
        tokio::select! {
            _ = shutdown.cancelled() => {
                info!("pubsub supervisor shutting down");
                break;
            }
            _ = interval.tick() => {
                reconcile_listeners(&repo, &queue, &shutdown, &mut listeners).await?;
            }
        }
    }

    for (account_id, handle) in listeners.into_iter() {
        handle.cancel.cancel();
        if let Err(err) = handle.task.await {
            warn!(account_id, error = ?err, "listener task join error");
        }
    }

    Ok(())
}

async fn reconcile_listeners(
    repo: &AccountRepository,
    queue: &JobQueue,
    shutdown: &CancellationToken,
    listeners: &mut HashMap<String, ListenerHandle>,
) -> Result<(), PubsubError> {
    let accounts = repo.list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID).await?;

    // Determine desired listeners based on account config.
    let mut desired: HashMap<String, DesiredListener> = HashMap::new();
    for account in accounts {
        if let (Some(subscription), Some(credentials)) = (
            account.config.pubsub.subscription.clone(),
            account.config.pubsub.service_account_json.clone(),
        ) {
            desired.insert(
                account.id.clone(),
                DesiredListener {
                    account_id: account.id,
                    subscription,
                    credentials,
                },
            );
        }
    }

    // Stop listeners that are no longer desired or have config changes.
    let existing_ids: Vec<String> = listeners.keys().cloned().collect();
    for account_id in existing_ids {
        // Restart listeners that exited unexpectedly even if config is unchanged.
        if let Some(existing) = listeners.get(&account_id) {
            if existing.task.is_finished() {
                let existing = listeners.remove(&account_id).expect("listener must exist");
                info!(account_id, "restarting listener after unexpected exit");
                if let Err(err) = existing.task.await {
                    warn!(account_id, error = ?err, "listener task join error after unexpected exit");
                }
            }
        }

        let Some(desired_cfg) = desired.get(&account_id) else {
            if let Some(existing) = listeners.remove(&account_id) {
                info!(
                    account_id,
                    "stopping listener (account removed or disabled)"
                );
                existing.cancel.cancel();
                if let Err(err) = existing.task.await {
                    warn!(account_id, error = ?err, "listener task join error after stop");
                }
            }
            continue;
        };

        if let Some(existing) = listeners.get(&account_id) {
            if existing.config.matches(desired_cfg) {
                continue;
            }
        }

        if let Some(existing) = listeners.remove(&account_id) {
            info!(account_id, "restarting listener after config change");
            existing.cancel.cancel();
            if let Err(err) = existing.task.await {
                warn!(account_id, error = ?err, "listener task join error during restart");
            }
        }
    }

    // Start missing listeners.
    for (account_id, cfg) in desired.into_iter() {
        if listeners.contains_key(&account_id) {
            continue;
        }

        let handle = spawn_listener_task(cfg, queue.clone(), shutdown.clone());
        listeners.insert(account_id, handle);
    }

    Ok(())
}

fn spawn_listener_task(
    cfg: DesiredListener,
    queue: JobQueue,
    shutdown: CancellationToken,
) -> ListenerHandle {
    let listener_shutdown = CancellationToken::new();
    let listener_shutdown_task = listener_shutdown.clone();
    let runtime_cfg = ListenerRuntimeConfig::from_desired(&cfg);
    let DesiredListener {
        account_id,
        subscription,
        credentials,
    } = cfg;

    let task = tokio::spawn(async move {
        let parent_shutdown = shutdown.child_token();

        let combined = CancellationToken::new();
        let combined_child = combined.child_token();
        // Cancel combined token when either parent or listener-specific cancellation fires.
        let cancel_watch = tokio::spawn({
            let combined = combined.clone();
            let listener_shutdown = listener_shutdown_task.clone();
            let parent_shutdown = parent_shutdown.clone();
            async move {
                tokio::select! {
                    _ = parent_shutdown.cancelled() => combined.cancel(),
                    _ = listener_shutdown.cancelled() => combined.cancel(),
                }
            }
        });

        if let Err(err) =
            run_account_listener(account_id, subscription, credentials, queue, combined_child).await
        {
            error!(error = %err, "account listener exited with error");
        }

        combined.cancel();
        cancel_watch.abort();
    });

    ListenerHandle {
        cancel: listener_shutdown,
        task,
        config: runtime_cfg,
    }
}

struct DesiredListener {
    account_id: String,
    subscription: String,
    credentials: String,
}

#[derive(Clone)]
struct ListenerRuntimeConfig {
    subscription: String,
    credentials_fingerprint: u64,
}

impl ListenerRuntimeConfig {
    fn from_desired(cfg: &DesiredListener) -> Self {
        Self {
            subscription: cfg.subscription.clone(),
            credentials_fingerprint: fingerprint(&cfg.credentials),
        }
    }

    fn matches(&self, desired: &DesiredListener) -> bool {
        self.subscription == desired.subscription
            && self.credentials_fingerprint == fingerprint(&desired.credentials)
    }
}

struct ListenerHandle {
    cancel: CancellationToken,
    task: JoinHandle<()>,
    config: ListenerRuntimeConfig,
}

fn fingerprint(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

fn build_poll_interval() -> Interval {
    let mut interval = tokio::time::interval(Duration::from_secs(SUPERVISOR_POLL_SECS));
    interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
    interval
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use tempfile::TempDir;

    async fn setup_queue() -> (Database, JobQueue, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        // Use a unique database filename to avoid any potential conflicts
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(db_path.as_path()).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (db.clone(), JobQueue::new(db), dir)
    }

    #[tokio::test]
    async fn enqueue_history_job_creates_idempotent_job() {
        let (db, queue, _dir) = setup_queue().await;
        let notification = GmailNotification {
            email_address: "user@example.com".into(),
            history_id: "99".into(),
        };

        enqueue_history_job("account-1", &notification, &queue)
            .await
            .expect("enqueue history job");

        let conn = db.connection().await.expect("conn");
        let mut rows = conn
            .query("SELECT type, payload_json, idempotency_key FROM jobs", ())
            .await
            .expect("query jobs");
        let row = rows.next().await.expect("row opt").expect("row");
        let job_type: String = row.get(0).expect("type");
        let payload_json: String = row.get(1).expect("payload");
        let idempotency: Option<String> = row.get(2).expect("idempotency");

        assert_eq!(job_type, JOB_TYPE_HISTORY_SYNC_GMAIL);
        let payload: serde_json::Value = serde_json::from_str(&payload_json).expect("payload json");
        assert_eq!(payload["account_id"], "account-1");
        assert_eq!(payload["history_id"], "99");
        assert_eq!(
            idempotency.as_deref(),
            Some("history.sync.gmail:account-1:99")
        );
        assert!(rows.next().await.expect("no extra").is_none());
    }

    #[tokio::test]
    async fn enqueue_history_job_is_idempotent_on_duplicate() {
        let (db, queue, _dir) = setup_queue().await;
        let notification = GmailNotification {
            email_address: "user@example.com".into(),
            history_id: "99".into(),
        };

        enqueue_history_job("account-1", &notification, &queue)
            .await
            .expect("first enqueue");
        enqueue_history_job("account-1", &notification, &queue)
            .await
            .expect("duplicate enqueue treated as success");

        let conn = db.connection().await.expect("conn");
        let mut rows = conn
            .query("SELECT count(*) FROM jobs", ())
            .await
            .expect("count jobs");
        let count: i64 = rows
            .next()
            .await
            .expect("row opt")
            .expect("row")
            .get(0)
            .expect("count");
        assert_eq!(count, 1, "duplicate enqueue should not add new job");
    }

    #[test]
    fn listener_runtime_config_detects_changes() {
        let desired = DesiredListener {
            account_id: "a1".into(),
            subscription: "sub-1".into(),
            credentials: "creds-1".into(),
        };

        let runtime = ListenerRuntimeConfig::from_desired(&desired);
        assert!(runtime.matches(&desired), "identical config should match");

        let different_subscription = DesiredListener {
            account_id: "a1".into(),
            subscription: "sub-2".into(),
            credentials: "creds-1".into(),
        };
        assert!(
            !runtime.matches(&different_subscription),
            "subscription change must trigger restart"
        );

        let different_creds = DesiredListener {
            credentials: "creds-2".into(),
            account_id: "a1".into(),
            subscription: "sub-1".into(),
        };
        assert!(
            !runtime.matches(&different_creds),
            "credential change must trigger restart"
        );
    }
}
