---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Actions: Core Operations"
goal: Implement Gmail API write operations and core action execution (archive,
  labels, read state, star, trash, delete) with pre-image capture for undo
  support
id: 22
uuid: c69a5bba-4a08-4a49-b841-03d396a6ba81
generatedBy: agent
status: pending
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
updatedAt: 2025-12-03T02:40:00.270Z
progressNotes: []
tasks:
  - title: Add modify_message method to GmailClient
    done: false
    description: Implement GmailClient::modify_message(message_id, add_labels,
      remove_labels) that calls POST /messages/{id}/modify. Returns the updated
      Message. Add ModifyMessageRequest type to types.rs. The add_labels and
      remove_labels parameters should be Option<Vec<String>> containing Gmail
      label IDs.
  - title: Add trash and untrash methods to GmailClient
    done: false
    description: Implement GmailClient::trash_message(message_id) calling POST
      /messages/{id}/trash and GmailClient::untrash_message(message_id) calling
      POST /messages/{id}/untrash.
  - title: Add delete_message method to GmailClient
    done: false
    description: Implement GmailClient::delete_message(message_id) calling DELETE
      /messages/{id}. Returns Result<(), GmailClientError> since Gmail returns
      204 No Content. Add a new send_empty_response helper method for operations
      that don't return a body. This is permanent deletion - document the danger
      clearly.
  - title: Add missing ActionType variants
    done: false
    description: Add RemoveLabel, Trash, and Restore variants to ActionType enum in
      llm/decision.rs. Update as_str(), from_str(), and danger_level()
      implementations. Trash should be Safe, Restore should be Safe, RemoveLabel
      should be Safe.
  - title: Implement pre-image capture helper
    done: false
    description: 'Create helper function to fetch current message state from Gmail
      before action execution. Use GmailClient::get_message to retrieve current
      labels. Build undo_hint_json with structure: {"pre_labels": [...],
      "pre_read": bool, "pre_starred": bool, "action": "archive",
      "inverse_action": "apply_label", "inverse_parameters": {...}}. Note:
      Actions store internal message_id but Gmail API needs provider_message_id
      - fetch from MessageRepository first.'
  - title: Implement archive action
    done: false
    description: "In action_gmail handler, implement archive: First fetch message
      from MessageRepository to get provider_message_id. Create GmailClient with
      account tokens from AccountRepository. Capture pre-image labels via
      get_message. Call modify_message to remove INBOX label. Update action's
      undo_hint_json with pre-image data. Mark action completed on success,
      failed on error with error_message."
  - title: Implement apply_label action
    done: false
    description: "Implement apply_label: extract label ID from parameters_json and
      call modify_message to add label. The label ID comes from the labels table
      (implemented in plan 25). Store original labels in undo_hint for potential
      removal."
  - title: Implement remove_label action
    done: false
    description: "Implement remove_label: extract label ID from parameters_json and
      call modify_message to remove label. The label ID comes from the labels
      table (implemented in plan 25). Store that label was present in undo_hint
      for potential restore."
  - title: Implement mark_read and mark_unread actions
    done: false
    description: Implement mark_read (remove UNREAD label) and mark_unread (add
      UNREAD label) using modify_message. Capture original read state in
      undo_hint.
  - title: Implement star and unstar actions
    done: false
    description: Implement star (add STARRED label) and unstar (remove STARRED
      label) using modify_message. Simple toggle - store original state in
      undo_hint.
  - title: Implement trash action
    done: false
    description: "Implement trash: call trash_message API method. Store pre-trash
      labels in undo_hint for restore. This is reversible via untrash."
  - title: Implement delete action
    done: false
    description: "Implement delete: call delete_message API method. This is
      PERMANENT and cannot be undone. Set undo_hint to indicate non-reversible.
      Ensure safety policy requires approval for this action."
  - title: Implement restore action
    done: false
    description: "Implement restore: call untrash_message API method. This reverses
      a trash action. Used by undo system."
  - title: Add integration tests for Gmail write operations
    done: false
    description: Add WireMock-based integration tests for all new GmailClient
      methods (modify, trash, untrash, delete). Test success cases, error
      handling (404, 429, 401).
  - title: Add integration tests for action execution
    done: false
    description: Add integration tests for action_gmail handler covering each action
      type. Mock Gmail API responses, verify correct API calls made, verify
      action status transitions and undo_hint population.
  - title: Add GmailClient factory in action_gmail handler
    done: false
    description: Create helper function to construct GmailClient for action
      execution. Load account config from AccountRepository using action's
      account_id. Extract OAuth tokens and create client with appropriate token
      store that persists refreshed tokens back to the database. Use
      dispatcher.http for the HTTP client and dispatcher.gmail_api_base for API
      URL override. This pattern will be reused by all action handlers.
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

### Risks & Constraints

1. **Gmail API Rate Limits**: The Gmail API has quotas (250 units/user/second). Mutation operations consume quota. Error mapping already handles 429 with retryable errors.

2. **Token Expiration During Execution**: The existing token refresh mechanism handles this automatically via `ensure_fresh_token()` with 401 retry.

3. **Permanent Delete is Irreversible**: The `delete_message` operation permanently removes the message. Safety policy should always require approval for Delete action type (already classified as Dangerous in `danger_level()`).

4. **Pre-image Race Condition**: Between fetching pre-image and applying action, message could change. Acceptable risk for undo hints - they're best-effort.

5. **Label Creation**: `apply_label` may need to create a new label if it doesn't exist. Gmail's `labels.create` API endpoint may be needed. Consider deferring label creation to a separate task or documenting as out-of-scope for initial implementation.

6. **Message Not Found**: If message was deleted externally, action should fail with Fatal error (handled by error mapping for 404).

7. **Account Token Storage**: GmailClient requires token store. In job handler, need to create client with account-specific tokens from AccountRepository.

8. **Idempotency**: Actions already have idempotency via status checks. If action is already Completed, handler returns Ok early.

9. **Testing Gmail Write Operations**: All tests should use WireMock mocks - never call real Gmail API in tests. Integration tests are feature-gated behind `llm-integration` flag.

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
