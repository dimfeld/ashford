-- Ensure idempotent upserts for threads and messages based on provider identifiers
CREATE UNIQUE INDEX IF NOT EXISTS threads_account_provider_uidx
  ON threads(account_id, provider_thread_id);

CREATE UNIQUE INDEX IF NOT EXISTS messages_account_provider_uidx
  ON messages(account_id, provider_message_id);
