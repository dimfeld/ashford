-- Track LLM requests and responses for auditing/telemetry.
CREATE TABLE llm_calls (
  id TEXT PRIMARY KEY,
  org_id INTEGER NOT NULL DEFAULT 1,
  user_id INTEGER NOT NULL DEFAULT 1,
  feature TEXT NOT NULL,
  context_json TEXT NOT NULL DEFAULT '{}',
  model TEXT NOT NULL,
  request_json TEXT NOT NULL,
  response_json TEXT,
  input_tokens INTEGER,
  output_tokens INTEGER,
  latency_ms INTEGER,
  error TEXT,
  trace_id TEXT,
  created_at TEXT NOT NULL
);

CREATE INDEX llm_calls_org_created_idx ON llm_calls(org_id, created_at);
CREATE INDEX llm_calls_feature_created_idx ON llm_calls(feature, created_at);
