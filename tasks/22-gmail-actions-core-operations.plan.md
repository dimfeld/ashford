---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Actions: Core Operations"
goal: Implement Gmail API write operations and core action execution (archive,
  labels, read state, star, trash, delete) with pre-image capture for undo
  support
id: 22
uuid: c69a5bba-4a08-4a49-b841-03d396a6ba81
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies:
  - 25
parent: 5
references:
  "5": 66785b19-e85d-4135-bbca-9d061a0394c7
  "25": 0402f4e3-9063-4655-b42d-cef6910a6827
issue: []
pullRequest: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
planGeneratedAt: 2025-12-03T02:21:58.830Z
promptsGeneratedAt: 2025-12-03T02:21:58.830Z
createdAt: 2025-12-03T02:21:14.664Z
updatedAt: 2025-12-03T10:53:15.938Z
progressNotes:
  - timestamp: 2025-12-03T09:09:25.439Z
    text: "Completed Tasks 1-4: Added Gmail write operations (modify_message,
      trash_message, untrash_message, delete_message) to GmailClient with
      ModifyMessageRequest type and send_empty_response helper. Added
      RemoveLabel, Trash, Restore ActionType variants with as_str, from_str, and
      danger_level implementations. Updated generate_undo_hint for new actions.
      All 438 tests pass."
    source: "implementer: Tasks 1-4"
  - timestamp: 2025-12-03T09:13:57.714Z
    text: "Added 8 new tests to improve coverage: 6 tests in gmail/types.rs for
      ModifyMessageRequest serialization (verifying skip_serializing_if behavior
      for None fields), and 2 tests in gmail/client.rs using WireMock body_json
      matcher to verify request body content. Total test count increased from
      438 to 446. All tests pass."
    source: "tester: Tasks 1-4"
  - timestamp: 2025-12-03T09:25:08.071Z
    text: "Completed Tasks 5-7: Added GmailClient factory (create_gmail_client),
      PreImageState struct with from_labels and build_undo_hint methods,
      capture_pre_image async helper, get_provider_message_id helper, and
      mark_completed_with_undo_hint method to ActionRepository. Added 12 new
      unit tests. All 458 tests pass."
    source: "implementer: Tasks 5-7"
  - timestamp: 2025-12-03T09:30:46.557Z
    text: "Added 15 new tests for Tasks 5-7. Tests include: 8 additional
      PreImageState edge cases (all_flags_true, case_sensitive, category_labels,
      remove_label, apply_label, restore, unstar, mark_unread), 2 tests for
      capture_pre_image with WireMock (returns_labels, handles_not_found), 2
      tests for get_provider_message_id (returns_message,
      returns_fatal_for_not_found), and 3 tests for create_gmail_client
      (uses_dispatcher_api_base, uses_default_api_base,
      uses_account_credentials). Total tests increased from 458 to 473 in
      ashford-core."
    source: "tester: Tasks 5-7"
  - timestamp: 2025-12-03T09:35:06.681Z
    text: "Fixed create_gmail_client function to properly call
      refresh_tokens_if_needed before creating GmailClient, following the
      pattern from ingest_gmail.rs. Changed signature from taking &Account to
      taking account_id: &str. Removed assert!(true) no-op assertions from tests
      - replaced with meaningful assertions including a new test for nonexistent
      account error handling. All 474 tests pass."
    source: "implementer: Task 5 fix"
  - timestamp: 2025-12-03T09:36:42.663Z
    text: Completed Tasks 5-7 implementing the foundational infrastructure for
      action handlers. Task 5 added create_gmail_client() that follows the
      ingest_gmail.rs pattern - calls refresh_tokens_if_needed() first then
      creates GmailClient with NoopTokenStore. Task 6 added
      ActionRepository::mark_completed_with_undo_hint() for atomic status +
      undo_hint_json updates. Task 7 added PreImageState struct with
      from_labels(), build_undo_hint(), plus capture_pre_image() and
      get_provider_message_id() helpers. All 474 tests pass including 28 new
      tests for these components.
    source: "orchestrator: Tasks 5-7"
  - timestamp: 2025-12-03T09:42:51.922Z
    text: "Implemented all action handlers (archive, apply_label, remove_label,
      mark_read, mark_unread, star, unstar, trash, delete, restore) in
      action_gmail.rs. Each handler follows the pattern: capture pre-image
      state, execute Gmail API mutation, build undo hint. The
      handle_action_gmail function now dispatches to appropriate handlers based
      on action_type. All 474 tests pass."
    source: "implementer: Tasks 8-15"
  - timestamp: 2025-12-03T09:48:47.229Z
    text: "Added 25 comprehensive tests for action handlers in action_gmail.rs.
      Tests cover: execute_archive (1), execute_apply_label (1),
      execute_remove_label (1), execute_mark_read (1), execute_mark_unread (1),
      execute_star (1), execute_unstar (1), execute_trash (1), execute_restore
      (1), execute_delete (1), execute_action dispatcher (3 tests for archive,
      delete without pre-image, unsupported action), and handle_action_gmail (13
      tests covering: successful archive execution, Gmail 404 error marking
      action failed, skipping completed actions, skipping approved_pending
      actions, rejecting mismatched accounts, rate limit retry, all 9 action
      types in loop, delete execution, status transitions from Queued to
      Completed, continuing execution if already Executing, missing action fatal
      error, invalid payload fatal error). All 499 tests pass."
    source: "tester: Tasks 8-15"
  - timestamp: 2025-12-03T09:59:21.995Z
    text: Completed Tasks 8-15 implementing all core Gmail action handlers. Added
      execute_archive, execute_apply_label, execute_remove_label,
      execute_mark_read, execute_mark_unread, execute_star, execute_unstar,
      execute_trash, execute_delete, and execute_restore functions. Updated
      handle_action_gmail to dispatch to appropriate handler based on
      action_type. Added InvalidParameter and UnsupportedAction error variants
      for validation. Added 31 new tests bringing total to 504. All tests pass.
    source: "orchestrator: Tasks 8-15"
  - timestamp: 2025-12-03T10:08:26.787Z
    text: "Completed Tasks 16 and 17: Added 5 missing Gmail client tests
      (trash_message_handles_rate_limit,
      trash_message_retries_after_unauthorized,
      untrash_message_handles_rate_limit,
      untrash_message_retries_after_unauthorized,
      delete_message_retries_after_unauthorized) and 6 worker-level integration
      tests in action_gmail_flow.rs covering archive, mark_read, apply_label,
      trash, delete, and 404 error handling. All 509 tests pass."
    source: "implementer: Tasks 16-17"
  - timestamp: 2025-12-03T10:14:00.696Z
    text: "Completed Tasks 16 and 17 adding comprehensive integration tests for
      Gmail write operations and action execution. Task 16 added 5 new tests to
      client.rs for 429 rate limit and 401 unauthorized retry scenarios:
      trash_message_handles_rate_limit,
      trash_message_retries_after_unauthorized,
      untrash_message_handles_rate_limit,
      untrash_message_retries_after_unauthorized,
      delete_message_retries_after_unauthorized. Task 17 added 6 worker-level
      integration tests in tests/action_gmail_flow.rs:
      worker_executes_archive_action_and_populates_undo_hint,
      worker_executes_mark_read_action_successfully,
      worker_executes_apply_label_action_successfully,
      worker_executes_trash_action_successfully,
      worker_executes_delete_action_with_irreversible_undo_hint,
      worker_marks_action_failed_on_gmail_404. Total test count increased from
      504 to 509 in ashford-core. All new tests pass."
    source: "orchestrator: Tasks 16-17"
  - timestamp: 2025-12-03T10:25:23.760Z
    text: "Reviewed complete implementation of Gmail Actions: Core Operations. All
      17 tasks implemented. Tests pass. Found minor issues: 1) Dead code warning
      for translate_label_name_to_id. 2) Archive action's undo hint uses
      ApplyLabel instead of Move to match classify.rs generate_undo_hint(). No
      critical issues found."
    source: "reviewer: code review"
  - timestamp: 2025-12-03T10:27:51.322Z
    text: "Identified critical issue: handle_action_gmail marks actions Failed even
      when returning retryable JobError (e.g., 401/429), so retries skip
      execution and actions stay failed."
    source: "reviewer: plan22"
  - timestamp: 2025-12-03T10:42:09.721Z
    text: "Addressed review issues: stop marking Gmail actions failed on retryable
      errors and switched pre-image fetch to minimal message format to avoid
      heavy payloads; added missing worker flow tests for
      remove_label/mark_unread/star/unstar/restore."
    source: "implementer: review-fixes"
  - timestamp: 2025-12-03T10:43:45.332Z
    text: Ran cargo test -p ashford-core from server; all 510 unit and integration
      tests currently pass.
    source: "tester: plan22"
