### **5.1 Concepts**

- **Jobs**:

    - ingest.gmail - Fetch and persist a Gmail message
    - classify - Evaluate rules and LLM to determine action
    - action.gmail - Execute Gmail actions (archive, apply_label, remove_label, mark_read, mark_unread, star, unstar, trash, restore, delete, snooze)
    - unsnooze.gmail - Restore snoozed messages to inbox at scheduled time
    - approval.notify - Request approval via Discord
    - undo.action - Reverse a previously completed action
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
- `JOB_TYPE_OUTBOUND_SEND` = "outbound.send"
- `JOB_TYPE_UNDO_ACTION` = "undo.action"

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
| `forward` | Enqueues `outbound.send` job | Forward message (irreversible) |
| `auto_reply` | Enqueues `outbound.send` job | Send reply (irreversible) |

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

### 5.9 Outbound Send Job

The outbound.send job sends emails for forward and auto_reply actions. It constructs RFC 2822 MIME messages and sends them via the Gmail API.

**Payload**:
```json
{
  "account_id": "uuid",
  "action_id": "uuid",
  "message_type": "forward" | "reply",
  "to": ["recipient@example.com"],
  "cc": ["cc@example.com"],
  "subject": "Re: Original Subject",
  "body_plain": "Plain text body",
  "body_html": "<p>HTML body</p>",
  "original_message_id": "uuid",
  "thread_id": "gmail_thread_id",
  "references": ["<message-id@domain.com>"],
  "attachments": [
    {
      "filename": "document.pdf",
      "mime_type": "application/pdf",
      "data": "base64-encoded-content"
    }
  ]
}
```

**Flow**:
1. Parse payload and validate action/account ownership
2. Load original message and thread from database
3. Build MIME message using `MimeMessage` builder:
   - Set From/To/CC/Subject headers
   - For replies: add `In-Reply-To` and `References` headers for threading
   - For forwards: omit threading headers (sends as new conversation)
   - Add plain text and/or HTML body parts
   - Decode and attach any attachments
4. Call Gmail `send_message` API with base64url-encoded RFC 2822 message
5. On success: mark action as `Completed` with irreversible undo hint containing `sent_message_id`
6. On failure: mark action as `Failed` with error message

**Message Type Differences**:

| Type | Threading | Headers | Gmail Thread |
|------|-----------|---------|--------------|
| `reply` | Maintains thread | Includes In-Reply-To, References | Uses original threadId |
| `forward` | New conversation | No threading headers | No threadId (new thread) |

**MIME Message Construction**:
The `MimeMessage` struct (in `server/crates/ashford-core/src/gmail/mime_builder.rs`) uses the `mail-builder` crate:

```rust
use ashford_core::gmail::{EmailAddress, MimeAttachment, MimeMessage};

let message = MimeMessage {
    from: EmailAddress::new(Some("Sender Name"), "sender@example.com"),
    to: vec![EmailAddress::new(None, "recipient@example.com")],
    cc: vec![],
    bcc: vec![],
    subject: Some("Re: Original Subject".to_string()),
    body_plain: Some("Plain text content".to_string()),
    body_html: Some("<p>HTML content</p>".to_string()),
    in_reply_to: Some("<original-message-id@gmail.com>".to_string()),
    references: vec!["<thread-references@gmail.com>".to_string()],
    attachments: vec![MimeAttachment {
        filename: "document.pdf".to_string(),
        content_type: "application/pdf".to_string(),
        data: file_bytes, // Vec<u8>
    }],
};

let raw_message = message.to_base64_url()?;
```

**Undo Hints**:
Forward and auto_reply actions are irreversible. The undo hint indicates this:
```json
{
  "action": "forward",
  "inverse_action": "none",
  "irreversible": true,
  "sent_message_id": "1234567890abcdef"
}
```

**Error Handling**:
- Invalid payload → Fatal error (missing required fields)
- Message/action not found → Fatal error
- Account mismatch → Fatal error
- Gmail API errors → Mapped via `map_gmail_error()` (see Error Handling section)

**Idempotency key**: `outbound.send:{account_id}:{action_id}`

### 5.10 Undo Action Job

The undo.action job reverses a previously completed Gmail action by executing its inverse operation.

**Payload**:
```json
{
  "account_id": "uuid",
  "original_action_id": "uuid"
}
```

**Flow**:
1. Parse payload and load original action from database
2. Validate action is undoable:
   - Status must be `Completed`
   - `undo_hint_json.inverse_action` must not be `"none"`
   - `undo_hint_json.irreversible` must not be `true`
   - No existing `action_link` with `undo_of` relation for this action
3. Create undo action record and action_link atomically (for idempotency across retries)
4. Execute inverse action via Gmail API based on `inverse_action` type
5. On success: mark undo action as `Completed` with non-reversible undo hint
6. On failure: mark undo action as `Failed` with error message

**Supported Inverse Actions**:

| Original Action | Inverse Action | Gmail API Call |
|-----------------|----------------|----------------|
| `archive` | `apply_label` | Add INBOX label |
| `apply_label` | `remove_label` | Remove the applied label |
| `remove_label` | `apply_label` | Add the removed label back |
| `mark_read` | `mark_unread` | Add UNREAD label |
| `mark_unread` | `mark_read` | Remove UNREAD label |
| `star` | `unstar` | Remove STARRED label |
| `unstar` | `star` | Add STARRED label |
| `trash` | `restore` | Call `untrash_message` |
| `restore` | `trash` | Call `trash_message` |
| `snooze` | (special) | Cancel unsnooze job + restore labels |

**Irreversible Actions** (undo rejected with fatal error):
- `delete` - Message permanently deleted
- `forward` - Email already sent
- `auto_reply` - Reply already sent

**Snooze Undo**:
Snooze undo requires special handling:
1. Extract `cancel_unsnooze_job_id` from `inverse_parameters`
2. Cancel the scheduled unsnooze job via `JobQueue::cancel()`
   - `Ok(())` → Job was pending, now canceled
   - `NotRunning` → Job already completed (unsnooze already ran)
   - `JobNotFound` → Job doesn't exist (already cleaned up)
3. Apply label changes from `inverse_parameters`:
   - Add labels from `add_labels` (typically INBOX)
   - Remove labels from `remove_labels` (the snooze label)

All three cancellation outcomes are treated as success—the undo proceeds with label changes regardless.

**Idempotency**:
The undo system is designed to be idempotent across retries and concurrent requests:
1. Before executing the inverse action, the handler creates an undo action record and action_link
2. The undo action stores the owning `job_id` to prevent other jobs from executing it
3. If another request tries to undo the same action, the unique constraint on action_links fails
4. On retry, the handler reuses the existing undo action record

**Action Link Semantics**:
The `action_link` created for undo uses:
- `cause_action_id`: The new undo action's ID
- `effect_action_id`: The original action's ID
- `relation_type`: `undo_of`

This means "the undo action is the undo of the original action."

**Retry Exhaustion**:
When a retryable error occurs on the final allowed attempt (job.attempts >= job.max_attempts), the handler marks the undo action as `Failed` before returning the error. This ensures undo actions reach a terminal state rather than remaining in `Executing` indefinitely.

**Error Handling**:
- Action not found → Fatal error
- Action not undoable → Fatal error (already undone, irreversible, or not completed)
- Message not found (404) → Action marked as failed (message deleted externally)
- Rate limiting (429) → Retryable with backoff
- Server error (5xx) → Retryable with backoff
- Account/ownership mismatch → Fatal error

**Idempotency key**: `undo.action:{account_id}:{original_action_id}`
