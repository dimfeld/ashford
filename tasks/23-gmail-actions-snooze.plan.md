---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Actions: Snooze"
goal: Implement snooze action with configurable label and scheduled restore job
id: 23
uuid: 7300ce7b-c38b-4fe6-ae96-ead50a3f1f05
generatedBy: agent
status: done
priority: medium
container: false
temp: false
dependencies:
  - 22
parent: 5
references:
  "5": 66785b19-e85d-4135-bbca-9d061a0394c7
  "22": c69a5bba-4a08-4a49-b841-03d396a6ba81
issue: []
pullRequest: []
docs:
  - docs/gmail_integration.md
planGeneratedAt: 2025-12-03T02:21:58.926Z
promptsGeneratedAt: 2025-12-03T02:21:58.926Z
createdAt: 2025-12-03T02:21:14.729Z
updatedAt: 2025-12-04T18:45:46.341Z
progressNotes:
  - timestamp: 2025-12-04T10:37:17.332Z
    text: Implemented snooze label config, enqueue_scheduled, Gmail create_label,
      snooze/unsnooze handlers; all tests passing.
    source: "implementer: snooze"
  - timestamp: 2025-12-04T10:39:09.095Z
    text: Ran cargo test -p ashford-core; all existing tests pass.
    source: "tester: snooze"
  - timestamp: 2025-12-04T10:40:41.016Z
    text: Added negative/invalid snooze parameter tests in action_gmail.rs and
      re-ran cargo test -p ashford-core (all 526 tests now passing).
    source: "tester: snooze"
  - timestamp: 2025-12-04T18:11:12.157Z
    text: Completed comprehensive code review of Gmail snooze implementation. All
      snooze-related tests pass. Found one minor issue with test structure and
      documentation reference. Implementation properly handles edge cases
      including 404 responses, label conflicts, and config changes. The
      snooze_label_id is correctly persisted in unsnooze job payloads per the
      fix in fc3ceab.
    source: "reviewer: full implementation review"
  - timestamp: 2025-12-04T18:11:20.211Z
    text: "Code review completed. Found issues: snooze label existence relies solely
      on local cache; if Gmail label deleted externally snooze/unsnooze will
      fail instead of recreating or proceeding, violating plan edge-case.
      Unsnooze job fails fatally when local message record missing, so
      deleted-while-snoozed case not tolerated. Undo hint for snooze stores
      inverse_action apply_label but inverse_parameters expect add/remove
      arrays, likely breaking undo executor."
    source: "reviewer: Gmail snooze"
  - timestamp: 2025-12-04T18:34:26.552Z
    text: "Addressed review issues: ensure snooze label recreates if removed,
      unsnooze skips missing local message, undo hint uses consistent
      inverse_action; added new tests and updated mocks; formatting done."
    source: "implementer: Gmail Actions: Snooze"
  - timestamp: 2025-12-04T18:39:09.847Z
    text: Ran cargo test -p ashford-core from /server; all 528 unit tests plus
      integration suites passed.
    source: "tester: plan23"