tasks:
  - title: Add modify_message method to GmailClient
    done: true
    description: Implement GmailClient::modify_message(message_id, add_labels,
      remove_labels) that calls POST /messages/{id}/modify. Returns the updated
      Message. Add ModifyMessageRequest type to types.rs. The add_labels and
      remove_labels parameters should be Option<Vec<String>> containing Gmail
      label IDs.
  - title: Add trash and untrash methods to GmailClient
    done: true
    description: Implement GmailClient::trash_message(message_id) calling POST
      /messages/{id}/trash and GmailClient::untrash_message(message_id) calling
      POST /messages/{id}/untrash.
  - title: Add delete_message method to GmailClient
    done: true
    description: Implement GmailClient::delete_message(message_id) calling DELETE
      /messages/{id}. Returns Result<(), GmailClientError> since Gmail returns
      204 No Content. Add a new send_empty_response helper method for operations
      that don't return a body. This is permanent deletion - document the danger
      clearly.
  - title: Add missing ActionType variants
    done: true
    description: Add RemoveLabel, Trash, and Restore variants to ActionType enum in
      llm/decision.rs. Update as_str(), from_str(), and danger_level()
      implementations. Trash should be Safe, Restore should be Safe, RemoveLabel
      should be Safe.
  - title: Add GmailClient factory in action_gmail handler
    done: true
    description: "Create helper function to construct GmailClient for action
      execution. Follow the pattern from ingest_gmail.rs: call
      AccountRepository.refresh_tokens_if_needed() before creating GmailClient,
      then use NoopTokenStore. Load account config from AccountRepository using
      action's account_id. Use dispatcher.http for the HTTP client and
      dispatcher.gmail_api_base for API URL override. This pattern will be
      reused by all action handlers."
  - title: Add mark_completed_with_undo_hint method to ActionRepository
    done: true
    description: "The current ActionRepository has mark_completed() but no way to
      update undo_hint_json atomically with status. Add
      mark_completed_with_undo_hint(org_id, user_id, id, undo_hint: Value)
      method that sets status to Completed AND updates undo_hint_json in a
      single database operation. This is needed by all action handlers to store
      pre-image data for undo functionality."
  - title: Implement pre-image capture helper
    done: true
    description: 'Create helper function to fetch current message state from Gmail
      before action execution. Use GmailClient::get_message to retrieve current
      labels. Build undo_hint_json with structure: {"pre_labels": [...],
      "pre_read": bool, "pre_starred": bool, "action": "archive",
      "inverse_action": "apply_label", "inverse_parameters": {...}}. Note:
      Actions store internal message_id but Gmail API needs provider_message_id
      - fetch from MessageRepository first.'
  - title: Implement archive action
    done: true
    description: "In action_gmail handler, implement archive: First fetch message
      from MessageRepository to get provider_message_id. Create GmailClient with
      account tokens from AccountRepository. Capture pre-image labels via
      get_message. Call modify_message to remove INBOX label. Update action's
      undo_hint_json with pre-image data. Mark action completed on success,
      failed on error with error_message."
  - title: Implement apply_label action
    done: true
    description: 'Implement apply_label: extract label ID directly from
      parameters_json["label"] (already translated to Gmail label ID by classify
      job). Call modify_message to add label. Store original labels in undo_hint
      for potential removal.'
  - title: Implement remove_label action
    done: true
    description: 'Implement remove_label: extract label ID directly from
      parameters_json["label"] (already translated to Gmail label ID by classify
      job). Call modify_message to remove label. Store that label was present in
      undo_hint for potential restore.'
  - title: Implement mark_read and mark_unread actions
    done: true
    description: Implement mark_read (remove UNREAD label) and mark_unread (add
      UNREAD label) using modify_message. Capture original read state in
      undo_hint.
  - title: Implement star and unstar actions
    done: true
    description: Implement star (add STARRED label) and unstar (remove STARRED
      label) using modify_message. Simple toggle - store original state in
      undo_hint.
  - title: Implement trash action
    done: true
    description: "Implement trash: call trash_message API method. Store pre-trash
      labels in undo_hint for restore. This is reversible via untrash."
  - title: Implement delete action
    done: true
    description: "Implement delete: call delete_message API method. This is
      PERMANENT and cannot be undone. Set undo_hint to indicate non-reversible.
      Ensure safety policy requires approval for this action."
  - title: Implement restore action
    done: true
    description: "Implement restore: call untrash_message API method. This reverses
      a trash action. Used by undo system."
  - title: Add integration tests for Gmail write operations
    done: true
    description: Add WireMock-based integration tests for all new GmailClient
      methods (modify, trash, untrash, delete). Test success cases, error
      handling (404, 429, 401).
  - title: Add integration tests for action execution
    done: true
    description: Add integration tests for action_gmail handler covering each action
      type. Mock Gmail API responses, verify correct API calls made, verify
      action status transitions and undo_hint population.
