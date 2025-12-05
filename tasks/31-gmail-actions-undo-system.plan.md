---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Actions: Undo System"
goal: Implement undo job handler that derives and executes inverse actions from
  undo_hint_json
id: 31
uuid: cc4bd313-ff47-4c58-87f2-30999a6058e2
generatedBy: agent
status: done
priority: medium
container: false
temp: false
dependencies: []
parent: 5
references:
  "5": 66785b19-e85d-4135-bbca-9d061a0394c7
issue: []
pullRequest: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
planGeneratedAt: 2025-12-05T09:23:43.074Z
promptsGeneratedAt: 2025-12-05T09:23:43.074Z
createdAt: 2025-12-04T20:29:59.381Z
updatedAt: 2025-12-06T01:49:18.039Z
progressNotes:
  - timestamp: 2025-12-05T09:36:58.589Z
    text: Implemented undo.action handler with validations, inverse execution,
      snooze special-case, and action_link creation. Added unit tests for
      archive, snooze, irreversible and already-undone cases; all tests passing.
    source: "implementer: undo.action"
  - timestamp: 2025-12-05T09:40:26.430Z
    text: "Added broad undo_action coverage: inverse label/read/star/trash paths,
      non-completed rejection, Gmail 404 handling; cargo test -p ashford-core
      undo_action passing."
    source: "tester: undo_action_tests"
  - timestamp: 2025-12-05T22:00:55.674Z
    text: "Reviewed undo.action implementation. Core handler logic is correct:
      validation, idempotency via job_id locking, snooze special-case, retry
      exhaustion handling. Tests cover all inverse action types but missing
      explicit test for snooze undo when job already ran (code handles it).
      Minor: inconsistent use of await in validate_action_undoable (currently
      async but no await). No critical issues."
    source: "reviewer: code review"
  - timestamp: 2025-12-06T01:49:18.034Z
    text: Ran cargo test -p ashford-core --lib from server; all 594 tests passed.
    source: "tester: Gmail Actions: Undo System"
tasks:
  - title: Create undo job type and handler skeleton
    done: true
    description: >-
      Create new file `server/crates/ashford-core/src/jobs/undo_action.rs` with:

      - `pub const JOB_TYPE: &str = "undo.action";`

      - `UndoPayload` struct with `account_id: String` and `original_action_id:
      String`

      - Skeleton `handle_undo_action()` function

      - Register in `mod.rs`: add module declaration, export
      `JOB_TYPE_UNDO_ACTION`, add dispatch case
  - title: Implement validation logic
    done: true
    description: >-
      Add validation functions to undo_action.rs:

      - `validate_action_undoable()`: Check action status is Completed,
      undo_hint_json has inverse_action != "none", irreversible flag is not true

      - `check_already_undone()`: Query
      ActionLinkRepository::get_by_effect_action_id(), check for UndoOf relation

      - Return appropriate JobError::Fatal messages for each validation failure
  - title: Implement inverse action execution
    done: true
    description: |-
      Add inverse action execution to handle_undo_action():
      - Parse inverse_action and inverse_parameters from undo_hint_json
      - Create Gmail client using `create_gmail_client()` helper
      - Execute based on inverse_action type:
        - apply_label/remove_label: `modify_message(add/remove labels)`
        - mark_read/mark_unread: `modify_message(UNREAD label)`
        - star/unstar: `modify_message(STARRED label)`
        - restore: `untrash_message()`
        - trash: `trash_message()`
      - Handle Gmail API errors with appropriate retryable/fatal classification
  - title: Implement snooze undo handling
    done: true
    description: >-
      Add special case for snooze undo in handle_undo_action():

      - Check if action was snooze (action == "snooze" in undo_hint)

      - Extract cancel_unsnooze_job_id from inverse_parameters

      - Call JobQueue::cancel() with graceful handling: Ok, NotRunning, and
      JobNotFound are all success cases

      - Apply label changes from inverse_parameters: add add_labels, remove
      remove_labels

      - Handle case where unsnooze job already ran (message may already be in
      INBOX)
  - title: Create undo action record and action_link
    done: true
    description: >-
      After successful inverse action execution:

      - Create new Action record with ActionRepository::create():
        - action_type: format!("undo_{}", original_action.action_type)
        - status: Queued initially, then mark_completed_with_undo_hint()
        - undo_hint_json: simple marker like {"note": "undo action - not reversible"} (no redo support needed)
      - Create ActionLink with ActionLinkRepository::create():
        - cause_action_id: undo_action.id
        - effect_action_id: original_action.id
        - relation_type: UndoOf
  - title: Add tests for undo system
    done: true
    description: >-
      Add comprehensive tests in undo_action.rs #[cfg(test)] module:

      - Test undo of each action type: archive, apply_label, remove_label,
      mark_read, mark_unread, star, unstar, trash, restore

      - Test rejection of irreversible actions (delete, forward, auto_reply)

      - Test rejection of already-undone actions

      - Test rejection of non-completed actions (Queued, Executing, Failed)

      - Test snooze undo with job cancellation

      - Test snooze undo when job already ran

      - Test Gmail API 404 handling

      - Test action_link creation

      - Use wiremock for Gmail API mocking, follow existing test patterns