tasks:
  - title: Add snooze configuration
    done: true
    description: "Add snooze_label field to config (default: 'Ashford/Snoozed'). Add
      to Config struct in config.rs. Ensure label is created on first use if it
      doesn't exist."
  - title: Create unsnooze job type
    done: true
    description: "Create new job type 'unsnooze.gmail' with handler in
      jobs/unsnooze_gmail.rs. Payload: {account_id, message_id, action_id}.
      Export JOB_TYPE_UNSNOOZE_GMAIL constant."
  - title: Implement snooze action in action_gmail
    done: true
    description: "Implement execute_snooze in action_gmail.rs: (1) Parse snooze time
      from parameters - support {until: ISO8601} or {amount, units} format,
      convert to DateTime<Utc>. (2) Ensure snooze label exists via helper (check
      local DB first, create via API if missing, upsert to DB). (3) Remove INBOX
      label, add snooze label via modify_message. (4) Enqueue unsnooze.gmail job
      with not_before=snooze_until. (5) Build undo_hint with pre-image,
      snooze_until, and unsnooze job ID."
  - title: Implement unsnooze job handler
    done: true
    description: "Implement handle_unsnooze_gmail: add INBOX label, remove snooze
      label (preserve other labels). Handle edge cases gracefully: message
      deleted (log and complete), label already removed (proceed with adding
      INBOX). No need to update original snooze action - unsnooze is a
      consequence, not an undo."
  - title: Add snooze parameter validation
    done: true
    description: "Validate snooze parameters: support two formats - {until: ISO8601}
      or {amount: number, units: 'minutes'|'hours'|'days'}. Validate: exactly
      one format provided, computed time is in the future, max duration 1 year.
      Return clear InvalidParameter errors for validation failures."
  - title: Add tests for snooze functionality
    done: true
    description: "Add unit tests for snooze action and unsnooze job handler. Test:
      successful snooze/unsnooze cycle, edge cases (message deleted while
      snoozed, invalid duration), job scheduling with correct not_before."
  - title: Add create_label method to GmailClient
    done: true
    description: 'Add create_label(name: &str) method to GmailClient that calls POST
      /users/{userId}/labels with body {"name": "label_name"}. Returns the
      created Label struct. Handle conflict case (label already exists) - could
      return existing label or specific error.'
  - title: Add enqueue_scheduled method to JobQueue
    done: true
    description: "Add enqueue_scheduled(job_type, payload, idempotency_key,
      priority, not_before: DateTime<Utc>) method to JobQueue. Similar to
      enqueue() but sets the not_before field to schedule delayed execution.
      Used by snooze action to schedule unsnooze jobs."
changedFiles:
  - server/crates/ashford-core/src/config.rs
  - server/crates/ashford-core/src/gmail/client.rs
  - server/crates/ashford-core/src/jobs/action_gmail.rs
  - server/crates/ashford-core/src/jobs/mod.rs
  - server/crates/ashford-core/src/jobs/unsnooze_gmail.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/queue.rs
  - server/crates/ashford-server/src/main.rs
tags:
  - actions
  - gmail
  - rust
---

Implement Gmail snooze functionality:

## Scope
- Add snooze label configuration (configurable with default "Ashford/Snoozed")
- Implement snooze action: archive message + apply snooze label
- Create unsnooze job type for scheduled restoration
- Unsnooze behavior: add INBOX label, remove snooze label, preserve other labels

## Out of Scope
- Snooze management UI (future milestone)

## Design Notes
- Gmail doesn't have native snooze, so we implement via labels + scheduled jobs
- Snooze parameters should include: duration or target datetime
- Need to handle edge cases: message deleted while snoozed, label removed externally

## Research

### Summary
- Gmail has no native snooze API, so we implement it via labels (archive + apply snooze label) and scheduled jobs (unsnooze at target time).
- The action system (plan 22) provides all necessary infrastructure: pre-image capture, undo hints, Gmail client methods, and action dispatch patterns.
- The job queue already supports delayed execution via the `not_before` field, but needs a new `enqueue_scheduled()` method for convenient scheduling.
- GmailClient currently lacks a `create_label()` method, which is needed to ensure the snooze label exists on first use.
- Configuration system has existing patterns for configurable labels (IMAP config has `snooze_folder`), but no Gmail-specific label config yet.

### Findings

#### Gmail Action System (jobs/action_gmail.rs)

The action system from plan 22 provides a solid foundation:

**PreImageState struct** captures message state before mutation:
```rust
pub struct PreImageState {
    pub labels: Vec<String>,
    pub is_unread: bool,
    pub is_starred: bool,
    pub is_in_inbox: bool,
    pub is_in_trash: bool,
}
```