changedFiles:
  - docs/data_model.md
  - docs/decision_engine.md
  - docs/gmail_integration.md
  - docs/job_queue.md
  - server/crates/ashford-core/src/decisions/repositories.rs
  - server/crates/ashford-core/src/gmail/client.rs
  - server/crates/ashford-core/src/gmail/types.rs
  - server/crates/ashford-core/src/jobs/action_gmail.rs
  - server/crates/ashford-core/src/jobs/classify.rs
  - server/crates/ashford-core/src/jobs/mod.rs
  - server/crates/ashford-core/src/llm/decision.rs
  - server/crates/ashford-core/tests/action_gmail_flow.rs
tags:
  - actions
  - gmail
  - rust
---

Implement the foundational Gmail action execution system:

## Scope
- Add write operation methods to GmailClient (modify_message, trash, untrash, delete)
- Add missing ActionType variants (RemoveLabel, Trash, Restore)
- Implement action execution in action.gmail job handler for: archive, apply_label, remove_label, mark_read, mark_unread, star, unstar, trash, delete
- Capture pre-action message state for reliable undo hints

## Out of Scope
- Snooze action (separate plan)
- Forward/auto-reply actions (separate plan)
- Undo job handler (separate plan)
- UI changes

## Key Files
- `server/crates/ashford-core/src/gmail/client.rs` - Add write methods
- `server/crates/ashford-core/src/jobs/action_gmail.rs` - Implement action execution
- `server/crates/ashford-core/src/llm/decision.rs` - Add ActionType variants

## Research

### Summary
- The Gmail client (`gmail/client.rs`) has a well-established pattern for API calls with automatic token refresh, retry on 401, and comprehensive error handling. New write methods (modify_message, trash, untrash, delete) follow the existing `send_json` pattern with minor variations for POST bodies and empty responses.
- The action_gmail job handler is currently a **stub** that only manages status transitions (Queued → Executing → Completed) without performing actual Gmail mutations. The infrastructure for action execution is ready - we need to add the actual Gmail API calls.
- ActionType enum has 15 variants with clear patterns for `as_str()`, `FromStr`, and `danger_level()`. Adding RemoveLabel, Trash, and Restore requires updating 4 methods and 3 test assertions.
- The undo_hint_json field exists in the Action struct but is not actively consumed yet - it's a placeholder for undo functionality.
- Testing uses WireMock for HTTP mocking, SQLite with TempDir for database isolation, and tokio::test for async tests. Patterns are well-established and should be followed.

### Findings

#### Gmail Client Analysis (`server/crates/ashford-core/src/gmail/client.rs`)

**Current Structure:**
- `GmailClient<S: TokenStore>` is generic over token storage
- Uses `RwLock<OAuthTokens>` for thread-safe token access
- `Mutex<()>` refresh_lock prevents concurrent token refreshes
- All read operations use GET with `send_json()` helper

**Existing Methods:**
- `get_message(message_id)` → GET `/messages/{id}?format=full`
- `get_thread(thread_id)` → GET `/threads/{id}?format=full`
- `list_history(start_history_id, page_token, max_results)` → GET `/history`
- `list_messages(query, page_token, include_spam_trash, max_results)` → GET `/messages`
- `get_profile()` → GET `/profile`

**Key Helper Methods:**
- `send_json<T, B>(&self, build: B)` - Performs authenticated request and deserializes JSON response
- `perform_authenticated<B>(&self, build)` - Handles auth token and 401 retry logic
- `ensure_fresh_token(force_refresh)` - Proactive token refresh with 5-min buffer

**Error Handling:**
```rust
pub enum GmailClientError {
    Http(reqwest::Error),      // Includes status codes via error.status()
    OAuth(OAuthError),         // Token refresh failures
    TokenStore(String),        // Custom token store errors
    Decode(serde_json::Error), // JSON parsing errors
    Unauthorized,              // 401 after successful refresh
}
```

