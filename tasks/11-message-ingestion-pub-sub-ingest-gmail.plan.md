---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: Message Ingestion (Pub/Sub + ingest.gmail)
goal: Implement Pub/Sub webhook and ingest.gmail job handler for real-time
  message ingestion
id: 11
uuid: 5b35e65e-3d87-45e5-98bc-45312701e05b
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies:
  - 10
parent: 3
references:
  "3": b93a0b33-fccb-4f57-8c97-002039917c44
  "10": a0d2a8da-0146-4e99-9f3b-8c526bad5524
issue: []
pullRequest: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
  - docs/job_queue.md
planGeneratedAt: 2025-11-29T19:22:23.357Z
promptsGeneratedAt: 2025-11-29T19:22:23.357Z
createdAt: 2025-11-29T07:42:47.263Z
updatedAt: 2025-11-30T00:12:20.517Z
progressNotes:
  - timestamp: 2025-11-29T19:29:32.339Z
    text: Added backward-compatibility and persistence tests for
      PubsubConfig.service_account_json and verified accounts test suite passes
      (cargo test -p ashford-core accounts).
    source: "tester: tasks 1-2"
  - timestamp: 2025-11-29T19:41:25.313Z
    text: Implemented Pub/Sub client wrapper, listener/supervisor scaffolding, and
      job dispatcher; added google cloud auth/gax/googleapis deps; cargo check
      passes.
    source: "implementer: tasks 3-6"
  - timestamp: 2025-11-29T19:45:41.930Z
    text: Added unit tests for Pub/Sub parsing/listener enqueue idempotency and
      JobDispatcher fatal-paths; cargo test -p ashford-core now passes (66
      tests).
    source: "tester: Task 11"
  - timestamp: 2025-11-29T20:14:54.933Z
    text: Implemented thread/message repositories with unique indexes, email parser,
      exports; added unit tests; cargo test -p ashford-core passing.
    source: "implementer: tasks7-9-14-16"
  - timestamp: 2025-11-29T20:16:04.148Z
    text: Ran cargo test -p ashford-core; all 79 core tests and 10 gmail-oauth tests
      pass. Reviewed new parser/threads/messages tests; coverage appears
      sufficient for current tasks, no additional gaps found.
    source: "tester: tasks 7-9,14-16"
  - timestamp: 2025-11-29T20:27:55.201Z
    text: Implemented ingest.gmail and history.sync.gmail handlers with Gmail
      fetch/parsing/upsert logic, error mapping, idempotent enqueues, plus
      dispatcher/main wiring; added unit tests with wiremock; cargo test -p
      ashford-core passing.
    source: "implementer: tasks 10-13"
  - timestamp: 2025-11-29T20:31:43.634Z
    text: Added error-handling and idempotency coverage for ingest.gmail and
      history.sync.gmail; cargo test -p ashford-core now includes new 404/429
      and duplicate scenarios (87 tests passing).
    source: "tester: Tasks10-13"
  - timestamp: 2025-11-29T20:38:06.804Z
    text: Added end-to-end worker tests covering history->ingest flow and duplicate
      message idempotency using wiremock.
    source: "implementer: Task 17"
  - timestamp: 2025-11-29T20:39:05.674Z
    text: Ran cargo test -p ashford-core --tests from server/; all 87 unit + 10
      oauth + 2 ingest_flow integration tests pass. Reviewed ingest_flow
      integration coverage; dedup + history->ingest paths verified; no
      additional gaps found.
    source: "tester: Task17"
  - timestamp: 2025-11-30T00:02:51.763Z
    text: "Fixed 4 review issues: (1) Added reqwest dependency to
      ashford-server/Cargo.toml using same version as ashford-core; (2) Fixed
      flaky messages tests by using unique db filenames with uuid in setup_repo;
      (3) Added MAX_MIME_DEPTH=50 constant and depth parameter to extract_bodies
      to prevent stack overflow; (4) Updated split_addresses to handle escaped
      quotes with backslash tracking and strip_quotes to unescape them. All 89
      tests pass consistently."
    source: "implementer: review fixes"
  - timestamp: 2025-11-30T00:10:48.042Z
    text: 'Applied UUID-based database naming pattern to all test modules to fix
      flaky tests: threads.rs, jobs/mod.rs, jobs/ingest_gmail.rs,
      jobs/history_sync_gmail.rs, pubsub_listener.rs, queue.rs, worker.rs,
      accounts.rs, and tests/ingest_flow.rs. Each test now uses
      format!("db_{}.sqlite", uuid::Uuid::new_v4()) for full isolation. Tests
      pass consistently with --test-threads=8.'
    source: "implementer: UUID database naming fix"