changedFiles:
  - server/crates/ashford-core/src/gmail/mime_builder.rs
  - server/crates/ashford-core/src/jobs/mod.rs
  - server/crates/ashford-core/src/jobs/undo_action.rs
  - server/crates/ashford-core/src/migrations.rs
  - server/migrations/007_add_unique_undo_links.sql
tags:
  - actions
  - gmail
  - rust
---

Implement the undo system for reversing Gmail actions:

## Scope
- Create undo.action job type and handler
- Load original action and validate it's undoable
- Derive inverse action from undo_hint_json
- Execute inverse action via Gmail API
- Create action_link with relation_type='undo_of'
- Handle non-undoable actions gracefully (delete, forward, auto_reply)

## Design Notes
- Undo hint structure already stored by existing actions contains inverse_action and inverse_parameters
- Some actions are irreversible (delete, forward, auto_reply) - undo handler should reject these
- Snooze undo requires canceling the scheduled unsnooze job
- Check for existing action_links to prevent double-undo

<!-- rmplan-generated-start -->
## Expected Behavior/Outcome

When a user requests to undo a previously executed action:
1. The system validates the action is undoable (completed, not irreversible, not already undone)
2. The inverse action is derived from the stored `undo_hint_json`
3. The inverse action is executed via the Gmail API
4. A new action record is created for the undo operation
5. An `action_link` with `relation_type='undo_of'` connects the undo action to the original

**States:**
- **Undoable**: Action is `Completed`, has valid `undo_hint_json` with `inverse_action != "none"`, and no existing `undo_of` link
- **Not Undoable**: Action is irreversible (`delete`, `forward`, `auto_reply`), not completed, or already undone

## Key Findings

### Product & User Story
The undo system enables users to reverse completed Gmail actions. This provides a safety net for automated decisions and allows quick correction of mistakes. The UI will trigger undo by enqueuing an `undo.action` job with the original action ID.

### Design & UX Approach
- Undo is a fire-and-forget operation: user clicks undo, job is enqueued
- Feedback is provided through action status updates (users can see the undo action status)
- Clear error messages for non-undoable actions (irreversible, already undone, etc.)

### Technical Plan & Risks
See detailed findings below and Risks & Constraints section.

### Pragmatic Effort Estimate
This is a moderate-complexity feature building on existing infrastructure. The undo hint system is already in place; this adds the handler to consume those hints.

## Acceptance Criteria

- [ ] `undo.action` job type is registered and dispatched correctly
- [ ] Undo of archive action restores message to INBOX
- [ ] Undo of apply_label removes the applied label
- [ ] Undo of remove_label restores the removed label
- [ ] Undo of mark_read marks message as unread
- [ ] Undo of mark_unread marks message as read
- [ ] Undo of star removes star
- [ ] Undo of unstar adds star back
- [ ] Undo of trash restores message from trash
- [ ] Undo of restore moves message back to trash
- [ ] Undo of snooze cancels scheduled unsnooze job and restores to inbox
- [ ] Irreversible actions (delete, forward, auto_reply) return clear error
- [ ] Already-undone actions return clear error
- [ ] Non-completed actions return clear error
- [ ] action_link with `undo_of` relation is created
- [ ] Snooze undo handles case where unsnooze job already ran
- [ ] Gmail API 404 errors are handled gracefully
- [ ] All new code paths are covered by tests

