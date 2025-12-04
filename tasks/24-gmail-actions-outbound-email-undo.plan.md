---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Actions: Outbound Email"
goal: Implement forward/auto-reply actions via outbound.send job with MIME
  message construction
id: 24
uuid: e3c7d618-82e3-4835-9f9c-441d596c2fc1
generatedBy: agent
status: in_progress
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
  - docs/data_model.md
planGeneratedAt: 2025-12-03T02:21:59.017Z
promptsGeneratedAt: 2025-12-03T02:21:59.017Z
createdAt: 2025-12-03T02:21:14.787Z
updatedAt: 2025-12-04T21:32:32.753Z
progressNotes: []
tasks:
  - title: Add send_message method to GmailClient
    done: false
    description: Implement GmailClient::send_message(raw_message) calling POST
      /messages/send with base64url-encoded RFC 2822 message. Add
      SendMessageRequest and SendMessageResponse types.
  - title: Create MIME message builder
    done: false
    description: "Add mail-builder crate dependency. Create MimeBuilder wrapper
      utility for constructing RFC 2822 email messages. Support: To/From/Subject
      headers, plain text and HTML body, In-Reply-To and References headers for
      threading, attachments. Output base64url-encoded string for Gmail API."
  - title: Create outbound.send job type
    done: false
    description: "Create new job type 'outbound.send' with handler in
      jobs/outbound_send.rs. Payload: {account_id, action_id, message_type:
      'forward'|'reply', to, subject, body, original_message_id, attachments}.
      Export JOB_TYPE_OUTBOUND_SEND constant."
  - title: Implement forward action
    done: false
    description: "In action_gmail, implement forward: extract recipients and
      optional note from parameters. Enqueue outbound.send job with
      message_type='forward'. Keep action in Executing status (job will mark
      completed). Include original message body (inline or as attachment based
      on config)."
  - title: Implement auto_reply action
    done: false
    description: "Implement auto_reply: extract reply content from parameters (may
      be LLM-generated). Enqueue outbound.send job with message_type='reply'.
      Keep action in Executing status. Set proper threading headers to keep in
      same thread."
  - title: Implement outbound.send job handler
    done: false
    description: "Implement handle_outbound_send: build MIME message using
      MimeBuilder, call send_message API. Mark the original action as Completed
      with irreversible undo hint, or Failed on error. Store sent message ID in
      action result for reference."
  - title: Add tests for outbound email
    done: false
    description: Add tests for MimeBuilder (headers, body, attachments, threading).
      Add integration tests for outbound.send job with mocked Gmail API. Verify
      correct MIME structure.
tags:
  - actions
  - gmail
  - rust
---

Implement outbound email sending functionality for forward and auto-reply actions:

## Scope
- Add send_message method to GmailClient
- Create MIME message builder utility
- Create outbound.send job handler
- MIME message construction with proper threading headers (In-Reply-To, References)
- Implement forward action (include original as attachment or inline)
- Implement auto_reply action (template or content from decision)

## Design Notes
- Forward needs to handle attachments from original message
- Auto-reply content comes from decision parameters (may be LLM-generated)
- Actions are marked completed only after email is successfully sent (transactional)
- Forward/auto_reply are irreversible - undo hint will mark them as such

## Related Plans
- Plan 27 (Gmail Actions: Undo System) implements the undo functionality and depends on this plan

## Research

### Summary
- This plan implements two major features: outbound email sending (forward/auto_reply actions) and an undo system
- The codebase has a well-established job system with clear patterns for adding new job types
- GmailClient is mature and extensible; adding `send_message` follows existing patterns
- The action system already captures pre-image state and stores `undo_hint_json` with inverse action info
- `action_links` table exists with `undo_of` relation type ready for use
- MIME message construction will need a new utility; base64 encoding is already available

### Findings

#### Gmail Client Implementation
**Files:** `server/crates/ashford-core/src/gmail/client.rs`, `types.rs`, `parser.rs`

The GmailClient struct is well-architected with:
- Robust OAuth token refresh with mutex-based concurrency control
- Generic `perform_authenticated()` helper for all API calls
- `send_json()` and `send_empty_response()` patterns for different response types
- Comprehensive error handling via `GmailClientError` enum

**Existing methods that demonstrate patterns to follow:**
- `modify_message()` - POST request with JSON body
- `delete_message()` - DELETE request returning empty response
- `create_label()` - POST request creating resource

**For `send_message`, the implementation should:**
1. Add `SendMessageRequest` struct in `types.rs`:
   ```rust
   #[derive(Debug, Serialize)]
   #[serde(rename_all = "camelCase")]
   pub struct SendMessageRequest {
       pub raw: String, // base64url-encoded RFC 2822 message
       #[serde(skip_serializing_if = "Option::is_none")]
       pub thread_id: Option<String>, // For replies to maintain threading
   }
   ```