**Action execution pattern** - Each action type has an `execute_*` function:
```rust
async fn execute_snooze(
    gmail_client: &GmailClient<NoopTokenStore>,
    provider_message_id: &str,
    pre_image: &PreImageState,
    parameters: &Value,
) -> Result<ActionExecutionResult, GmailClientError>
```

**Existing actions**: archive, apply_label, remove_label, mark_read, mark_unread, star, unstar, trash, delete, restore

**ActionType enum** already includes `Snooze` variant (llm/decision.rs line 27), classified as "Reversible" in `danger_level()`.

**Key files**:
- `server/crates/ashford-core/src/jobs/action_gmail.rs` - Action execution handlers
- `server/crates/ashford-core/src/llm/decision.rs` - ActionType enum
- `server/crates/ashford-core/src/gmail/client.rs` - Gmail API client
- `server/crates/ashford-core/src/decisions/repositories.rs` - ActionRepository

#### Job Queue System (queue.rs)

**Job schema** (migrations/001_initial.sql):
```sql
CREATE TABLE jobs (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  state TEXT NOT NULL CHECK (state IN ('queued','running','completed','failed','canceled')),
  attempts INTEGER NOT NULL DEFAULT 0,
  max_attempts INTEGER NOT NULL DEFAULT 5,
  not_before TEXT,  -- KEY: RFC3339 timestamp for delayed execution
  idempotency_key TEXT,
  ...
);
```

**Job claiming logic** (queue.rs lines 145-170):
```sql
SELECT id FROM jobs
WHERE state = 'queued' AND (not_before IS NULL OR not_before <= ?1)
ORDER BY priority DESC, created_at
LIMIT 1
```

**Gap identified**: The current `enqueue()` method sets `not_before` to NULL. Need to add `enqueue_scheduled()`:
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

**Job type constants** (jobs/mod.rs):
```rust
pub const JOB_TYPE_ACTION_GMAIL: &str = "action.gmail";
pub const JOB_TYPE_CLASSIFY: &str = "classify";
// Add: pub const JOB_TYPE_UNSNOOZE_GMAIL: &str = "unsnooze.gmail";
```

**JobDispatcher** routes jobs to handlers via match in `JobExecutor::execute()`.

#### Configuration System (config.rs)

**Config structure**:
```rust
pub struct Config {
    pub app: AppConfig,
    pub gmail: GmailConfig,
    pub imap: ImapConfig,  // Has snooze_folder field
    pub policy: PolicyConfig,
    // ...
}
```

**GmailConfig** (lines 59-63):
```rust
pub struct GmailConfig {
    pub use_pubsub: bool,
    pub project_id: String,
    pub subscription: String,
    // No snooze_label field yet
}
```

**ImapConfig** has `snooze_folder: String` with default "Snoozed". Consider either:
1. Add `snooze_label` to GmailConfig (recommended for clarity)
2. Reuse `imap.snooze_folder` for Gmail labels (confusing)

**Environment override pattern** in `apply_env_overrides()` and `resolve_env_markers()`.

#### Gmail Label Management (labels.rs)

**Labels table schema** (migrations/006_add_labels_table.sql):
```sql
CREATE TABLE labels (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  provider_label_id TEXT NOT NULL,
  name TEXT NOT NULL,
  label_type TEXT NOT NULL CHECK (label_type IN ('system', 'user')),
  ...
);
```

**LabelRepository methods**:
- `upsert()` - Insert or update preserving user-editable fields
- `get_by_account()` - All labels for account
- `get_by_provider_id()` - Lookup by Gmail label ID
- `get_by_name()` - Case-insensitive name lookup

**Missing**: GmailClient has no `create_label()` method. Need to add:
```rust
pub async fn create_label(&self, name: &str) -> Result<Label, GmailClientError>
```

Gmail API: `POST /users/{userId}/labels` with body `{"name": "Ashford/Snoozed"}`

