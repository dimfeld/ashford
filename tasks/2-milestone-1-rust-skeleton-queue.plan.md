---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 1: Rust Skeleton & Queue"
goal: Set up Rust project structure, libsql database with migrations, job queue,
  health endpoint, and OpenTelemetry
id: 2
uuid: 85389e56-6e82-4b14-b6ab-153a10439a6e
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies: []
parent: 1
issue: []
docs:
  - docs/job_queue.md
  - docs/data_model.md
  - docs/configuration.md
  - docs/opentelemetry.md
planGeneratedAt: 2025-11-29T01:23:11.754Z
promptsGeneratedAt: 2025-11-29T01:23:11.754Z
createdAt: 2025-11-29T01:21:26.633Z
updatedAt: 2025-11-29T01:23:11.754Z
tasks:
  - title: Initialize Cargo workspace
    done: false
    description: "Convert server/ to a Cargo workspace with 2 crates: ashford-core (library with shared logic) and ashford-server (single binary running both API and queue worker). Create root Cargo.toml with [workspace] and shared dependency versions. Move existing dependencies to workspace level. ashford-server depends on ashford-core."
  - title: Implement TOML configuration
    done: false
    description: "In ashford-core, create config module with structs matching configuration.md: AppConfig, PathsConfig, TelemetryConfig, ModelConfig, DiscordConfig, GmailConfig, ImapConfig, PolicyConfig. Implement TOML loading with serde, env var overrides (APP_PORT, OTLP_ENDPOINT, etc.), 'env:' prefix parsing for secrets, and tilde expansion for paths. Add config crate dependency."
  - title: Set up libsql connection
    done: false
    description: "In ashford-core, create db module with Database struct wrapping libsql::Database. Implement connect() that reads path from config, enables PRAGMA foreign_keys=ON, and returns connection. Support both local file paths and remote Turso URLs. Add connection health check method for /healthz."
  - title: Create database migrations
    done: false
    description: "In ashford-core, create migrations module with version tracking (schema_migrations table). Implement run_migrations() that executes SQL files in order. Create migrations/001_initial.sql with all 14 tables from data_model.md in correct dependency order: accounts, threads, messages, decisions, actions, action_links, jobs, job_steps, discord_whitelist, deterministic_rules, llm_rules, directions, rules_chat_sessions, rules_chat_messages. Include all indexes."
  - title: Add structured logging
    done: false
    description: "In ashford-core, create telemetry module. Set up tracing-subscriber with JSON formatting for production, pretty formatting for dev. Configure log levels via RUST_LOG env var. Create init_logging() function called early in both binaries. Add tracing and tracing-subscriber dependencies."
  - title: Set up OpenTelemetry
    done: false
    description: "Extend telemetry module with OpenTelemetry initialization. Configure OTLP exporter (HTTP to configured endpoint). Set resource attributes: service.name, service.version (from CARGO_PKG_VERSION), deployment.environment, host.arch, os.type. Create TracingLayer that attaches trace IDs to logs. Handle missing OTLP endpoint gracefully (disable export). Add opentelemetry, opentelemetry-otlp, tracing-opentelemetry dependencies."
  - title: Build job queue core
    done: false
    description: "In ashford-core, create queue module. Implement JobQueue struct with methods: enqueue(job_type, payload, idempotency_key, priority) -> Result<JobId>, claim_next() -> Result<Option<Job>> using atomic UPDATE...RETURNING, heartbeat(job_id), complete(job_id, result), fail(job_id, error, should_retry), cancel(job_id). Implement exponential backoff with jitter for retry scheduling (base 2s, max 5min, ±25% jitter). Add uuid, chrono, rand dependencies."
  - title: Add job step tracking
    done: false
    description: "Extend queue module with JobStep tracking. Add start_step(job_id, name) -> StepId, finish_step(step_id, result_json). Create JobContext struct that tracks current job and provides step helpers. Steps automatically record started_at/finished_at timestamps."
  - title: Create queue worker loop
    done: false
    description: "In ashford-core, implement worker module with async worker loop. Poll for jobs with configurable interval (default 1s). Claim and execute jobs, catching panics with std::panic::catch_unwind. Spawn heartbeat task that updates every 30s during execution. Respect not_before scheduling. Create JobExecutor trait for future job type handlers (stub implementation for now that just completes jobs). Return a future that can be spawned by the server binary."
  - title: Implement HTTP server with /healthz
    done: false
    description: "In ashford-server binary, create main() that initializes config, logging, telemetry, database, and runs migrations. Spawn queue worker loop as background task. Start axum HTTP server on configured port. Add GET /healthz endpoint returning JSON {status, version, database}. Check database connectivity, return 200 if healthy, 503 if unhealthy. Implement graceful shutdown on SIGTERM/SIGINT that stops both HTTP server and queue worker."
  - title: Add unit tests
    done: false
    description: "In ashford-core, add tests for: config loading with env overrides, queue operations (enqueue, claim, complete, fail, retry scheduling), idempotency key deduplication, job step tracking. Use in-memory libsql database (':memory:') for tests. Test concurrent claim behavior."