**Patterns for New Write Methods:**

1. **`modify_message`** - POST with JSON body:
```rust
pub async fn modify_message(
    &self,
    message_id: &str,
    add_labels: Option<Vec<String>>,
    remove_labels: Option<Vec<String>>,
) -> Result<Message, GmailClientError>
// POST /messages/{id}/modify with body {"addLabelIds": [...], "removeLabelIds": [...]}
```

2. **`trash_message`** / **`untrash_message`** - POST with empty body:
```rust
pub async fn trash_message(&self, message_id: &str) -> Result<Message, GmailClientError>
// POST /messages/{id}/trash (no body)
```

3. **`delete_message`** - DELETE with no response body:
```rust
pub async fn delete_message(&self, message_id: &str) -> Result<(), GmailClientError>
// DELETE /messages/{id} - returns 204 No Content
```

**Need to add:**
- `ModifyMessageRequest` struct in `types.rs` for the request body
- New helper `send_empty_response<B>(&self, build)` for delete operation that returns no body

#### Action Gmail Job Handler (`server/crates/ashford-core/src/jobs/action_gmail.rs`)

**Current Implementation (STUB):**
```rust
pub const JOB_TYPE: &str = "action.gmail";

#[derive(Debug, Deserialize)]
struct ActionJobPayload {
    pub account_id: String,
    pub action_id: String,
}
```

The handler currently:
1. Deserializes payload with `account_id` and `action_id`
2. Loads action from `ActionRepository`
3. Validates action belongs to specified account
4. Transitions status: Queued → Executing → Completed (without actual Gmail calls)
5. Returns success

**Comment in code (lines 18-20):**
> "This is currently a placeholder that marks the action as executed so the pipeline can continue; provider-side mutations will be implemented in a later phase."

**Required Changes:**
1. After marking as Executing, dispatch to action-specific handlers based on `action.action_type`
2. Each handler should:
   - Fetch current message state for pre-image capture
   - Perform Gmail API mutation
   - Populate `undo_hint_json` with reverse operation info
   - Mark completed or failed based on result

**Error Mapping (from `jobs/mod.rs`):**
```rust
pub(crate) fn map_gmail_error(context: &str, err: GmailClientError) -> JobError {
    match err {
        GmailClientError::Unauthorized => JobError::retryable(...),
        GmailClientError::Http(ref http_err) => {
            match status {
                StatusCode::NOT_FOUND => JobError::Fatal(...),
                StatusCode::TOO_MANY_REQUESTS | StatusCode::FORBIDDEN => JobError::retryable(...),
                status if status.is_server_error() => JobError::retryable(...),
                status => JobError::Fatal(...),
            }
        }
        GmailClientError::OAuth(err) => JobError::retryable(...),
        GmailClientError::TokenStore(err) => JobError::Fatal(...),
        GmailClientError::Decode(err) => JobError::Fatal(...),
    }
}
```

**JobDispatcher provides:**
- `db`: Database connection
- `http`: HTTP client for Gmail API calls
- `gmail_api_base`: Optional API base URL override
- `llm_client`: LLM client (not needed for actions)
- `policy_config`: Safety policy configuration

#### ActionType Enum (`server/crates/ashford-core/src/llm/decision.rs`)

**Current Variants (15 total):**
```rust
pub enum ActionType {
    ApplyLabel, MarkRead, MarkUnread, Archive, Delete, Move,
    Star, Unstar, Forward, AutoReply, CreateTask, Snooze,
    AddNote, Escalate, None,
}
```

**Methods to Update:**

1. **Add to enum** (line 13-29): Add `RemoveLabel`, `Trash`, `Restore`

2. **`as_str()`** (lines 32-50): Add:
   - `ActionType::RemoveLabel => "remove_label"`
   - `ActionType::Trash => "trash"`
   - `ActionType::Restore => "restore"`

3. **`FromStr`** (lines 86-104): Add:
   - `"remove_label" => Ok(Self::RemoveLabel)`
   - `"trash" => Ok(Self::Trash)`
   - `"restore" => Ok(Self::Restore)`

4. **`danger_level()`** (lines 57-80): Add all three to Safe category:
   - RemoveLabel, Trash, Restore → `ActionDangerLevel::Safe`

**Test Updates Required:**
- `action_type_round_trips` (line 392-416): Add new variants to iteration
- `action_type_danger_level_classifications` (line 634-681): Add to Safe category test
- `all_action_types_have_danger_level` (line 685-712): Update count from 15 to 18

#### Action Repository and Undo Hints (`server/crates/ashford-core/src/decisions/`)

**Action struct** (`types.rs` lines 132-148):
```rust
pub struct Action {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub decision_id: Option<String>,
    pub action_type: String,          // snake_case action name
    pub parameters_json: Value,       // Action-specific params
    pub status: ActionStatus,
    pub error_message: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    pub undo_hint_json: Value,        // Reverse operation info
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Status State Machine** (valid transitions):
- `Queued` → `Executing`, `Canceled`, `Rejected`, `ApprovedPending`, `Failed`
- `Executing` → `Completed`, `Failed`, `Canceled`
- `ApprovedPending` → `Queued`, `Canceled`, `Rejected`
- Terminal states (`Completed`, `Failed`, `Canceled`, `Rejected`) → no transitions

**Repository Methods:**
- `mark_executing(org_id, user_id, id)` - Sets status to Executing, sets executed_at
- `mark_completed(org_id, user_id, id)` - Sets status to Completed
- `mark_failed(org_id, user_id, id, error_message)` - Sets status to Failed with error

**Undo Hint Structure** (from `llm/decision.rs`):
```rust
pub struct UndoHint {
    pub inverse_action: ActionType,
    pub inverse_parameters: Value,
}
```

**Pre-image Capture Strategy:**
For reliable undo, capture current state before mutation:
```json
{
  "pre_labels": ["INBOX", "IMPORTANT"],
  "action": "archive",
  "inverse_action": "apply_label",
  "inverse_parameters": {"label": "INBOX"}
}
```

#### Testing Patterns

**Test Infrastructure:**
- **WireMock** for HTTP mocking (version 0.6.5)
- **tokio::test** for async tests
- **tempfile::TempDir** for isolated SQLite databases
- **uuid::Uuid::new_v4()** for unique database names

**Database Setup Pattern:**
```rust
async fn setup_db() -> (Database, TempDir) {
    let dir = TempDir::new().expect("temp dir");
    let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
    let db_path = dir.path().join(db_name);
    let db = Database::new(&db_path).await.expect("create db");
    run_migrations(&db).await.expect("migrations");
    (db, dir)
}
```

**WireMock Pattern for Gmail API:**
```rust
let server = MockServer::start().await;

