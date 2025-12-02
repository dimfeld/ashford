use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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
    /// Maximum time to wait for in-flight jobs during graceful shutdown
    pub drain_timeout: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(1),
            heartbeat_interval: Duration::from_secs(30),
            drain_timeout: Duration::from_secs(30),
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
    #[error("retryable: {message}")]
    Retryable {
        message: String,
        retry_after: Option<Duration>,
    },
    #[error("fatal: {0}")]
    Fatal(String),
}

impl JobError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, JobError::Retryable { .. })
    }

    fn message(&self) -> &str {
        match self {
            JobError::Retryable { message, .. } | JobError::Fatal(message) => message,
        }
    }

    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            JobError::Retryable { retry_after, .. } => *retry_after,
            JobError::Fatal(_) => None,
        }
    }

    pub fn retryable(message: impl Into<String>) -> Self {
        JobError::Retryable {
            message: message.into(),
            retry_after: None,
        }
    }

    pub fn retryable_after(message: impl Into<String>, retry_after: Duration) -> Self {
        JobError::Retryable {
            message: message.into(),
            retry_after: Some(retry_after),
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
    let in_flight = Arc::new(AtomicUsize::new(0));
    let hard_shutdown = CancellationToken::new();

    // Spawn drain timeout watcher - triggers hard shutdown after drain_timeout
    let drain_handle = {
        let shutdown = shutdown.clone();
        let hard_shutdown = hard_shutdown.clone();
        let drain_timeout = config.drain_timeout;
        tokio::spawn(async move {
            shutdown.cancelled().await;
            info!("graceful shutdown initiated, waiting for in-flight jobs");
            tokio::time::sleep(drain_timeout).await;
            warn!("drain timeout exceeded, initiating hard shutdown");
            hard_shutdown.cancel();
        })
    };

    loop {
        if shutdown.is_cancelled() {
            // Graceful shutdown: stop accepting new jobs, wait for in-flight to complete
            if in_flight.load(Ordering::SeqCst) == 0 {
                break;
            }
            tokio::select! {
                _ = hard_shutdown.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_millis(100)) => continue,
            }
        }

        match queue.claim_next().await {
            Ok(Some(job)) => {
                in_flight.fetch_add(1, Ordering::SeqCst);

                handle_job(
                    queue.clone(),
                    executor.clone(),
                    config,
                    hard_shutdown.clone(),
                    job,
                )
                .await;

                in_flight.fetch_sub(1, Ordering::SeqCst);
            }
            Ok(None) => sleep(config.poll_interval).await,
            Err(err) => {
                error!(error = %err, "failed to claim next job");
                sleep(config.poll_interval).await;
            }
        }
    }

    drain_handle.abort();
    info!("worker shutdown complete");
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
        // Use a unique database filename to avoid any potential conflicts
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(db_path.as_path()).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (JobQueue::new(db), dir)
    }

    fn fast_config() -> WorkerConfig {
        WorkerConfig {
            poll_interval: Duration::from_millis(5),
            heartbeat_interval: Duration::from_millis(10),
            drain_timeout: Duration::from_secs(5),
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
            Err(JobError::retryable(format!("retry {}", job.id)))
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

    struct RetryAfterExecutor;

    #[async_trait]
    impl JobExecutor for RetryAfterExecutor {
        async fn execute(&self, job: Job, _ctx: JobContext) -> Result<(), JobError> {
            Err(JobError::retryable_after(
                format!("retry-after {}", job.id),
                Duration::from_millis(400),
            ))
        }
    }

    #[tokio::test]
    async fn worker_respects_retry_after_hint() {
        let (queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("retry-after", json!({}), None, 0)
            .await
            .expect("enqueue");

        let shutdown = CancellationToken::new();
        let worker = tokio::spawn(run_worker(
            queue.clone(),
            RetryAfterExecutor,
            fast_config(),
            shutdown.clone(),
        ));

        timeout(Duration::from_secs(2), async {
            loop {
                let job = queue.fetch_job(&job_id).await.expect("fetch");
                if matches!(job.state, JobState::Queued) && job.last_error.is_some() {
                    let not_before = job.not_before.expect("retry should set not_before");
                    let delta_ms = (not_before - job.updated_at).num_milliseconds();
                    assert!(
                        (300..=900).contains(&delta_ms),
                        "expected retry-after to set ~400ms delay, got {delta_ms}ms",
                    );
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("job should be requeued with retry-after");

        shutdown.cancel();
        let _ = worker.await;
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

    struct SlowExecutor {
        delay: Duration,
    }

    #[async_trait]
    impl JobExecutor for SlowExecutor {
        async fn execute(&self, _job: Job, _ctx: JobContext) -> Result<(), JobError> {
            sleep(self.delay).await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn worker_completes_job_during_graceful_shutdown() {
        let (queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("slow", json!({}), None, 0)
            .await
            .expect("enqueue");

        let mut config = fast_config();
        config.drain_timeout = Duration::from_secs(5);

        let shutdown = CancellationToken::new();
        let worker = tokio::spawn(run_worker(
            queue.clone(),
            SlowExecutor {
                delay: Duration::from_millis(100),
            },
            config,
            shutdown.clone(),
        ));

        // Wait for job to start running
        timeout(Duration::from_secs(2), async {
            loop {
                let job = queue.fetch_job(&job_id).await.expect("fetch");
                if matches!(job.state, JobState::Running) {
                    break;
                }
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("job should start running");

        // Trigger graceful shutdown while job is running
        shutdown.cancel();

        // Worker should complete the job before exiting
        let _ = timeout(Duration::from_secs(2), worker)
            .await
            .expect("worker should exit within timeout");

        let job = queue.fetch_job(&job_id).await.expect("fetch final");
        assert!(
            matches!(job.state, JobState::Completed),
            "job should be completed during graceful shutdown, got {:?}",
            job.state
        );
    }

    #[tokio::test]
    async fn worker_exits_after_drain_timeout() {
        // This test verifies that hard shutdown abandons finalization.
        // Note: Job execution itself isn't interrupted - jobs should implement
        // their own cancellation logic if needed. This test uses a fast executor
        // but simulates slow finalization by testing the shutdown path.
        let (queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("test", json!({}), None, 0)
            .await
            .expect("enqueue");

        let mut config = fast_config();
        // Very short drain timeout
        config.drain_timeout = Duration::from_millis(50);

        let shutdown = CancellationToken::new();
        let worker = tokio::spawn(run_worker(
            queue.clone(),
            SlowExecutor {
                delay: Duration::from_millis(200), // Longer than drain timeout but reasonable
            },
            config,
            shutdown.clone(),
        ));

        // Wait for job to start running
        timeout(Duration::from_secs(2), async {
            loop {
                let job = queue.fetch_job(&job_id).await.expect("fetch");
                if matches!(job.state, JobState::Running) {
                    break;
                }
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("job should start running");

        // Trigger shutdown immediately
        shutdown.cancel();

        // Worker should eventually exit. The job execution (200ms) will complete,
        // but by then hard_shutdown has fired (after 50ms), so finalization
        // will be abandoned.
        let _ = timeout(Duration::from_millis(500), worker)
            .await
            .expect("worker should exit after job completes");

        // Job should still be in running state since finalization was abandoned
        // due to hard shutdown firing before the job finished.
        let job = queue.fetch_job(&job_id).await.expect("fetch final");
        assert!(
            matches!(job.state, JobState::Running),
            "job should still be running after hard shutdown abandoned finalization, got {:?}",
            job.state
        );
    }

    #[tokio::test]
    async fn worker_stops_claiming_new_jobs_during_shutdown() {
        let (queue, _dir) = setup_queue().await;

        // Enqueue two jobs
        let job1_id = queue
            .enqueue("job1", json!({}), None, 0)
            .await
            .expect("enqueue job1");
        let job2_id = queue
            .enqueue("job2", json!({}), None, 0)
            .await
            .expect("enqueue job2");

        let mut config = fast_config();
        config.drain_timeout = Duration::from_secs(5);

        let shutdown = CancellationToken::new();
        let worker = tokio::spawn(run_worker(
            queue.clone(),
            SlowExecutor {
                delay: Duration::from_millis(100),
            },
            config,
            shutdown.clone(),
        ));

        // Wait for first job to start running
        timeout(Duration::from_secs(2), async {
            loop {
                let job = queue.fetch_job(&job1_id).await.expect("fetch");
                if matches!(job.state, JobState::Running) {
                    break;
                }
                sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("job1 should start running");

        // Trigger shutdown while first job is running
        shutdown.cancel();

        // Wait for worker to exit
        let _ = timeout(Duration::from_secs(2), worker)
            .await
            .expect("worker should exit");

        // First job should be completed
        let job1 = queue.fetch_job(&job1_id).await.expect("fetch job1");
        assert!(
            matches!(job1.state, JobState::Completed),
            "job1 should be completed, got {:?}",
            job1.state
        );

        // Second job should still be queued (not claimed during shutdown)
        let job2 = queue.fetch_job(&job2_id).await.expect("fetch job2");
        assert!(
            matches!(job2.state, JobState::Queued),
            "job2 should remain queued during shutdown, got {:?}",
            job2.state
        );
    }
}

async fn handle_job<E: JobExecutor>(
    queue: JobQueue,
    executor: Arc<E>,
    config: WorkerConfig,
    hard_shutdown: CancellationToken,
    job: Job,
) {
    info!(job_id = %job.id, job_type = %job.job_type, "processing job");
    let heartbeat_cancel = hard_shutdown.child_token();
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
            retry_after: job_err.retry_after(),
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
                retry_after: None,
            }
        }
    };

    if let Err(err) = finalize_job(
        queue.clone(),
        &job,
        finalize,
        &heartbeat_cancel,
        heartbeat_interval,
        hard_shutdown,
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
    Fail {
        message: String,
        retry: bool,
        retry_after: Option<Duration>,
    },
}

async fn finalize_job(
    queue: JobQueue,
    job: &Job,
    action: FinalizeAction,
    heartbeat_cancel: &CancellationToken,
    heartbeat_interval: Duration,
    hard_shutdown: CancellationToken,
) -> Result<(), QueueError> {
    let mut attempt: u32 = 0;

    loop {
        if hard_shutdown.is_cancelled() {
            warn!(job_id = %job.id, "hard shutdown: abandoning finalization");
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
                    _ = hard_shutdown.cancelled() => {
                        warn!(job_id = %job.id, "hard shutdown: abandoning finalization");
                        return Ok(());
                    },
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
            FinalizeAction::Fail { message, retry, .. } => {
                let retry_after = match &action {
                    FinalizeAction::Fail { retry_after, .. } => *retry_after,
                    FinalizeAction::Complete => None,
                };
                queue
                    .fail(&job.id, message.clone(), *retry, retry_after)
                    .await
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
                    _ = hard_shutdown.cancelled() => {
                        warn!(job_id = %job.id, "hard shutdown: abandoning finalization");
                        return Ok(());
                    },
                    _ = sleep(backoff) => {},
                    _ = heartbeat_cancel.cancelled() => return Err(err),
                }
            }
        }
    }
}
