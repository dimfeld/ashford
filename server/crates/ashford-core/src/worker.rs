use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::FutureExt;
use serde_json::json;
use thiserror::Error;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use crate::queue::{Job, JobContext, JobQueue, JobState, QueueError};

#[derive(Clone, Copy)]
pub struct WorkerConfig {
    pub poll_interval: Duration,
    pub heartbeat_interval: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("queue error: {0}")]
    Queue(#[from] QueueError),
}

#[derive(Debug, Error)]
pub enum JobError {
    #[error("retryable: {0}")]
    Retryable(String),
    #[error("fatal: {0}")]
    Fatal(String),
}

impl JobError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, JobError::Retryable(_))
    }

    fn message(&self) -> &str {
        match self {
            JobError::Retryable(msg) | JobError::Fatal(msg) => msg,
        }
    }
}

#[async_trait]
pub trait JobExecutor: Send + Sync {
    async fn execute(&self, job: Job, ctx: JobContext) -> Result<(), JobError>;
}

pub struct NoopExecutor;

#[async_trait]
impl JobExecutor for NoopExecutor {
    async fn execute(&self, job: Job, ctx: JobContext) -> Result<(), JobError> {
        let step_id = ctx
            .start_step("noop")
            .await
            .map_err(|err| JobError::Fatal(err.to_string()))?;
        ctx.finish_step(&step_id, Some(json!({"job": job.id, "status": "ok"})))
            .await
            .map_err(|err| JobError::Fatal(err.to_string()))?;
        Ok(())
    }
}