## Dependencies & Constraints

**Dependencies:**
- Existing `undo_hint_json` infrastructure in `action_gmail.rs`
- `ActionRepository` and `ActionLinkRepository` in `decisions/repositories.rs`
- `JobQueue::cancel()` method in `queue.rs`
- Gmail API client methods (`modify_message`, `untrash_message`, etc.)

**Technical Constraints:**
- Must handle race conditions where undo is requested during original action execution
- Snooze undo must gracefully handle case where unsnooze job has already run
- Gmail API 404 errors should result in action being marked as failed (message deleted externally)

## Implementation Notes

### Recommended Approach
1. Create `undo_action.rs` following existing job handler patterns (see `action_gmail.rs`, `outbound_send.rs`)
2. Implement validation logic as helper functions for testability
3. Reuse Gmail client creation from `action_gmail.rs` (`create_gmail_client`)
4. Execute inverse actions by directly calling Gmail API methods (similar to `action_gmail.rs`)
5. Use wiremock for Gmail API mocking in tests

### Potential Gotchas
- The snooze undo hint uses `cancel_unsnooze_job_id` field but may also have `unsnooze_job_id` - verify which is the correct field to use for cancellation
- Job cancellation returns `NotRunning` error if job is already completed - treat this as success (unsnooze already ran)
- The `action_link` semantics: `{cause: undo_action, effect: original_action, relation_type: "undo_of"}` means "the cause (undo action) is the undo of the effect (original action)"

## Research

### Summary
- The undo system leverages the existing `undo_hint_json` infrastructure already populated by action handlers
- `action_links` table exists with `undo_of` relation type ready for use
- Most actions already store inverse action info in their undo hints
- Special handling needed for snooze (cancel scheduled job) and irreversible actions (delete, forward, auto_reply)

### Findings

#### Job Handler Architecture

**File:** `server/crates/ashford-core/src/jobs/mod.rs`

All job handlers follow this pattern:
```rust
pub const JOB_TYPE: &str = "undo.action";

pub async fn handle_undo_action(dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError> {
    // 1. Parse payload
    let payload: UndoPayload = serde_json::from_value(job.payload.clone())
        .map_err(|err| JobError::Fatal(format!("invalid undo.action payload: {err}")))?;

    // 2. Load dependencies and validate
    // 3. Execute inverse action
    // 4. Create action record and action_link
    Ok(())
}
```

**Registration:** Add to `mod.rs`:
- Define `mod undo_action;`
- Export `pub const JOB_TYPE_UNDO_ACTION: &str = undo_action::JOB_TYPE;`
- Add routing case in `JobDispatcher::execute()` match statement

**Error Types:**
- `JobError::Retryable { message, retry_after }` - For transient failures (network, rate limits)
- `JobError::Fatal(String)` - For permanent failures (invalid data, missing resources)

#### Undo Hint Structures by Action Type

**Archive** (`server/crates/ashford-core/src/jobs/action_gmail.rs:903-920`):
```json
{
    "pre_labels": ["INBOX", "UNREAD"],
    "pre_unread": true,
    "pre_starred": false,
    "pre_in_inbox": true,
    "pre_in_trash": false,
    "action": "archive",
    "inverse_action": "apply_label",
    "inverse_parameters": {"label": "INBOX"}
}
```

**Apply Label** (lines 922-950):
```json
{
    "action": "apply_label",
    "inverse_action": "remove_label",
    "inverse_parameters": {"label": "Label_Applied"}
}
```

**Remove Label** (lines 952-980):
```json
{
    "action": "remove_label",
    "inverse_action": "apply_label",
    "inverse_parameters": {"label": "Label_Removed"}
}
```

**Mark Read/Unread** (lines 982-1012):
```json
{
    "action": "mark_read",
    "inverse_action": "mark_unread",
    "inverse_parameters": {}
}
```

**Star/Unstar** (lines 1014-1042):
```json
{
    "action": "star",
    "inverse_action": "unstar",
    "inverse_parameters": {}
}
```

