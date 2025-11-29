### **5.1 Concepts**

- **Jobs**:

    - ingest.gmail

    - classify

    - action.gmail

    - approval.notify

    - undo

    - outbound.send (for auto_reply/forward)

    - backfill.gmail

    - history.sync.gmail
- **States**:

    - queued → running → completed | failed | canceled
- **Retries**:

    - Per-type default max attempts (e.g., 5 for classify/action).

    - Exponential backoff with jitter.
- **Idempotency**:

    - idempotency_key (e.g., gmail:acct:msg:classify or gmail:acct:msg:action:archive).

    - Unique index to dedupe jobs.

  

### **5.2 libsql Schema (Queue Tables)**

  

    CREATE TABLE jobs (
      id TEXT PRIMARY KEY,
      type TEXT NOT NULL,
      payload_json TEXT NOT NULL,
      priority INTEGER NOT NULL DEFAULT 0,
      state TEXT NOT NULL CHECK(state IN ('queued','running','completed','failed','canceled')),
      attempts INTEGER NOT NULL DEFAULT 0,
      max_attempts INTEGER NOT NULL DEFAULT 5,
      not_before TEXT,
      idempotency_key TEXT,
      last_error TEXT,
      heartbeat_at TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
    
    CREATE INDEX jobs_state_idx ON jobs(state, priority, not_before);
    CREATE UNIQUE INDEX jobs_idem_idx ON jobs(idempotency_key);
    
    CREATE TABLE job_steps (
      id TEXT PRIMARY KEY,
      job_id TEXT NOT NULL,
      name TEXT NOT NULL,
      started_at TEXT NOT NULL,
      finished_at TEXT,
      result_json TEXT
    );

Workers in Rust:

- Poll jobs where state='queued' and not_before <= now.
- Claim job with transactional update to state='running'.
- Update heartbeat_at periodically.
- On completion/failure, update state, attempts, last_error, and possibly requeue.

