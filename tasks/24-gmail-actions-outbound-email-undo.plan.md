---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Gmail Actions: Outbound Email"
goal: Implement forward/auto-reply actions via outbound.send job with MIME
  message construction
id: 24
uuid: e3c7d618-82e3-4835-9f9c-441d596c2fc1
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
  - docs/data_model.md
planGeneratedAt: 2025-12-03T02:21:59.017Z
promptsGeneratedAt: 2025-12-03T02:21:59.017Z
createdAt: 2025-12-03T02:21:14.787Z
updatedAt: 2025-12-05T09:05:54.937Z
progressNotes:
  - timestamp: 2025-12-04T21:51:53.926Z
    text: Implemented Gmail send_message API client, MIME builder utility, and
      outbound.send job with send + completion handling; added unit test for
      MIME builder and integration-style test for outbound send using mock
      Gmail.
    source: "implementer: Tasks1-3-6-7"
  - timestamp: 2025-12-04T21:58:32.544Z
    text: Expanded outbound.send coverage (thread lookup fallback, forward
      attachments, invalid attachments). Fixed handler to mark actions failed on
      attachment decode errors. All ashford-core tests passing.
    source: "tester: outbound email"
  - timestamp: 2025-12-04T22:25:27.109Z
    text: Implemented forward/auto_reply branches in action_gmail to enqueue
      outbound.send with MIME-ready payloads. Added inline/attachment body
      builders, thread lookup for replies, idempotent enqueue, and tests
      covering forward/auto-reply enqueue behavior and action status staying
      Executing. cargo test -p ashford-core action_gmail passing.
    source: "implementer: Tasks4-5"
  - timestamp: 2025-12-04T22:30:09.215Z
    text: Added missing forward attachment and auto-reply recipient validation
      tests; verified action_gmail suite green.
    source: "tester: outbound email"
  - timestamp: 2025-12-05T05:55:34.791Z
    text: "Completed thorough code review. All 7 tasks implemented: send_message
      method added to GmailClient, MimeMessage builder created with mail-builder
      dependency, outbound.send job handler implemented, forward and auto_reply
      actions enqueue correctly leaving actions in Executing, threading headers
      omitted for forwards. Tests cover main paths. Found one minor issue with
      references payload handling for auto_reply actions."
    source: "reviewer: Gmail Outbound Email"
  - timestamp: 2025-12-05T08:21:13.779Z
    text: "Addressed review issues: added idempotent sent-metadata persistence to
      outbound.send and action type validation; added regression test to ensure
      retries don't resend."
    source: "implementer: outbound.send"
  - timestamp: 2025-12-05T08:22:31.008Z
    text: "Ran cargo test -p ashford-core jobs::outbound_send::tests and
      action_gmail:: suite; all current tests passing."
    source: "tester: outbound.send verification"
  - timestamp: 2025-12-05T08:23:22.960Z
    text: Added regression test for action type mismatch to ensure outbound.send
      refuses mismatched actions without calling Gmail; reran cargo test -p
      ashford-core jobs::outbound_send::tests (all green).
    source: "tester: outbound.send verification"
  - timestamp: 2025-12-05T08:26:24.000Z
    text: "Found remaining issues: outbound.send will resend on retry if undo-hint
      update DB write fails; existing-sent retry path still loads original
      message and can fail if message missing."
    source: "reviewer: outbound.send"
  - timestamp: 2025-12-05T08:59:31.617Z
    text: Addressed three code duplication issues from review. 1) Consolidated Gmail
      client creation by renaming create_gmail_client to
      create_gmail_client_with_account (returns Account + client), added
      convenience wrapper create_gmail_client, removed duplicate
      load_account_and_client from outbound_send.rs. 2) Made
      normalize_message_id public in mime_builder.rs and removed duplicate from
      outbound_send.rs. 3) Added new dedup_message_ids function in
      mime_builder.rs, refactored combined_references to use it, replaced
      dedup_ids calls in outbound_send.rs with dedup_message_ids. All
      outbound_send and mime_builder tests pass.
    source: "implementer: code review fixes"
  - timestamp: 2025-12-05T09:03:06.069Z
    text: "Verified test coverage for the three code deduplication fixes. Found that
      normalize_message_id and dedup_message_ids in mime_builder.rs had no
      dedicated unit tests. Added 6 new tests:
      normalize_message_id_strips_brackets_and_whitespace,
      normalize_message_id_returns_none_for_empty,
      dedup_message_ids_removes_duplicates_preserving_order,
      dedup_message_ids_normalizes_before_deduplicating,
      dedup_message_ids_filters_empty_ids,
      dedup_message_ids_handles_empty_input. All 578 tests in ashford-core pass.
      Two pre-existing flaky tests
      (queue::tests::concurrent_claim_allows_single_runner and
      worker_executes_archive_action_and_populates_undo_hint) occasionally fail
      when running full suite but pass individually - these are unrelated to the
      Plan 24 changes."
    source: "tester: test coverage verification"
  - timestamp: 2025-12-05T09:05:31.488Z
    text: "Verified all three duplication issues are properly fixed: (1)
      create_gmail_client duplicates replaced with
      create_gmail_client_with_account in action_gmail.rs, (2) dedup_ids
      replaced with dedup_message_ids from mime_builder, (3)
      normalize_message_id made public in mime_builder and reused in
      outbound_send. All 578 tests pass. The module dependency from
      outbound_send to action_gmail is consistent with existing patterns (e.g.,
      unsnooze_gmail also imports from action_gmail)."
    source: "reviewer: autofix review"
  - timestamp: 2025-12-05T09:05:54.931Z
    text: "Completed autofix for code review issues. All 3 duplication issues
      resolved: (1) Gmail client creation consolidated to action_gmail.rs, (2)
      dedup_message_ids extracted to mime_builder.rs, (3) normalize_message_id
      made public in mime_builder.rs. Added 6 new unit tests. All 578 tests
      pass. Reviewer approved with ACCEPTABLE verdict."
    source: "orchestrator: autofix"