2. Add `SendMessageResponse` struct:
   ```rust
   #[derive(Debug, Deserialize)]
   #[serde(rename_all = "camelCase")]
   pub struct SendMessageResponse {
       pub id: String,
       pub thread_id: String,
       pub label_ids: Vec<String>,
   }
   ```

3. API endpoint: `POST {api_base}/{user_id}/messages/send`

4. Use `base64::engine::general_purpose::URL_SAFE_NO_PAD` for encoding (already used in `parser.rs`)

#### Job System Implementation
**Files:** `server/crates/ashford-core/src/jobs/mod.rs`, `queue.rs`, `worker.rs`

**Job type registration pattern:**
1. Create handler module (e.g., `jobs/outbound_send.rs`)
2. Define `pub const JOB_TYPE: &str = "outbound.send";`
3. Implement `pub async fn handle_outbound_send(dispatcher: &JobDispatcher, job: Job) -> Result<(), JobError>`
4. Add to `mod.rs`:
   - Import module: `mod outbound_send;`
   - Export constant: `pub const JOB_TYPE_OUTBOUND_SEND: &str = outbound_send::JOB_TYPE;`
   - Add match arm in `JobExecutor::execute()`

**Existing job types for reference:**
- `action.gmail` - Executes Gmail mutations, captures pre-image, stores undo hints
- `ingest.gmail` - Fetches and persists messages
- `unsnooze.gmail` - Scheduled job that wakes snoozed messages

**JobError patterns:**
- `JobError::Fatal(String)` - Non-retryable errors (e.g., invalid payload, resource not found)
- `JobError::Retryable { message, retry_after }` - Transient failures
- Use `map_gmail_error()`, `map_account_error()` helpers for consistent error mapping

**Job enqueueing:**
```rust
let queue = JobQueue::new(dispatcher.db.clone());
queue.enqueue(
    JOB_TYPE_OUTBOUND_SEND,
    json!({ "account_id": ..., "action_id": ... }),
    Some(idempotency_key),
    priority
).await
```

#### Action System Implementation
**Files:** `server/crates/ashford-core/src/jobs/action_gmail.rs`, `decisions/types.rs`, `decisions/repositories.rs`

**Action execution flow:**
1. Load action from `ActionRepository`
2. Check status (skip if terminal: Completed, Failed, Canceled, Rejected)
3. Mark as Executing
4. Capture pre-image state via `capture_pre_image()`
5. Execute Gmail API call
6. Call `mark_completed_with_undo_hint()` with undo hint JSON
7. On failure: `mark_failed()` with error message

**Pre-image state capture (`PreImageState`):**
```rust
pub struct PreImageState {
    pub labels: Vec<String>,
    pub is_unread: bool,
    pub is_starred: bool,
    pub is_in_inbox: bool,
    pub is_in_trash: bool,
}
```

**Undo hint structure (stored in `undo_hint_json`):**
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

**For forward/auto_reply actions:**
- These are marked as "dangerous" in `decisions/policy.rs` - require approval
- Undo hint should mark them as irreversible (similar to delete):
```json
{
    "action": "forward",
    "inverse_action": "none",
    "inverse_parameters": {"note": "cannot undo forward - email already sent"},
    "irreversible": true,
    "sent_message_id": "..." // Store for reference
}
```

**ActionType enum (`llm/decision.rs`) already includes:**
- `Forward` - defined, not yet implemented
- `AutoReply` - defined, not yet implemented

**Action handler pattern for forward/auto_reply:**
```rust
match action.action_type.as_str() {
    // ... existing cases
    "forward" => execute_forward(dispatcher, gmail_client, &message, &action).await,
    "auto_reply" => execute_auto_reply(dispatcher, gmail_client, &message, &action).await,
    // ...
}
```

These handlers should:
1. Extract parameters (recipients, subject, body/note)
2. Enqueue `outbound.send` job
3. Keep action in "Executing" status (do NOT mark completed yet)
4. The `outbound.send` job will mark the action completed after successful send

**Design Decision:** Actions are marked completed only after the email is successfully sent (transactional approach). This ensures action status accurately reflects whether the email was delivered.

#### Action Links Table
**Schema (`migrations/001_initial.sql`):**
```sql
CREATE TABLE action_links (
  id TEXT PRIMARY KEY,
  cause_action_id TEXT NOT NULL,
  effect_action_id TEXT NOT NULL,
  relation_type TEXT NOT NULL CHECK (
    relation_type IN ('undo_of','approval_for','spawned','related')
  )
);
```

**Repository methods (`ActionLinkRepository`):**
- `create(new_link)` - Create new link
- `get_by_cause_action_id(cause_id)` - Find effects of an action
- `get_by_effect_action_id(effect_id)` - Find what caused an action