Mock::given(method("POST"))
    .and(path("/gmail/v1/users/me/messages/msg123/modify"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "id": "msg123",
        "labelIds": ["INBOX", "IMPORTANT"],
    })))
    .expect(1)
    .mount(&server)
    .await;

let client = make_client(&server, tokens, store);
```

**Error Handling Tests:**
- Test 404 responses → `GmailClientError::Http` with NOT_FOUND status
- Test 429 responses → `GmailClientError::Http` with TOO_MANY_REQUESTS status
- Test 401 responses → Trigger refresh + retry, then Unauthorized if still fails
- Test invalid JSON → `GmailClientError::Decode`

**MockLLMClient** (`llm/mock.rs`):
- Queue-based mock with `enqueue_response()`
- Call count tracking via `AtomicUsize`
- Used for testing classify job, not needed for action execution

#### ActionRepository Gap: Missing update_undo_hint Method

The current `ActionRepository` (`decisions/repositories.rs`) has these status update methods:
- `mark_executing(org_id, user_id, id)` - Sets status to Executing, sets executed_at
- `mark_completed(org_id, user_id, id)` - Sets status to Completed
- `mark_failed(org_id, user_id, id, error_message)` - Sets status to Failed with error

**Missing Method Needed:**
The repository currently has no way to update `undo_hint_json` after action execution. We need to add:
- `mark_completed_with_undo_hint(org_id, user_id, id, undo_hint: Value)` - Sets status to Completed AND updates undo_hint_json

Alternatively, we could add a dedicated `update_undo_hint(org_id, user_id, id, undo_hint: Value)` method that can be called before `mark_completed`, but a combined method is more atomic and efficient.

#### TokenStore Implementation for Job Handler

When creating GmailClient in the job handler, we need a TokenStore implementation that:
1. Receives refreshed tokens from GmailClient's auto-refresh mechanism
2. Persists them back to the database via AccountRepository

The existing `NoopTokenStore` in ingest_gmail.rs doesn't persist tokens. For action execution, we should either:
- Use `NoopTokenStore` (simpler, relies on AccountRepository.refresh_tokens_if_needed before creating client)
- Create an `AccountTokenStore` that wraps AccountRepository for persistence

The `refresh_tokens_if_needed` approach is already used in `ingest_gmail.rs` and `history_sync_gmail.rs`, so this pattern should be followed.

### Risks & Constraints

1. **Gmail API Rate Limits**: The Gmail API has quotas (250 units/user/second). Mutation operations consume quota. Error mapping already handles 429 with retryable errors.

2. **Token Expiration During Execution**: The existing token refresh mechanism handles this automatically via `ensure_fresh_token()` with 401 retry.

3. **Permanent Delete is Irreversible**: The `delete_message` operation permanently removes the message. Safety policy should always require approval for Delete action type (already classified as Dangerous in `danger_level()`).

4. **Pre-image Race Condition**: Between fetching pre-image and applying action, message could change. Acceptable risk for undo hints - they're best-effort.

5. **Label Creation**: `apply_label` may need to create a new label if it doesn't exist. Gmail's `labels.create` API endpoint may be needed. Consider deferring label creation to a separate task or documenting as out-of-scope for initial implementation. Note: Plan 25 (Gmail Label Management System) syncs labels from Gmail but doesn't create them - labels must exist in Gmail first.

6. **Message Not Found**: If message was deleted externally, action should fail with Fatal error (handled by error mapping for 404).

7. **Account Token Storage**: GmailClient requires token store. In job handler, use the pattern from `ingest_gmail.rs`: call `AccountRepository.refresh_tokens_if_needed()` before creating GmailClient, then use `NoopTokenStore`.

8. **Idempotency**: Actions already have idempotency via status checks. If action is already Completed, handler returns Ok early.

9. **Testing Gmail Write Operations**: All tests should use WireMock mocks - never call real Gmail API in tests. Integration tests are feature-gated behind `llm-integration` flag.

10. **Labels Table Dependency**: This plan depends on Plan 25 (Gmail Label Management System) which implements the labels table and repository. The `apply_label` and `remove_label` actions will use `LabelRepository::get_by_provider_id()` to translate label names to IDs.

## Expected Behavior/Outcome

After implementation, the system will:
1. **Execute Gmail actions** when jobs are processed - actions created by the classify job will result in actual Gmail API mutations
2. **Capture pre-images** before each mutation to enable future undo functionality
3. **Handle all core action types**: archive, apply_label, remove_label, mark_read, mark_unread, star, unstar, trash, delete, restore
4. **Properly transition action status** through Queued → Executing → Completed/Failed based on API results
5. **Store undo hints** in the action record containing the inverse operation and pre-mutation state

**Action Behavior Matrix:**

| Action | Gmail API Call | Pre-image Captured | Undo Possible |
|--------|---------------|-------------------|---------------|
| archive | modify_message (remove INBOX) | Current labels | Yes - restore INBOX label |
| apply_label | modify_message (add label) | Current labels | Yes - remove label |
| remove_label | modify_message (remove label) | Current labels | Yes - add label back |
| mark_read | modify_message (remove UNREAD) | Read state | Yes - add UNREAD |
| mark_unread | modify_message (add UNREAD) | Read state | Yes - remove UNREAD |
| star | modify_message (add STARRED) | Star state | Yes - remove STARRED |
| unstar | modify_message (remove STARRED) | Star state | Yes - add STARRED |
| trash | trash_message | Pre-trash labels | Yes - untrash |
| delete | delete_message | None (irreversible) | No |
| restore | untrash_message | N/A | N/A |

## Acceptance Criteria

### Functional Criteria
- [ ] GmailClient has `modify_message(message_id, add_labels, remove_labels)` method
- [ ] GmailClient has `trash_message(message_id)` and `untrash_message(message_id)` methods
- [ ] GmailClient has `delete_message(message_id)` method
- [ ] ActionType enum includes RemoveLabel, Trash, and Restore variants
- [ ] action_gmail job handler executes actual Gmail API calls based on action_type
- [ ] Pre-image (current message labels/state) is captured before mutations
- [ ] undo_hint_json is populated with inverse operation info after successful execution
- [ ] Actions transition to Completed on success, Failed on error
- [ ] Failed actions include error_message describing the failure

### Technical Criteria
- [ ] All new GmailClient methods follow existing authentication/retry patterns
- [ ] Error mapping (map_gmail_error) correctly classifies errors as retryable vs fatal
- [ ] ActionType danger_level() returns Safe for RemoveLabel, Trash, Restore
- [ ] All code paths have unit tests with WireMock mocks
- [ ] Integration tests verify end-to-end action execution flow

## Dependencies & Constraints

**Dependencies:**
- Existing GmailClient authentication and token refresh infrastructure
- ActionRepository for loading/updating action records
- AccountRepository for fetching account credentials
- JobDispatcher infrastructure for accessing database and HTTP client

**Technical Constraints:**
- Must use existing `send_json` pattern for authenticated requests
- Delete operation returns 204 No Content (needs new helper for empty responses)
- All Gmail API calls must go through authenticated client (no direct reqwest calls)
- Pre-image capture adds one extra API call per action execution

## Implementation Notes

### Recommended Approach

**Phase 1: GmailClient Write Methods**
1. Add `ModifyMessageRequest` struct to `types.rs`
2. Add `send_empty_response` helper for delete operation
3. Implement `modify_message`, `trash_message`, `untrash_message`, `delete_message`
4. Add comprehensive WireMock tests for each method

**Phase 2: ActionType Variants**
1. Add RemoveLabel, Trash, Restore to enum
2. Update as_str(), FromStr, danger_level()
3. Update tests with new variants

**Phase 3: Pre-image Capture Helper**
1. Create helper function that fetches current message state
2. Returns struct with current labels, read status for undo hints
3. Called before each mutation in action handlers

**Phase 4: Action Handlers**
1. Refactor action_gmail.rs to dispatch based on action_type
2. Implement each action handler (archive, apply_label, etc.)
3. Each handler: capture pre-image → execute mutation → build undo_hint
4. Handle errors appropriately (mark_failed with message)

**Phase 5: Integration Tests**
1. Add tests for action execution flow with mocked Gmail API
2. Verify status transitions and undo_hint population

### Potential Gotchas

1. **GmailClient Creation**: The job handler needs to construct a GmailClient with account-specific tokens. Use AccountRepository to fetch config, then create client with a token store that saves back to the database.

2. **Message ID vs Provider Message ID**: Actions store `message_id` (internal UUID), but Gmail API needs `provider_message_id`. Need to fetch message record to get the Gmail message ID.

3. **Empty POST Bodies**: For trash/untrash, use `.post(&url)` without `.json()` - reqwest sends empty body.

4. **Delete Response Parsing**: Gmail delete returns 204 No Content. Don't try to parse JSON from empty response.

5. **Label Names vs IDs**: Gmail API uses label IDs internally. For user labels, may need to look up or create label. System labels like INBOX, UNREAD, STARRED are well-known constants.

## Tasks 1-4: GmailClient Write Methods and ActionType Variants

### Task 1: Add modify_message method to GmailClient
**Files Modified:**
- `server/crates/ashford-core/src/gmail/client.rs`
- `server/crates/ashford-core/src/gmail/types.rs`

Implemented `GmailClient::modify_message(message_id, add_labels, remove_labels)` that calls `POST /messages/{id}/modify`. The method follows the existing `send_json` pattern with authentication/retry handling. Added `ModifyMessageRequest` struct in types.rs with `#[serde(skip_serializing_if = "Option::is_none")]` to omit null fields in the JSON body, matching Gmail API expectations. The method accepts `Option<Vec<String>>` for both add and remove label IDs, allowing flexible label management.

