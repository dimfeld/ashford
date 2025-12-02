### **5.1 Concepts**

- **Jobs**:

    - ingest.gmail - Fetch and persist a Gmail message
    - classify - Evaluate rules and LLM to determine action
    - action.gmail - Execute Gmail actions (archive, label, etc.)
    - approval.notify - Request approval via Discord
    - undo - Reverse a previous action
    - outbound.send - Send auto_reply/forward emails
    - backfill.gmail - Bulk sync historical messages
    - history.sync.gmail - Incremental sync via Gmail History API

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

### **5.3 JobDispatcher**

The `JobDispatcher` struct routes jobs to their handlers and provides shared dependencies:

```rust
pub struct JobDispatcher {
    pub db: Database,
    pub http: reqwest::Client,
    pub gmail_api_base: Option<String>,
    pub llm_client: Arc<dyn LLMClient>,
    pub policy_config: PolicyConfig,
}
```

- **db**: Database connection for persistence
- **http**: Shared HTTP client for external API calls
- **gmail_api_base**: Optional override for Gmail API (testing)
- **llm_client**: LLM provider for classify jobs (use `MockLLMClient` in tests)
- **policy_config**: Safety policy configuration (approval thresholds, dangerous action list)

Job type constants are defined in `server/crates/ashford-core/src/jobs/mod.rs`:
- `JOB_TYPE_INGEST_GMAIL` = "ingest.gmail"
- `JOB_TYPE_CLASSIFY` = "classify"
- `JOB_TYPE_HISTORY_SYNC_GMAIL` = "history.sync.gmail"
- `JOB_TYPE_BACKFILL_GMAIL` = "backfill.gmail"

### **5.4 Error Handling**

Job handlers return `Result<(), JobError>` where `JobError` has two variants:

- **Fatal**: Job failed permanently, will not be retried
- **Retryable**: Temporary failure, will be retried with backoff

Error mapping utilities convert domain errors to appropriate `JobError` variants:
- `map_gmail_error(context, err)` - Maps `GmailClientError` to `JobError`
- `map_llm_error(context, err)` - Maps `LLMError` to `JobError`
- `map_account_error(context, err)` - Maps `AccountError` to `JobError`
- `map_executor_error(context, err)` - Maps `ExecutorError` (rules engine) to `JobError`:
  - `RuleLoader` errors → Retryable (database issues)
  - `Condition` errors → Fatal (invalid regex, missing field)

### **5.5 Classify Job**

The classify job orchestrates the full decision pipeline:

**Payload**:
```json
{
  "account_id": "uuid",
  "message_id": "uuid"
}
```

**Flow**:
1. Parse payload and load message/account from database (via `MessageRepository::get_by_id`)
2. Evaluate deterministic rules via `RuleExecutor` (fast path)
3. If match: convert `RuleMatch` to `DecisionOutput` with confidence=1.0
4. If no match, invoke LLM decision engine (slow path):
   - Load enabled directions
   - Load LLM rules from all applicable scopes (global, account, domain, sender)
   - Build prompt via `PromptBuilder`
   - Call LLM and parse tool call response
5. Apply safety enforcement via `SafetyEnforcer` (applies to both paths)
6. Persist `Decision` record (source: `deterministic` or `llm`)
7. Create `Action` record with status `Queued` or `ApprovedPending`

**Idempotency key**: `classify:{account_id}:{message_id}`

**LLM Rule Scoping**: When loading LLM rules, the handler queries all applicable scopes:
- Global rules (no scope_ref)
- Account rules (scope_ref = account_id)
- Domain rules (scope_ref = sender domain, extracted from email)
- Sender rules (scope_ref = sender email, case-insensitive)

Rules are merged and deduplicated by ID.

The classify job is enqueued by `ingest.gmail` after a message is persisted.

