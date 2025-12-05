### **6.1 Gmail Accounts**

- Single user but support multiple Gmail accounts conceptually.
- config_json holds:

    - Gmail OAuth tokens (indirectly; actual secrets may be in system keychain).

    - Pub/Sub subscription name.

    - History sync state (last historyId, etc.), possibly also in state_json.

```
CREATE TABLE accounts (
      id TEXT PRIMARY KEY,
      provider TEXT NOT NULL CHECK(provider IN ('gmail')),
      email TEXT NOT NULL,
      display_name TEXT,
      config_json TEXT NOT NULL,   -- provider specific, redacted in UI
      state_json TEXT NOT NULL DEFAULT '{}',
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );
```

### **6.1.1 Gmail Labels**

Labels are synced from Gmail and stored locally to:
1. Provide semantic context to the LLM during classification (names + descriptions)
2. Enable stable rule references using label IDs (survives Gmail label renames)
3. Detect deleted labels and auto-disable affected rules

**Label Sync Process:**
- Labels are fetched via Gmail's `users.labels.list` API
- Sync occurs on account setup and periodically thereafter
- Local labels are upserted by (account_id, provider_label_id)
- User-editable fields (description, available_to_classifier) are preserved on sync
- Labels deleted in Gmail trigger soft-disabling of dependent rules

**Label Translation:**
- Gmail messages store label IDs (e.g., "Label_123456789" for custom labels)
- LLM prompts show human-readable names + descriptions for better classification
- LLM responses with label names are translated back to IDs before action storage
- System labels (INBOX, SENT, etc.) have IDs matching their names

See the `labels` table in data_model.md for schema details.
  

### **6.2 Pub/Sub + History Flow**

1. **Pub/Sub Listener**:

    - Rust service exposes an HTTP endpoint or uses a Pull subscription to receive messages (flexible depending on deployment).

    - For each notification:

        - Decode message.

        - Identify account_id and historyId or message id.

2. **Enqueue Ingest**:

    - Insert ingest.gmail job with payload containing:

        - account_id, history_id (if using History) or message id.

3. **Ingest Job**:

    - If using History:

        - Call Gmail History API to list deltas since last known historyId.

        - For each new message:

            - Fetch message metadata + body.

            - Upsert into threads and messages.

            - Enqueue classify job.

        - Update state_json with new historyId.

    - For direct message notification:

        - Fetch message from Gmail.

        - Upsert; enqueue classify.

4. **Backfill / Catchup**:

    - backfill.gmail jobs to load the last N days/weeks of messages for newly-configured accounts.

    - Use Gmail search queries (e.g., newer_than:30d) or History at an older baseline.

### **6.3 Gmail Write Operations**

The `GmailClient` provides methods for mutating Gmail messages:

**Label Modification**:
- `modify_message(message_id, add_labels, remove_labels)` - Add/remove labels from a message
- Used for: archive (remove INBOX), apply_label, remove_label, mark_read/unread (UNREAD label), star/unstar (STARRED label), snooze (remove INBOX, add snooze label)

**Trash Operations**:
- `trash_message(message_id)` - Move message to trash (reversible)
- `untrash_message(message_id)` - Restore message from trash

**Permanent Deletion**:
- `delete_message(message_id)` - Permanently delete a message (irreversible, use with caution)

**Label Management**:
- `create_label(name)` - Create a new Gmail label (returns existing label if name already exists)
- `list_labels()` - List all labels in the account

All write operations use the same authentication and token refresh mechanism as read operations. Errors are mapped to `GmailClientError` with appropriate retry behavior (see job_queue.md for error handling details).

### **6.4 Snooze Operations**

Gmail does not have a native snooze API. Ashford implements snooze via labels and scheduled jobs:

**Snooze Flow**:
1. Message is archived (INBOX label removed)
2. Snooze label is applied (configurable via `gmail.snooze_label`, default: "Ashford/Snoozed")
3. An `unsnooze.gmail` job is scheduled with `not_before` set to the target wake time
4. At the scheduled time, the unsnooze job adds INBOX back and removes the snooze label

**Label Auto-Creation**:
When snoozing for the first time, Ashford checks if the snooze label exists:
1. First checks the local labels database cache
2. If not found locally, creates the label via Gmail API (`create_label`)
3. Upserts the new label to the local database
4. Uses the provider label ID for subsequent operations

This ensures the snooze label is available even if it was deleted externally.

**Snooze Parameters**:
The snooze action accepts two parameter formats:
```json
// Absolute datetime (ISO 8601)
{"until": "2024-12-25T09:00:00Z"}

// Relative duration
{"amount": 2, "units": "hours"}
```
Valid units: `minutes`, `hours`, `days`. Maximum duration: 1 year.

**Edge Cases**:
- If the message is deleted while snoozed, the unsnooze job completes successfully (no-op)
- If the snooze label is removed externally, unsnooze still adds INBOX back
- If the snooze label config changes, unsnooze uses the label ID stored in the job payload

