---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 2: Gmail Ingest (Pub/Sub + History)"
goal: Implement Gmail integration with Pub/Sub notifications, History API sync,
  and message ingestion
id: 3
uuid: b93a0b33-fccb-4f57-8c97-002039917c44
generatedBy: agent
status: in_progress
priority: high
container: true
temp: false
dependencies:
  - 2
  - 10
  - 11
  - 12
parent: 1
issue: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T01:23:11.905Z
promptsGeneratedAt: 2025-11-29T01:23:11.905Z
createdAt: 2025-11-29T01:21:26.709Z
updatedAt: 2025-11-29T07:56:18.923Z
tasks: []
tags:
  - gmail
  - pubsub
  - rust
---

Gmail integration for receiving and storing emails:
- Account configuration and OAuth token management
- Pub/Sub message handler endpoint
- ingest.gmail job handler
- History API integration for catchup/gap filling
- Message and thread storage (accounts, threads, messages tables)
- Backfill job for initial account setup

## Research

### Summary
- This milestone implements Gmail integration for the Ashford email assistant, enabling real-time message ingestion via Pub/Sub and catchup/gap-filling via the History API.
- The existing codebase provides a solid foundation with a working job queue, database migrations, HTTP server (Axum), and configuration system. The schema already includes accounts, threads, and messages tables.
- Key challenge is implementing the Gmail API client with OAuth2 token management, including secure storage and automatic refresh.
- The Pub/Sub webhook and History API sync need careful idempotency handling to avoid duplicate message processing.

### Findings

#### 1. Codebase Architecture Overview

**Directory Structure:**
```
server/
├── crates/
│   ├── ashford-core/          # Core domain logic
│   │   └── src/
│   │       ├── config.rs      # Configuration with GmailConfig already defined
│   │       ├── db.rs          # Database wrapper (libsql)
│   │       ├── queue.rs       # Job queue implementation
│   │       ├── worker.rs      # Job worker loop
│   │       ├── migrations.rs  # Database migrations
│   │       └── telemetry.rs   # OpenTelemetry setup
│   └── ashford-server/        # HTTP API server
│       └── src/main.rs        # Axum server, routes, startup
└── migrations/
    ├── 001_initial.sql        # Core schema with accounts, threads, messages
    └── 002_add_job_completion_fields.sql
```

**Framework Stack:**
- HTTP: Axum 0.8.7
- Database: libsql (SQLite-compatible)
- Async Runtime: Tokio
- Serialization: serde/serde_json
- Telemetry: OpenTelemetry with tracing

#### 2. Job Queue System (Already Implemented)

**File:** `server/crates/ashford-core/src/queue.rs`

The job queue is fully implemented with:
- **Job States:** Queued → Running → Completed/Failed/Canceled
- **Retry Logic:** Exponential backoff with jitter (2^attempts seconds, capped at 300s, ±25% jitter)
- **Idempotency:** Unique constraint on `idempotency_key` column prevents duplicate jobs
- **Heartbeat:** `heartbeat_at` field for detecting stale jobs
- **Job Steps:** `job_steps` table for tracking sub-operations within a job

**Key Methods:**
- `JobQueue::enqueue(job_type, payload, idempotency_key, priority)` - Create new job
- `JobQueue::claim_next()` - Atomic claim for worker
- `JobQueue::complete(job_id, result)` - Mark complete
- `JobQueue::fail(job_id, error, should_retry)` - Handle failure with optional retry
- `JobQueue::start_step()` / `finish_step()` - Track job progress

**JobExecutor Trait** (`worker.rs`):
```rust
#[async_trait]
pub trait JobExecutor: Send + Sync {
    async fn execute(&self, job: Job, ctx: JobContext) -> Result<(), JobError>;
}

pub enum JobError {
    Retryable(String),  // Will retry with backoff
    Fatal(String),      // No retry, mark as failed
}
```

**To add new job types:** Implement a dispatcher that routes by `job.job_type` string and delegates to specific handlers.

#### 3. Database Schema (Already Exists)

**File:** `server/migrations/001_initial.sql`

**accounts table:**
```sql
CREATE TABLE accounts (
  id TEXT PRIMARY KEY,
  provider TEXT NOT NULL CHECK (provider IN ('gmail')),
  email TEXT NOT NULL,
  display_name TEXT,
  config_json TEXT NOT NULL,           -- OAuth tokens, settings
  state_json TEXT NOT NULL DEFAULT '{}', -- historyId, sync state
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX accounts_email_idx ON accounts(email);
```