**For undo system:**
- When undoing action A, create action B for the inverse
- Create link: `{cause: B, effect: A, relation_type: "undo_of"}`
- This means "B is the undo of A"

#### Database Schema
**Messages table (`headers_json` column):**
Headers are stored as JSON array of `{name, value}` objects:
```rust
pub struct Header {
    pub name: String,
    pub value: String,
}
```

**Threading headers available:**
- `Message-ID` - Unique message identifier
- `In-Reply-To` - Message-ID being replied to
- `References` - Full thread reference chain

**For outbound email, extract from original message:**
```rust
fn get_header(headers: &[Header], name: &str) -> Option<&str> {
    headers.iter()
        .find(|h| h.name.eq_ignore_ascii_case(name))
        .map(|h| h.value.as_str())
}
```

#### MIME Message Building
**Current dependencies:**
- `base64 = "0.22.1"` - For base64url encoding
- No MIME library currently included

**Decision:** Use `mail-builder` crate for lightweight MIME construction. It provides:
- Header building (From, To, Subject, In-Reply-To, References)
- Multipart message support (text/plain + text/html)
- Attachment handling
- Proper encoding of headers and bodies

**Example structure:**
```rust
pub struct MimeBuilder {
    from: String,
    to: Vec<String>,
    cc: Vec<String>,
    subject: String,
    body_plain: Option<String>,
    body_html: Option<String>,
    in_reply_to: Option<String>,
    references: Option<String>,
    attachments: Vec<Attachment>,
}

impl MimeBuilder {
    pub fn build(&self) -> Result<String, MimeError>;
    pub fn to_base64url(&self) -> Result<String, MimeError>;
}
```

#### Outbound Send Job Design
**Payload structure:**
```rust
#[derive(Debug, Deserialize)]
struct OutboundSendPayload {
    account_id: String,
    action_id: String,
    message_type: OutboundMessageType, // "forward" | "reply"
    to: Vec<String>,
    cc: Option<Vec<String>>,
    subject: String,
    body_plain: Option<String>,
    body_html: Option<String>,
    original_message_id: String, // Internal message UUID
    thread_id: Option<String>, // Gmail thread ID for replies
    include_original: bool, // For forwards: inline or attachment
}
```

**Handler flow:**
1. Parse payload
2. Load original action (should be in "Executing" status)
3. Load original message for threading headers and body
4. Build MIME message with MimeBuilder
5. Call `gmail_client.send_message()`
6. On success: Mark action "Completed" with irreversible undo hint containing `sent_message_id`
7. On failure: Mark action "Failed" with error message

**Key responsibility:** This job owns the action's final status transition since forward/auto_reply handlers leave the action in "Executing".

#### Forward Action Parameters
Expected parameters from decision:
```json
{
    "to": ["recipient@example.com"],
    "cc": ["cc@example.com"],
    "note": "Please review attached email", // Optional note to prepend
    "include_original": "inline" | "attachment"
}
```

**Forward body construction:**
- If `include_original = "inline"`: Prepend note, then `---------- Forwarded message ----------` followed by original body
- If `include_original = "attachment"`: Note as body, original message as .eml attachment

#### Auto-Reply Action Parameters
Expected parameters from decision:
```json
{
    "body": "Thank you for your email. I will get back to you shortly.",
    "body_html": "<p>Thank you for your email...</p>" // Optional
}
```

**Reply construction:**
- Set `In-Reply-To` to original message's `Message-ID`
- Set `References` to original's `References` + original's `Message-ID`
- Set `thread_id` in send request to maintain Gmail threading

### Risks & Constraints

1. **Gmail API Rate Limits**
   - Sending emails counts against daily quota (varies by account type)
   - Should implement rate limiting or exponential backoff for 429 errors
   - Consider adding retry logic specifically for rate limit responses

2. **Threading Header Extraction**
   - `headers_json` stores headers from ingested messages
   - Need to ensure `Message-ID`, `In-Reply-To`, `References` are captured during ingest
   - Check `ingest_gmail.rs` to verify header preservation

3. **Forward Attachment Handling**
   - Original message may have large attachments
   - Gmail API has size limits (25MB total)
   - Need to fetch attachments via `messages.attachments.get` API
   - Consider chunked/streaming for large attachments

4. **Irreversible Action Warnings**
   - Forward and auto_reply send real emails - cannot be undone
   - UI should clearly warn users before executing these
   - Safety policy already marks these as "dangerous" requiring approval

5. **Test Coverage Dependencies**
   - Tests require wiremock for Gmail API mocking (already in dev-dependencies)
   - MIME output testing needs careful assertion on structure
   - Consider recording real API responses for test fixtures