tasks:
  - title: Extend PubsubConfig with service account
    done: true
    description: "Update PubsubConfig in accounts.rs to add service_account_json:
      Option<String> field for storing the GCP service account JSON key directly
      in the database. This allows each account to have its own Pub/Sub
      subscription with separate credentials."
  - title: Add google-cloud-pubsub dependency
    done: true
    description: Add google-cloud-pubsub crate (or google-cloud-googleapis with
      pubsub feature) to ashford-core/Cargo.toml for Pub/Sub StreamingPull
      support. Also add tokio-stream if needed for async streaming.
  - title: Create Pub/Sub client wrapper
    done: true
    description: Create server/crates/ashford-core/src/pubsub.rs with helper to
      create authenticated Pub/Sub subscriber client from service account JSON
      string. Parse GmailNotification from received messages (emailAddress,
      historyId). Export from lib.rs.
  - title: Implement per-account Pub/Sub listener
    done: true
    description: "Create server/crates/ashford-core/src/pubsub_listener.rs with
      run_account_listener(account_id, subscription, credentials, queue) async
      function. Uses StreamingPull to receive messages. For each message: decode
      GmailNotification, enqueue history.sync.gmail job with idempotency key,
      ack the message. Handle reconnection on stream errors."
  - title: Implement Pub/Sub supervisor
    done: true
    description: "In pubsub_listener.rs, add run_pubsub_supervisor(db, queue,
      shutdown) that: loads all accounts with configured subscriptions, spawns
      listener task per account, watches for account changes (poll periodically
      or on signal), restarts failed listeners with backoff, gracefully shuts
      down all listeners on shutdown signal."
  - title: Create job dispatcher infrastructure
    done: true
    description: Create server/crates/ashford-core/src/jobs/mod.rs with
      JobDispatcher struct implementing JobExecutor trait. Holds Database and
      reqwest::Client. Routes jobs by job.job_type string to handler functions.
      Unknown job types return JobError::Fatal. Export job type constants
      (JOB_TYPE_INGEST_GMAIL, JOB_TYPE_HISTORY_SYNC_GMAIL).
  - title: Create thread repository
    done: true
    description: "Create server/crates/ashford-core/src/threads.rs with
      ThreadRepository following accounts.rs patterns. Implement:
      upsert(account_id, provider_thread_id, subject, snippet, last_message_at,
      raw_json) using INSERT ON CONFLICT, get_by_provider_id(account_id,
      provider_thread_id), update_last_message_at(id, timestamp). Return Thread
      struct with all fields. Add ThreadError enum."
  - title: Create message repository
    done: true
    description: "Create server/crates/ashford-core/src/messages.rs with
      MessageRepository. Implement: upsert(NewMessage struct with all fields)
      using INSERT ON CONFLICT on (account_id, provider_message_id),
      get_by_provider_id(account_id, provider_message_id), exists(account_id,
      provider_message_id) for quick dedup check. Add MessageError enum."
  - title: Implement email content parser
    done: true
    description: "Create server/crates/ashford-core/src/gmail/parser.rs with
      ParsedMessage struct and parse_message(Message) -> ParsedMessage function.
      Extract: from_email/from_name from From header (handle 'Name <email>'
      format), to/cc/bcc as Vec<Recipient>, subject from Subject header,
      body_plain and body_html by recursively searching MessagePart tree for
      text/plain and text/html mime_types. Decode base64url body data. Export
      from gmail/mod.rs."
  - title: Implement ingest.gmail job handler
    done: true
    description: Create server/crates/ashford-core/src/jobs/ingest_gmail.rs with
      handle_ingest_gmail(job, dispatcher) async function. Parse payload for
      account_id and message_id. Load account, refresh tokens if needed. Create
      GmailClient with NoopTokenStore (tokens already refreshed). Fetch message
      via client.get_message(). Parse content with parser. Upsert thread first
      (get thread_id), then upsert message with thread_id. Map errors to
      JobError (404->Fatal, 401/429/5xx->Retryable).
  - title: Implement minimal history.sync.gmail job handler
    done: true
    description: "Create server/crates/ashford-core/src/jobs/history_sync_gmail.rs
      with handle_history_sync(job, dispatcher) async function. Parse payload
      for account_id and history_id. Load account, get stored history_id from
      state. Call client.list_history(stored_history_id). For each messagesAdded
      event, enqueue ingest.gmail job with idempotency key. Update account state
      with new history_id. Note: pagination and gap handling deferred to Plan
      12."
  - title: Handle Gmail API errors in job handlers
    done: true
    description: "In both ingest_gmail.rs and history_sync_gmail.rs, implement error
      mapping: GmailClientError::Unauthorized -> JobError::Retryable (token
      issue), reqwest 404 -> JobError::Fatal (message/history deleted), reqwest
      429 -> JobError::Retryable (rate limit), reqwest 5xx ->
      JobError::Retryable (server error). Log errors with tracing."
  - title: Wire job dispatcher and Pub/Sub supervisor in main.rs
    done: true
    description: "In main.rs: Replace NoopExecutor with JobDispatcher. Spawn Pub/Sub
      supervisor task alongside worker. Pass shutdown token to both. Update
      imports from ashford_core. Supervisor and worker run concurrently, both
      drain gracefully on shutdown."
  - title: Export new modules from lib.rs
    done: true
    description: "Update server/crates/ashford-core/src/lib.rs to export: jobs
      module (JobDispatcher, job type constants), threads module
      (ThreadRepository, Thread, ThreadError), messages module
      (MessageRepository, Message as StoredMessage, MessageError), pubsub
      module, pubsub_listener module (run_pubsub_supervisor). Update
      gmail/mod.rs to export parser."
  - title: Write unit tests for parser
    done: true
    description: "Add tests in gmail/parser.rs for: simple single-part message,
      multipart/alternative with plain and html, nested multipart/mixed, various
      From header formats ('Name <email>', '<email>', 'email'), multiple
      recipients in To/CC, base64url decoding, missing optional fields."
  - title: Write unit tests for repositories
    done: true
    description: "Add tests in threads.rs and messages.rs using temporary in-memory
      database: upsert creates new record, upsert updates existing record,
      get_by_provider_id returns correct record, exists returns true/false
      correctly, last_message_at updates thread timestamp."
  - title: Write integration tests for ingest flow
    done: true
    description: "Add tests for end-to-end flow: ingest.gmail job with mocked Gmail
      API response creates thread and message records, idempotency prevents
      duplicate message insertion, history.sync.gmail enqueues ingest jobs for
      messagesAdded. Use wiremock for Gmail API mocking. Note: Pub/Sub listener
      testing may require integration test with emulator or be deferred."
