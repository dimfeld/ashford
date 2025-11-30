---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: History Sync & Backfill
goal: Complete the backfill.gmail job handler implementation (history sync is
  already complete)
id: 12
uuid: 58b733a3-2f88-4b04-890f-23d394b9550b
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies:
  - 11
parent: 3
references: {}
issue: []
pullRequest: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
  - docs/job_queue.md
planGeneratedAt: 2025-11-30T00:50:01.839Z
promptsGeneratedAt: 2025-11-30T00:50:01.839Z
createdAt: 2025-11-29T07:42:47.306Z
updatedAt: 2025-11-30T01:06:40.630Z
progressNotes:
  - timestamp: 2025-11-30T01:01:04.437Z
    text: "Completed all backfill.gmail implementation tasks: added get_profile() to
      Gmail client, extended BackfillPayload with page_token, implemented full
      handler logic with pagination, and wrote 12 comprehensive tests. All 103
      tests pass."
    source: "implementer: Tasks 1-4"
tasks:
  - title: Add get_profile method to Gmail client
    done: true
    description: Add a `get_profile()` method to GmailClient in `gmail/client.rs`
      that calls `GET /users/{userId}/profile`. Parse the response to extract
      `historyId` which is needed to resume normal sync after backfill. Follow
      the existing pattern using `send_json()` helper.
  - title: Add page_token to backfill payload
    done: true
    description: "Extend `BackfillPayload` struct in `backfill_gmail.rs` to include
      an optional `page_token: Option<String>` field. This enables pagination by
      self-enqueueing with the next page token rather than processing all pages
      in a single job."
  - title: Implement backfill.gmail job handler
    done: true
    description: "Replace the stub implementation with full backfill logic: 1)
      Refresh account tokens, 2) Create Gmail client, 3) Call `list_messages()`
      with query and page_token, 4) Enqueue `ingest.gmail` job for each message
      ID with idempotency key `ingest.gmail:{account_id}:{message_id}`, 5) If
      `next_page_token` exists, enqueue next backfill job with same query and
      new page_token at priority -10, 6) If final page, call `get_profile()` to
      get fresh historyId, update account state to `sync_status=Normal` with new
      historyId and `last_sync_at`."
  - title: Write backfill job tests
    done: true
    description: "Add comprehensive tests: 1) Basic flow - list messages, enqueue
      ingest jobs, update state, 2) Pagination - verify next backfill job
      enqueued with page_token, 3) Empty results - handle gracefully, 4) Error
      handling - 429/403 return Retryable, 404 returns Fatal, 5) Idempotency -
      duplicate message IDs don't create duplicate ingest jobs, 6) Profile fetch
      - verify historyId updated on final page. Use wiremock for mocking Gmail
      API responses."
changedFiles:
  - .rmfilter/config/rmplan.yml
  - server/Cargo.lock
  - server/crates/ashford-core/Cargo.toml
  - server/crates/ashford-core/src/accounts.rs
  - server/crates/ashford-core/src/gmail/client.rs
  - server/crates/ashford-core/src/gmail/mod.rs
  - server/crates/ashford-core/src/gmail/parser.rs
  - server/crates/ashford-core/src/gmail/types.rs
  - server/crates/ashford-core/src/jobs/backfill_gmail.rs
  - server/crates/ashford-core/src/jobs/history_sync_gmail.rs
  - server/crates/ashford-core/src/jobs/ingest_gmail.rs
  - server/crates/ashford-core/src/jobs/mod.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/messages.rs
  - server/crates/ashford-core/src/migrations.rs
  - server/crates/ashford-core/src/pubsub.rs
  - server/crates/ashford-core/src/pubsub_listener.rs
  - server/crates/ashford-core/src/queue.rs
  - server/crates/ashford-core/src/threads.rs
  - server/crates/ashford-core/src/worker.rs
  - server/crates/ashford-core/tests/ingest_flow.rs
  - server/crates/ashford-server/Cargo.toml
  - server/crates/ashford-server/src/main.rs
  - server/migrations/003_add_thread_message_unique_indices.sql
tags:
  - gmail
  - rust
---

This plan implements catchup and historical sync:

- history.sync.gmail job handler using Gmail History API
- Detection and handling of history gaps (404 when historyId too old)
- Fallback to search-based sync when history unavailable
- backfill.gmail job for initial account setup (last N days)
- Pagination handling for large result sets
- Lower priority for backfill jobs to not block real-time ingestion

Depends on: Message Ingestion plan (uses ingest.gmail job)

<!-- rmplan-generated-start -->
## Expected Behavior/Outcome