### Task 2: Add trash and untrash methods to GmailClient
**Files Modified:**
- `server/crates/ashford-core/src/gmail/client.rs`

Implemented `trash_message(message_id)` calling `POST /messages/{id}/trash` and `untrash_message(message_id)` calling `POST /messages/{id}/untrash`. Both methods use `.post(&url).body("")` to send an empty body (required by Gmail API for these endpoints), then use `send_json` pattern for authentication. Both return the updated `Message` struct.

### Task 3: Add delete_message method to GmailClient
**Files Modified:**
- `server/crates/ashford-core/src/gmail/client.rs`

Implemented `delete_message(message_id)` calling `DELETE /messages/{id}`. Since Gmail returns 204 No Content, added a new `send_empty_response` helper method that performs authentication but doesn't parse response JSON. The method returns `Result<(), GmailClientError>`. Added extensive documentation warning about permanent deletion and irreversibility.

### Task 4: Add missing ActionType variants
**Files Modified:**
- `server/crates/ashford-core/src/llm/decision.rs`
- `server/crates/ashford-core/src/jobs/classify.rs`

Added three new variants to ActionType enum: `RemoveLabel`, `Trash`, and `Restore`. Updated:
- `as_str()`: returns "remove_label", "trash", "restore"
- `FromStr`: parses the snake_case strings
- `danger_level()`: all three classified as `ActionDangerLevel::Safe`
- `generate_undo_hint()` in classify.rs: added mappings for new types (RemoveLabel -> ApplyLabel, Trash -> Restore, Restore -> Trash)