**Label sync job** (jobs/labels_sync_gmail.rs) syncs labels from Gmail but doesn't create them.

#### Testing Patterns (tests/)

**Integration test structure** (tests/action_gmail_flow.rs):
1. Setup database with TempDir
2. Create account, message, action records
3. Start MockServer (wiremock) for Gmail API
4. Create JobDispatcher with mocked API base
5. Spawn worker with fast config (5ms poll)
6. Enqueue job
7. Poll until completion with timeout
8. Assert status and undo_hint

**MockServer pattern**:
```rust
let server = MockServer::start().await;
Mock::given(method("POST"))
    .and(path(format!("/gmail/v1/users/{}/messages/{}/modify", email, msg_id)))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({...})))
    .expect(1)
    .mount(&server)
    .await;
```

**Fast worker config**:
```rust
WorkerConfig {
    poll_interval: Duration::from_millis(5),
    heartbeat_interval: Duration::from_millis(10),
    drain_timeout: Duration::from_secs(5),
}
```

**Mocking guidelines**: Per CLAUDE.md, avoid mocks except for external services. Gmail API calls are external, so wiremock is appropriate. Use real SQLite databases.

### Risks & Constraints

1. **No Gmail create_label method**: GmailClient currently cannot create labels. Must add this before snooze can work with custom labels. Could use a workaround where snooze fails if label doesn't exist, but this is poor UX.

2. **Scheduled job queue method missing**: Need to add `enqueue_scheduled()` to JobQueue for convenient delayed job creation. Alternative: direct SQL insert with not_before, but this bypasses proper encapsulation.