When a Gmail account's historyId becomes stale (404 from History API), the system automatically:
1. Triggers a `backfill.gmail` job with a search query based on last sync time
2. Backfill job lists all messages matching the query (e.g., `newer_than:7d`)
3. Enqueues `ingest.gmail` jobs for each message at high priority (1)
4. Handles pagination by self-enqueueing next page at low priority (-10)
5. After final page, fetches fresh historyId from Profile API
6. Updates account state to `sync_status=Normal` with new historyId
7. Normal history sync resumes with the new historyId

**States:**
- `SyncStatus::Normal` - History sync working, no backfill needed
- `SyncStatus::NeedsBackfill` - History API returned 404, backfill triggered
- `SyncStatus::Backfilling` - (Optional) Could be set while backfill is running

## Key Findings

**Product & User Story:**
When a user hasn't synced their Gmail for a while (historyId expired, typically >7 days), the system should automatically backfill recent messages without manual intervention. The backfill runs at lower priority to not block real-time message ingestion.

**Design & UX Approach:**
Fully automated - no user interaction required. Background job handles recovery transparently.

**Technical Plan & Risks:**
- Most infrastructure already exists (history sync, ingest job, job queue)
- Need to add `get_profile()` to Gmail client for historyId retrieval
- Pagination handled via self-enqueueing to avoid job timeouts
- Race condition mitigated by idempotency keys on ingest jobs

**Pragmatic Effort Estimate:**
Small - ~50-100 lines of implementation code plus tests. Most patterns already established.

## Acceptance Criteria

- [x] history.sync.gmail job calls History API with correct startHistoryId (DONE)
- [x] All messagesAdded events result in ingest.gmail jobs being enqueued (DONE)
- [x] History sync pagination is handled correctly (DONE)
- [x] Account historyId is updated after successful history sync (DONE)
- [x] History gap (404) triggers backfill.gmail fallback (DONE)
- [ ] Gmail client has get_profile() method returning historyId
- [ ] backfill.gmail job lists messages using search query
- [ ] Backfill enqueues ingest.gmail job for each message
- [ ] Backfill pagination creates follow-up jobs for next pages
- [ ] Backfill jobs use priority -10 (lower than real-time)
- [ ] Final backfill page fetches fresh historyId and updates account state
- [ ] All new code paths are covered by tests

## Dependencies & Constraints

**Dependencies:**
- Plan 11 (Message Ingestion) - COMPLETE - `ingest.gmail` handler fully functional
- Gmail client with `list_messages()` - COMPLETE
- Job queue with priority support - COMPLETE

**Technical Constraints:**
- Must handle potentially large result sets (use pagination with self-enqueue)
- Backfill jobs must not block real-time ingestion (use lower priority)
- Must recover gracefully if interrupted (idempotency keys)

## Implementation Notes

**Recommended Approach:**
1. Add `get_profile()` to Gmail client first (simple addition)
2. Extend `BackfillPayload` to include optional `page_token`
3. Implement handler following the TODO comments already in the stub
4. Follow patterns from `history_sync_gmail.rs` for error handling and ingest enqueueing
5. Add tests using established wiremock patterns

**Potential Gotchas:**
- The `list_messages()` API returns only message IDs (not full messages), so each still needs an ingest job
- Gmail's search queries have a max of 500 results per page - need pagination
- The `historyId` from profile may be different from messages' internal dates - this is expected
- Don't forget to set `sync_status` back to `Normal` after backfill completes

**Files to Modify:**
- `server/crates/ashford-core/src/gmail/client.rs` - Add get_profile()
- `server/crates/ashford-core/src/jobs/backfill_gmail.rs` - Full implementation
<!-- rmplan-generated-end -->

## Research

### Summary

After thorough codebase exploration, I discovered that **most of this plan has already been implemented**. The `history.sync.gmail` job handler is fully functional with pagination, backfill triggering on 404 errors, and proper state management. The Gmail client already has both `list_history` and `list_messages` methods implemented. The only remaining work is implementing the actual backfill logic in the `backfill.gmail` handler, which currently exists as a stub.

Key discoveries:
- `history.sync.gmail` is **fully implemented** with pagination, idempotent ingest job enqueueing, and account state updates
- `backfill.gmail` handler exists as a **stub** with TODO comments - this is the only significant implementation work remaining
- Gmail client methods `list_history` and `list_messages` are **already complete**
- Job dispatcher is already configured to route both job types
- Comprehensive test coverage exists for history sync; backfill tests are minimal stubs

### Findings

#### Gmail Client (Already Implemented)
**File:** `server/crates/ashford-core/src/gmail/client.rs`

