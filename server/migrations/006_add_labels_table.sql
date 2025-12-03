-- Create labels table for Gmail label synchronization
CREATE TABLE labels (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  provider_label_id TEXT NOT NULL,
  name TEXT NOT NULL,
  label_type TEXT NOT NULL CHECK (label_type IN ('system', 'user')),
  description TEXT,
  available_to_classifier INTEGER NOT NULL DEFAULT 1,
  message_list_visibility TEXT,
  label_list_visibility TEXT,
  background_color TEXT,
  text_color TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  org_id INTEGER NOT NULL DEFAULT 1,
  user_id INTEGER NOT NULL DEFAULT 1,
  FOREIGN KEY (account_id) REFERENCES accounts(id)
);

-- Unique constraint on account + provider label ID
CREATE UNIQUE INDEX labels_account_provider_uidx ON labels(account_id, provider_label_id);

-- Standard org/user index for multi-tenancy
CREATE INDEX labels_org_user_idx ON labels(org_id, user_id);

-- Index for fetching labels by account
CREATE INDEX labels_account_idx ON labels(account_id);

-- Add disabled_reason column to deterministic_rules for tracking why a rule was disabled
ALTER TABLE deterministic_rules ADD COLUMN disabled_reason TEXT;
