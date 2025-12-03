---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 4: Gmail Actions"
goal: Implement Gmail action execution with undo support
id: 5
uuid: 66785b19-e85d-4135-bbca-9d061a0394c7
generatedBy: agent
status: in_progress
priority: high
container: true
temp: false
dependencies:
  - 4
  - 25
  - 23
  - 24
  - 22
parent: 1
references:
  "1": 076d03b1-833c-4982-b0ca-1d8868d40e31
  "4": 5cf4cc37-3eb8-4f89-adae-421a751d13a1
  "22": c69a5bba-4a08-4a49-b841-03d396a6ba81
  "23": 7300ce7b-c38b-4fe6-ae96-ead50a3f1f05
  "24": e3c7d618-82e3-4835-9f9c-441d596c2fc1
  "25": 0402f4e3-9063-4655-b42d-cef6910a6827
issue: []
pullRequest: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T01:23:12.234Z
promptsGeneratedAt: 2025-11-29T01:23:12.234Z
createdAt: 2025-11-29T01:21:26.875Z
updatedAt: 2025-12-03T07:45:17.152Z
progressNotes: []
tasks:
  - title: Define action types enum
    done: false
    description: "Create ActionType enum: archive, apply_label, remove_label,
      mark_read, mark_unread, delete, trash, restore, star, unstar, snooze,
      forward, auto_reply."
  - title: Implement archive action
    done: false
    description: Gmail API call to remove INBOX label. Store undo_hint with original
      labels. Mark action completed.
  - title: Implement label actions
    done: false
    description: "apply_label: add label (create if needed). remove_label: remove
      label. Store original label state in undo_hint."
  - title: Implement read state actions
    done: false
    description: "mark_read: remove UNREAD label. mark_unread: add UNREAD label.
      Track original state for undo."
  - title: Implement delete/trash actions
    done: false
    description: "trash: move to trash. delete: permanent delete (dangerous,
      requires approval). Store pre-delete state for trash undo."
  - title: Implement star actions
    done: false
    description: "star: add STARRED label. unstar: remove STARRED label. Simple
      toggle with undo."
  - title: Implement snooze action
    done: false
    description: Move to snooze folder (configurable), schedule restore job for
      snooze end time. Store original location for undo.
  - title: Implement forward action
    done: false
    description: Create outbound.send job with forward parameters. Include original
      message as attachment or inline. Store in actions table.
  - title: Implement auto_reply action
    done: false
    description: Create outbound.send job with reply parameters. Generate reply
      using template or LLM. Track reply in actions table.
  - title: Build outbound.send job handler
    done: false
    description: Send email via Gmail API (messages.send). Handle drafts,
      attachments, threading (In-Reply-To, References headers).
  - title: Create undo job handler
    done: false
    description: Load original action, derive inverse action from undo_hint_json,
      execute inverse, create action_link with relation_type='undo_of'.
  - title: Implement action.gmail job handler
    done: false
    description: Route to appropriate action implementation based on action_type.
      Update action status, capture errors, record executed_at.
  - title: Capture pre-images for undo
    done: false
    description: Before executing action, fetch current message state (labels,
      folder, read status). Store in undo_hint_json for reliable undo.
tags:
  - actions
  - gmail
  - rust
---

Gmail action execution:
- action.gmail job handler
- Action types: archive, apply_label, remove_label, mark_read, mark_unread, delete, trash, star, unstar, snooze
- Forward and auto-reply actions (outbound.send job)
- Undo job handler with inverse action derivation
- undo_hint_json storage for reversibility
- actions and action_links tables
- Pre-image capture for undo operations

## Research

### Summary
- This milestone implements the actual Gmail API mutations that have been stubbed out in the current codebase. The infrastructure for actions (database tables, job handlers, status transitions) is already in place.
- The current `action.gmail` job handler is a placeholder that only marks actions as completed without making any Gmail API calls.
- The `ActionType` enum already exists with all required action types, but several need to be added (remove_label, trash, restore) and the Gmail client needs write operation methods.
- Undo infrastructure exists (undo_hint_json field, action_links table with `undo_of` relation type, generate_undo_hint function) but the undo job handler is not implemented.
- Email sending (forward, auto_reply) will require significant new functionality as no Gmail send operations exist yet.

### Findings

#### Job System Architecture
**Location:** `server/crates/ashford-core/src/jobs/` and `server/crates/ashford-core/src/queue.rs`

The job system is production-ready with:
- **Job Types:** String constants defined in handler modules, exported from `jobs/mod.rs`:
  - `JOB_TYPE_BACKFILL_GMAIL`: "backfill.gmail"
  - `JOB_TYPE_ACTION_GMAIL`: "action.gmail"
  - `JOB_TYPE_APPROVAL_NOTIFY`: "approval.notify"
  - `JOB_TYPE_CLASSIFY`: "classify"
  - `JOB_TYPE_INGEST_GMAIL`: "ingest.gmail"
  - `JOB_TYPE_HISTORY_SYNC_GMAIL`: "history.sync.gmail"