**Trash** (lines 1044-1055):
```json
{
    "action": "trash",
    "inverse_action": "restore",
    "inverse_parameters": {}
}
```

**Restore** (lines 1076-1087):
```json
{
    "action": "restore",
    "inverse_action": "trash",
    "inverse_parameters": {}
}
```

**Delete** (lines 1057-1074) - IRREVERSIBLE:
```json
{
    "action": "delete",
    "inverse_action": "none",
    "inverse_parameters": {"note": "cannot undo delete - message permanently deleted"},
    "irreversible": true
}
```

**Snooze** (lines 1089-1160) - SPECIAL HANDLING:
```json
{
    "pre_labels": ["INBOX"],
    "pre_unread": false,
    "pre_starred": false,
    "pre_in_inbox": true,
    "pre_in_trash": false,
    "action": "snooze",
    "inverse_action": "none",
    "inverse_parameters": {
        "add_labels": ["INBOX"],
        "remove_labels": ["Label_Snoozed"],
        "cancel_unsnooze_job_id": "job-uuid-123",
        "note": "Undo snooze by returning to inbox and removing snooze label"
    },
    "snooze_until": "2024-12-11T15:30:00Z",
    "snooze_label": "Label_Snoozed",
    "unsnooze_job_id": "job-uuid-123"
}
```

**Forward/AutoReply** (`server/crates/ashford-core/src/jobs/outbound_send.rs:296-305`) - IRREVERSIBLE:
```json
{
    "action": "forward|auto_reply",
    "inverse_action": "none",
    "inverse_parameters": {"note": "cannot undo outbound send"},
    "irreversible": true,
    "sent_message_id": "gmail-message-id",
    "sent_thread_id": "gmail-thread-id"
}
```

#### Action Repository Methods

**File:** `server/crates/ashford-core/src/decisions/repositories.rs`

Key methods for undo handler:
```rust
// Load original action
pub async fn get_by_id(org_id, user_id, id) -> Result<Action, ActionError>

// Create undo action record
pub async fn create(new_action: NewAction) -> Result<Action, ActionError>

// Mark undo action as completed with undo hint
pub async fn mark_completed_with_undo_hint(org_id, user_id, id, undo_hint) -> Result<Action, ActionError>
```

**ActionStatus enum:**
- `Queued` - Initial state
- `Executing` - Currently running
- `Completed` - Successfully executed (ONLY undoable state)
- `Failed` - Execution failed
- `Canceled` - User canceled
- `Rejected` - Approval rejected
- `ApprovedPending` - Awaiting approval

#### Action Link Repository

**File:** `server/crates/ashford-core/src/decisions/repositories.rs` (lines 602-697)

```rust
// Check if action already undone
pub async fn get_by_cause_action_id(cause_action_id: &str) -> Result<Vec<ActionLink>, ActionLinkError>

// Create undo link
pub async fn create(new_link: NewActionLink) -> Result<ActionLink, ActionLinkError>
```

**Link semantics for undo:**
```rust
NewActionLink {
    cause_action_id: undo_action.id,    // The undo action
    effect_action_id: original_action.id, // The action being undone
    relation_type: ActionLinkRelationType::UndoOf,
}
```
Meaning: "undo_action is the undo of original_action"

**Checking if already undone:**
```rust
let links = action_link_repo.get_by_cause_action_id(&original_action_id).await?;
let already_undone = links.iter().any(|l| l.relation_type == ActionLinkRelationType::UndoOf);
```

Wait - this is backwards. If we want to check if `original_action` has been undone, we should check links where `original_action` is the `effect_action_id`:
```rust
let links = action_link_repo.get_by_effect_action_id(&original_action_id).await?;
let already_undone = links.iter().any(|l| l.relation_type == ActionLinkRelationType::UndoOf);
```

#### Job Queue Cancellation

**File:** `server/crates/ashford-core/src/queue.rs` (lines 311-327)

```rust
pub async fn cancel(&self, job_id: &str) -> Result<(), QueueError>
```