changedFiles:
  - server/Cargo.lock
  - server/crates/ashford-core/Cargo.toml
  - server/crates/ashford-core/src/accounts.rs
  - server/crates/ashford-core/src/gmail/mod.rs
  - server/crates/ashford-core/src/gmail/parser.rs
  - server/crates/ashford-core/src/jobs/history_sync_gmail.rs
  - server/crates/ashford-core/src/jobs/ingest_gmail.rs
  - server/crates/ashford-core/src/jobs/mod.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/messages.rs
  - server/crates/ashford-core/src/migrations.rs
  - server/crates/ashford-core/src/pubsub.rs
  - server/crates/ashford-core/src/pubsub_listener.rs
  - server/crates/ashford-core/src/threads.rs
  - server/crates/ashford-core/tests/ingest_flow.rs
  - server/crates/ashford-server/src/main.rs
  - server/migrations/003_add_thread_message_unique_indices.sql
tags:
  - gmail
  - pubsub
  - rust
---

This plan implements real-time message ingestion:

- Pub/Sub webhook endpoint to receive Gmail notifications
- Job dispatcher infrastructure to route jobs by type
- ingest.gmail job handler that fetches message from Gmail API
- Email content parsing (from, to, subject, body_plain, body_html)
- Thread and message upsert operations
- Idempotency handling to prevent duplicate processing

Depends on: Gmail API Client & Account Management plan

<!-- rmplan-generated-start -->
This plan implements real-time message ingestion with end-to-end functionality:

- Pub/Sub Pull subscription with StreamingPull for receiving Gmail notifications
- Per-account listener architecture with supervisor for managing multiple accounts
- Job dispatcher infrastructure to route jobs by type
- Minimal history.sync.gmail handler to enumerate new messages (full version in Plan 12)
- ingest.gmail job handler that fetches messages from Gmail API
- Email content parsing (from, to, subject, body_plain, body_html)
- Thread and message repositories with upsert operations
- Idempotency handling to prevent duplicate processing

Depends on: Plan 10 (Gmail API Client & Account Management) - COMPLETED

## Expected Behavior/Outcome

When a new email arrives in Gmail:
1. Gmail publishes notification to account's Pub/Sub topic
2. Per-account Pub/Sub listener receives message via StreamingPull
3. Listener decodes notification, enqueues `history.sync.gmail` job
4. History sync job calls Gmail History API, enqueues `ingest.gmail` job for each new message
5. Ingest job fetches full message, parses content, upserts thread and message records
6. Message is acknowledged after job is enqueued

**Architecture:**
- Pub/Sub supervisor manages lifecycle of per-account listeners
- Each account has its own subscription and service account credentials
- Listeners reconnect automatically on stream errors
- Supervisor watches for account additions/removals

**States:**
- Job states: queued → running → completed/failed (existing infrastructure)
- Duplicate notifications handled via idempotency keys
- Failed jobs retry with exponential backoff (up to 5 attempts)