- **JobDispatcher** (`jobs/mod.rs:37-66`): Central registry and execution coordinator
  ```rust
  pub struct JobDispatcher {
      pub db: Database,
      pub http: reqwest::Client,
      pub gmail_api_base: Option<String>,
      pub llm_client: Arc<dyn LLMClient>,
      pub policy_config: PolicyConfig,
  }
  ```

- **Error Mapping Functions** (`jobs/mod.rs:83-196`):
  - `map_gmail_error()`: Maps Gmail API errors to JobError (404→Fatal, 401/403/429→Retryable)
  - `map_llm_error()`: Maps LLM errors with rate limit handling
  - `map_account_error()`: Maps account repository errors
  - `map_action_error()`: Maps action repository errors

- **Job Lifecycle:**
  - States: Queued → Running → Completed/Failed/Canceled
  - Retry with exponential backoff (2^n seconds, max 5 minutes, ±25% jitter)
  - Idempotency keys prevent duplicate execution
  - Heartbeat tracking for long-running jobs

- **Handler Pattern:**
  ```rust
  pub async fn handle_{job_type}(
      dispatcher: &JobDispatcher,
      job: Job
  ) -> Result<(), JobError>
  ```

#### Current action.gmail Handler (STUB)
**Location:** `server/crates/ashford-core/src/jobs/action_gmail.rs`

```rust
pub const JOB_TYPE: &str = "action.gmail";

#[derive(Debug, Deserialize)]
struct ActionJobPayload {
    pub account_id: String,
    pub action_id: String,
}

/// Execute a Gmail action. This is currently a placeholder that marks the
/// action as executed so the pipeline can continue; provider-side mutations
/// will be implemented in a later phase.
pub async fn handle_action_gmail(
    dispatcher: &JobDispatcher,
    job: Job,
) -> Result<(), JobError> {
    // Currently: loads action, validates account, marks executing → completed
    // Does NOT make any Gmail API calls
}
```

#### Gmail API Client
**Location:** `server/crates/ashford-core/src/gmail/client.rs`

**Existing Operations (Read-Only):**
- `get_message(message_id)` → Message
- `get_thread(thread_id)` → Thread
- `list_history(start_history_id, page_token, max_results)` → ListHistoryResponse
- `list_messages(query, page_token, include_spam_trash, max_results)` → ListMessagesResponse
- `get_profile()` → Profile

**Missing Write Operations (Need Implementation):**
- `modify_message(message_id, add_labels, remove_labels)` - For label changes, archive, star, read state
- `trash_message(message_id)` - Move to trash
- `untrash_message(message_id)` - Restore from trash
- `delete_message(message_id)` - Permanent deletion
- `send_message(raw_message)` - For forward/auto_reply

**Token Management:**
- Automatic refresh with 5-minute buffer
- Thread-safe with RwLock + Mutex
- Retry-once-on-401 pattern

**Error Handling:**
```rust
pub enum GmailClientError {
    Http(reqwest::Error),
    OAuth(OAuthError),
    TokenStore(String),
    Decode(serde_json::Error),
    Unauthorized,
}
```

#### ActionType Enum
**Location:** `server/crates/ashford-core/src/llm/decision.rs:11-106`

**Current Types:**
```rust
pub enum ActionType {
    ApplyLabel,
    MarkRead,
    MarkUnread,
    Archive,
    Delete,
    Move,
    Star,
    Unstar,
    Forward,
    AutoReply,
    CreateTask,
    Snooze,
    AddNote,
    Escalate,
    None,
}
```

**Missing Types (per plan):**
- `RemoveLabel` - Currently handled as part of `ApplyLabel` but should be separate
- `Trash` - Currently using `Delete` but trash is different from permanent delete
- `Restore` - Undo trash operation

**Danger Levels:**
- Safe: ApplyLabel, MarkRead, MarkUnread, Archive, Move, None
- Reversible: Star, Unstar, Snooze, AddNote, CreateTask
- Dangerous: Delete, Forward, AutoReply, Escalate

#### Actions & Decisions System
**Location:** `server/crates/ashford-core/src/decisions/`