tags:
  - libsql
  - otel
  - queue
  - rust
---

Initial Rust project setup with core infrastructure:
- Cargo workspace structure
- libsql connection and migrations for all tables
- Jobs and job_steps tables for durable queue
- Basic queue runner with claim/heartbeat/complete flow
- /healthz endpoint
- OpenTelemetry tracing initialization
- TOML configuration loading with env overrides

## Research

### Summary
- This is a greenfield Rust project for an AI-powered mail agent (Ashford)
- The current codebase has minimal scaffolding: a single `server/` directory with a "Hello, world!" main.rs
- The project is very well-documented with comprehensive specs in the `docs/` directory
- Milestone 1 focuses on foundational infrastructure: workspace structure, database migrations, job queue, config loading, and observability
- The web frontend (SvelteKit) is already initialized and ready for backend integration

### Findings

#### Current Codebase State

**Project Stage**: Bootstrap - just two commits ("init" and "add rmplan config")

**Repository Structure**:
```
ashford/
├── .claude/               # Claude Code settings
├── .jj/                   # Jujutsu version control
├── .rmfilter/             # rmplan configuration
├── CLAUDE.md              # Project coding guidelines
├── docs/                  # Complete specification documentation
├── server/                # Rust backend (minimal)
│   ├── Cargo.toml        # Has axum, libsql, tokio, serde
│   ├── Cargo.lock
│   └── src/
│       └── main.rs       # Just "Hello, world!"
├── web/                   # SvelteKit frontend (initialized)
│   ├── package.json
│   ├── pnpm-lock.yaml
│   └── src/
└── tasks/                 # 8 rmplan task files
```

**Existing Rust Dependencies** (server/Cargo.toml):
```toml
[package]
name = "ashford-server"
version = "0.1.0"
edition = "2024"

[dependencies]
axum = "0.8.7"
libsql = "0.9.29"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.145"
tokio = { version = "1.48.0", features = ["full"] }
```

**Dependencies to Add for Milestone 1**:
- `toml` or `config` crate for configuration loading
- `tracing`, `tracing-subscriber`, `tracing-opentelemetry` for logging
- `opentelemetry`, `opentelemetry-otlp` for telemetry export
- `uuid` for ID generation
- `chrono` for timestamp handling
- `thiserror` or `anyhow` for error handling
- `rand` for exponential backoff jitter

**Workspace Structure** (2 crates):
```
server/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── ashford-core/       # Shared library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs
│   │       ├── db.rs
│   │       ├── migrations.rs
│   │       ├── queue.rs
│   │       ├── worker.rs
│   │       └── telemetry.rs
│   └── ashford-server/     # Combined API + queue worker binary
│       ├── Cargo.toml
│       └── src/
│           └── main.rs
└── migrations/
    └── 001_initial.sql
```

---

#### Database Schema (from docs/data_model.md)

**14 Tables Total** - All tables use TEXT PRIMARY KEYs (UUIDs) and TEXT timestamps (ISO 8601):

**Core Tables**:
1. `accounts` - Email account credentials and sync state
2. `threads` - Email thread metadata from Gmail
3. `messages` - Individual email messages within threads
4. `decisions` - AI/deterministic classification decisions
5. `actions` - Executable actions (archive, label, delete, etc.)
6. `action_links` - Relationships between actions (undo, approval, spawned)

