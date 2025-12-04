### **5.1 Concepts**

- **Jobs**:

    - ingest.gmail - Fetch and persist a Gmail message
    - classify - Evaluate rules and LLM to determine action
    - action.gmail - Execute Gmail actions (archive, apply_label, remove_label, mark_read, mark_unread, star, unstar, trash, restore, delete, snooze)
    - unsnooze.gmail - Restore snoozed messages to inbox at scheduled time
    - approval.notify - Request approval via Discord
    - undo - Reverse a previous action
    - outbound.send - Send auto_reply/forward emails
    - backfill.gmail - Bulk sync historical messages
    - history.sync.gmail - Incremental sync via Gmail History API
    - labels.sync.gmail - Sync labels from Gmail API and handle deleted labels

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
- `JOB_TYPE_ACTION_GMAIL` = "action.gmail"
- `JOB_TYPE_UNSNOOZE_GMAIL` = "unsnooze.gmail"
- `JOB_TYPE_HISTORY_SYNC_GMAIL` = "history.sync.gmail"
- `JOB_TYPE_BACKFILL_GMAIL` = "backfill.gmail"
- `JOB_TYPE_LABELS_SYNC_GMAIL` = "labels.sync.gmail"

### 5.3.1 Scheduled Jobs

Jobs can be scheduled for future execution using the `not_before` field. The `JobQueue` provides an `enqueue_scheduled()` method for this:

```rust
pub async fn enqueue_scheduled(
    &self,
    job_type: impl Into<String>,
    payload: Value,
    idempotency_key: Option<String>,
    priority: i64,
    not_before: DateTime<Utc>,
) -> Result<String, QueueError>
```

Scheduled jobs remain in `queued` state but are not claimed by workers until `not_before <= now()`. This is used by the snooze action to schedule unsnooze jobs at the target wake time.

### **5.4 Error Handling**

Job handlers return `Result<(), JobError>` where `JobError` has two variants:

- **Fatal**: Job failed permanently, will not be retried
- **Retryable**: Temporary failure, will be retried with backoff

Error mapping utilities convert domain errors to appropriate `JobError` variants:
- `map_gmail_error(context, err)` - Maps `GmailClientError` to `JobError`:
  - `Http` with 404 → Fatal (message not found)
  - `Http` with 429/403 → Retryable (rate limiting)
  - `Http` with 5xx → Retryable (server error)
  - `Unauthorized` → Retryable (triggers token refresh)
  - `OAuth` → Retryable (token refresh failure)
  - `TokenStore`, `Decode` → Fatal (configuration error)
  - `InvalidParameter` → Fatal (missing/empty action parameter)
  - `UnsupportedAction` → Fatal (action type not yet implemented)
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

### **5.6 Labels Sync Job**

The labels sync job synchronizes Gmail labels to the local database and handles deleted labels.

**Payload**:
```json
{
  "account_id": "uuid"
}
```

**Flow**:
1. Parse payload and load account from database
2. Refresh OAuth tokens if needed
3. Call Gmail `users.labels.list` API
4. Detect deleted labels by comparing local DB with API response
5. For each deleted label:
   - Find rules referencing the deleted label (in conditions or action parameters)
   - Disable each rule with reason: "Label 'X' was deleted from Gmail"
   - Delete the label from local database
6. Upsert all labels from API response (preserves user-editable fields like description and available_to_classifier)

**Idempotency key**: `labels.sync.gmail:{account_id}`

**Triggering**:
- On account setup (initial sync)
- Periodically on a schedule (e.g., hourly)
- Can be triggered manually/on-demand

The labels sync job ensures rules remain valid by soft-disabling rules that reference deleted labels rather than deleting them, allowing users to review and fix affected rules.

### **5.7 Action Gmail Job**

The action.gmail job executes Gmail mutations based on action records created by the classify job.

**Payload**:
```json
{
  "account_id": "uuid",
  "action_id": "uuid"
}
```

**Flow**:
1. Parse payload and load action from database
2. Validate action belongs to specified account
3. Mark action as `Executing`
4. Capture pre-image (current message labels/state) for undo hints
5. Execute Gmail API mutation based on `action_type`
6. On success: populate `undo_hint_json` and mark `Completed`
7. On failure: mark `Failed` with error message

**Supported Actions**:

| Action Type | Gmail API Call | Description |
|-------------|----------------|-------------|
| `archive` | `modify_message` (remove INBOX) | Remove from inbox |
| `apply_label` | `modify_message` (add label) | Add a label |
| `remove_label` | `modify_message` (remove label) | Remove a label |
| `mark_read` | `modify_message` (remove UNREAD) | Mark as read |
| `mark_unread` | `modify_message` (add UNREAD) | Mark as unread |
| `star` | `modify_message` (add STARRED) | Star message |
| `unstar` | `modify_message` (remove STARRED) | Unstar message |
| `trash` | `trash_message` | Move to trash |
| `restore` | `untrash_message` | Restore from trash |
| `delete` | `delete_message` | Permanently delete (irreversible) |
| `snooze` | `modify_message` + schedule job | Archive and schedule unsnooze |

**Parameter Validation**:
- `apply_label` and `remove_label` require a non-empty `label` field in `parameters_json`
- Missing or empty label parameters result in `InvalidParameter` error (fatal, no retry)

**Undo Hints**:
Each action captures pre-mutation state and stores the inverse operation in `undo_hint_json`. For example, archiving captures the current labels so the message can be restored to INBOX. The `delete` action is irreversible and stores a marker indicating it cannot be undone.

**Snooze Action**:
The snooze action has special behavior:
1. Validates parameters: accepts `{until: ISO8601}` or `{amount: number, units: "minutes"|"hours"|"days"}`
2. Ensures the snooze label exists (creates via Gmail API if needed)
3. Removes INBOX label and adds the snooze label
4. Schedules an `unsnooze.gmail` job with `not_before` set to the snooze target time
5. Stores enriched undo_hint with `snooze_until`, `snooze_label`, `snooze_label_id`, and `unsnooze_job_id`

The snooze label is configurable via `gmail.snooze_label` (default: "Ashford/Snoozed"). The unsnooze job payload includes the `snooze_label_id` so unsnooze removes the correct label even if the config is changed later.

**Error Handling**:
- 404 Not Found → Fatal error (message deleted externally)
- 429 Too Many Requests → Retryable with backoff
- 401 Unauthorized → Retryable (triggers token refresh)
- 5xx Server Error → Retryable with backoff
- Invalid parameters → Fatal error (misconfigured action)

**Idempotency key**: `action.gmail:{account_id}:{action_id}`

### 5.8 Unsnooze Gmail Job

The unsnooze.gmail job restores snoozed messages to the inbox at the scheduled time.

**Payload**:
```json
{
  "account_id": "uuid",
  "message_id": "uuid",
  "action_id": "uuid",
  "snooze_label_id": "Label_123456789"
}
```

**Flow**:
1. Parse payload and refresh OAuth tokens
2. Add INBOX label to the message
3. Remove the snooze label (uses `snooze_label_id` from payload, falls back to current config if missing)
4. Handle edge cases gracefully

**Edge Case Handling**:
- **Message deleted**: If Gmail returns 404, the job completes successfully (nothing to restore)
- **Label already removed**: The job proceeds with adding INBOX; label removal is a no-op
- **Rate limiting**: Retried with exponential backoff

**Idempotency key**: `unsnooze.gmail:{account_id}:{action_id}`

The unsnooze job is scheduled by the snooze action with `not_before` set to the snooze target time. It runs as a consequence of snoozing, not as a user-initiated undo operation.