Both required methods are fully implemented:

**`list_history()`** (lines 90-111):
- Accepts `start_history_id`, optional `page_token`, and optional `max_results`
- Returns `ListHistoryResponse` with `history` records, `next_page_token`, and `history_id`
- Supports full pagination via `nextPageToken`

**`list_messages()`** (lines 113-138):
- Accepts optional `query` (Gmail search syntax like `newer_than:7d`), `page_token`, `include_spam_trash`, and `max_results`
- Returns `ListMessagesResponse` with message ID stubs, `next_page_token`, and `result_size_estimate`
- Perfect for backfill search queries

Both methods follow the established pattern:
- Use `send_json()` helper with closure-based request building
- Automatic token refresh on 401
- Proper error propagation via `GmailClientError`

#### Job System Architecture
**File:** `server/crates/ashford-core/src/jobs/mod.rs`

The job dispatcher is already configured (lines 44-54):
```rust
impl JobExecutor for JobDispatcher {
    async fn execute(&self, job: Job, _ctx: JobContext) -> Result<(), JobError> {
        match job.job_type.as_str() {
            JOB_TYPE_BACKFILL_GMAIL => handle_backfill_gmail(self, job).await,
            JOB_TYPE_INGEST_GMAIL => handle_ingest_gmail(self, job).await,
            JOB_TYPE_HISTORY_SYNC_GMAIL => handle_history_sync_gmail(self, job).await,
            other => Err(JobError::Fatal(format!("unknown job type: {other}"))),
        }
    }
}
```

**Priority conventions established:**
- High priority (1): `ingest.gmail` jobs enqueued by history sync (line 130 of history_sync_gmail.rs)
- Low priority (-10): `backfill.gmail` jobs (line 185 of history_sync_gmail.rs)

**Error mapping utilities** (`map_gmail_error`, `map_account_error`) are shared across handlers.

#### History Sync Job (Fully Implemented)
**File:** `server/crates/ashford-core/src/jobs/history_sync_gmail.rs`

Complete implementation with:
- **Pagination loop** (lines 62-97): Processes all pages, tracking `latest_history_id`
- **Idempotent ingest enqueueing** (lines 118-142): Uses idempotency key `ingest.gmail:{account_id}:{message_id}`
- **404 handling for stale historyId** (lines 69-76): Triggers backfill via `trigger_backfill()`
- **Account state update** (lines 99-106): Updates `history_id` and `last_sync_at` after all pages processed
- **Backfill triggering** (lines 152-198):
  - Sets `sync_status = SyncStatus::NeedsBackfill`
  - Clears stale `history_id`
  - Calculates query based on `last_sync_at` (capped at 30 days)
  - Enqueues backfill job at priority -10

**Test coverage** (lines 200-618):
- `history_sync_enqueues_ingest_jobs_and_updates_state`
- `history_sync_retries_on_rate_limit`
- `history_sync_triggers_backfill_on_not_found`
- `history_sync_prefers_account_state_history_id`
- `history_sync_is_idempotent_for_ingest_jobs`
- `history_sync_handles_pagination`
- `history_sync_retries_on_403_rate_limit`

#### Backfill Job (Stub - Needs Implementation)
**File:** `server/crates/ashford-core/src/jobs/backfill_gmail.rs`

Current implementation (lines 17-33) is just a stub:
```rust
pub async fn handle_backfill_gmail(_dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    let payload: BackfillPayload = serde_json::from_value(job.payload.clone())?;

    // TODO: Implement actual backfill logic
    // 1. List messages using query (with pagination)
    // 2. Enqueue ingest.gmail jobs for each message
    // 3. After final page, get fresh historyId from profile
    // 4. Update account state to Normal with new historyId
    warn!("backfill.gmail handler not yet implemented - job completing without action");
    Ok(())
}
```

**Payload structure** (lines 11-15):
```rust
struct BackfillPayload {
    account_id: String,
    query: String,
}
```

**Implementation requirements** based on TODOs and established patterns:
1. Call `client.list_messages(Some(&payload.query), page_token, false, Some(max_results))`
2. For each message ID in response, enqueue `ingest.gmail` job with idempotency key
3. Handle pagination:
   - Option A: Loop within job (like history sync)
   - Option B: Enqueue next backfill job with `page_token` in payload (for very large backfills)
4. After final page: need to get fresh historyId (requires adding `get_profile()` to Gmail client)
5. Update account state: `sync_status = SyncStatus::Normal`, set new `history_id`

#### Account State Management
**File:** `server/crates/ashford-core/src/accounts.rs`

