-- Add org_id/user_id columns to prepare for future multi-tenancy.

-- User-owned data: require org_id and user_id with defaults.
ALTER TABLE accounts ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE accounts ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS accounts_org_user_idx ON accounts(org_id, user_id);

ALTER TABLE threads ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE threads ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS threads_org_user_idx ON threads(org_id, user_id);

ALTER TABLE messages ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE messages ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS messages_org_user_idx ON messages(org_id, user_id);

ALTER TABLE decisions ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE decisions ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS decisions_org_user_idx ON decisions(org_id, user_id);

ALTER TABLE actions ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE actions ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS actions_org_user_idx ON actions(org_id, user_id);

-- Org-wide data: org_id required, user_id optional (nullable).
ALTER TABLE deterministic_rules ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE deterministic_rules ADD COLUMN user_id INTEGER;
UPDATE deterministic_rules SET user_id = NULL;
CREATE INDEX IF NOT EXISTS deterministic_rules_org_user_idx
  ON deterministic_rules(org_id, user_id);

ALTER TABLE llm_rules ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE llm_rules ADD COLUMN user_id INTEGER;
UPDATE llm_rules SET user_id = NULL;
CREATE INDEX IF NOT EXISTS llm_rules_org_user_idx
  ON llm_rules(org_id, user_id);

ALTER TABLE directions ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE directions ADD COLUMN user_id INTEGER;
UPDATE directions SET user_id = NULL;
CREATE INDEX IF NOT EXISTS directions_org_user_idx ON directions(org_id, user_id);

ALTER TABLE rules_chat_sessions ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE rules_chat_sessions ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS rules_chat_sessions_org_user_idx
  ON rules_chat_sessions(org_id, user_id);

ALTER TABLE rules_chat_messages ADD COLUMN org_id INTEGER NOT NULL DEFAULT 1;
ALTER TABLE rules_chat_messages ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS rules_chat_messages_org_user_idx
  ON rules_chat_messages(org_id, user_id);
