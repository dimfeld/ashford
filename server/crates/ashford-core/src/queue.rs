use std::time::Duration;

use chrono::{DateTime, SecondsFormat, Utc};
use libsql::{Connection, Row, params};
use rand::Rng;
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Database, DbError};

const JOB_COLUMNS: &str = "id, type, payload_json, priority, state, attempts, max_attempts, not_before, idempotency_key, last_error, heartbeat_at, created_at, updated_at, finished_at, result_json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
    Canceled,
}

impl JobState {
    fn as_str(&self) -> &'static str {
        match self {
            JobState::Queued => "queued",
            JobState::Running => "running",
            JobState::Completed => "completed",
            JobState::Failed => "failed",
            JobState::Canceled => "canceled",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(JobState::Queued),
            "running" => Some(JobState::Running),
            "completed" => Some(JobState::Completed),
            "failed" => Some(JobState::Failed),
            "canceled" => Some(JobState::Canceled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    pub id: String,
    pub job_type: String,
    pub payload: Value,
    pub priority: i64,
    pub state: JobState,
    pub attempts: i64,
    pub max_attempts: i64,
    pub not_before: Option<DateTime<Utc>>,
    pub idempotency_key: Option<String>,
    pub last_error: Option<String>,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub result: Option<Value>,
}

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("payload json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("job not found: {0}")]
    JobNotFound(String),
    #[error("job is not running: {0}")]
    NotRunning(String),
    #[error("invalid job state value {0}")]
    InvalidState(String),
    #[error("duplicate idempotency key {key}, existing job {existing_job_id:?}")]
    DuplicateIdempotency {
        key: String,
        existing_job_id: Option<String>,
    },
    #[error("step not found: {0}")]
    StepNotFound(String),
}

#[derive(Clone)]
pub struct JobQueue {
    db: Database,
}

impl JobQueue {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub async fn enqueue(
        &self,
        job_type: impl Into<String>,
        payload: Value,
        idempotency_key: Option<String>,
        priority: i64,
    ) -> Result<String, QueueError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let payload_json = serde_json::to_string(&payload)?;
        let idempotency = idempotency_key.clone();
        let mut conn = self.db.connection().await?;

        let result = conn
            .execute(
                "INSERT INTO jobs (id, type, payload_json, priority, state, attempts, max_attempts, not_before, idempotency_key, last_error, heartbeat_at, created_at, updated_at, finished_at, result_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, 5, NULL, ?6, NULL, NULL, ?7, ?7, NULL, NULL)",
                params![
                    id.clone(),
                    job_type.into(),
                    payload_json,
                    priority,
                    JobState::Queued.as_str(),
                    idempotency.clone(),
                    now.clone()
                ],
            )
            .await;

        match result {
            Ok(_) => Ok(id),
            Err(err) if is_unique_violation(&err) && idempotency.is_some() => {
                let existing =
                    lookup_job_by_idempotency(&mut conn, idempotency.as_deref().unwrap())
                        .await
                        .ok()
                        .flatten();
                let key = idempotency.unwrap();
                Err(QueueError::DuplicateIdempotency {
                    key,
                    existing_job_id: existing,
                })
            }
            Err(err) => Err(err.into()),
        }
    }

    pub async fn claim_next(&self) -> Result<Option<Job>, QueueError> {
        let now = now_rfc3339();
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!(
                    "UPDATE jobs
                     SET state = ?2, attempts = attempts + 1, heartbeat_at = ?3, updated_at = ?3
                     WHERE id = (
                         SELECT id FROM jobs
                         WHERE state = 'queued' AND (not_before IS NULL OR not_before <= ?1)
                         ORDER BY priority DESC, created_at
                         LIMIT 1
                     )
                     RETURNING {JOB_COLUMNS}"
                ),
                params![now.clone(), JobState::Running.as_str(), now.clone()],
            )
            .await?;

        let maybe_row = rows.next().await?;
        match maybe_row {
            Some(row) => Ok(Some(row_to_job(row)?)),
            None => Ok(None),
        }
    }

    pub async fn heartbeat(&self, job_id: &str) -> Result<(), QueueError> {
        let now = now_rfc3339();
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "UPDATE jobs SET heartbeat_at = ?2, updated_at = ?2 WHERE id = ?1 AND state = 'running' RETURNING id",
                params![job_id, now],
            )
            .await?;