pub async fn run_worker<E: JobExecutor>(
    queue: JobQueue,
    executor: E,
    config: WorkerConfig,
    shutdown: CancellationToken,
) {
    let executor = Arc::new(executor);
    loop {
        if shutdown.is_cancelled() {
            break;
        }

        match queue.claim_next().await {
            Ok(Some(job)) => {
                handle_job(
                    queue.clone(),
                    executor.clone(),
                    config,
                    shutdown.clone(),
                    job,
                )
                .await
            }
            Ok(None) => sleep(config.poll_interval).await,
            Err(err) => {
                error!(error = %err, "failed to claim next job");
                sleep(config.poll_interval).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queue::JobState;
    use crate::{Database, migrations::run_migrations};
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::time::{sleep, timeout};

    async fn setup_queue() -> (JobQueue, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_path = dir.path().join("db.sqlite");
        let db = Database::new(db_path.as_path()).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (JobQueue::new(db), dir)
    }

    fn fast_config() -> WorkerConfig {
        WorkerConfig {
            poll_interval: Duration::from_millis(5),
            heartbeat_interval: Duration::from_millis(10),
        }
    }

    #[tokio::test]
    async fn worker_completes_job() {
        let (queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("noop", json!({"task":1}), None, 0)
            .await
            .expect("enqueue");

        let shutdown = CancellationToken::new();
        let worker = tokio::spawn(run_worker(
            queue.clone(),
            NoopExecutor,
            fast_config(),
            shutdown.clone(),
        ));

        timeout(Duration::from_secs(2), async {
            loop {
                let job = queue.fetch_job(&job_id).await.expect("fetch");
                if matches!(job.state, JobState::Completed) {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("job should complete within timeout");

        shutdown.cancel();
        let _ = worker.await;

        let job = queue.fetch_job(&job_id).await.expect("fetch final");
        assert!(matches!(job.state, JobState::Completed));
        assert!(job.last_error.is_none());
    }

    struct RetryExecutor;

    #[async_trait]
    impl JobExecutor for RetryExecutor {
        async fn execute(&self, job: Job, _ctx: JobContext) -> Result<(), JobError> {
            Err(JobError::Retryable(format!("retry {}", job.id)))
        }
    }

    #[tokio::test]
    async fn worker_retries_on_retryable_error() {
        let (queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("failable", json!({}), None, 0)
            .await
            .expect("enqueue");

        let shutdown = CancellationToken::new();
        let worker = tokio::spawn(run_worker(
            queue.clone(),
            RetryExecutor,
            fast_config(),
            shutdown.clone(),
        ));

        timeout(Duration::from_secs(2), async {
            loop {
                let job = queue.fetch_job(&job_id).await.expect("fetch");
                if matches!(job.state, JobState::Queued) && job.last_error.is_some() {
                    assert!(job.not_before.is_some(), "retry should schedule not_before");
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("job should be requeued");

        shutdown.cancel();
        let _ = worker.await;

        let job = queue.fetch_job(&job_id).await.expect("fetch final");
        assert!(matches!(job.state, JobState::Queued));
        assert!(job.last_error.unwrap().contains("retry"));
        assert!(job.not_before.is_some());
    }

    struct FatalExecutor;

    #[async_trait]
    impl JobExecutor for FatalExecutor {
        async fn execute(&self, job: Job, _ctx: JobContext) -> Result<(), JobError> {
            Err(JobError::Fatal(format!("fatal {}", job.id)))
        }
    }

    #[tokio::test]
    async fn worker_marks_fatal_as_failed() {
        let (queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("fatal", json!({}), None, 0)
            .await
            .expect("enqueue");

        let shutdown = CancellationToken::new();
        let worker = tokio::spawn(run_worker(
            queue.clone(),
            FatalExecutor,
            fast_config(),
            shutdown.clone(),
        ));

        timeout(Duration::from_secs(2), async {
            loop {
                let job = queue.fetch_job(&job_id).await.expect("fetch");
                if matches!(job.state, JobState::Failed) {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("job should fail");

        shutdown.cancel();
        let _ = worker.await;

        let job = queue.fetch_job(&job_id).await.expect("fetch final");
        assert!(matches!(job.state, JobState::Failed));
        assert!(job.not_before.is_none());
        assert!(job.last_error.unwrap().contains("fatal"));
    }

    struct PanicExecutor;

    #[async_trait]
    impl JobExecutor for PanicExecutor {
        async fn execute(&self, _job: Job, _ctx: JobContext) -> Result<(), JobError> {
            panic!("panic in executor");
        }
    }

    #[tokio::test]
    async fn worker_marks_panic_as_retryable_failure() {
        let (queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("panic", json!({}), None, 0)
            .await
            .expect("enqueue");

        let shutdown = CancellationToken::new();
        let worker = tokio::spawn(run_worker(
            queue.clone(),
            PanicExecutor,
            fast_config(),
            shutdown.clone(),
        ));

        timeout(Duration::from_secs(2), async {
            loop {
                let job = queue.fetch_job(&job_id).await.expect("fetch");
                if matches!(job.state, JobState::Queued) && job.last_error.is_some() {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("job should be requeued after panic");

        shutdown.cancel();
        let _ = worker.await;

        let job = queue.fetch_job(&job_id).await.expect("fetch final");
        assert!(matches!(job.state, JobState::Queued));
        assert!(job.last_error.unwrap().contains("panic"));
        assert!(job.not_before.is_some());
    }
}

async fn handle_job<E: JobExecutor>(
    queue: JobQueue,
    executor: Arc<E>,
    config: WorkerConfig,
    shutdown: CancellationToken,
    job: Job,
) {
    info!(job_id = %job.id, job_type = %job.job_type, "processing job");
    let heartbeat_cancel = shutdown.child_token();
    let heartbeat_queue = queue.clone();
    let job_id = job.id.clone();
    let heartbeat_interval = config.heartbeat_interval;

    let heartbeat_task = tokio::spawn({
        let heartbeat_cancel = heartbeat_cancel.clone();
        async move {
            loop {
                tokio::select! {
                    _ = heartbeat_cancel.cancelled() => break,
                    _ = sleep(heartbeat_interval) => {
                        if let Err(err) = heartbeat_queue.heartbeat(&job_id).await {
                            warn!(job_id = %job_id, error = %err, "heartbeat failed");
                        }
                    }
                }
            }
        }
    });

    let ctx = JobContext::new(queue.clone(), job.clone());
    let result = AssertUnwindSafe(executor.execute(job.clone(), ctx))
        .catch_unwind()
        .await;

    let finalize = match result {
        Ok(Ok(())) => FinalizeAction::Complete,
        Ok(Err(job_err)) => FinalizeAction::Fail {
            message: job_err.message().to_string(),
            retry: job_err.is_retryable(),
        },
        Err(panic) => {
            let err_msg = if let Some(msg) = panic.downcast_ref::<&str>() {
                msg.to_string()
            } else if let Some(msg) = panic.downcast_ref::<String>() {
                msg.clone()
            } else {
                "worker panic".to_string()
            };
            warn!(job_id = %job.id, "job panicked: {err_msg}");
            FinalizeAction::Fail {
                message: err_msg,
                retry: true,
            }
        }
    };

    if let Err(err) = finalize_job(
        queue.clone(),
        &job,
        finalize,
        &heartbeat_cancel,
        heartbeat_interval,
        shutdown,
    )
    .await
    {
        error!(job_id = %job.id, error = %err, "failed to persist job outcome");
    }

    heartbeat_cancel.cancel();
    let _ = heartbeat_task.await;
}

enum FinalizeAction {
    Complete,
    Fail { message: String, retry: bool },
}

async fn finalize_job(
    queue: JobQueue,
    job: &Job,
    action: FinalizeAction,
    heartbeat_cancel: &CancellationToken,
    heartbeat_interval: Duration,
    shutdown: CancellationToken,
) -> Result<(), QueueError> {
    let mut attempt: u32 = 0;

    loop {
        if shutdown.is_cancelled() {
            return Ok(());
        }

        let current_state = match queue.fetch_job(&job.id).await {
            Ok(job) => job,
            Err(err) => {
                attempt = attempt.saturating_add(1);
                let mut backoff = heartbeat_interval / 2;
                if backoff < Duration::from_millis(10) {
                    backoff = Duration::from_millis(10);
                }
                if backoff > Duration::from_secs(5) {
                    backoff = Duration::from_secs(5);
                }
                warn!(job_id = %job.id, attempt, error = %err, "failed to fetch job for finalize; retrying");

                tokio::select! {
                    _ = shutdown.cancelled() => return Ok(()),
                    _ = sleep(backoff) => {},
                    _ = heartbeat_cancel.cancelled() => return Err(err),
                }

                continue;
            }
        };
        if matches!(current_state.state, JobState::Canceled) {
            info!(job_id = %job.id, "job canceled during execution; skipping finalize");
            return Ok(());
        }

        let result = match &action {
            FinalizeAction::Complete => queue.complete(&job.id, None).await,
            FinalizeAction::Fail { message, retry } => {
                queue.fail(&job.id, message.clone(), *retry).await
            }
        };

        match result {
            Ok(_) => {
                match action {
                    FinalizeAction::Complete => info!(job_id = %job.id, "job completed"),
                    FinalizeAction::Fail { retry: true, .. } => {
                        warn!(job_id = %job.id, "job failed and will retry")
                    }
                    FinalizeAction::Fail { retry: false, .. } => {
                        warn!(job_id = %job.id, "job failed permanently")
                    }
                }
                return Ok(());
            }
            Err(QueueError::NotRunning(_)) => {
                info!(job_id = %job.id, "job already moved out of running state");
                return Ok(());
            }
            Err(err) => {
                attempt = attempt.saturating_add(1);
                let mut backoff = heartbeat_interval / 2;
                if backoff < Duration::from_millis(10) {
                    backoff = Duration::from_millis(10);
                }
                if backoff > Duration::from_secs(5) {
                    backoff = Duration::from_secs(5);
                }
                warn!(job_id = %job.id, attempt, error = %err, "failed to persist job outcome; retrying");

                // Keep heartbeat alive while retrying to avoid stale running jobs.
                tokio::select! {
                    _ = shutdown.cancelled() => return Ok(()),
                    _ = sleep(backoff) => {},
                    _ = heartbeat_cancel.cancelled() => return Err(err),
                }
            }
        }
    }
}