Updated test assertions to reflect 18 total action types (up from 15).

### Tests Added
Added 21 new tests with WireMock-based integration tests:
- `modify_message_*`: 7 tests covering add/remove labels, error handling, request body verification, 401 retry
- `trash_message_*`: 2 tests for success and 404 error handling
- `untrash_message_*`: 2 tests for success and 404 error handling  
- `delete_message_*`: 3 tests covering 204 response, 404, and 429 rate limiting
- `ModifyMessageRequest` serialization: 6 tests in types.rs for JSON serialization

All 446 tests pass.

## Tasks 5-7: Infrastructure for Action Handlers

### Task 5: GmailClient Factory (`action_gmail.rs`)

Implemented `create_gmail_client(dispatcher: &JobDispatcher, account_id: &str) -> Result<GmailClient<NoopTokenStore>, JobError>` that:
1. Creates an AccountRepository from the dispatcher's database
2. Calls `refresh_tokens_if_needed()` to ensure fresh OAuth tokens before client creation
3. Creates GmailClient with NoopTokenStore (tokens are already refreshed)
4. Configures api_base from dispatcher.gmail_api_base or defaults to Gmail API URL

This follows the exact pattern from ingest_gmail.rs, ensuring consistent token management across all Gmail operations.

### Task 6: mark_completed_with_undo_hint (`repositories.rs`)

Added `ActionRepository::mark_completed_with_undo_hint(org_id, user_id, id, undo_hint: Value) -> Result<Action, ActionError>` that:
1. Validates the action exists and is in Executing status
2. Updates both status to Completed AND undo_hint_json in a single atomic SQL UPDATE
3. Uses COALESCE to preserve executed_at if already set
4. Serializes the undo_hint Value to JSON string for database storage
5. Returns the updated Action record

This enables action handlers to store pre-image data for undo functionality in one atomic operation.

### Task 7: Pre-image Capture Helper (`action_gmail.rs`)

Implemented several related components:

1. **PreImageState struct** - Captures Gmail message state with fields:
   - `labels: Vec<String>` - All current label IDs
   - `is_unread`, `is_starred`, `is_in_inbox`, `is_in_trash` - Derived boolean flags

2. **PreImageState::from_labels(labels: &[String])** - Creates PreImageState from label list, detecting system labels (UNREAD, STARRED, INBOX, TRASH)

3. **PreImageState::build_undo_hint(action_type, inverse_action_type, inverse_parameters)** - Builds JSON undo hint containing:
   - `pre_labels`, `pre_unread`, `pre_starred`, `pre_in_inbox`, `pre_in_trash`
   - `action`, `inverse_action`, `inverse_parameters`

4. **capture_pre_image(client, provider_message_id)** - Async function that fetches current message state from Gmail API via GmailClient::get_message

5. **get_provider_message_id(dispatcher, account_id, internal_message_id)** - Translates internal message UUID to Gmail provider_message_id via MessageRepository

### Files Modified
- `server/crates/ashford-core/src/jobs/action_gmail.rs` - Added create_gmail_client, PreImageState, capture_pre_image, get_provider_message_id
- `server/crates/ashford-core/src/decisions/repositories.rs` - Added mark_completed_with_undo_hint method

### Tests Added (28 total)
- 18 unit tests for PreImageState (label detection, undo hint building for all action types)
- 4 integration tests for create_gmail_client (API base configuration, error handling)
- 4 integration tests for capture_pre_image and get_provider_message_id (WireMock-based)
- 2 repository tests for mark_completed_with_undo_hint

## Tasks 8-15: Action Handler Implementations

Implemented all core Gmail action handlers in `server/crates/ashford-core/src/jobs/action_gmail.rs`.

### New Functions Added

1. **`execute_archive`** (Task 8) - Removes the INBOX label from a message using `modify_message`. Captures pre-image state for undo.

2. **`execute_apply_label`** (Task 9) - Adds a label to a message. Extracts label ID from `parameters_json["label"]` with validation for non-empty values. Returns `GmailClientError::InvalidParameter` if label is missing or empty.

3. **`execute_remove_label`** (Task 10) - Removes a label from a message. Same validation as apply_label.

4. **`execute_mark_read`** (Task 11) - Removes the UNREAD label.

5. **`execute_mark_unread`** (Task 11) - Adds the UNREAD label.

6. **`execute_star`** (Task 12) - Adds the STARRED label.

7. **`execute_unstar`** (Task 12) - Removes the STARRED label.

8. **`execute_trash`** (Task 13) - Calls `trash_message` API method. Stores pre-trash labels for potential restore.

9. **`execute_delete`** (Task 14) - Calls `delete_message` API method. Permanent deletion - no pre-image capture, undo hint marked as irreversible.

10. **`execute_restore`** (Task 15) - Calls `untrash_message` API method. Used to reverse trash actions.

### Supporting Infrastructure

- **`ActionExecutionResult`** struct - Contains the undo_hint JSON for each action
- **`execute_action`** dispatcher - Routes to appropriate handler based on action_type string

### Updated `handle_action_gmail`