**Queue Tables**:
7. `jobs` - Background job queue entries with state machine
8. `job_steps` - Detailed execution steps within jobs

**Access Control**:
9. `discord_whitelist` - Discord user allowlist

**Rules Tables**:
10. `deterministic_rules` - Structured condition-based rules
11. `llm_rules` - Natural-language rules for LLM evaluation
12. `directions` - Global behavioral instructions

**Rules Assistant Tables**:
13. `rules_chat_sessions` - Conversation sessions
14. `rules_chat_messages` - Individual chat messages

**Key Schema Patterns**:
- Foreign key constraints for referential integrity
- CHECK constraints for enum-like fields (status, scope, role, etc.)
- Composite indexes for common query patterns
- UNIQUE index on `jobs.idempotency_key` for deduplication
- JSON columns (`*_json`) for flexible nested data

**Migration Order** (respecting foreign keys):
1. accounts (no dependencies)
2. threads (depends on accounts)
3. messages (depends on accounts, threads)
4. decisions (depends on accounts, messages)
5. actions (depends on accounts, messages, decisions)
6. action_links (depends on actions)
7. jobs (no dependencies)
8. job_steps (depends on jobs)
9. discord_whitelist (no dependencies)
10. deterministic_rules (no dependencies)
11. llm_rules (no dependencies)
12. directions (no dependencies)
13. rules_chat_sessions (no dependencies)
14. rules_chat_messages (depends on rules_chat_sessions)

---

#### Job Queue Design (from docs/job_queue.md)

**Job Types**:
- `ingest.gmail` - Gmail message ingestion
- `classify` - Message classification
- `action.gmail` - Gmail-specific actions
- `approval.notify` - Approval notifications
- `undo` - Undo operations
- `outbound.send` - Auto-reply/forward sending
- `backfill.gmail` - Gmail backfill operations
- `history.sync.gmail` - Gmail history synchronization

**Job State Machine**:
```
queued → running → completed | failed | canceled
```

**Queue Operations**:
1. **Poll**: Query for jobs where `state='queued'` AND `not_before <= now`
2. **Claim**: Atomic transactional update of job state to `running`
3. **Heartbeat**: Periodic updates to `heartbeat_at` during long operations
4. **Complete**: Update state to `completed` and set `finished_at`
5. **Fail**: Update state to `failed`, record `last_error`, potentially requeue
6. **Cancel**: Update state to `canceled`

**Retry Strategy**:
- Default 5 max attempts per job type
- Exponential backoff with jitter to prevent thundering herd
- `not_before` column schedules retry timing

**Idempotency**:
- Idempotency keys prevent duplicate job creation
- Pattern: `gmail:acct:msg:classify`, `gmail:acct:msg:action:archive`
- UNIQUE index on `idempotency_key` enforces at database level

**Critical Implementation Notes**:
- Claim operation MUST be atomic (single UPDATE with WHERE state='queued')
- Heartbeat timeout detection for stuck jobs
- Step tracking via `job_steps` table for observability

---

#### Configuration (from docs/configuration.md)

**TOML Sections**:
```toml
[app]
service_name = "ashford"
port = 17800              # web UI
env = "dev"               # dev|prod

[paths]
database = "~/Library/Application Support/ai-mail-agent/app.db"

[telemetry]
otlp_endpoint = "http://localhost:4318"
export_traces = true

[model]
provider = "vercel"
model = "gemini-1.5-pro"
temperature = 0.2
max_output_tokens = 1024

[discord]
bot_token = "env:DISCORD_BOT_TOKEN"
channel_id = "1234567890"
whitelist = ["Daniel#1234"]

[gmail]
use_pubsub = true
project_id = "your-gcp-project"
subscription = "gmail-sub"

[imap]
idle = true
backfill_days = 30
archive_folder = "Archive"
snooze_folder = "Snoozed"

[policy]
approval_always = ["delete","forward","auto_reply","escalate"]
confidence_default = 0.7
```