**threads table:**
```sql
CREATE TABLE threads (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  provider_thread_id TEXT NOT NULL,     -- Gmail thread ID
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
CREATE INDEX threads_last_message_idx ON threads(account_id, last_message_at);
```

**messages table:**
```sql
CREATE TABLE messages (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  thread_id TEXT NOT NULL,
  provider_message_id TEXT NOT NULL,
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
CREATE INDEX messages_thread_idx ON messages(thread_id, received_at);
CREATE INDEX messages_from_idx ON messages(account_id, from_email);
```

**Key Observation:** The schema is already designed for Gmail integration. No migrations needed for core tables.

#### 4. Configuration (Already Supports Gmail)

**File:** `server/crates/ashford-core/src/config.rs`

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct GmailConfig {
    pub use_pubsub: bool,
    pub project_id: String,
    pub subscription: String,
}
```

The config supports `env:` prefixes for secret injection:
```toml
[gmail]
use_pubsub = true
project_id = "env:GMAIL_PROJECT"
subscription = "env:GMAIL_SUB"
```

**Extension Needed:** Add fields for OAuth client ID/secret reference, redirect URI, etc.

#### 5. HTTP Server Patterns

**File:** `server/crates/ashford-server/src/main.rs`

Current router:
```rust
fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .with_state(state)
}

#[derive(Clone)]
struct AppState {
    db: Database,
}
```

**Handler Pattern:**
```rust
async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    // Implementation
}
```

**To add Pub/Sub webhook:**
```rust
.route("/webhooks/gmail-pubsub", post(gmail_pubsub_webhook))
```

**Worker initialization needs update** - currently uses `NoopExecutor`, needs to be replaced with real executor routing.

#### 6. Query Patterns

The codebase uses raw SQL with parameterized queries (libsql `params![]` macro):

```rust
conn.execute(
    "INSERT INTO jobs (...) VALUES (?1, ?2, ?3, ...)",
    params![id, job_type, payload_json, ...],
).await?;