        if rows.next().await?.is_none() {
            return Err(QueueError::NotRunning(job_id.to_string()));
        }
        Ok(())
    }

    pub async fn complete(&self, job_id: &str, result: Option<Value>) -> Result<(), QueueError> {
        let now = now_rfc3339();
        let serialized = result.map(|val| serde_json::to_string(&val)).transpose()?;
        let finished_at = now.clone();
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "UPDATE jobs
                 SET state = 'completed',
                     last_error = NULL,
                     finished_at = ?3,
                     result_json = ?4,
                     updated_at = ?3
                 WHERE id = ?1 AND state = 'running'
                 RETURNING id",
                params![job_id, finished_at.clone(), finished_at, serialized],
            )
            .await?;
        if rows.next().await?.is_none() {
            return self.resolve_missing_state(job_id).await;
        }
        Ok(())
    }

    pub async fn fail(
        &self,
        job_id: &str,
        error: String,
        should_retry: bool,
        retry_after: Option<Duration>,
    ) -> Result<(), QueueError> {
        let job = self.fetch_job(job_id).await?;
        let now = now_rfc3339();
        let allow_retry = should_retry && job.attempts < job.max_attempts;
        let next_not_before = if allow_retry {
            let delay = retry_after.unwrap_or_else(|| backoff_with_jitter(job.attempts));
            let scheduled = Utc::now() + chrono::Duration::from_std(delay).unwrap();
            Some(scheduled.to_rfc3339_opts(SecondsFormat::Millis, true))
        } else {
            None
        };

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "UPDATE jobs
                 SET state = ?2, not_before = ?3, last_error = ?4, finished_at = ?5, updated_at = ?6
                 WHERE id = ?1 AND state = 'running'
                 RETURNING id",
                params![
                    job_id,
                    if allow_retry {
                        JobState::Queued.as_str()
                    } else {
                        JobState::Failed.as_str()
                    },
                    next_not_before,
                    error,
                    if allow_retry {
                        None::<String>
                    } else {
                        Some(now.clone())
                    },
                    now
                ],
            )
            .await?;

        if rows.next().await?.is_none() {
            return self.resolve_missing_state(job_id).await;
        }
        Ok(())
    }

    pub async fn cancel(&self, job_id: &str) -> Result<(), QueueError> {
        let now = now_rfc3339();
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "UPDATE jobs
                 SET state = 'canceled', finished_at = ?2, updated_at = ?2
                 WHERE id = ?1 AND state IN ('queued','running')
                 RETURNING id",
                params![job_id, now],
            )
            .await?;
        if rows.next().await?.is_none() {
            return self.resolve_missing_state(job_id).await;
        }
        Ok(())
    }

    pub async fn start_step(
        &self,
        job_id: &str,
        name: impl Into<String>,
    ) -> Result<String, QueueError> {
        let id = Uuid::new_v4().to_string();
        let now = now_rfc3339();
        let conn = self.db.connection().await?;
        conn.execute(
            "INSERT INTO job_steps (id, job_id, name, started_at, finished_at, result_json)
                 VALUES (?1, ?2, ?3, ?4, NULL, NULL)",
            params![id.clone(), job_id, name.into(), now],
        )
        .await?;
        Ok(id)
    }

    pub async fn finish_step(
        &self,
        step_id: &str,
        result_json: Option<Value>,
    ) -> Result<(), QueueError> {
        let now = now_rfc3339();
        let serialized = result_json
            .map(|val| serde_json::to_string(&val))
            .transpose()?;

        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                "UPDATE job_steps
                 SET finished_at = ?2, result_json = ?3
                 WHERE id = ?1
                 RETURNING id",
                params![step_id, now, serialized],
            )
            .await?;

        if rows.next().await?.is_none() {
            return Err(QueueError::StepNotFound(step_id.to_string()));
        }
        Ok(())
    }

    pub async fn fetch_job(&self, job_id: &str) -> Result<Job, QueueError> {
        let conn = self.db.connection().await?;
        let mut rows = conn
            .query(
                &format!("SELECT {JOB_COLUMNS} FROM jobs WHERE id = ?1"),
                params![job_id],
            )
            .await?;

        match rows.next().await? {
            Some(row) => row_to_job(row),
            None => Err(QueueError::JobNotFound(job_id.to_string())),
        }
    }
}