## Acceptance Criteria

- [ ] PubsubConfig extended with service_account_json field
- [ ] Pub/Sub supervisor spawns listener per account with configured subscription
- [ ] Listeners receive messages via StreamingPull and enqueue history.sync.gmail jobs
- [ ] Listeners reconnect automatically on connection failures
- [ ] history.sync.gmail job calls History API and enqueues ingest.gmail jobs for messagesAdded
- [ ] ingest.gmail job fetches message from Gmail API
- [ ] Email content (from, to, cc, bcc, subject, bodies) is correctly parsed from MIME structure
- [ ] Thread records are created/updated with correct last_message_at
- [ ] Message records are upserted with all fields populated
- [ ] Duplicate messages are handled via idempotency keys (no duplicate DB records)
- [ ] Gmail API errors (401, 404, 429, 5xx) are mapped to appropriate JobError variants
- [ ] Graceful shutdown stops supervisor and all listeners
- [ ] All new code paths are covered by unit and integration tests

## Dependencies & Constraints

- **Dependencies**: Plan 10 (Gmail API Client) - completed; provides GmailClient, AccountRepository, OAuthTokens
- **New dependency**: google-cloud-pubsub crate for StreamingPull
- **Technical Constraints**: 
  - Each account needs GCP service account with pubsub.subscriber role
  - Service account JSON stored in AccountConfig.pubsub.service_account_json
  - Gmail uses base64url encoding (not standard base64) for message bodies
  - Thread must be upserted before message due to foreign key constraint
  - History API pagination deferred to Plan 12

## Files to Create

- `server/crates/ashford-core/src/pubsub.rs` - Pub/Sub client wrapper and types
- `server/crates/ashford-core/src/pubsub_listener.rs` - Per-account listener and supervisor
- `server/crates/ashford-core/src/gmail/parser.rs` - Email content parser
- `server/crates/ashford-core/src/jobs/mod.rs` - Job dispatcher
- `server/crates/ashford-core/src/jobs/ingest_gmail.rs` - Ingest job handler
- `server/crates/ashford-core/src/jobs/history_sync_gmail.rs` - History sync job handler (minimal)
- `server/crates/ashford-core/src/threads.rs` - Thread repository
- `server/crates/ashford-core/src/messages.rs` - Message repository

## Files to Modify

- `server/crates/ashford-core/Cargo.toml` - Add google-cloud-pubsub dependency
- `server/crates/ashford-core/src/accounts.rs` - Extend PubsubConfig with service_account_json
- `server/crates/ashford-server/src/main.rs` - Wire dispatcher, spawn Pub/Sub supervisor
- `server/crates/ashford-core/src/lib.rs` - Export new modules
- `server/crates/ashford-core/src/gmail/mod.rs` - Export parser

## Implementation Notes

**Recommended Approach:**
1. Extend PubsubConfig with service_account_json field
2. Add google-cloud-pubsub dependency
3. Build repositories (threads.rs, messages.rs) - foundational, easy to test
4. Build parser - pure functions, easy to unit test
5. Create Pub/Sub client wrapper and listener infrastructure
6. Create job dispatcher with routing logic
7. Implement ingest.gmail handler (core functionality)
8. Add minimal history.sync.gmail handler
9. Implement Pub/Sub supervisor
10. Wire everything in main.rs
11. Integration tests

**Pub/Sub Listener Design:**
```rust
// Per-account listener
async fn run_account_listener(
    account_id: String,
    subscription: String,
    credentials: ServiceAccountCredentials,
    queue: JobQueue,
    shutdown: CancellationToken,
) -> Result<(), PubsubError>

// Supervisor manages all listeners
async fn run_pubsub_supervisor(
    db: Database,
    queue: JobQueue,
    shutdown: CancellationToken,
) -> Result<(), PubsubError>
```

**Potential Gotchas:**
- Service account JSON parsing - use serde to deserialize from string in PubsubConfig
- StreamingPull connection management - need reconnection logic with backoff
- Gmail's base64url encoding requires `URL_SAFE_NO_PAD` config in base64 crate
- From header parsing must handle multiple formats: "Name <email>", "<email>", "email"
- Thread upsert must happen before message insert (FK constraint)
- Supervisor needs to handle account additions/removals (poll DB periodically)

**TokenStore Strategy:**
Rather than implementing a full `DatabaseTokenStore`, the job handlers will:
1. Load account via `AccountRepository::get_by_id()`
2. Call `AccountRepository::refresh_tokens_if_needed()` to ensure fresh tokens
3. Create `GmailClient` with `NoopTokenStore` using the refreshed tokens
4. If 401 occurs during API call, return `JobError::Retryable` (next attempt will refresh)

**Scope Boundaries:**
- This plan: Single-page history sync, basic error handling, per-account Pull listeners
- Plan 12: Full pagination, gap detection (404 on old historyId), backfill jobs