tasks:
  - title: Add send_message method to GmailClient
    done: true
    description: Implement GmailClient::send_message(raw_message) calling POST
      /messages/send with base64url-encoded RFC 2822 message. Add
      SendMessageRequest and SendMessageResponse types.
  - title: Create MIME message builder
    done: true
    description: "Add mail-builder crate dependency. Create MimeBuilder wrapper
      utility for constructing RFC 2822 email messages. Support: To/From/Subject
      headers, plain text and HTML body, In-Reply-To and References headers for
      threading, attachments. Output base64url-encoded string for Gmail API."
  - title: Create outbound.send job type
    done: true
    description: "Create new job type 'outbound.send' with handler in
      jobs/outbound_send.rs. Payload: {account_id, action_id, message_type:
      'forward'|'reply', to, subject, body, original_message_id, attachments}.
      Export JOB_TYPE_OUTBOUND_SEND constant."
  - title: Implement forward action
    done: true
    description: "In action_gmail, implement forward: extract recipients and
      optional note from parameters. Enqueue outbound.send job with
      message_type='forward'. Keep action in Executing status (job will mark
      completed). Include original message body (inline or as attachment based
      on config)."
  - title: Implement auto_reply action
    done: true
    description: "Implement auto_reply: extract reply content from parameters (may
      be LLM-generated). Enqueue outbound.send job with message_type='reply'.
      Keep action in Executing status. Set proper threading headers to keep in
      same thread."
  - title: Implement outbound.send job handler
    done: true
    description: "Implement handle_outbound_send: build MIME message using
      MimeBuilder, call send_message API. Mark the original action as Completed
      with irreversible undo hint, or Failed on error. Store sent message ID in
      action result for reference."
  - title: Add tests for outbound email
    done: true
    description: Add tests for MimeBuilder (headers, body, attachments, threading).
      Add integration tests for outbound.send job with mocked Gmail API. Verify
      correct MIME structure.
