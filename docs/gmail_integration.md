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