#[derive(Clone)]
pub struct JobContext {
    queue: JobQueue,
    job: Job,
}

impl JobContext {
    pub fn new(queue: JobQueue, job: Job) -> Self {
        Self { queue, job }
    }

    pub fn job(&self) -> &Job {
        &self.job
    }

    pub async fn start_step(&self, name: impl Into<String>) -> Result<String, QueueError> {
        self.queue.start_step(&self.job.id, name).await
    }

    pub async fn finish_step(
        &self,
        step_id: &str,
        result_json: Option<Value>,
    ) -> Result<(), QueueError> {
        self.queue.finish_step(step_id, result_json).await
    }

    pub async fn heartbeat(&self) -> Result<(), QueueError> {
        self.queue.heartbeat(&self.job.id).await
    }
}

fn row_to_job(row: Row) -> Result<Job, QueueError> {
    let id: String = row.get(0)?;
    let job_type: String = row.get(1)?;
    let payload_json: String = row.get(2)?;
    let priority: i64 = row.get(3)?;
    let state_str: String = row.get(4)?;
    let attempts: i64 = row.get(5)?;
    let max_attempts: i64 = row.get(6)?;
    let not_before: Option<String> = row.get(7)?;
    let idempotency_key: Option<String> = row.get(8)?;
    let last_error: Option<String> = row.get(9)?;
    let heartbeat_at: Option<String> = row.get(10)?;
    let created_at: String = row.get(11)?;
    let updated_at: String = row.get(12)?;
    let finished_at: Option<String> = row.get(13)?;
    let result_json: Option<String> = row.get(14)?;

    let state =
        JobState::from_str(&state_str).ok_or_else(|| QueueError::InvalidState(state_str))?;

    let payload: Value = serde_json::from_str(&payload_json)?;

    Ok(Job {
        id,
        job_type,
        payload,
        priority,
        state,
        attempts,
        max_attempts,
        not_before: parse_timestamp(not_before)?,
        idempotency_key,
        last_error,
        heartbeat_at: parse_timestamp(heartbeat_at)?,
        created_at: parse_timestamp(Some(created_at))?.expect("created_at required"),
        updated_at: parse_timestamp(Some(updated_at))?.expect("updated_at required"),
        finished_at: parse_timestamp(finished_at)?,
        result: result_json
            .map(|val| serde_json::from_str(&val))
            .transpose()?,
    })
}

impl JobQueue {
    async fn resolve_missing_state(&self, job_id: &str) -> Result<(), QueueError> {
        match self.fetch_job(job_id).await {
            Ok(_) => Err(QueueError::NotRunning(job_id.to_string())),
            Err(QueueError::JobNotFound(_)) => Err(QueueError::JobNotFound(job_id.to_string())),
            Err(other) => Err(other),
        }
    }
}

