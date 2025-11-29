9.1 Core Tables

accounts

CREATE TABLE accounts (
  id TEXT PRIMARY KEY,
  provider TEXT NOT NULL CHECK (provider IN ('gmail')),
  email TEXT NOT NULL,
  display_name TEXT,
  config_json TEXT NOT NULL,           -- provider-specific config, secrets redacted in UI
  state_json TEXT NOT NULL DEFAULT '{}', -- sync state, historyId, etc.
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX accounts_email_idx ON accounts(email);


⸻

threads

CREATE TABLE threads (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  provider_thread_id TEXT NOT NULL,     -- Gmail thread ID
  subject TEXT,
  snippet TEXT,
  last_message_at TEXT,                 -- ISO timestamp of last message
  metadata_json TEXT NOT NULL DEFAULT '{}', -- summary info, label hints, etc.
  raw_json TEXT NOT NULL,               -- raw Gmail thread or representative metadata
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,

  FOREIGN KEY (account_id) REFERENCES accounts(id)
);

CREATE INDEX threads_account_thread_idx
  ON threads(account_id, provider_thread_id);

CREATE INDEX threads_last_message_idx
  ON threads(account_id, last_message_at);


⸻

messages

CREATE TABLE messages (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  thread_id TEXT NOT NULL,
  provider_message_id TEXT NOT NULL,   -- Gmail message ID
  from_email TEXT,
  from_name TEXT,
  to_json TEXT NOT NULL DEFAULT '[]',  -- list of recipients
  cc_json TEXT NOT NULL DEFAULT '[]',
  bcc_json TEXT NOT NULL DEFAULT '[]',
  subject TEXT,
  snippet TEXT,
  received_at TEXT,                    -- when Gmail says it was received
  internal_date TEXT,                  -- Gmail internal date
  labels_json TEXT NOT NULL DEFAULT '[]',   -- Gmail labels
  headers_json TEXT NOT NULL DEFAULT '{}',  -- parsed headers
  body_plain TEXT,
  body_html TEXT,
  raw_json TEXT NOT NULL,              -- raw Gmail message JSON
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,

  FOREIGN KEY (account_id) REFERENCES accounts(id),
  FOREIGN KEY (thread_id) REFERENCES threads(id)
);

CREATE INDEX messages_account_msg_idx
  ON messages(account_id, provider_message_id);

CREATE INDEX messages_thread_idx
  ON messages(thread_id, received_at);

CREATE INDEX messages_from_idx
  ON messages(account_id, from_email);


⸻

decisions

CREATE TABLE decisions (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  message_id TEXT NOT NULL,
  source TEXT NOT NULL CHECK (source IN ('llm','deterministic')),
  decision_json TEXT NOT NULL,         -- full decision contract from engine
  action_type TEXT,                    -- convenience copy of primary action
  confidence REAL,                     -- primary confidence, if applicable
  needs_approval INTEGER NOT NULL DEFAULT 0,
  rationale TEXT,                      -- short explanation string
  telemetry_json TEXT NOT NULL DEFAULT '{}', -- model, tokens, latency, etc.
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,

  FOREIGN KEY (account_id) REFERENCES accounts(id),
  FOREIGN KEY (message_id) REFERENCES messages(id)
);

CREATE INDEX decisions_message_idx
  ON decisions(message_id);

CREATE INDEX decisions_created_idx
  ON decisions(created_at);


⸻

actions

CREATE TABLE actions (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  message_id TEXT NOT NULL,
  decision_id TEXT,                    -- nullable (manual action, etc.)
  action_type TEXT NOT NULL,           -- archive|apply_label|delete|forward|...
  parameters_json TEXT NOT NULL,       -- structured parameters
  status TEXT NOT NULL CHECK (
    status IN (
      'queued','executing','completed','failed',
      'canceled','rejected','approved_pending'
    )
  ),
  error_message TEXT,
  executed_at TEXT,
  undo_hint_json TEXT NOT NULL DEFAULT '{}', -- inverse action info
  trace_id TEXT,                             -- OpenTelemetry trace
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,

  FOREIGN KEY (account_id) REFERENCES accounts(id),
  FOREIGN KEY (message_id) REFERENCES messages(id),
  FOREIGN KEY (decision_id) REFERENCES decisions(id)
);

CREATE INDEX actions_message_idx
  ON actions(message_id, created_at);

CREATE INDEX actions_status_idx
  ON actions(status, created_at);


⸻

action_links

CREATE TABLE action_links (
  id TEXT PRIMARY KEY,
  cause_action_id TEXT NOT NULL,
  effect_action_id TEXT NOT NULL,
  relation_type TEXT NOT NULL CHECK (
    relation_type IN ('undo_of','approval_for','spawned','related')
  ),

  FOREIGN KEY (cause_action_id) REFERENCES actions(id),
  FOREIGN KEY (effect_action_id) REFERENCES actions(id)
);

CREATE INDEX action_links_cause_idx
  ON action_links(cause_action_id);

CREATE INDEX action_links_effect_idx
  ON action_links(effect_action_id);


⸻

jobs

CREATE TABLE jobs (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,                  -- ingest.gmail|classify|action.gmail|...
  payload_json TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  state TEXT NOT NULL CHECK (
    state IN ('queued','running','completed','failed','canceled')
  ),
  attempts INTEGER NOT NULL DEFAULT 0,
  max_attempts INTEGER NOT NULL DEFAULT 5,
  not_before TEXT,                     -- schedule for future
  idempotency_key TEXT,
  last_error TEXT,
  heartbeat_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX jobs_state_idx
  ON jobs(state, priority, not_before);

CREATE UNIQUE INDEX jobs_idempotency_idx
  ON jobs(idempotency_key);


⸻

job_steps

CREATE TABLE job_steps (
  id TEXT PRIMARY KEY,
  job_id TEXT NOT NULL,
  name TEXT NOT NULL,                  -- e.g. "fetch_message", "call_llm"
  started_at TEXT NOT NULL,
  finished_at TEXT,
  result_json TEXT,

  FOREIGN KEY (job_id) REFERENCES jobs(id)
);

CREATE INDEX job_steps_job_idx
  ON job_steps(job_id, started_at);


⸻

discord_whitelist

CREATE TABLE discord_whitelist (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,               -- Discord snowflake
  username TEXT NOT NULL,              -- "name#discriminator" or global name
  created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX discord_whitelist_user_idx
  ON discord_whitelist(user_id);


⸻

9.2 Rules Tables

deterministic_rules

CREATE TABLE deterministic_rules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  scope TEXT NOT NULL CHECK (scope IN ('global','account','sender','domain')),
  scope_ref TEXT,                      -- account_id, domain, or sender email
  priority INTEGER NOT NULL DEFAULT 100,
  enabled INTEGER NOT NULL DEFAULT 1,
  conditions_json TEXT NOT NULL,       -- structured condition tree
  action_type TEXT NOT NULL,           -- primary action
  action_parameters_json TEXT NOT NULL,
  safe_mode TEXT NOT NULL CHECK (safe_mode IN ('default','always_safe','dangerous_override')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX deterministic_rules_scope_idx
  ON deterministic_rules(scope, scope_ref);

CREATE INDEX deterministic_rules_priority_idx
  ON deterministic_rules(enabled, priority);


⸻

llm_rules

CREATE TABLE llm_rules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  scope TEXT NOT NULL CHECK (scope IN ('global','account','sender','domain')),
  scope_ref TEXT,
  rule_text TEXT NOT NULL,             -- natural-language description
  enabled INTEGER NOT NULL DEFAULT 1,
  metadata_json TEXT NOT NULL DEFAULT '{}', -- hints, tags, etc.
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX llm_rules_scope_idx
  ON llm_rules(scope, scope_ref);

CREATE INDEX llm_rules_enabled_idx
  ON llm_rules(enabled, created_at);


CREATE TABLE directions (
  id TEXT PRIMARY KEY,
  content TEXT NOT NULL,                -- natural-language instruction
  enabled INTEGER NOT NULL DEFAULT 1,   -- 1 = active, 0 = ignored
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX directions_enabled_idx
  ON directions(enabled, created_at);


⸻

9.3 Rules Assistant Tables

rules_chat_sessions

CREATE TABLE rules_chat_sessions (
  id TEXT PRIMARY KEY,
  title TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX rules_chat_sessions_created_idx
  ON rules_chat_sessions(created_at);


⸻

rules_chat_messages

CREATE TABLE rules_chat_messages (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('user','assistant','system')),
  content TEXT NOT NULL,
  created_at TEXT NOT NULL,

  FOREIGN KEY (session_id) REFERENCES rules_chat_sessions(id)
);

CREATE INDEX rules_chat_messages_session_idx
  ON rules_chat_messages(session_id, created_at);


⸻

If you’d like, I can also:
	•	Draft a single consolidated migration file with these in dependency order, or
	•	Add a directions table back in explicitly if you still want global “instructions” separate from rules.