**Action Model** (`types.rs:132-148`):
```rust
pub struct Action {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub decision_id: Option<String>,
    pub action_type: String,
    pub parameters_json: Value,
    pub status: ActionStatus,
    pub error_message: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    pub undo_hint_json: Value,  // Stores inverse action info
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**ActionStatus** (`types.rs:31-66`):
```rust
pub enum ActionStatus {
    Queued,           // Ready for execution
    Executing,        // Currently being processed
    Completed,        // Successfully executed
    Failed,           // Execution failed
    Canceled,         // User or system canceled
    Rejected,         // User rejected during approval
    ApprovedPending,  // Awaiting user approval
}
```

**Valid Transitions** (`repositories.rs:592-603`):
- Queued → Executing, Canceled, Rejected, ApprovedPending, Failed
- Executing → Completed, Failed, Canceled
- ApprovedPending → Queued, Canceled, Rejected
- Completed/Failed/Canceled/Rejected → (terminal)

**ActionRepository Methods:**
- `create(new_action)` → Action
- `get_by_id(org_id, user_id, id)` → Action
- `get_by_decision_id(org_id, user_id, decision_id)` → Vec<Action>
- `list_by_message_id(org_id, user_id, message_id)` → Vec<Action>
- `list_by_status(org_id, user_id, status, account_id)` → Vec<Action>
- `update_status(org_id, user_id, id, next_status, error_message, executed_at)` → Action
- `mark_executing()`, `mark_completed()`, `mark_failed()` convenience methods

#### Action Links
**Location:** `server/crates/ashford-core/src/decisions/types.rs:68-96` and `repositories.rs:495-590`

**Relation Types:**
```rust
pub enum ActionLinkRelationType {
    UndoOf,       // Effect undoes the Cause
    ApprovalFor,  // Effect is an approval for Cause
    Spawned,      // Effect was spawned by Cause
    Related,      // Generic relationship
}
```

**ActionLinkRepository Methods:**
- `create(new_link)` → ActionLink
- `get_by_cause_action_id(cause_action_id)` → Vec<ActionLink>
- `get_by_effect_action_id(effect_action_id)` → Vec<ActionLink>
- `delete(id)` → ()

#### Undo Infrastructure (Existing)
**Location:** `server/crates/ashford-core/src/jobs/classify.rs:395-417`

```rust
fn generate_undo_hint(
    action: ActionType,
    _parameters: &serde_json::Value,
) -> (ActionType, serde_json::Value) {
    match action {
        ActionType::Archive => (ActionType::Move, json!({"destination": "INBOX"})),
        ActionType::Delete => (ActionType::None, json!({"note": "cannot undo delete"})),
        ActionType::MarkRead => (ActionType::MarkUnread, json!({})),
        ActionType::MarkUnread => (ActionType::MarkRead, json!({})),
        ActionType::Star => (ActionType::Unstar, json!({})),
        ActionType::Unstar => (ActionType::Star, json!({})),
        // ... other mappings
    }
}
```

**Current Limitation:** This is generated at decision time, but for reliable undo we need to capture the **actual pre-action state** (e.g., what labels existed before archiving).

#### Database Schema
**Location:** `server/migrations/001_initial.sql`

**Relevant Tables (Already Exist):**
- `actions` - Action records with undo_hint_json column
- `action_links` - Relationships between actions (undo_of, etc.)
- `messages` - Contains labels_json with current label state
- `jobs` - Job queue table

**No Schema Changes Required** - All necessary tables and columns exist.

#### Project Patterns

**Error Handling:**
- All errors use `thiserror::Error`
- JobError has Retryable vs Fatal distinction
- Gmail errors mapped to appropriate job error types

**Testing:**
- Unit tests co-located with source in `#[cfg(test)] mod tests`
- Integration tests in `server/crates/ashford-core/tests/`
- WireMock for mocking Gmail API
- TempDir for isolated test databases
- Avoid mocks except for external services

**Async Pattern:**
- `#[async_trait]` for async trait methods
- Tokio runtime
- Clone-heavy design (Database, repos use Arc internally)

**Constants:**
- `DEFAULT_ORG_ID = 1`, `DEFAULT_USER_ID = 1` in `constants.rs`
- Job type constants exported from `jobs/mod.rs`

### Risks & Constraints

1. **Gmail API Rate Limits:** Write operations have stricter rate limits than reads. Need careful error handling for 429/403 responses with appropriate retry logic.

2. **Pre-image Capture Timing:** Must fetch message state BEFORE executing action to ensure accurate undo_hint. Race condition risk if message modified externally.

3. **Permanent Deletion Risk:** `delete` action is irreversible. Must have strict approval requirements and clear user warnings.

4. **Email Sending Complexity:** Forward and auto_reply require:
   - MIME message construction
   - Proper threading headers (In-Reply-To, References)
   - Attachment handling for forwards
   - Template or LLM-generated content for auto_reply

5. **Snooze Implementation:** Gmail doesn't have native snooze. Options:
   - Remove from INBOX, add custom label, schedule restore job
   - Use draft API as placeholder
   - Need configurable snooze folder

6. **Undo Window:** Some actions may become un-undoable over time (e.g., if message is externally modified). May need undo expiration.

7. **Concurrent Execution:** Multiple actions on same message could conflict. May need optimistic locking or action serialization per message.

8. **Label Creation:** `apply_label` may need to create labels that don't exist. Gmail API requires labels.create() first.

9. **Dependency on Milestone 4:** This milestone depends on completion of Milestone 4 (decision engine and safety policies) as actions flow from decisions.