## Review Fixes (Autofix Session)

Fixed all 4 issues identified in the code review:

1. **CRITICAL: Missing reqwest dependency in ashford-server** - Added reqwest dependency to `server/crates/ashford-server/Cargo.toml` with version 0.12.24, matching ashford-core. Uses rustls-tls feature.

2. **MAJOR: Flaky test in messages module** - Fixed by using UUID-based unique database filenames instead of hardcoded 'db.sqlite' in test setup helpers. The pattern `format!("db_{}.sqlite", uuid::Uuid::new_v4())` ensures each test gets an isolated database even when tests run in parallel. Applied consistently across 9 test files: messages.rs, threads.rs, jobs/mod.rs, jobs/ingest_gmail.rs, jobs/history_sync_gmail.rs, pubsub_listener.rs, queue.rs, worker.rs, accounts.rs, and tests/ingest_flow.rs.

3. **MINOR: Unbounded recursion in MIME parser** - Added `MAX_MIME_DEPTH` constant (50 levels) in `server/crates/ashford-core/src/gmail/parser.rs`. The `extract_bodies` function now takes a depth parameter and returns early if depth exceeds the limit. Added test `depth_limit_prevents_stack_overflow`.

4. **MINOR: Escaped quotes in address parsing** - Improved `split_addresses` function to track the previous character and detect escaped quotes (backslash before quote). Updated `strip_quotes` to unescape internal escaped quotes. Added test `handles_escaped_quotes_in_names`.

All 102 tests pass consistently, verified with multiple runs using `--test-threads=8`.
<!-- rmplan-generated-end -->

## Research

### Summary

This plan implements the core message ingestion pipeline for Ashford, enabling real-time Gmail message processing through Google Cloud Pub/Sub Pull subscriptions. The implementation builds on Plan 10's completed Gmail API client and account management system. The key deliverables are:

1. **Pub/Sub Pull subscription with StreamingPull** - Per-account listeners for receiving Gmail notifications
2. **Pub/Sub supervisor** - Manages lifecycle of per-account listeners
3. **Job dispatcher infrastructure** - Routes jobs by type to specific handlers
4. **Thread and message repositories** - CRUD operations for email storage
5. **Email content parser** - Extracts headers, bodies, and metadata from Gmail MIME structure
6. **ingest.gmail job handler** - Fetches messages from Gmail API and persists them
7. **Minimal history.sync.gmail handler** - Calls History API and enqueues ingest jobs

The existing job queue system provides idempotency, retries with exponential backoff, and graceful shutdown - all needed for reliable message processing.

### Findings

#### Pub/Sub Pull Subscription Architecture

**Why Pull instead of Push:**
- No public HTTPS endpoint required - works behind NAT/firewall
- Better for local development and self-hosted deployments
- More control over rate of message consumption
- No webhook security concerns - authenticated via service account
- Can pause/resume consumption (useful during deployments)

**Per-account configuration:**
- Each Gmail account may be in a different GCP project
- Each account has its own Pub/Sub subscription and service account
- Service account JSON stored in `AccountConfig.pubsub.service_account_json`
- Service account needs `roles/pubsub.subscriber` on the subscription

**Gmail notification format** (decoded from Pub/Sub message data):
```json
{
  "emailAddress": "user@example.com",
  "historyId": "12345"
}
```

**Supervisor architecture:**
- Loads all accounts with configured Pub/Sub subscriptions
- Spawns one StreamingPull listener task per account
- Watches for account additions/removals (poll DB periodically)
- Restarts failed listeners with exponential backoff
- Gracefully shuts down all listeners on shutdown signal

**Design decision:** The listener enqueues a `history.sync.gmail` job (not `ingest.gmail` directly) because Pub/Sub only provides the historyId, not specific message IDs. The history sync job calls the History API to enumerate new messages and enqueues individual `ingest.gmail` jobs.

#### Job Queue System

Location: `server/crates/ashford-core/src/queue.rs` and `worker.rs`

**Job Structure:**
```rust
pub struct Job {
    pub id: String,
    pub job_type: String,
    pub payload: Value,              // serde_json::Value
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
```

**JobExecutor Trait:**
```rust
#[async_trait]
pub trait JobExecutor: Send + Sync {
    async fn execute(&self, job: Job, ctx: JobContext) -> Result<(), JobError>;
}
```

**JobError variants:**
- `JobError::Retryable(String)` - triggers retry with exponential backoff
- `JobError::Fatal(String)` - permanent failure, no retry

**Current setup in main.rs:**
- Worker uses `NoopExecutor` (placeholder)
- Need to replace with `JobDispatcher` that routes by `job.job_type`