let mut rows = conn.query(
    "SELECT ... FROM jobs WHERE id = ?1",
    params![job_id],
).await?;
```

**Row Deserialization:** Manual field-by-field extraction using `row.get(index)`:
```rust
fn row_to_job(row: Row) -> Result<Job, QueueError> {
    let id: String = row.get(0)?;
    let job_type: String = row.get(1)?;
    // ...
}
```

#### 7. Gmail API Integration Considerations

**Gmail API Client Options:**
1. **google-gmail crate** - Full Google API client (heavy dependency)
2. **reqwest + manual REST** - Lighter, more control
3. **google-apis-rs** - Google's official but complex

**Recommended Approach:** Use `reqwest` with a thin wrapper for Gmail REST API. This matches the existing pattern of minimal dependencies and gives full control over OAuth flow.

**Gmail Pub/Sub Message Format:**
```json
{
  "message": {
    "data": "base64-encoded-json",
    "attributes": {
      "email": "user@gmail.com"
    },
    "messageId": "123",
    "publishTime": "2024-01-01T00:00:00Z"
  },
  "subscription": "projects/project-id/subscriptions/subscription-name"
}
```

**Decoded data contains:**
```json
{
  "emailAddress": "user@gmail.com",
  "historyId": "12345"
}
```

**Gmail History API Response:**
```json
{
  "history": [
    {
      "id": "12346",
      "messagesAdded": [
        { "message": { "id": "msg123", "threadId": "thread123" } }
      ],
      "labelsAdded": [...],
      "labelsRemoved": [...]
    }
  ],
  "historyId": "12350",
  "nextPageToken": "..."
}
```

#### 8. OAuth2 Token Management

**Token Storage Decision:** Store in `config_json` within the accounts table. For a single-user local app, this is acceptable and simplifies implementation/testing. Structure:
```json
{
  "oauth": {
    "access_token": "...",
    "refresh_token": "...",
    "expires_at": "2024-01-01T00:00:00Z"
  },
  "pubsub": {
    "topic": "projects/.../topics/gmail",
    "subscription": "projects/.../subscriptions/..."
  }
}
```

**Initial Token Acquisition:**
A standalone script (`scripts/gmail-oauth`) handles the "Desktop app" OAuth flow:
1. Opens browser to Google consent screen
2. Runs temporary localhost HTTP server to receive callback
3. Exchanges authorization code for tokens
4. Outputs JSON for user to copy into account creation

This keeps OAuth complexity out of the main application.

**Token Refresh Flow:**
1. Before Gmail API call, check `expires_at`
2. If expired (or within 5 minutes), refresh using `refresh_token`
3. Update `config_json` with new tokens
4. Retry original request

#### 9. Job Types and Payloads

**Documented Job Types** (from `docs/job_queue.md`):
- `ingest.gmail` - Fetch and store a specific message
- `backfill.gmail` - Initial bulk import
- `history.sync.gmail` - Sync using History API

**Proposed Payloads:**

`ingest.gmail`:
```json
{
  "account_id": "uuid",
  "message_id": "gmail_message_id",
  "thread_id": "gmail_thread_id"
}
```

`history.sync.gmail`:
```json
{
  "account_id": "uuid",
  "start_history_id": "12345"
}
```

`backfill.gmail`:
```json
{
  "account_id": "uuid",
  "query": "newer_than:30d",
  "page_token": null
}
```

**Idempotency Keys:**
- `ingest.gmail:{account_id}:{message_id}`
- `history.sync.gmail:{account_id}:{history_id}`
- `backfill.gmail:{account_id}:{query_hash}:{page_token}`

#### 10. Email Content Parsing

**MIME Parsing:** Use `mail-parser` crate for robust RFC 5322 parsing:
- Handles multipart/alternative, multipart/mixed
- Extracts plain text and HTML bodies
- Parses headers into structured data
- Handles character encoding

**Gmail API provides:**
- `payload.headers[]` - Parsed headers
- `payload.body.data` - Base64-encoded body (simple messages)
- `payload.parts[]` - Multipart structure
- `snippet` - Pre-extracted text snippet

**Strategy:** Use Gmail's parsed structure when available, fall back to `mail-parser` for raw content parsing if needed.

### Risks & Constraints

#### Technical Risks

1. **OAuth Token Refresh Races:**
   - Multiple concurrent jobs could try to refresh simultaneously
   - Mitigation: Use optimistic locking on `config_json.oauth.expires_at` or implement a token refresh mutex/semaphore

2. **History API Gaps:**
   - Gmail only retains history for ~7 days
   - If `historyId` is too old, API returns 404
   - Mitigation: Implement fallback to search-based sync (using `after:` date query)

3. **Pub/Sub Delivery Guarantees:**
   - Pub/Sub guarantees at-least-once delivery (duplicates possible)
   - Mitigation: Idempotency keys on jobs prevent duplicate processing

4. **Rate Limiting:**
   - Gmail API has quota limits (250 units/user/second, daily quotas)
   - Mitigation: Implement exponential backoff on 429 errors; batch message fetches where possible

5. **Large Mailboxes:**
   - Backfill of large accounts could create thousands of jobs
   - Mitigation: Batch processing with pagination; consider lower priority for backfill jobs

#### Dependencies

- **Prerequisite:** Plan 2 (Milestone 1) provides the job queue infrastructure
- **External:** Google Cloud Project with Gmail API enabled, Pub/Sub topic configured
- **OAuth Consent Screen:** Must be configured in Google Cloud Console

#### Security Considerations

1. **Token Storage:** OAuth tokens grant full access to user's Gmail. Consider:
   - Database encryption at rest
   - Limiting token scope to minimum needed
   - Clear documentation about token location

2. **Webhook Validation:** Pub/Sub webhooks should validate:
   - Message signature (using Google's public keys)
   - Subscription matches expected value
   - Email address matches known account

3. **Input Validation:** All Gmail API data should be treated as untrusted:
   - Sanitize email content before storage
   - Validate message IDs match expected format

### Expected Behavior/Outcome

**User-Facing Behavior:**
- After configuring a Gmail account with OAuth, the system automatically receives and stores new emails
- Historical emails from the past N days are backfilled on account setup
- Email content (sender, subject, body, labels) is accessible in the database
- System recovers gracefully from temporary Gmail API failures or gaps

**States:**
- **Account:** inactive (no tokens) → active (valid tokens) → needs_reauth (refresh failed)
- **Sync:** idle → syncing → caught_up
- **Message:** pending_ingest → ingested → classification_queued

### Acceptance Criteria

- [ ] Gmail OAuth2 flow completes successfully and stores tokens
- [ ] Tokens are automatically refreshed before expiration
- [ ] Pub/Sub webhook receives notifications and enqueues `ingest.gmail` jobs
- [ ] `ingest.gmail` job fetches message from Gmail API and stores in database
- [ ] Thread records are created/updated with correct `last_message_at`
- [ ] Message content (from, to, subject, body_plain, body_html) is correctly parsed and stored
- [ ] `history.sync.gmail` job processes History API response and enqueues message ingests
- [ ] History gap (404) triggers fallback sync mechanism
- [ ] `backfill.gmail` job imports historical messages with pagination
- [ ] All jobs use appropriate idempotency keys to prevent duplicate processing
- [ ] Rate limit errors (429) are handled with exponential backoff
- [ ] All new code paths are covered by tests

### Dependencies & Constraints

**Dependencies:**
- Plan 2 (Milestone 1): Job queue, database, migrations, HTTP server
- Google Cloud Project with Gmail API enabled
- Pub/Sub topic and push subscription configured
- OAuth consent screen approved (or in testing mode)

**Technical Constraints:**
- Gmail History API retains data for ~7 days only
- Gmail API quotas must be respected
- Single-user assumption (no multi-tenant token isolation needed)
- SQLite/libsql database (no native JSON operators, text-based timestamps)

### Implementation Notes

**Recommended Approach:**

1. **Start with Gmail API Client Module:**
   - Create `server/crates/ashford-core/src/gmail/` module
   - Implement OAuth2 token management first (required for all other operations)
   - Use `reqwest` for HTTP calls to Gmail REST API

2. **Add Account Management Repository:**
   - CRUD operations for accounts table
   - Token refresh logic with optimistic locking
   - State management (historyId tracking)

3. **Implement Pub/Sub Webhook:**
   - Simple endpoint that decodes and enqueues
   - Add to AppState for access to queue and database

4. **Build Job Handlers Incrementally:**
   - Start with `ingest.gmail` (single message fetch/store)
   - Add `history.sync.gmail` (loops and enqueues ingest jobs)
   - Add `backfill.gmail` (search-based bulk import)

5. **Create Job Dispatcher:**
   - Replace `NoopExecutor` with routing by `job.job_type`
   - Each job type gets its own handler module

**Potential Gotchas:**

1. **Gmail Message Format:** The `payload.parts` structure is recursive for nested multipart messages. Need to handle arbitrary depth.

2. **Timezone Handling:** Gmail returns timestamps in various formats. Normalize to UTC RFC3339 consistently.

3. **Thread ID Timing:** A message's `threadId` may reference a thread not yet in the database. Need to handle thread creation during message ingest.

4. **Label Changes:** History API includes label changes, but `ingest.gmail` only needs `messagesAdded`. Filter appropriately.

5. **Empty Bodies:** Some messages (especially calendar invites) may have no plain text or HTML body. Handle gracefully.

**Files to Create/Modify:**

**New Files:**
- `server/crates/ashford-core/src/gmail/mod.rs` - Gmail module root
- `server/crates/ashford-core/src/gmail/client.rs` - Gmail REST API client
- `server/crates/ashford-core/src/gmail/oauth.rs` - OAuth2 token management
- `server/crates/ashford-core/src/gmail/types.rs` - Gmail API response types
- `server/crates/ashford-core/src/gmail/parser.rs` - Email content parser
- `server/crates/ashford-core/src/accounts.rs` - Account repository
- `server/crates/ashford-core/src/threads.rs` - Thread repository
- `server/crates/ashford-core/src/messages.rs` - Message repository
- `server/crates/ashford-core/src/jobs/mod.rs` - Job dispatcher
- `server/crates/ashford-core/src/jobs/ingest_gmail.rs` - Ingest handler
- `server/crates/ashford-core/src/jobs/history_sync_gmail.rs` - History sync handler
- `server/crates/ashford-core/src/jobs/backfill_gmail.rs` - Backfill handler

**Modified Files:**
- `server/crates/ashford-core/src/lib.rs` - Export new modules
- `server/crates/ashford-core/src/config.rs` - Extend GmailConfig with OAuth fields
- `server/crates/ashford-core/Cargo.toml` - Add reqwest, base64, mail-parser dependencies
- `server/crates/ashford-server/src/main.rs` - Add webhook route, wire job dispatcher