fn parse_timestamp(ts: Option<String>) -> Result<Option<DateTime<Utc>>, QueueError> {
    match ts {
        Some(value) => {
            let dt = DateTime::parse_from_rfc3339(&value)
                .map_err(|_| QueueError::InvalidState(value.clone()))?
                .with_timezone(&Utc);
            Ok(Some(dt))
        }
        None => Ok(None),
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn backoff_with_jitter(attempts: i64) -> Duration {
    let attempts = attempts.max(1);
    let exp = attempts.min(20); // clamp to avoid overflow; cap at ~1M seconds before jitter then min below.
    let base = 2_i64.saturating_pow(exp as u32);
    let delay_secs = base.min(300);
    let mut rng = rand::thread_rng();
    let factor: f64 = rng.gen_range(0.75..=1.25);
    Duration::from_secs_f64((delay_secs as f64) * factor)
}

fn is_unique_violation(err: &libsql::Error) -> bool {
    err.to_string()
        .to_ascii_lowercase()
        .contains("unique constraint failed")
}

async fn lookup_job_by_idempotency(
    conn: &mut Connection,
    key: &str,
) -> Result<Option<String>, libsql::Error> {
    let mut rows = conn
        .query(
            "SELECT id FROM jobs WHERE idempotency_key = ?1",
            params![key],
        )
        .await?;
    let Some(row) = rows.next().await? else {
        return Ok(None);
    };
    let id: String = row.get(0)?;
    Ok(Some(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations::run_migrations;
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::task;

    async fn setup_queue() -> (JobQueue, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        // Use a unique database filename to avoid any potential conflicts
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(db_path.as_path()).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (JobQueue::new(db), dir)
    }

    #[tokio::test]
    async fn enqueue_and_claim_returns_running_job() {
        let (queue, _dir) = setup_queue().await;
        let id = queue
            .enqueue("ingest.gmail", json!({"msg":1}), Some("k1".into()), 1)
            .await
            .expect("enqueue");

        let claimed = queue.claim_next().await.expect("claim").expect("job");
        assert_eq!(claimed.id, id);
        assert_eq!(claimed.state, JobState::Running);
        assert_eq!(claimed.attempts, 1);
        assert_eq!(claimed.priority, 1);
        assert_eq!(claimed.payload["msg"], 1);
    }

    #[tokio::test]
    async fn idempotency_conflict_is_reported() {
        let (queue, _dir) = setup_queue().await;
        queue
            .enqueue("ingest.gmail", json!({"msg":1}), Some("dup".into()), 1)
            .await
            .expect("enqueue");
        let err = queue
            .enqueue("ingest.gmail", json!({"msg":2}), Some("dup".into()), 1)
            .await
            .expect_err("should fail");
        match err {
            QueueError::DuplicateIdempotency { key, .. } => assert_eq!(key, "dup"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn fail_requeues_with_backoff_until_max_attempts() {
        let (queue, _dir) = setup_queue().await;
        let id = queue
            .enqueue("classify", json!({}), None, 0)
            .await
            .expect("enqueue");
        let job = queue.claim_next().await.expect("claim").expect("job");
        assert_eq!(job.attempts, 1);

        queue
            .fail(&id, "temporary".into(), true, None)
            .await
            .expect("fail");

        let updated = queue.fetch_job(&id).await.expect("fetch");
        assert_eq!(updated.state, JobState::Queued);
        assert!(updated.not_before.is_some());
    }

    #[tokio::test]
    async fn fail_uses_explicit_retry_after_when_provided() {
        let (queue, _dir) = setup_queue().await;
        let id = queue
            .enqueue("classify", json!({}), None, 0)
            .await
            .expect("enqueue");
        let _job = queue.claim_next().await.expect("claim").expect("job");

        queue
            .fail(
                &id,
                "rate limited".into(),
                true,
                Some(Duration::from_millis(2000)),
            )
            .await
            .expect("fail");

        let updated = queue.fetch_job(&id).await.expect("fetch");
        let not_before = updated.not_before.expect("retry should set not_before");
        let millis = (not_before - Utc::now()).num_milliseconds();
        assert!(
            (1500..=2200).contains(&millis),
            "expected not_before about 2s in future, got {millis}ms"
        );
    }

    #[tokio::test]
    async fn max_attempts_moves_to_failed_state() {
        let (queue, _dir) = setup_queue().await;
        let id = queue
            .enqueue("classify", json!({}), None, 0)
            .await
            .expect("enqueue");

        // Force max_attempts to 1 so the first failure exhausts retries.
        let conn = queue.db.connection().await.expect("conn");
        conn.execute(
            "UPDATE jobs SET max_attempts = 1 WHERE id = ?1",
            params![id.as_str()],
        )
        .await
        .expect("update max attempts");

        let _ = queue.claim_next().await.expect("claim").expect("job");
        queue
            .fail(&id, "boom".into(), true, None)
            .await
            .expect("fail");
        let updated = queue.fetch_job(&id).await.expect("fetch");
        assert_eq!(updated.state, JobState::Failed);
        assert!(updated.not_before.is_none());
    }

    #[tokio::test]
    async fn job_steps_record_start_and_finish() {
        let (queue, _dir) = setup_queue().await;
        let id = queue
            .enqueue("classify", json!({}), None, 0)
            .await
            .expect("enqueue");
        let job = queue.claim_next().await.expect("claim").expect("job");
        assert_eq!(job.id, id);

        let step_id = queue
            .start_step(&job.id, "download")
            .await
            .expect("start step");
        queue
            .finish_step(&step_id, Some(json!({"ok":true})))
            .await
            .expect("finish step");

        let conn = queue.db.connection().await.expect("conn");
        let mut rows = conn
            .query(
                "SELECT finished_at, result_json FROM job_steps WHERE id = ?1",
                params![step_id.as_str()],
            )
            .await
            .expect("query");
        let row = rows.next().await.expect("row option").expect("row");
        let finished_at: Option<String> = row.get(0).expect("finished_at");
        assert!(finished_at.is_some(), "finished_at should be set");
        let stored: Option<String> = row.get(1).expect("result json");
        assert_eq!(stored, Some(r#"{"ok":true}"#.to_string()));
    }

    #[tokio::test]
    async fn concurrent_claim_allows_single_runner() {
        let (queue, _dir) = setup_queue().await;
        queue
            .enqueue("classify", json!({}), None, 0)
            .await
            .expect("enqueue");

        let queue_a = queue.clone();
        let queue_b = queue.clone();

        let t1 = task::spawn(async move { queue_a.claim_next().await.unwrap() });
        let t2 = task::spawn(async move { queue_b.claim_next().await.unwrap() });

        let r1 = t1.await.expect("task 1");
        let r2 = t2.await.expect("task 2");

        let taken = r1.is_some() as i32 + r2.is_some() as i32;
        assert_eq!(taken, 1, "only one claim should succeed");
    }

    #[tokio::test]
    async fn claim_prefers_higher_priority_then_fifo() {
        let (queue, _dir) = setup_queue().await;
        let first = queue
            .enqueue("classify", json!({"order":1}), None, 1)
            .await
            .expect("enqueue first");
        tokio::time::sleep(Duration::from_millis(5)).await;
        let second = queue
            .enqueue("classify", json!({"order":2}), None, 5)
            .await
            .expect("enqueue second");

        let job1 = queue.claim_next().await.expect("claim").expect("job");
        assert_eq!(job1.id, second, "higher priority should win even if newer");

        let job2 = queue.claim_next().await.expect("claim").expect("job");
        assert_eq!(job2.id, first, "next claim should take remaining job");
    }

    #[tokio::test]
    async fn claim_skips_jobs_with_future_not_before() {
        let (queue, _dir) = setup_queue().await;
        let future = queue
            .enqueue("future", json!({}), None, 0)
            .await
            .expect("enqueue future");
        let ready = queue
            .enqueue("ready", json!({}), None, 0)
            .await
            .expect("enqueue ready");

        // push future job out of eligibility window
        let conn = queue.db.connection().await.expect("conn");
        let future_time = (Utc::now() + chrono::Duration::seconds(2))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        conn.execute(
            "UPDATE jobs SET not_before = ?2 WHERE id = ?1",
            params![future.as_str(), future_time],
        )
        .await
        .expect("set not_before");

        let claimed = queue.claim_next().await.expect("claim").expect("job");
        assert_eq!(claimed.id, ready);

        // future job becomes eligible after we move the clock back
        let conn = queue.db.connection().await.expect("conn2");
        let past = (Utc::now() - chrono::Duration::seconds(1))
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        conn.execute(
            "UPDATE jobs SET state = 'queued', not_before = ?2 WHERE id = ?1",
            params![future.as_str(), past],
        )
        .await
        .expect("clear not_before");

        let next = queue.claim_next().await.expect("claim").expect("job");
        assert_eq!(next.id, future);
    }

    #[tokio::test]
    async fn cancel_moves_job_to_canceled_state() {
        let (queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("cancel", json!({}), None, 0)
            .await
            .expect("enqueue");

        queue.cancel(&job_id).await.expect("cancel");
        let job = queue.fetch_job(&job_id).await.expect("fetch");
        assert!(matches!(job.state, JobState::Canceled));
    }

    #[tokio::test]
    async fn heartbeat_errors_when_job_not_running() {
        let (queue, _dir) = setup_queue().await;
        let job_id = queue
            .enqueue("pending", json!({}), None, 0)
            .await
            .expect("enqueue");

        let err = queue.heartbeat(&job_id).await.expect_err("should fail");
        assert!(matches!(err, QueueError::NotRunning(_)));
    }
}