**Idempotency pattern:**
```rust
queue.enqueue(
    "ingest.gmail",
    json!({"account_id": "...", "message_id": "..."}),
    Some(format!("ingest.gmail:{}:{}", account_id, message_id)),  // idempotency key
    1,  // priority
).await?
```

#### Gmail Client (Plan 10 - Completed)

Location: `server/crates/ashford-core/src/gmail/`

**Available methods:**
```rust
impl<S: TokenStore> GmailClient<S> {
    pub async fn get_message(&self, message_id: &str) -> Result<Message, GmailClientError>
    pub async fn get_thread(&self, thread_id: &str) -> Result<Thread, GmailClientError>
    pub async fn list_messages(...) -> Result<ListMessagesResponse, GmailClientError>
    pub async fn list_history(...) -> Result<ListHistoryResponse, GmailClientError>
}
```

**Gmail Message type** (from `gmail/types.rs`):
```rust
pub struct Message {
    pub id: String,
    pub thread_id: Option<String>,
    pub label_ids: Vec<String>,
    pub snippet: Option<String>,
    pub history_id: Option<String>,
    pub internal_date: Option<String>,
    pub payload: Option<MessagePart>,
    pub size_estimate: Option<u64>,
    pub raw: Option<String>,
}

pub struct MessagePart {
    pub part_id: Option<String>,
    pub mime_type: Option<String>,
    pub filename: Option<String>,
    pub headers: Vec<Header>,
    pub body: Option<MessagePartBody>,
    pub parts: Vec<MessagePart>,  // nested for multipart
}

pub struct MessagePartBody {
    pub size: i64,
    pub data: Option<String>,  // base64-encoded
    pub attachment_id: Option<String>,
}
```

**Error types:**
- `GmailClientError::Unauthorized` - 401, token refresh failed
- `GmailClientError::Http(reqwest::Error)` - includes status codes (404, 429, 5xx)

#### Account Management (Plan 10 - Completed)

Location: `server/crates/ashford-core/src/accounts.rs`

**AccountRepository methods:**
```rust
pub async fn get_by_id(&self, id: &str) -> Result<Account, AccountError>
pub async fn get_by_email(&self, email: &str) -> Result<Account, AccountError>
pub async fn update_state(&self, id: &str, state: &AccountState) -> Result<Account, AccountError>
pub async fn refresh_tokens_if_needed(&self, account_id: &str, http: &Client) -> Result<Account, AccountError>
```

**TokenStore integration:**
The job dispatcher will need to implement `TokenStore` to persist refreshed tokens back to the database. The existing `AccountRepository::refresh_tokens_if_needed` can be used, but the GmailClient expects a `TokenStore` implementation.

#### Database Schema for Threads and Messages

Location: `server/migrations/001_initial.sql`

**threads table:**
```sql
CREATE TABLE threads (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  provider_thread_id TEXT NOT NULL,  -- Gmail thread ID
  subject TEXT,
  snippet TEXT,
  last_message_at TEXT,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  raw_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(id)
);
CREATE INDEX threads_account_thread_idx ON threads(account_id, provider_thread_id);
```

**messages table:**
```sql
CREATE TABLE messages (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  thread_id TEXT NOT NULL,
  provider_message_id TEXT NOT NULL,  -- Gmail message ID
  from_email TEXT,
  from_name TEXT,
  to_json TEXT NOT NULL DEFAULT '[]',
  cc_json TEXT NOT NULL DEFAULT '[]',
  bcc_json TEXT NOT NULL DEFAULT '[]',
  subject TEXT,
  snippet TEXT,
  received_at TEXT,
  internal_date TEXT,
  labels_json TEXT NOT NULL DEFAULT '[]',
  headers_json TEXT NOT NULL DEFAULT '{}',
  body_plain TEXT,
  body_html TEXT,
  raw_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(id),
  FOREIGN KEY (thread_id) REFERENCES threads(id)
);
CREATE INDEX messages_account_msg_idx ON messages(account_id, provider_message_id);
```

**Upsert pattern:** Use `INSERT ... ON CONFLICT DO UPDATE` with the unique index on `(account_id, provider_message_id)`.

#### Main.rs Integration

Location: `server/crates/ashford-server/src/main.rs`

**Current structure:**
```rust
#[derive(Clone)]
struct AppState {
    db: Database,
}
```

**Changes needed:**
- Spawn Pub/Sub supervisor task alongside worker
- Pass shutdown token to both supervisor and worker
- Both run concurrently and drain gracefully on shutdown

```rust
// In main()
let supervisor_handle = tokio::spawn(run_pubsub_supervisor(
    db.clone(),
    queue.clone(),
    shutdown.child_token(),
));

// Wait for both on shutdown
tokio::select! {
    _ = worker_handle => {},
    _ = supervisor_handle => {},
}
```