changedFiles:
  - docs/gmail_integration.md
  - docs/job_queue.md
  - server/Cargo.lock
  - server/crates/ashford-core/Cargo.toml
  - server/crates/ashford-core/src/gmail/client.rs
  - server/crates/ashford-core/src/gmail/mime_builder.rs
  - server/crates/ashford-core/src/gmail/mod.rs
  - server/crates/ashford-core/src/gmail/types.rs
  - server/crates/ashford-core/src/jobs/action_gmail.rs
  - server/crates/ashford-core/src/jobs/mod.rs
  - server/crates/ashford-core/src/jobs/outbound_send.rs
  - server/crates/ashford-core/src/jobs/unsnooze_gmail.rs
  - server/crates/ashford-core/src/threads.rs
  - server/crates/ashford-core/tests/export_ts_types.rs
  - web/src/lib/types/generated/LogicalCondition.ts
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

# Implementation Notes

Implemented Gmail outbound send infrastructure. Added mail-builder dependency and new Gmail types for send_message plus GmailClient::send_message with wiremock coverage (server/crates/ashford-core/src/gmail/types.rs, client.rs). Introduced gmail/mime_builder.rs providing EmailAddress/MimeMessage and attachment support with threading headers and base64url output, along with unit tests. Registered new outbound.send job (jobs/outbound_send.rs) and constant in jobs/mod.rs: parses payload, loads action/message/thread (new ThreadRepository::get_by_id), builds MIME via MimeMessage, decodes attachments, uses refreshed Gmail client to POST messages, then marks actions completed with irreversible undo hints or failed on fatal errors. Added outbound send test using mock Gmail to verify encoded payload and action completion. ThreadRepository gained get_by_id helper; cargo fmt touched unsnooze_gmail.rs test and export_ts_types.rs formatting only.

Wrapped outbound send pre-flight (account/message ownership checks, message/thread lookup, attachment decode, MIME build, Gmail send) into the result flow so fatal errors now reach the final match and mark_failed path; moved the action/account validation there to avoid actions getting stuck executing when validation fails. Forward handling now explicitly skips threadId resolution and threading headers: build_thread_headers returns empty data for Forward, references from payload are only applied for Reply, and send_message is called with None so Gmail delivers forwards as new conversations. Updated server/crates/ashford-core/src/jobs/outbound_send.rs forward test to assert threadId and References/In-Reply-To are absent while keeping attachment coverage; reran cargo test -p ashford-core jobs::outbound_send::tests to verify. Tasks: fix forward threading, ensure fatal pre-send errors mark actions failed, and add test coverage to catch threading regressions.

Implemented Tasks 4 and 5 (forward and auto_reply actions) by adding enqueue paths inside action_gmail. Introduced recipient parsing, body builders, and HTML escaping helpers plus an idempotent enqueue_outbound_send helper and thread lookup for replies in server/crates/ashford-core/src/jobs/action_gmail.rs. Forward actions now build inline forwarded content (including minimal header summary) or attach an eml copy of the original when include_original=attachment, set a sensible Fwd: subject, and enqueue outbound.send without completing the action. Auto-reply actions derive recipients (defaulting to the original sender), generate Re: subjects, honor body/body_html parameters, include threading info via provider thread id lookup, and enqueue outbound.send leaving the action in Executing. Added tests in action_gmail action_handler_tests to assert forward/auto-reply enqueue behavior, payload contents, and that actions stay Executing; kept build/test coverage with cargo test -p ashford-core action_gmail. These changes keep undo hints unchanged (handled by outbound.send) and rely on outbound.send to mark completion after successful send.