**Behavior:**
- Cancels jobs in `Queued` or `Running` state
- Returns `QueueError::NotRunning(job_id)` if job is already terminal (Completed/Failed/Canceled)
- Returns `QueueError::JobNotFound(job_id)` if job doesn't exist

**For snooze undo:**
```rust
match queue.cancel(&undo_hint["cancel_unsnooze_job_id"]).await {
    Ok(()) => { /* Job was pending, now canceled */ }
    Err(QueueError::NotRunning(_)) => { /* Job already ran, that's fine */ }
    Err(QueueError::JobNotFound(_)) => { /* Job doesn't exist, that's fine */ }
    Err(err) => return Err(JobError::retryable(format!("cancel unsnooze job: {err}")))
}
```

#### Gmail Client Methods

**File:** `server/crates/ashford-core/src/gmail/client.rs`

For executing inverse actions:
```rust
// Apply/remove labels (archive undo, apply_label undo, remove_label undo, mark_read/unread undo, star/unstar undo)
pub async fn modify_message(
    message_id: &str,
    add_labels: Option<Vec<&str>>,
    remove_labels: Option<Vec<&str>>
) -> Result<GmailMessage, GmailClientError>

// Restore from trash (trash undo)
pub async fn untrash_message(message_id: &str) -> Result<GmailMessage, GmailClientError>

// Move to trash (restore undo)
pub async fn trash_message(message_id: &str) -> Result<GmailMessage, GmailClientError>
```

**Client creation helper** (`action_gmail.rs`):
```rust
pub async fn create_gmail_client(
    dispatcher: &JobDispatcher,
    account_id: &str,
) -> Result<GmailClient<NoopTokenStore>, JobError>
```

#### Test Patterns

**File:** Various test modules in `server/crates/ashford-core/src/jobs/`

**Setup pattern:**
```rust
async fn setup_db() -> (Database, TempDir) {
    let dir = TempDir::new().expect("temp dir");
    let db_path = dir.path().join(format!("db_{}.sqlite", uuid::Uuid::new_v4()));
    let db = Database::new(&db_path).await.expect("create db");
    run_migrations(&db).await.expect("migrations");
    (db, dir)
}
```

**Gmail API mocking with wiremock:**
```rust
let server = MockServer::start().await;
Mock::given(method("POST"))
    .and(path("/gmail/v1/users/user@example.com/messages/msg-1/modify"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({...})))
    .expect(1)
    .mount(&server)
    .await;

let dispatcher = dispatcher(db).with_gmail_api_base(format!("{}/gmail/v1/users", &server.uri()));
```

**Action verification:**
```rust
let repo = ActionRepository::new(db.clone());
let action = repo.get_by_id(DEFAULT_ORG_ID, DEFAULT_USER_ID, &action_id).await?;
assert!(matches!(action.status, ActionStatus::Completed));
```

### Risks & Constraints

1. **Double-Undo Prevention**
   - Check for existing `action_link` with `relation_type = "undo_of"` where the original action is the `effect_action_id`
   - Return `JobError::Fatal` if action already undone

2. **Race Conditions**
   - User may request undo while original action is still executing
   - Only allow undo of actions in `Completed` status
   - Return `JobError::Fatal` for non-completed actions

3. **External State Changes**
   - Message may be deleted externally before undo
   - Handle 404 from Gmail API: mark undo action as failed with descriptive error

4. **Snooze Job Timing**
   - Unsnooze job may have already run when user requests undo
   - Treat `NotRunning` and `JobNotFound` from `queue.cancel()` as success cases
   - Still apply label changes even if job cancellation fails (message is still snoozed in Gmail)

5. **Undo Hint Missing Fields**
   - Some older actions may have incomplete undo hints
   - Validate required fields exist before proceeding
   - Return `JobError::Fatal` with clear message if undo hint is malformed

### Design Decision: Single-Level Undo

Undo actions do not need to support "redo" (undoing the undo). The undo action's `undo_hint_json` can be set to a simple marker like `{"note": "undo action - not reversible"}` rather than building a full inverse. This keeps the implementation straightforward.
<!-- rmplan-generated-end -->