#### Repository Pattern Reference

From `accounts.rs`, the established pattern:
- Struct with `db: Database` field
- `COLUMNS` constant for SELECT queries
- Helper function `row_to_entity` for deserializing rows
- Custom error enum with thiserror
- All operations async, using `db.connection()` for each query
- JSON serialization for complex nested data
- RFC3339 timestamps with millisecond precision via `now_rfc3339()` helper

#### Email Content Parsing Requirements

The parser needs to extract from Gmail's MIME structure:
1. **From header:** Parse "Display Name <email@example.com>" format
2. **To/CC/BCC:** Parse multiple recipients, each in the same format
3. **Subject:** Direct header extraction
4. **body_plain:** Find `text/plain` part in MIME tree
5. **body_html:** Find `text/html` part in MIME tree

**MIME traversal:**
- Single part message: `payload.body.data` contains the content
- Multipart: Recursively search `payload.parts` for desired mime_type
- Common structures:
  - `multipart/mixed` → contains `multipart/alternative` + attachments
  - `multipart/alternative` → contains `text/plain` and `text/html`
- Body data is base64url-encoded (Gmail uses URL-safe base64)

#### Existing Dependencies

From `server/crates/ashford-core/Cargo.toml`:
- `reqwest` (0.12.24) - HTTP client
- `serde`/`serde_json` - JSON serialization
- `chrono` - DateTime handling
- `uuid` - ID generation
- `thiserror` - Error handling
- `async-trait` - Async traits
- `tracing` - Logging
- `base64` - Already available for decoding

No additional dependencies needed for basic parsing.

### Risks & Constraints

1. **StreamingPull connection management:** The Pub/Sub StreamingPull connection can drop due to network issues or server-side disconnects. The listener must implement reconnection with exponential backoff. Consider using the library's built-in retry mechanisms if available.

2. **Service account credential management:** Each account stores service account JSON in the database. This is sensitive data - ensure proper access controls. The JSON must be parsed into credentials for each listener.

3. **Supervisor complexity:** Managing multiple concurrent listener tasks requires careful lifecycle handling:
   - Detect and restart failed listeners
   - Handle account additions/removals (poll DB or signal-based)
   - Graceful shutdown must wait for all listeners to stop
   - Avoid spawning duplicate listeners for the same account

4. **TokenStore implementation gap:** The GmailClient requires a `TokenStore` implementation to persist refreshed tokens. The job handler will use `NoopTokenStore` with pre-refreshed tokens from `AccountRepository::refresh_tokens_if_needed`.

5. **Thread creation race condition:** When ingesting messages, the thread may not exist yet. Solution: upsert thread first (get thread_id), then upsert message with that thread_id.

6. **Message deletion handling:** Gmail Pub/Sub notifications include message deletions. This plan focuses on `ingest.gmail` which handles additions. Deletions should be handled in Plan 12 (`history.sync.gmail`).

7. **Rate limiting:** Gmail API has rate limits (250 quota units per user per second). The job queue's retry with exponential backoff handles 429 responses, but backfill operations (Plan 12) should use lower priority.

8. **MIME complexity:** Some emails have deeply nested MIME structures or unusual encodings. The parser should handle common cases first and log warnings for unexpected structures rather than failing.

9. **Base64 URL-safe encoding:** Gmail uses base64url (with `-` and `_`) instead of standard base64. The `base64` crate's `URL_SAFE_NO_PAD` configuration handles this.

10. **Large message bodies:** Some emails have very large HTML bodies. Consider truncation limits or separate storage for the database.

11. **Dependency on Plan 10:** All Gmail client functionality is already implemented and tested. This plan can proceed.

12. **google-cloud-pubsub crate selection:** Need to evaluate available Rust crates for Pub/Sub. Options include `google-cloud-pubsub`, `google-cloud-googleapis`, or direct gRPC with `tonic`. Choose based on StreamingPull support and maintenance status.

Added service_account_json to PubsubConfig to hold per-account Pub/Sub service account credentials, marking the field with serde(default) so existing stored configs deserialize cleanly and extending Default/sample_config accordingly (server/crates/ashford-core/src/accounts.rs). Added google-cloud-pubsub dependency to ashford-core with auth/google-cloud-auth and rustls-tls features while disabling default features to avoid native OpenSSL, preparing for upcoming Pub/Sub listener work; Cargo.lock updated via cargo check. Tasks: Extend PubsubConfig with service account; Add google-cloud-pubsub dependency. Ran cargo check -p ashford-core and cargo test -p ashford-core accounts to validate builds.