Handled reviewer fixes for Gmail forward/auto_reply actions. execute_forward now creates a Gmail client to fetch the original message, downloads attachment bodies via attachments.get, and includes them when forwarding inline. Forward-as-attachment now attaches the true raw RFC 822 from Gmail (message/rfc822) instead of a fabricated stub, preserving original headers and attachments. Added header lookup helper and Reply-To preference in execute_auto_reply, falling back to From only when Reply-To is absent. Updated tests in action_gmail action_handler suite to mock Gmail endpoints, verify forwarded attachments (both inline and .eml) and ensure auto-reply targets Reply-To. Touched files: server/crates/ashford-core/src/jobs/action_gmail.rs, server/crates/ashford-core/src/gmail/client.rs, server/crates/ashford-core/src/gmail/types.rs. Added new Gmail client APIs get_message_raw/get_attachment and MessageAttachment type. Ran cargo test --manifest-path server/crates/ashford-core/Cargo.toml jobs::action_gmail::tests::action_handler_tests -- --nocapture successfully.

Implemented review fixes for outbound email idempotency and safety. Added ActionRepository::update_undo_hint (server/crates/ashford-core/src/decisions/repositories.rs) with a dedicated test to persist undo_hint_json without changing status, allowing outbound jobs to stash irreversible send metadata mid-execution. In outbound.send handler (server/crates/ashford-core/src/jobs/outbound_send.rs), introduced action type validation so only forward/auto_reply actions may be executed, and added sent-metadata detection/persistence to prevent retries from resending after a successful Gmail call—on success we now store sent_message_id/thread_id in undo_hint_json before completion, and retries reuse that metadata to finalize the action without another send. Added helper functions for expected action type, send undo hint construction, and persisted send metadata; logging now uses stored message IDs. A new regression test verifies that existing sent metadata skips the Gmail send while still completing the action, and cargo tests confirm behavior. These changes directly address the critical duplicate-send risk and the missing action-type guard while preserving the plan’s irreversible completion semantics.

Implemented safeguards in outbound.send to eliminate duplicate sends when post-send persistence fails by making undo-hint writes fatal/non-retryable after a successful Gmail send while keeping retryable writes for the already-sent fast path. Reordered the existing-sent short-circuit to run before message/thread lookups so retries with stored sent metadata skip DB fetches and cannot fail fatally if the source message is missing or moved. Extended regression coverage with a test that enqueues a retry using stored sent metadata and a missing original_message_id payload to ensure Gmail is not called and the action completes using persisted ids. Updated persist_send_hint to accept a retryable_on_failure flag and adjusted call sites accordingly. Files modified: server/crates/ashford-core/src/jobs/outbound_send.rs. Tasks: Issue 1 – prevent outbound.send duplicate resend; Issue 2 – enforce outbound.send action type resilience on retries.

Autofix: Resolved three code duplication issues identified in code review. Issue 1 (duplicate Gmail client creation): Renamed create_gmail_client to create_gmail_client_with_account in action_gmail.rs to return both Account and GmailClient tuple, added a convenience wrapper create_gmail_client that returns only the client, removed duplicate load_account_and_client from outbound_send.rs, and updated outbound_send.rs to import create_gmail_client_with_account from action_gmail. Issue 2 (duplicate dedup_ids logic): Added new public dedup_message_ids function in mime_builder.rs that normalizes message IDs before deduplication, refactored combined_references to use dedup_message_ids, removed dedup_ids from outbound_send.rs, and updated imports. Issue 3 (duplicate normalize_message_id): Made normalize_message_id public in mime_builder.rs with proper documentation and removed the duplicate from outbound_send.rs. Files modified: server/crates/ashford-core/src/jobs/action_gmail.rs, server/crates/ashford-core/src/jobs/outbound_send.rs, server/crates/ashford-core/src/gmail/mime_builder.rs. Added 6 new unit tests for the refactored public functions (4 for dedup_message_ids, 2 for normalize_message_id). All 578 tests pass.