Completely rewrote the main handler to:
1. Load and validate the action from database
2. Check status and handle terminal states (Completed, Failed, Canceled, Rejected, ApprovedPending)
3. Mark action as Executing
4. Get provider_message_id from internal message record
5. Create Gmail client via `create_gmail_client` factory
6. Execute action via `execute_action` dispatcher
7. On success: call `mark_completed_with_undo_hint` with undo hint
8. On failure: call `mark_failed` with error message

### Error Handling Enhancements

Added two new error variants to `GmailClientError`:
- `InvalidParameter(String)` - For missing or invalid action parameters (e.g., empty label)
- `UnsupportedAction(String)` - For action types not yet implemented

Both map to `JobError::Fatal` in `map_gmail_error`.

### Files Modified
- `server/crates/ashford-core/src/jobs/action_gmail.rs` - Main implementation
- `server/crates/ashford-core/src/gmail/client.rs` - Added InvalidParameter and UnsupportedAction error variants
- `server/crates/ashford-core/src/jobs/mod.rs` - Updated map_gmail_error for new error types

### Tests Added
Added 31 new tests covering:
- Individual action handler tests (10 tests for each action type)
- Execute action dispatcher tests (3 tests)
- Handle action gmail handler tests (12 end-to-end tests)
- Parameter validation tests (6 tests for label validation)

Total test count increased from 474 to 504. All tests pass.

## Tasks 16-17: Integration Tests for Gmail Write Operations and Action Execution

Completed the final tasks for plan 22 by adding comprehensive integration tests for Gmail write operations and action execution.

### Task 16: Add integration tests for Gmail write operations
**Files Modified:**
- `server/crates/ashford-core/src/gmail/client.rs`

Added 5 new WireMock-based integration tests to cover error handling scenarios that were missing:

1. **`trash_message_handles_rate_limit`** - Tests that `trash_message` returns 429 TOO_MANY_REQUESTS error properly
2. **`trash_message_retries_after_unauthorized`** - Tests that `trash_message` retries with fresh token after 401
3. **`untrash_message_handles_rate_limit`** - Tests that `untrash_message` returns 429 error properly  
4. **`untrash_message_retries_after_unauthorized`** - Tests that `untrash_message` retries after 401
5. **`delete_message_retries_after_unauthorized`** - Tests that `delete_message` retries after 401

The 401 retry tests follow the established pattern from `modify_message_retries_after_unauthorized`: mock the first request to return 401, mock the token refresh endpoint, then mock the retry request to succeed. This verifies the automatic token refresh and retry behavior works correctly.

**Complete Coverage Matrix for Gmail Write Methods:**
| Method | Success | 404 | 429 | 401 Retry |
|--------|---------|-----|-----|-----------|
| modify_message | 5 tests | Yes | N/A | Yes |
| trash_message | Yes | Yes | Yes | Yes |
| untrash_message | Yes | Yes | Yes | Yes |
| delete_message | Yes | Yes | Yes | Yes |

### Task 17: Add integration tests for action execution
**Files Created:**
- `server/crates/ashford-core/tests/action_gmail_flow.rs`

Added a new integration test file following the pattern from `ingest_flow.rs`. These tests run through the complete job worker pipeline:

1. **`worker_executes_archive_action_and_populates_undo_hint`** - Tests end-to-end archive execution with full undo_hint verification including pre-image capture
2. **`worker_executes_mark_read_action_successfully`** - Tests mark_read action removes UNREAD label
3. **`worker_executes_apply_label_action_successfully`** - Tests apply_label with parameters extraction and label addition
4. **`worker_executes_trash_action_successfully`** - Tests trash action with pre-trash state in undo_hint
5. **`worker_executes_delete_action_with_irreversible_undo_hint`** - Tests delete marks undo as irreversible
6. **`worker_marks_action_failed_on_gmail_404`** - Tests proper error handling when Gmail returns 404

**Key Infrastructure:**
- Uses `fast_worker_config()` with 5ms poll interval for quick test execution
- Uses `run_worker` to start actual job processing loop
- Uses WireMock for Gmail API mocking
- Uses `timeout()` with polling to wait for action completion
- Isolated SQLite databases with TempDir

### Test Results
Total tests in ashford-core increased from 504 to 509 (includes 36 Gmail client tests and 56 action_gmail tests, plus 6 new integration tests). All tests pass.

Implemented review fixes for plan 22. Issue 1: handle_action_gmail now maps Gmail errors to JobError first and only marks actions failed for fatal errors, leaving retryable failures in Executing so the worker can retry; updated unit test to assert status remains Executing on rate limits. Issue 2: added GmailClient::get_message_minimal plus internal get_message_with_format helper; capture_pre_image now uses the minimal format to fetch only labels and avoid downloading full payloads, with tests updated to assert the minimal query param. Issue 3: expanded worker flow coverage by adding end-to-end tests for remove_label, mark_unread, star, unstar, and restore actions; each test mocks Gmail calls, runs the worker, and checks undo hints and status transitions. Also added a client test for get_message_minimal. Files touched: gmail/client.rs (new methods and tests), jobs/action_gmail.rs (retry handling, minimal pre-image fetch, test updates), tests/action_gmail_flow.rs (new worker integration cases). Kept existing functionality intact and reran targeted cargo tests (action_gmail suite and new star flow) to verify changes.

Addressed the reviewer-raised stuck Executing issue for Gmail actions. Updated server/crates/ashford-core/src/jobs/action_gmail.rs so handle_action_gmail now marks the action Failed when a Gmail error is fatal or when retryable errors occur on the final allowed attempt (job.attempts >= job.max_attempts), preventing actions from remaining Executing after the job is terminal. Added a new integration test handle_action_gmail_marks_failed_when_retry_exhausted in the same file to simulate a final-attempt 429 rate limit and assert the action transitions to Failed with an error message. Kept retryable errors unchanged for intermediate attempts to preserve existing retry semantics; the change is minimal and isolated to the action handler logic for the Gmail Actions: Core Operations task.