Implemented Pub/Sub client helper (task 3) in server/crates/ashford-core/src/pubsub.rs to build authenticated subscribers from service account JSON and parse base64 Gmail notifications into GmailNotification with rich PubsubError including Account/Queue conversions. Added per-account listener and supervisor scaffolding (tasks 4 & 5) in server/crates/ashford-core/src/pubsub_listener.rs: StreamingPull loop with reconnection/backoff, ack/nack handling, idempotent enqueue of history.sync.gmail jobs, and supervisor that polls AccountRepository, restarts listeners on config changes, and cancels on shutdown. Introduced JobDispatcher stub with job type constants (task 6) in server/crates/ashford-core/src/jobs/mod.rs for future handlers. Exported new modules via server/crates/ashford-core/src/lib.rs and added google-cloud-auth/gax/googleapis deps in Cargo.toml; cargo check -p ashford-core passes.

Resolved reviewer fixes for Pub/Sub ingestion (Task 11). Updated Gmail notification parsing in server/crates/ashford-core/src/pubsub.rs to decode URL-safe, no-padding base64 as required by Gmail Pub/Sub; adjusted tests to match. Hardened supervisor resiliency in server/crates/ashford-core/src/pubsub_listener.rs by detecting finished listener tasks (panic/exit) and restarting them on the next reconcile tick, instead of leaving dead handles that halt consumption. Changed JobDispatcher behavior in server/crates/ashford-core/src/jobs/mod.rs so known Gmail jobs now return Retryable errors rather than Fatal, preventing premature permanent failure until handlers are implemented. Ran cargo fmt and cargo test -p ashford-core to validate changes.

Implemented ThreadRepository and MessageRepository with deterministic upserts keyed on provider IDs via new unique indexes (migration 003_add_thread_message_unique_indices.sql). Threads upsert/update last_message_at with monotonic logic; messages upsert handles full envelope/body fields plus exists/get helpers. Added gmail/parser.rs with Recipient/ParsedMessage types, header parsing for From/To/Cc/Bcc, base64url body decoding, and recursive MIME traversal. Exported new modules via gmail/mod.rs and lib.rs re-exports. Added comprehensive unit tests for repositories (seeding accounts/threads to satisfy FKs) and parser cases covering single/multipart bodies, nested parts, base64url, and varied address formats. All changes validated with cargo test -p ashford-core.

Implemented ingest.gmail and history.sync.gmail job handling (Tasks 10-13). Added new job modules server/crates/ashford-core/src/jobs/ingest_gmail.rs and history_sync_gmail.rs that parse payloads, refresh account tokens via AccountRepository, build GmailClient (NoopTokenStore) with optional gmail_api_base from JobDispatcher, and map Gmail/Account errors to JobError. ingest.gmail fetches Gmail message, parses content with gmail::parser, converts internalDate millis to DateTime, upserts thread (ThreadRepository) then message (MessageRepository) with parsed headers/bodies/recipients/labels and shared raw JSON, and logs completion. history.sync.gmail calls list_history starting from stored or payload historyId, enqueues ingest jobs idempotently (ingest.gmail:{account}:{message}), and updates account state history_id + last_sync_at. Enhanced JobDispatcher (jobs/mod.rs) with optional gmail_api_base, builder, dispatcher routing to handlers, and shared error mappers. Updated server/crates/ashford-server/src/main.rs to run worker with JobDispatcher instead of NoopExecutor and spawn run_pubsub_supervisor alongside worker under shared shutdown. Added wiremock-backed unit tests for both handlers validating Gmail call, persistence, job enqueues, and state updates; removed old placeholder dispatcher tests. cargo fmt and cargo test -p ashford-core pass.

Implemented Task 17 integration tests by adding server/crates/ashford-core/tests/ingest_flow.rs. Introduced two end-to-end worker tests using wiremock and the real JobDispatcher/worker pipeline: (1) worker_processes_history_and_ingests_message spins up a temp DB, seeds a Gmail account, mocks Gmail history and message endpoints, runs run_worker with a fast config, and verifies the history.sync job enqueues ingest, ingests the message, upserts thread/message records, and updates account.history_id. (2) worker_deduplicates_ingest_for_same_message mocks history returning the same message twice, ensures the worker only calls Gmail once (wiremock expect(1)), and asserts the database contains a single message row while the ingest job reaches the completed state. Tests use CancellationToken to stop the worker after assertions. No production code changes required; only new integration coverage was added.

Addressed the reviewer-reported flakiness in the ingest flow integration test (Task 17: Write integration tests for ingest flow). In server/crates/ashford-core/tests/ingest_flow.rs the worker_deduplicates_ingest_for_same_message test now waits for the ingest job to reach a terminal state instead of exiting once the message appears. The polling loop checks the jobs table ordered by created_at, breaking only when state becomes completed and failing fast on failed/canceled, so we assert after the worker finishes rather than while it may still be running. This keeps the test deterministic while preserving the existing message count and job state assertions. Verified with cargo test -p ashford-core --tests from server/.
