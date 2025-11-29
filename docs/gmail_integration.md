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