**Environment Overrides**:
- `APP_PORT` - Override web UI port
- `OTLP_ENDPOINT` - Override OpenTelemetry endpoint
- `MODEL` - Override model identifier
- `DISCORD_BOT_TOKEN` - Discord bot token

**Secret Handling**:
- `env:` prefix for environment variable indirection
- OAuth tokens stored in OS keychain or encrypted store (not in TOML)

---

#### OpenTelemetry (from docs/opentelemetry.md)

**Resource Attributes**:
- `service.name` = "ai-mail-agent-rust"
- `service.version` (from Cargo.toml)
- `deployment.environment` (dev/prod from config)
- `host.arch`, `os.type`

**Span Types**:
- `email.receive` - Email reception/fetching
- `email.classify` - Classification processing
- `email.action` - Action execution
- `email.approval` - Approval workflows
- `queue.job` - Job queue operations

**Propagation**:
- Trace IDs attached to log lines for correlation
- SvelteKit can accept trace IDs from Rust responses (optional)

**OTLP Export**:
- Default receiver: localhost:4318 (HTTP) or :4317 (gRPC)
- Supports remote exporters like Honeycomb

---

#### Project Conventions (from CLAUDE.md)

**Source Control**:
- Uses `jj` (Jujutsu) instead of git
- `jj commit -m "..."` to commit (files auto-tracked)
- `jj git push` to push

**Testing**:
- Avoid mocks in backend tests unless calling external services
- Prefer regular for loops over `it.each` for table-driven tests
- Use `vi.waitFor` for assertions that may not be immediately met
- No `console` functions in tests (use `debug` module)

**Code Style**:
- Never use `any` in TypeScript (use `unknown`)
- Svelte 5 runes (`$state`) not old `$:` syntax
- Comments describe current state, not change history

---

### Risks & Constraints

#### Technical Risks

1. **libsql Migration System**: libsql doesn't have a built-in migration system like sqlx. Need to implement custom migration tracking (version table + SQL files).

2. **Atomic Job Claims**: The job claim operation must be a single atomic UPDATE statement to prevent race conditions. SQLite/libsql doesn't have row-level locking, so the UPDATE with WHERE clause is the correct approach.

3. **OpenTelemetry Crate Ecosystem**: The opentelemetry-rs ecosystem has multiple versions and breaking changes. Need to pin compatible versions of `opentelemetry`, `opentelemetry-otlp`, `tracing-opentelemetry`.

4. **Heartbeat Detection**: Need a mechanism to detect jobs that have stalled (worker crashed). This typically requires a background task that checks `heartbeat_at` timestamps.

5. **Exponential Backoff Calculation**: Must handle overflow for high attempt counts and ensure jitter prevents thundering herd.

#### Constraints

1. **libsql Edition**: Using Rust 2024 edition (experimental) - may have compatibility issues with some crates.

2. **Database Location**: Need to handle tilde expansion (`~`) in database paths for cross-platform support.

3. **Foreign Key Enforcement**: SQLite foreign keys are off by default. Need to enable with `PRAGMA foreign_keys = ON` on each connection.

4. **Idempotency Key Uniqueness**: The UNIQUE index on `idempotency_key` includes NULL values in SQLite, which may need special handling if idempotency_key is optional.

5. **Timestamp Format**: All timestamps must be ISO 8601 format (TEXT columns). Need consistent formatting across the codebase.

#### Dependencies to Respect

- **Jujutsu VCS**: Use `jj commit` instead of `git commit`
- **Existing Cargo.toml**: Build on existing dependencies rather than starting fresh
- **Documentation**: Follow patterns described in docs/ directory

---

## Expected Behavior/Outcome

After Milestone 1 completion, the system should:

1. **Start a Rust HTTP server** on the configured port (default 17800)
2. **Serve `/healthz` endpoint** that returns:
   - Service status (healthy/unhealthy)
   - Database connectivity status
   - Service version from Cargo.toml
3. **Connect to libsql database** (local file or remote Turso endpoint)
4. **Run migrations automatically** on startup, creating all 14 tables
5. **Load configuration from TOML** with environment variable overrides
6. **Export OpenTelemetry traces** to configured OTLP endpoint
7. **Process jobs from the queue** in a worker loop with:
   - Atomic job claiming
   - Heartbeat updates during execution
   - Proper completion/failure handling
   - Exponential backoff with jitter for retries