### **6.5 Outbound Email (Send)**

The `GmailClient` supports sending emails for forward and auto_reply actions:

**Send Message API**:
- `send_message(raw_message, thread_id)` - Send an RFC 2822 MIME message via Gmail API
- `raw_message` is base64url-encoded (no padding)
- `thread_id` is optional; when provided, Gmail adds the sent message to an existing thread

**API Request**:
```rust
// Types in server/crates/ashford-core/src/gmail/types.rs
pub struct SendMessageRequest {
    pub raw: String,        // base64url-encoded RFC 2822 message
    pub thread_id: Option<String>,
}

pub struct SendMessageResponse {
    pub id: String,         // Gmail message ID of sent message
    pub thread_id: String,  // Thread the message belongs to
    pub label_ids: Vec<String>,
}
```

**MIME Message Builder**:
The `MimeMessage` struct (in `server/crates/ashford-core/src/gmail/mime_builder.rs`) constructs RFC 2822 compliant emails using the `mail-builder` crate:

```rust
use ashford_core::gmail::{EmailAddress, MimeAttachment, MimeMessage};

// Build an email message (fields are plain structs, no builder methods)
let message = MimeMessage {
    from: EmailAddress::new(Some("Sender Name"), "sender@gmail.com"),
    to: vec![EmailAddress::new(None, "recipient@example.com")],
    cc: vec![EmailAddress::new(Some("CC Name"), "cc@example.com")],
    bcc: vec![],
    subject: Some("Re: Original Subject".to_string()),
    body_plain: Some("Plain text body".to_string()),
    body_html: Some("<p>HTML body</p>".to_string()),
    in_reply_to: Some("<original-message-id@gmail.com>".to_string()),  // For replies
    references: vec!["<ref1@gmail.com>".to_string(), "<ref2@gmail.com>".to_string()], // Thread chain
    attachments: vec![MimeAttachment {
        filename: "document.pdf".to_string(),
        content_type: "application/pdf".to_string(),
        data: file_bytes, // Vec<u8>
    }],
};

// Get base64url-encoded output for Gmail API
let raw = message.to_base64_url()?;
```

**EmailAddress**:
```rust
pub struct EmailAddress {
    pub email: String,
    pub name: Option<String>,
}

// Creates: "Display Name <email@example.com>" or just "email@example.com"
EmailAddress::new(Some("Display Name"), "email@example.com")
```

**Threading Headers**:
For replies, proper threading requires:
- `In-Reply-To`: The `Message-ID` of the message being replied to
- `References`: The full chain of `Message-ID`s in the thread

For forwards, these headers are omitted so Gmail creates a new conversation thread.

**Attachments**:
The builder supports binary attachments:
```rust
message.attachment(
    "filename.pdf",           // Filename shown to recipient
    "application/pdf",        // MIME type
    file_bytes                // Vec<u8> of file content
);
```

Attachments are Base64-encoded in the MIME message per RFC 2045.

**Error Handling**:
`send_message` errors map to `GmailClientError`:
- HTTP 400 → Invalid message format or encoding
- HTTP 403 → Insufficient permissions or sending quota exceeded
- HTTP 429 → Rate limited (retryable)
- HTTP 5xx → Server error (retryable)

See job_queue.md section 5.9 for the `outbound.send` job that orchestrates email sending.

### **6.6 Undo Operations**

Ashford supports undoing most Gmail actions by storing undo hints when actions are executed and reversing them on request.

**Undo Hint Structure**:
Each completed action stores its undo information in `undo_hint_json`:
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

**Undoable Actions**:
| Action | Inverse Operation |
|--------|-------------------|
| Archive | Add INBOX label |
| Apply label | Remove the applied label |
| Remove label | Add the removed label |
| Mark read | Mark unread (add UNREAD) |
| Mark unread | Mark read (remove UNREAD) |
| Star | Unstar (remove STARRED) |
| Unstar | Star (add STARRED) |
| Trash | Restore from trash |
| Restore | Move to trash |
| Snooze | Cancel unsnooze job + restore to inbox |

**Irreversible Actions**:
These actions cannot be undone and store `{"inverse_action": "none", "irreversible": true}`:
- `delete` - Message permanently deleted from Gmail
- `forward` - Email already sent to recipient
- `auto_reply` - Reply already sent

**Snooze Undo**:
Snooze undo is more complex because it involves both Gmail state and a scheduled job:
1. The undo handler cancels the scheduled `unsnooze.gmail` job
2. Restores the message to INBOX (adds INBOX label)
3. Removes the snooze label

If the unsnooze job has already run, the cancel is a no-op but label changes still apply.

**Double-Undo Prevention**:
Each action can only be undone once. The system uses a unique constraint on `action_links` to enforce this:
- When an undo is executed, an `action_link` with `relation_type='undo_of'` is created
- Subsequent undo attempts fail with "action already undone"

See job_queue.md section 5.10 for the `undo.action` job implementation details.