**SyncStatus enum** (lines 44-54):
```rust
pub enum SyncStatus {
    Normal,       // Default - history sync working
    NeedsBackfill, // History ID stale, backfill needed
    Backfilling,  // Backfill in progress (not currently used)
}
```

**AccountState structure** (lines 56-62):
```rust
pub struct AccountState {
    pub history_id: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_status: SyncStatus,
}
```

**State update method** `update_state()` (lines 182-206): Simple UPDATE, no optimistic locking needed.

#### Ingest Job (Reference Implementation)
**File:** `server/crates/ashford-core/src/jobs/ingest_gmail.rs`

Complete implementation showing:
- How to create Gmail client with token refresh
- Message fetching and parsing
- Thread and message upsert
- Error handling patterns

#### Testing Patterns
Tests use `wiremock` for HTTP mocking with:
- `MockServer::start().await` for isolated mock servers
- Custom `Respond` implementations for stateful responses (pagination)
- `.expect(n)` for verifying call counts
- `tempfile::TempDir` for isolated test databases

Integration tests in `tests/ingest_flow.rs` show full worker-based testing patterns.

### Risks & Constraints

1. **Gmail Profile API for historyId**: The backfill implementation needs to fetch a fresh `historyId` after completion to resume normal sync. This requires adding a `get_profile()` method to the Gmail client that calls `GET /users/{userId}/profile` and extracts the `historyId` field.

2. **Pagination strategy**: For very large backfills (thousands of messages), processing all pages in a single job could:
   - Take too long and time out
   - Create many ingest jobs that overwhelm the queue

   Consider either:
   - Self-enqueueing next page as a new backfill job (with page_token in payload)
   - Using smaller batches with rate limiting between pages

3. **Race condition on account state**: If history sync runs while backfill is in progress, they could both try to update account state. The `trigger_backfill` function already clears `history_id` when starting backfill, which should cause history sync to trigger another backfill if it runs concurrently (idempotent via idempotency key).

4. **Test isolation**: Existing tests create unique database files per test to avoid conflicts. New backfill tests should follow this pattern.

5. **Dependencies**: This plan depends on Plan 11 (Message Ingestion) being complete, which it is - the `ingest.gmail` handler and job dispatcher are fully functional.

Completed all 4 tasks for the backfill.gmail job handler implementation:

**Task 1: Added get_profile() method to Gmail client**
- File: server/crates/ashford-core/src/gmail/client.rs (lines 140-144)
- Added Profile struct in gmail/types.rs (lines 114-125) with email_address, messages_total, threads_total, and history_id fields
- Uses existing send_json() helper for consistency with other Gmail client methods

**Task 2: Extended BackfillPayload with page_token**
- File: server/crates/ashford-core/src/jobs/backfill_gmail.rs (lines 22-28)
- Added optional page_token field with serde(default) for backwards compatibility
- Enables self-enqueueing pagination pattern for large result sets

**Task 3: Implemented full backfill.gmail job handler**
- File: server/crates/ashford-core/src/jobs/backfill_gmail.rs (lines 30-187)
- Full implementation replaces previous stub that just logged a warning
- Key flow: refresh tokens -> create client -> list messages -> enqueue ingest jobs -> handle pagination or finalize
- Uses BACKFILL_PRIORITY=-10 for continuation jobs and INGEST_PRIORITY=1 for message ingest jobs
- Idempotency keys follow format ingest.gmail:{account_id}:{message_id}
- On final page, calls get_profile() to fetch fresh historyId and updates account state to SyncStatus::Normal

**Task 4: Added comprehensive test coverage**
- 13 tests total covering all required scenarios:
  - Basic flow: list messages, enqueue ingest jobs, update state
  - Pagination: verifies next backfill job enqueued with page_token
  - Empty results: handles gracefully without errors
  - Error handling: 429/403 return Retryable, 404 returns Fatal
  - Idempotency: duplicate message IDs don't create duplicate ingest jobs
  - Profile fetch: verifies historyId updated on final page
  - Added test for get_profile failure after reviewer feedback

**Design Decisions:**
- Used self-enqueueing pagination pattern (new job per page) rather than loop within single job to avoid job timeouts on large backfills
- Ingest jobs enqueued at high priority (1) to process messages quickly
- Backfill continuation jobs enqueued at low priority (-10) to not block real-time ingestion
- Pattern follows history_sync_gmail.rs for consistency with error mapping via map_gmail_error and map_account_error

All 117 tests pass. The parent plan 'Milestone 2: Gmail Ingest (Pub/Sub + History)' has been automatically marked as complete since all children plans are now done.