8. **Log in structured JSON format** with trace ID correlation

---

## Key Findings

### Product & User Story
This is internal infrastructure with no direct user-facing behavior. The "user" is the system itself and developers. The queue provides the foundation for all async processing in subsequent milestones.

### Design & UX Approach
Not applicable for this infrastructure milestone. Focus is on correctness, observability, and operational reliability.

### Technical Plan & Risks
- **Critical Path**: Database migrations → Configuration → Database connection → Job queue → HTTP server → OpenTelemetry
- **Highest Risk**: Job queue atomicity and retry logic correctness
- **Testing Strategy**: Unit tests for queue operations, integration tests for full claim/execute/complete flow

### Pragmatic Effort Estimate
This is a foundational milestone with well-defined requirements. The main complexity is in the job queue implementation and ensuring atomicity.

---

## Acceptance Criteria

- [ ] Cargo workspace compiles with `cargo build` in the server directory
- [ ] All 14 database tables are created via migrations on first startup
- [ ] Configuration loads from TOML file with `env:` indirection working
- [ ] Environment variables override TOML values
- [ ] `/healthz` endpoint returns 200 with JSON status when database is connected
- [ ] `/healthz` endpoint returns 503 when database is unreachable
- [ ] Jobs can be enqueued with idempotency key (duplicate inserts rejected)
- [ ] Worker claims jobs atomically (no double-processing in concurrent workers)
- [ ] Heartbeat updates occur during job execution
- [ ] Failed jobs are retried with exponential backoff + jitter
- [ ] Jobs exceeding max_attempts transition to `failed` state permanently
- [ ] Job steps are recorded in `job_steps` table
- [ ] OpenTelemetry traces are exported to configured OTLP endpoint
- [ ] Logs are JSON-formatted with trace ID correlation
- [ ] Unit tests cover queue operations (enqueue, claim, complete, fail, retry)
- [ ] Integration tests verify end-to-end job processing

---

## Dependencies & Constraints

### Dependencies
- **libsql**: Already in Cargo.toml, provides SQLite-compatible database
- **axum**: Already in Cargo.toml, provides HTTP server
- **tokio**: Already in Cargo.toml, provides async runtime
- **serde/serde_json**: Already in Cargo.toml, provides serialization

### Technical Constraints
- Must use SQLite-compatible SQL (no PostgreSQL-specific features)
- Must handle missing OTLP endpoint gracefully (disable tracing if not configured)
- Database path must support tilde expansion and relative paths
- All timestamps must be ISO 8601 TEXT format
- Job claim must be single atomic UPDATE (no SELECT-then-UPDATE)

---

## Implementation Notes

### Recommended Approach

1. **Start with configuration** - Build config structs and loading first, as everything else depends on it
2. **Database connection next** - Establish libsql connection pool with the config
3. **Migrations early** - Get all tables created before building features
4. **Queue operations in isolation** - Build and test queue functions independently
5. **Worker loop last** - Only after queue operations are solid
6. **OpenTelemetry throughout** - Add tracing incrementally as features are built

### Potential Gotchas

1. **libsql async API**: libsql has both sync and async APIs. Use the async API with tokio.
2. **SQLite busy handling**: May need to configure busy timeout for concurrent access.
3. **PRAGMA execution**: Foreign keys and other PRAGMAs must be set on each new connection.
4. **UUID generation**: Use `uuid` crate with v4 for random IDs, or v7 for time-ordered.
5. **Heartbeat interval**: Choose a reasonable interval (e.g., 30 seconds) that balances overhead vs detection latency.
6. **Stale job recovery**: Need a startup or periodic task to reset jobs stuck in `running` state with old heartbeats.

### Testing Approach

- **Unit tests**: Queue operations with in-memory libsql database
- **Integration tests**: Full job lifecycle with real database file
- **Concurrency tests**: Multiple workers claiming jobs simultaneously
- **Failure tests**: Worker crash simulation, database disconnect handling