3. **Message deletion during snooze**: If user deletes message while snoozed, unsnooze job should handle gracefully (log and complete, don't fail). Gmail API returns 404.

4. **Label removed externally**: If someone removes the snooze label via Gmail UI, unsnooze should still try to add INBOX (will succeed) and remove snooze label (will silently succeed or be a no-op).

5. **Maximum snooze duration**: Consider enforcing a reasonable max (e.g., 1 year) to prevent accidental far-future snoozes.

6. **Timezone handling**: Snooze times should be stored as UTC DateTime. Client must send UTC or include timezone. Parse with chrono's DateTime<Utc>.

7. **Snooze label naming**: Default "Ashford/Snoozed" uses "/" as Gmail's hierarchy separator. Ensure parent "Ashford" label exists or will be auto-created.

8. **Cancellation**: If user wants to unsnooze early, need a way to cancel the pending unsnooze job. Could look up job by idempotency key and set state to 'canceled'. Consider as future enhancement.

9. **Undo hint structure**: Snooze undo_hint should store snooze_until and original labels so a manual "undo snooze" could cancel the job and restore inbox immediately.

10. **Retry behavior**: If unsnooze fails due to rate limiting, it will retry with backoff. This is fine - message just stays snoozed a bit longer.

## Expected Behavior/Outcome

After implementation:

1. **Snooze action execution**:
   - User (via LLM or rules) triggers snooze with `snooze_until` parameter
   - Message is archived (INBOX removed) and snooze label applied
   - Unsnooze job is scheduled for `snooze_until` time
   - Action marked completed with undo_hint containing original state

2. **Unsnooze job execution**:
   - At scheduled time, job claims and runs
   - INBOX label added, snooze label removed
   - Other labels preserved
   - Original action's undo_hint optionally updated to mark snooze completed

3. **Edge case handling**:
   - Message deleted while snoozed → unsnooze logs and completes successfully
   - Snooze label already removed → unsnooze adds INBOX and completes
   - Rate limited → retries with exponential backoff

4. **Configuration**:
   - `gmail.snooze_label` setting with default "Ashford/Snoozed"
   - Label auto-created on first snooze if it doesn't exist

## Acceptance Criteria

### Functional Criteria
- [ ] Snooze action removes INBOX label and adds configured snooze label
- [ ] Snooze action schedules unsnooze.gmail job with not_before = snooze_until
- [ ] Unsnooze job adds INBOX label and removes snooze label
- [ ] Snooze label is auto-created if it doesn't exist in Gmail
- [ ] Snooze parameters validated (snooze_until must be in the future)
- [ ] Configuration supports customizable snooze label name

### Technical Criteria
- [ ] GmailClient has create_label() method
- [ ] JobQueue has enqueue_scheduled() method for delayed jobs
- [ ] Config struct includes gmail.snooze_label field with default
- [ ] Unsnooze job handles 404 (deleted message) gracefully
- [ ] All new code paths covered by tests with wiremock mocks

## Dependencies & Constraints

**Dependencies**:
- Plan 22 (Gmail Actions: Core Operations) - DONE - provides action execution infrastructure
- Existing labels table and LabelRepository
- Existing job queue infrastructure with not_before support

**Technical Constraints**:
- Must use existing action dispatch pattern from action_gmail.rs
- Must follow established testing patterns with wiremock
- Snooze label must be valid Gmail label (no special characters except /)

## Implementation Notes

### Recommended Approach

**Phase 1: Infrastructure**
1. Add `snooze_label` to GmailConfig with default "Ashford/Snoozed"
2. Add `create_label()` method to GmailClient
3. Add `enqueue_scheduled()` method to JobQueue

**Phase 2: Unsnooze Job**
1. Create `jobs/unsnooze_gmail.rs` with handler
2. Define JOB_TYPE_UNSNOOZE_GMAIL constant
3. Register in JobDispatcher match statement
4. Handle edge cases (deleted message, missing label)

**Phase 3: Snooze Action**
1. Implement `execute_snooze()` in action_gmail.rs
2. Add snooze to execute_action dispatcher
3. Ensure label exists (create if needed)
4. Enqueue unsnooze job with scheduled time
5. Build undo_hint with snooze metadata

**Phase 4: Testing**
1. Unit tests for snooze action execution
2. Unit tests for unsnooze job handler
3. Integration tests for full snooze/unsnooze cycle
4. Tests for edge cases and parameter validation

### Potential Gotchas

1. **Label resolution flow**: Use local DB cache with fallback to create:
   - First, check local labels table by name (LabelRepository::get_by_name)
   - If found, use cached provider_label_id
   - If not found, create via Gmail API, upsert to local DB, then use
   - This handles the case where label is deleted externally (next snooze recreates it)

2. **Idempotency key for unsnooze**: Use `unsnooze.gmail:{account_id}:{action_id}` to prevent duplicate unsnooze jobs for same snooze action.

3. **Parent label**: Gmail may auto-create parent labels (e.g., "Ashford" when creating "Ashford/Snoozed"), but verify this behavior.

4. **Snooze parameters structure**: The `parameters_json` should contain either `until` or `amount`+`units`, not both. Validate this in the action handler.

5. **Snooze time parameter structure**: Support both absolute and relative times with structured data:
   ```json
   // Absolute datetime
   { "until": "2024-12-05T10:00:00Z" }

   // Relative duration
   { "amount": 2, "units": "hours" }
   ```
   Valid units: `minutes`, `hours`, `days`. The action handler should convert relative durations to absolute UTC datetime before scheduling the unsnooze job.

Implemented Gmail snooze flow end-to-end (Tasks: Add snooze configuration; Create unsnooze job type; Implement snooze action in action_gmail; Implement unsnooze job handler; Add snooze parameter validation; Add tests for snooze functionality; Add create_label method to GmailClient; Add enqueue_scheduled method to JobQueue).

Key changes:
- Added gmail.snooze_label with default 'Ashford/Snoozed' (config.rs) and carried GmailConfig on JobDispatcher with a builder to inject real config from ashford-server main.
- GmailClient now supports create_label with conflict fallback via list_labels; added unit tests for success/409 paths.
- JobQueue gained enqueue_scheduled to set not_before timestamps for delayed jobs plus tests verifying stored values.
- New unsnooze.gmail job handler (jobs/unsnooze_gmail.rs) that adds INBOX and removes the snooze label, tolerating missing labels and 404 responses; registered new job type and re-exported constant.
- Snooze action implementation in jobs/action_gmail.rs: parameter validation for until/amount+units with 1-year cap, pre-image capture, ensure/create snooze label via Gmail + LabelRepository, apply archive+snooze label, enqueue scheduled unsnooze job with idempotency key (reusing existing job id on duplicate), and enriched undo_hint (snooze_until, snooze_label, unsnooze_job_id).
- JobDispatcher now holds GmailConfig so snooze label is configurable; ashford-server passes config.gmail on construction.

Tests:
- Expanded client, queue, snooze parsing, snooze scheduling, and unsnooze handler coverage plus full  after changes (all passing).

Design notes: unsnooze handler treats Gmail 404 as success to handle deleted messages; label lookup uses local cache first then creates remotely; undo_hint retains pre-action state plus scheduling metadata for future undo or management.

Addressed review fixes for snooze/unsnooze flow. Implemented payload persistence of the snooze label id so unsnooze jobs remove the exact label even if GmailConfig.snooze_label is renamed later; the unsnooze handler now prefers the job-provided label id with a fallback lookup by current config name (file: server/crates/ashford-core/src/jobs/unsnooze_gmail.rs, function handle_unsnooze_gmail). Updated the snooze scheduling path to carry snooze_label_id into the unsnooze job payload and enriched the snooze undo metadata to describe a full unsnooze: inverse_parameters now include add_labels:[INBOX], remove_labels:[snooze_label_id], and cancel_unsnooze_job_id alongside the existing snooze_label/unsnooze_job_id fields (file: server/crates/ashford-core/src/jobs/action_gmail.rs, execute_snooze). Strengthened tests to reflect the new contract: unsnooze tests now include snooze_label_id in payloads and still remove the label when the local DB copy is missing, and the snooze integration test asserts the expanded inverse_parameters and unsnooze job payload contents. Ran cargo test -p ashford-core from server/; all suites pass.

Implemented fixes for review items on Gmail snooze/unsnooze. Addressed task scope 'Gmail Actions: Snooze' by hardening snooze label handling, unsnooze edge cases, and undo metadata consistency. Updated files: server/crates/ashford-core/src/jobs/action_gmail.rs, server/crates/ashford-core/src/jobs/unsnooze_gmail.rs, server/crates/ashford-core/src/labels.rs, plus associated tests. Key changes: ensure_snooze_label now always verifies Gmail-side labels via list_labels, recreates missing labels, updates local cache, and deletes stale provider IDs when Gmail returns a different label ID; added helper to build NewLabel and new LabelRepository::delete_by_provider_id for cleanup. Snooze undo_hint now uses inverse_action 'none' with structured inverse_parameters and note to avoid mismatched apply_label contract. Unsnooze handler now tolerates missing local messages (logs and exits) and still prefers job-provided snooze_label_id with graceful 404 handling; added new test for missing local message. Added new snooze tests to cover label recreation when Gmail label is deleted externally and adjusted mocks to include label listing; snooze scheduling test updated for new undo semantics. Ran cargo fmt and cargo test -p ashford-core (all passing).

Updated unsnooze Gmail handler to tolerate missing snooze labels from Gmail and still add INBOX. In ,  now retries  without  when Gmail returns a 400 for the snooze label, so the job no longer fails if the label was deleted or renamed externally. Added new regression test  in the same file to cover this scenario by mocking a 400 response on the first attempt and verifying a second attempt without label removal succeeds. Relevant tasks: Gmail Actions: Snooze / unsnooze robustness. Rationale: meet edge-case requirement that unsnooze succeeds even when snooze label disappears, ensuring INBOX is applied and job remains non-fatal.