Implemented undo.action job handler and registration. Added validation to allow undo only for completed, reversible actions without prior undo links; rejects irreversible or already-undone actions with clear fatal errors. Handler derives inverse_action from stored undo_hint_json, loads message/account, creates an undo action record, executes inverse via Gmail client (apply/remove label, mark read/unread, star/unstar, trash/restore) with 404 handled as non-fatal NotFound result, and records outcome. Snooze undo cancels unsnooze job via JobQueue with graceful handling of NotRunning/JobNotFound, then applies label changes. Successful undo creates action_link relation_type=undo_of; undo actions are marked completed with non-reversible hint; failures mark undo action failed when fatal.

Files touched: server/crates/ashford-core/src/jobs/undo_action.rs (new handler, helpers, tests), server/crates/ashford-core/src/jobs/mod.rs (module registration and dispatch). Key functions: handle_undo_action, validate_action_undoable, execute_inverse_action, undo_snooze, labels_from_array; mapping helpers for message and action_link errors. Tests (wiremock + temp DB) cover archive undo path, irreversible rejection, already-undone rejection, snooze undo with job cancellation, and ensure link creation and undo action status. Added trait import for JobExecutor in tests to call dispatcher.execute. All ashford-core tests now passing (cargo test -p ashford-core --lib).

Hardened undo.action to be idempotent across retries and concurrent requests. The handler now loads any existing UndoOf link up front, validates against it, and creates the undo action/link pair before calling Gmail so retries reuse the same record. Each undo action stores the owning job_id; a fatal guard prevents other jobs from executing an in‑flight undo. Link creation now tolerates unique conflicts by failing the losing action and reusing the existing locked undo action. Added a unique partial index on action_links(effect_action_id) for relation_type='undo_of' with migration 007 and updated migration wiring. Adjusted tests to match the new lock semantics and added coverage for undoing apply_label (removing the label via removeLabelIds). Ran cargo test -p ashford-core undo_action -- --nocapture successfully. Files: server/crates/ashford-core/src/jobs/undo_action.rs, server/crates/ashford-core/src/migrations.rs, server/migrations/007_add_unique_undo_links.sql, tests in undo_action.rs.

Ensured undo actions reach a terminal state when retryable errors exhaust retries. Updated server/crates/ashford-core/src/jobs/undo_action.rs so handle_undo_action now marks the undo action Failed for non-retryable errors and also on the final allowed attempt (job.attempts >= job.max_attempts), preventing stuck Executing states after the job terminates. Added regression test retryable_error_on_final_attempt_marks_undo_failed in the same file, mocking a Gmail 500 response and forcing attempts to max_attempts to simulate retry exhaustion; the test asserts the undo action transitions to Failed and preserves the error message. Task: fix reviewer-reported stuck Executing undo on retry exhaustion.

Autofix: Addressed three code review issues. (1) Removed unnecessary 'async' keyword from validate_action_undoable function in server/crates/ashford-core/src/jobs/undo_action.rs - the function performs only synchronous validation checks and had no await points, so the async keyword was adding unnecessary overhead. Both call sites at lines 62 and 128 were updated to remove the .await calls. (2) Added new test 'undo_snooze_when_unsnooze_job_already_ran' in undo_action.rs (lines 929-1028) to cover the acceptance criteria scenario where the unsnooze job has already completed before undo is requested. The test creates a job, manually sets it to 'completed' state via raw SQL to simulate the already-ran scenario, then verifies that undo still succeeds by correctly handling QueueError::NotRunning as a success case. The test validates label modifications are applied, undo action is marked Completed, and action_link with UndoOf relation is created. (3) Fixed documentation mismatch in docs/data_model.md line 240 - updated index name from 'action_links_undo_of_uidx' to 'action_links_effect_undo_unique' to match the actual migration file 007_add_unique_undo_links.sql. All 594 ashford-core tests pass after these changes.

Updated Gmail MIME builder docs to match the actual Rust API for task 'Gmail Actions: Undo System'. Corrected examples in docs/gmail_integration.md and docs/job_queue.md to construct MimeMessage via struct literal with MimeAttachment, use EmailAddress::new(Some(name), email) argument order, and call to_base64_url instead of the non-existent builder-style methods. Added explicit note that the builder is field-based to avoid future confusion. This fixes the review issue about non-compiling documentation snippets.
