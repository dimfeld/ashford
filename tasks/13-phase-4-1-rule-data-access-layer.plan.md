---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.1: Rule Data Access Layer"
goal: Create repository classes and domain types for deterministic rules, LLM
  rules, directions, decisions, and actions
id: 13
uuid: 38766c9f-5711-40f8-b264-292a865ef49e
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies: []
parent: 4
references: {}
issue: []
pullRequest: []
docs:
  - docs/data_model.md
  - docs/rules_engine.md
planGeneratedAt: 2025-11-30T01:24:24.156Z
promptsGeneratedAt: 2025-11-30T01:24:24.156Z
createdAt: 2025-11-30T01:14:18.743Z
updatedAt: 2025-11-30T02:06:15.923Z
progressNotes:
  - timestamp: 2025-11-30T01:29:45.557Z
    text: Created rules module with types/enums and repository implementations for
      deterministic rules, llm rules, and directions; updated lib exports. Tests
      still to run/finish.
    source: "implementer: tasks1-4-9"
  - timestamp: 2025-11-30T01:31:21.219Z
    text: Ran cargo test -p ashford-core; all 109 unit tests plus binary and
      integration tests pass. Rules repository tests present and passing.
    source: "tester: tasks 1-4,9"
  - timestamp: 2025-11-30T01:31:59.687Z
    text: Added list_all coverage tests for deterministic and LLM rules; re-ran
      cargo test -p ashford-core, now 111 tests pass (all green).
    source: "tester: tasks 1-4,9"
  - timestamp: 2025-11-30T01:41:52.630Z
    text: Implemented decisions module with Decision/Action/ActionLink types,
      repositories, and status validation; added comprehensive repo tests and
      updated lib exports; cargo test -p ashford-core now passes.
    source: "implementer: tasks5-11"
  - timestamp: 2025-11-30T01:45:24.672Z
    text: Added more decisions repository coverage (not-found paths, action list by
      message, approved_pending transitions, action link effect lookup) and
      verified constraints reject invalid enum strings. cargo test -p
      ashford-core now 121 tests passing.
    source: "tester: tasks5-11"
tasks:
  - title: Create rules module structure and shared types
    done: true
    description: "Create the `rules/` module directory structure with `mod.rs`,
      `types.rs`, and `repositories.rs`. Define shared enums: `RuleScope`
      (Global, Account, Sender, Domain) and `SafeMode` (Default, AlwaysSafe,
      DangerousOverride). Include `as_str()` and `from_str()` methods for
      database serialization. Add the module to `lib.rs`."
  - title: Implement DeterministicRule type and repository
    done: true
    description: "In `rules/types.rs`: Define `DeterministicRule` struct with all
      fields (id, name, description, scope, scope_ref, priority, enabled,
      conditions_json as Value, action_type, action_parameters_json as Value,
      safe_mode, created_at, updated_at). Define `NewDeterministicRule` for
      creation. In `rules/repositories.rs`: Implement
      `DeterministicRuleRepository` with methods: `create()`, `get_by_id()`,
      `list_all()`, `list_enabled_by_scope(scope, scope_ref)`, `update()`,
      `delete()`. Define `DeterministicRuleError`. Include
      `row_to_deterministic_rule()` helper."
  - title: Implement LlmRule type and repository
    done: true
    description: "In `rules/types.rs`: Define `LlmRule` struct (id, name,
      description, scope, scope_ref, rule_text, enabled, metadata_json as Value,
      created_at, updated_at) and `NewLlmRule`. In `rules/repositories.rs`:
      Implement `LlmRuleRepository` with methods: `create()`, `get_by_id()`,
      `list_all()`, `list_enabled_by_scope(scope, scope_ref)`, `update()`,
      `delete()`. Define `LlmRuleError`. Include `row_to_llm_rule()` helper."
  - title: Implement Direction type and repository
    done: true
    description: "In `rules/types.rs`: Define `Direction` struct (id, content,
      enabled, created_at, updated_at) and `NewDirection`. In
      `rules/repositories.rs`: Implement `DirectionsRepository` with methods:
      `create()`, `get_by_id()`, `list_all()`, `list_enabled()`, `update()`,
      `delete()`. Define `DirectionError`. Include `row_to_direction()` helper."
  - title: Create decisions module structure and types
    done: true
    description: "Create the `decisions/` module directory structure with `mod.rs`,
      `types.rs`, and `repositories.rs`. Define enums: `DecisionSource` (Llm,
      Deterministic), `ActionStatus` (Queued, Executing, Completed, Failed,
      Canceled, Rejected, ApprovedPending), `ActionLinkRelationType` (UndoOf,
      ApprovalFor, Spawned, Related). Include `as_str()` and `from_str()`
      methods. Add the module to `lib.rs`."
  - title: Implement Decision type and repository
    done: true
    description: "In `decisions/types.rs`: Define `Decision` struct (id, account_id,
      message_id, source, decision_json as Value, action_type, confidence,
      needs_approval, rationale, telemetry_json as Value, created_at,
      updated_at) and `NewDecision`. In `decisions/repositories.rs`: Implement
      `DecisionRepository` with methods: `create()`, `get_by_id()`,
      `get_by_message_id()`, `list_by_account()`, `list_recent()`. Define
      `DecisionError`. Include `row_to_decision()` helper."
  - title: Implement Action type and repository
    done: true
    description: "In `decisions/types.rs`: Define `Action` struct (id, account_id,
      message_id, decision_id, action_type, parameters_json as Value, status,
      error_message, executed_at, undo_hint_json as Value, trace_id, created_at,
      updated_at) and `NewAction`. In `decisions/repositories.rs`: Implement
      `ActionRepository` with methods: `create()`, `get_by_id()`,
      `get_by_decision_id()`, `list_by_message_id()`, `list_by_status()`,
      `update_status()`, `mark_executing()`, `mark_completed()`,
      `mark_failed()`. Define `ActionError`. Include status transition
      validation."
  - title: Implement ActionLink type and repository
    done: true
    description: "In `decisions/types.rs`: Define `ActionLink` struct (id,
      cause_action_id, effect_action_id, relation_type). In
      `decisions/repositories.rs`: Implement `ActionLinkRepository` with
      methods: `create()`, `get_by_cause_action_id()`,
      `get_by_effect_action_id()`, `delete()`. Define `ActionLinkError`. Include
      `row_to_action_link()` helper."
  - title: Add tests for rules repositories
    done: true
    description: Create comprehensive tests in `rules/repositories.rs` for all three
      repositories. Test CRUD operations, scope filtering for deterministic and
      LLM rules, enabled filtering for directions, not-found error cases, and
      JSON field round-tripping. Use the established test setup pattern with
      TempDir and unique database names.
  - title: Add tests for decisions repositories
    done: true
    description: Create comprehensive tests in `decisions/repositories.rs` for all
      three repositories. Test CRUD operations, foreign key relationships (seed
      accounts, threads, messages first), status transitions for actions, action
      link relationships, and not-found error cases. Verify that invalid status
      transitions are rejected.
  - title: Export types from lib.rs and verify compilation
    done: true
    description: "Update `lib.rs` to export all new public types: rules module
      (RuleScope, SafeMode, DeterministicRule, LlmRule, Direction, and their
      repositories/errors), decisions module (DecisionSource, ActionStatus,
      ActionLinkRelationType, Decision, Action, ActionLink, and their
      repositories/errors). Run `cargo build` and `cargo test` to verify
      everything compiles and tests pass."
changedFiles:
  - server/crates/ashford-core/src/decisions/mod.rs
  - server/crates/ashford-core/src/decisions/repositories.rs
  - server/crates/ashford-core/src/decisions/types.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/rules/mod.rs
  - server/crates/ashford-core/src/rules/repositories.rs
  - server/crates/ashford-core/src/rules/types.rs
tags: []
---

Foundation layer that provides data access for all rule types. This must be completed before any rule evaluation logic can be implemented.

## Key Components

### Domain Types
- `DeterministicRule` struct matching schema (id, name, scope, priority, conditions_json, action_type, etc.)
- `LLMRule` struct (id, name, scope, rule_text, metadata_json, etc.)
- `Direction` struct (id, content, enabled)
- `Decision` struct matching the JSON contract in decision_engine.md
- `Action` struct with status tracking

### Repositories
- `DeterministicRuleRepository` - load by scope (global, account, sender, domain), list all, create
- `LLMRuleRepository` - load by scope, list all, create
- `DirectionsRepository` - load all enabled, list all, create
- `DecisionRepository` - create, get by id, get by message_id
- `ActionRepository` - create, update status, get by decision_id

### Patterns to Follow
- Match existing repository patterns (AccountRepository, MessageRepository)
- Use `Database` wrapper with async methods
- Return `Result<T, XyzError>` with appropriate error types
- JSON serialization for complex fields using serde_json

### File Organization
```
ashford-core/src/
├── rules/
│   ├── mod.rs
│   ├── types.rs          # Domain structs
│   └── repositories.rs   # All rule repositories
├── decisions/
│   ├── mod.rs
│   ├── types.rs          # Decision & Action structs
│   └── repositories.rs   # Decision & Action repositories
```

### Testing
- Unit tests with in-memory database
- Test CRUD operations for each repository
- Test scope filtering for rules

<!-- rmplan-generated-start -->
## Expected Behavior/Outcome

This plan creates the foundational data access layer for the rules engine. Upon completion:

- **Rules Module**: Three repository classes (`DeterministicRuleRepository`, `LlmRuleRepository`, `DirectionsRepository`) provide full CRUD operations for rule management
- **Decisions Module**: Three repository classes (`DecisionRepository`, `ActionRepository`, `ActionLinkRepository`) provide data access for decision tracking and action execution
- **Type Safety**: All domain types are strongly typed with proper enums for constrained fields (scopes, statuses, relation types)
- **Pattern Consistency**: All new code follows existing repository patterns, making the codebase consistent and maintainable

## Acceptance Criteria

- [ ] All 6 entity types have corresponding Rust structs with proper derives
- [ ] All 6 repositories implement create, get_by_id, and list operations
- [ ] Scope-based filtering works for deterministic rules and LLM rules (global, account, sender, domain)
- [ ] Action status transitions are validated (no invalid state changes)
- [ ] All JSON fields round-trip correctly through the database
- [ ] Foreign key relationships are respected (decisions → messages, actions → decisions)
- [ ] All tests pass with `cargo test`
- [ ] Code compiles without warnings with `cargo build`

## Dependencies & Constraints

- **Dependencies**: Relies on existing `Database` wrapper, `run_migrations()`, and established test utilities
- **Technical Constraints**: Must use existing patterns (thiserror for errors, libsql for queries, serde_json for JSON fields)
- **Schema Constraint**: Database tables already exist; no migrations needed

## Implementation Notes

### Recommended Approach
1. Start with the rules module since it has no foreign key dependencies on other new types
2. Implement types first, then repositories, then tests for each module
3. Use `serde_json::Value` for flexible JSON fields initially (conditions_json, decision_json, etc.)
4. Follow the exact error type pattern from existing repositories

### Potential Gotchas
- **Scope filtering**: When filtering by scope, remember that `scope_ref` is NULL for global rules but contains a value for account/sender/domain scopes
- **Boolean fields**: SQLite stores booleans as INTEGER (0/1), not native booleans
- **Nullable fields**: Use `Option<T>` for nullable database columns and handle them in row conversion
- **Action status validation**: The `update_status()` method should validate that the transition is allowed (e.g., can't go from `completed` back to `queued`)
<!-- rmplan-generated-end -->

## Research

### Summary
- The ashford-core codebase has well-established patterns for repositories, domain types, and error handling that should be followed precisely
- All database tables for rules, decisions, and actions already exist in the schema (migration 001_initial.sql)
- The implementation requires creating domain types for 6 entity types (deterministic rules, LLM rules, directions, decisions, actions, action_links) and their repositories
- The codebase uses LibSQL (SQLite-compatible) with JSON serialization for complex fields, RFC3339 timestamps, and UUID primary keys

### Findings

#### Existing Repository Patterns

The codebase follows a consistent repository pattern across all modules:

**Repository Struct Pattern** (from `accounts.rs`, `messages.rs`, `threads.rs`):
```rust
#[derive(Clone)]
pub struct {Entity}Repository {
    db: Database,
}

impl {Entity}Repository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
    // async methods returning Result<T, {Entity}Error>
}
```

**Error Type Pattern** (using `thiserror`):
```rust
#[derive(Debug, Error)]
pub enum {Entity}Error {
    #[error("database error: {0}")]
    Database(#[from] DbError),
    #[error("sql error: {0}")]
    Sql(#[from] libsql::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("datetime parse error: {0}")]
    DateTimeParse(#[from] chrono::ParseError),
    #[error("{entity} not found: {0}")]
    NotFound(String),
}
```

**Row Conversion Pattern**:
```rust
fn row_to_{entity}(row: Row) -> Result<{Entity}, {Entity}Error> {
    let json_field: String = row.get(N)?;
    let timestamp: String = row.get(M)?;
    Ok({Entity} {
        id: row.get(0)?,
        // ...
        json_field: serde_json::from_str(&json_field)?,
        timestamp: DateTime::parse_from_rfc3339(&timestamp)?.with_timezone(&Utc),
    })
}
```

**Timestamp Utilities** (repeated in each module):
```rust
fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn to_rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}
```

**Key Files:**
- `server/crates/ashford-core/src/accounts.rs` - AccountRepository with CRUD, optimistic locking
- `server/crates/ashford-core/src/messages.rs` - MessageRepository with upsert pattern
- `server/crates/ashford-core/src/threads.rs` - ThreadRepository with upsert
- `server/crates/ashford-core/src/queue.rs` - JobQueue with state machine transitions
- `server/crates/ashford-core/src/db.rs` - Database wrapper (Arc<LibSqlDatabase>, enables foreign keys)

#### Database Schema for Rules and Decisions

All tables exist in `server/migrations/001_initial.sql`:

**deterministic_rules** table:
```sql
CREATE TABLE deterministic_rules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  scope TEXT NOT NULL CHECK (scope IN ('global','account','sender','domain')),
  scope_ref TEXT,                      -- account_id, domain, or sender email
  priority INTEGER NOT NULL DEFAULT 100,
  enabled INTEGER NOT NULL DEFAULT 1,
  conditions_json TEXT NOT NULL,       -- structured condition tree
  action_type TEXT NOT NULL,
  action_parameters_json TEXT NOT NULL,
  safe_mode TEXT NOT NULL CHECK (safe_mode IN ('default','always_safe','dangerous_override')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX deterministic_rules_scope_idx ON deterministic_rules(scope, scope_ref);
CREATE INDEX deterministic_rules_priority_idx ON deterministic_rules(enabled, priority);
```

**llm_rules** table:
```sql
CREATE TABLE llm_rules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  scope TEXT NOT NULL CHECK (scope IN ('global','account','sender','domain')),
  scope_ref TEXT,
  rule_text TEXT NOT NULL,             -- natural-language description
  enabled INTEGER NOT NULL DEFAULT 1,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX llm_rules_scope_idx ON llm_rules(scope, scope_ref);
CREATE INDEX llm_rules_enabled_idx ON llm_rules(enabled, created_at);
```

**directions** table:
```sql
CREATE TABLE directions (
  id TEXT PRIMARY KEY,
  content TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX directions_enabled_idx ON directions(enabled, created_at);
```

**decisions** table:
```sql
CREATE TABLE decisions (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  message_id TEXT NOT NULL,
  source TEXT NOT NULL CHECK (source IN ('llm','deterministic')),
  decision_json TEXT NOT NULL,
  action_type TEXT,
  confidence REAL,
  needs_approval INTEGER NOT NULL DEFAULT 0,
  rationale TEXT,
  telemetry_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(id),
  FOREIGN KEY (message_id) REFERENCES messages(id)
);
CREATE INDEX decisions_message_idx ON decisions(message_id);
CREATE INDEX decisions_created_idx ON decisions(created_at);
```

**actions** table:
```sql
CREATE TABLE actions (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  message_id TEXT NOT NULL,
  decision_id TEXT,                    -- nullable (manual action)
  action_type TEXT NOT NULL,
  parameters_json TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('queued','executing','completed','failed','canceled','rejected','approved_pending')),
  error_message TEXT,
  executed_at TEXT,
  undo_hint_json TEXT NOT NULL DEFAULT '{}',
  trace_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (account_id) REFERENCES accounts(id),
  FOREIGN KEY (message_id) REFERENCES messages(id),
  FOREIGN KEY (decision_id) REFERENCES decisions(id)
);
CREATE INDEX actions_message_idx ON actions(message_id, created_at);
CREATE INDEX actions_status_idx ON actions(status, created_at);
```

**action_links** table:
```sql
CREATE TABLE action_links (
  id TEXT PRIMARY KEY,
  cause_action_id TEXT NOT NULL,
  effect_action_id TEXT NOT NULL,
  relation_type TEXT NOT NULL CHECK (relation_type IN ('undo_of','approval_for','spawned','related')),
  FOREIGN KEY (cause_action_id) REFERENCES actions(id),
  FOREIGN KEY (effect_action_id) REFERENCES actions(id)
);
CREATE INDEX action_links_cause_idx ON action_links(cause_action_id);
CREATE INDEX action_links_effect_idx ON action_links(effect_action_id);
```

#### Domain Type Patterns

**Common Derives**:
- Domain types: `#[derive(Debug, Clone, PartialEq)]` or `#[derive(Debug, Clone, PartialEq, Eq)]`
- Serializable types: Add `Serialize, Deserialize`
- Default constructors: Add `Default` derive

**Enum Pattern with Database String Mapping** (from `queue.rs:JobState`):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Completed,
    Failed,
    Canceled,
}

impl JobState {
    fn as_str(&self) -> &'static str {
        match self {
            JobState::Queued => "queued",
            // ...
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "queued" => Some(Self::Queued),
            // ...
            _ => None,
        }
    }
}
```

**Alternative: Serde rename_all** (from `accounts.rs:SyncStatus`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    #[default]
    Normal,
    NeedsBackfill,
    Backfilling,
}
```

**Enums Required for Rules Implementation:**
```rust
// Rule scopes (used by both deterministic_rules and llm_rules)
pub enum RuleScope {
    Global,
    Account,
    Sender,
    Domain,
}

// Safe mode levels for deterministic rules
pub enum SafeMode {
    Default,
    AlwaysSafe,
    DangerousOverride,
}

// Decision source
pub enum DecisionSource {
    Llm,
    Deterministic,
}

// Action status (7 states)
pub enum ActionStatus {
    Queued,
    Executing,
    Completed,
    Failed,
    Canceled,
    Rejected,
    ApprovedPending,
}

// Action link relation types
pub enum ActionLinkRelationType {
    UndoOf,
    ApprovalFor,
    Spawned,
    Related,
}
```

#### Test Patterns

**Test Setup Pattern** (from all repository tests):
```rust
async fn setup_repo() -> ({Entity}Repository, TempDir) {
    let dir = TempDir::new().expect("temp dir");
    let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
    let db_path = dir.path().join(db_name);
    let db = Database::new(&db_path).await.expect("create db");
    run_migrations(&db).await.expect("migrations");
    ({Entity}Repository::new(db), dir)
}
```

**CRUD Test Pattern**:
```rust
#[tokio::test]
async fn create_and_lookup() {
    let (repo, _dir) = setup_repo().await;
    let created = repo.create(...).await.expect("create");
    let fetched = repo.get_by_id(&created.id).await.expect("get");
    assert_eq!(created, fetched);
}

#[tokio::test]
async fn not_found_returns_error() {
    let (repo, _dir) = setup_repo().await;
    let err = repo.get_by_id("nonexistent").await.expect_err("should fail");
    assert!(matches!(err, {Entity}Error::NotFound(_)));
}
```

**Multi-Repository Seeding** (from `messages.rs` tests):
```rust
async fn seed_account(db: &Database) -> String {
    let repo = AccountRepository::new(db.clone());
    repo.create(...).await.expect("create").id
}
```

**Test Dependencies:**
- `tempfile` - Isolated test databases
- `tokio::test` - Async test runner
- `uuid::Uuid::new_v4()` - Unique database names

#### Module Organization

**Current `lib.rs` exports pattern:**
```rust
pub mod {module};
pub use {module}::{Type1, Type2, Repository};
```

**Proposed file structure:**
```
server/crates/ashford-core/src/
├── rules/
│   ├── mod.rs           # Re-exports types and repositories
│   ├── types.rs         # RuleScope, SafeMode, DeterministicRule, LlmRule, Direction
│   └── repositories.rs  # DeterministicRuleRepository, LlmRuleRepository, DirectionsRepository
├── decisions/
│   ├── mod.rs           # Re-exports
│   ├── types.rs         # DecisionSource, ActionStatus, ActionLinkRelationType, Decision, Action, ActionLink
│   └── repositories.rs  # DecisionRepository, ActionRepository, ActionLinkRepository
```

#### Rules Engine Evaluation Flow

From `docs/rules_engine.md`:

1. **Deterministic Rules First**: Evaluated by ascending priority, if any match → produce actions, skip LLM
2. **Directions**: Global constraints always injected into LLM prompt and enforced in Rust post-processing
3. **LLM Rules**: Situational guidance loaded based on scope, included in prompt after directions

**Hierarchy**: Deterministic Rules > Directions > LLM Rules

#### Key Dependencies (from Cargo.toml)

- `libsql` - Database driver with async support
- `serde` + `serde_json` - JSON serialization
- `chrono` - DateTime handling (RFC3339 format)
- `uuid` - ID generation
- `thiserror` - Error handling derives
- `tokio` - Async runtime

### Risks & Constraints

1. **Foreign Key Dependencies**: Actions reference decisions (nullable), and decisions reference messages. Tests must seed parent entities first.

2. **Scope Filtering Complexity**: Rules have 4 scope types (global, account, sender, domain) with optional `scope_ref`. The repository query methods need to handle:
   - Global rules (scope_ref is NULL)
   - Account-specific rules (scope_ref = account_id)
   - Sender-specific rules (scope_ref = email address)
   - Domain-specific rules (scope_ref = domain string)

3. **JSON Field Flexibility**: The `conditions_json` field for deterministic rules stores a "structured condition tree" but the exact schema isn't defined in the database. The repository should treat it as opaque JSON (`serde_json::Value`) initially, with structured types added in the evaluation layer.

4. **decision_json Contract**: The full decision contract is stored as JSON. Initial implementation can use `serde_json::Value`, but a typed struct matching the contract from the rules engine documentation would be better for type safety.

5. **ActionStatus State Machine**: The 7 action statuses form a state machine. Update methods should validate state transitions (e.g., only `queued` → `executing`, not `completed` → `queued`).

6. **Nullable Fields**: Several fields are nullable:
   - `decisions.action_type` and `decisions.confidence` (convenience copies)
   - `actions.decision_id` (supports manual actions)
   - `actions.error_message` and `actions.executed_at`
   - Rules' `description` and `scope_ref`

7. **Test Isolation**: Each test needs its own database file (using UUID in filename) to allow parallel test execution without conflicts.

8. **Timestamp Handling**: All timestamps stored as RFC3339 TEXT strings. Use `DateTime::<Utc>::parse_from_rfc3339()` for parsing and helper functions for formatting.

Implemented rules data access layer (Tasks 1-4 & 9). Added new rules module with enums RuleScope and SafeMode plus domain types for DeterministicRule, LlmRule, Direction and their New* builders in src/rules/types.rs. Implemented DeterministicRuleRepository, LlmRuleRepository, and DirectionsRepository in src/rules/repositories.rs following existing repository patterns: scoped create/get/list/update/delete methods, scope filtering helpers, JSON serialization for structured fields, RFC3339 timestamps, and error enums covering database, parsing, not-found, and invalid enum values. Added module exports in lib.rs for new types/repositories. Comprehensive unit tests in rules/repositories.rs exercise CRUD flows, scope filtering, enabled filtering, update behaviors, and JSON round-tripping using in-memory migrated databases. All ashford-core tests pass after cargo test.

Implemented Task 5 (decisions module types), Task 6 (Decision repository), Task 7 (Action repository), Task 8 (ActionLink repository), Task 10 (tests), and Task 11 (exports/build). Added new module at server/crates/ashford-core/src/decisions with mod.rs, types.rs, and repositories.rs. types.rs defines DecisionSource, ActionStatus, ActionLinkRelationType enums with as_str/from_str plus domain structs Decision, NewDecision, Action, NewAction, ActionLink, NewActionLink. repositories.rs implements DecisionRepository (create/get_by_id/get_by_message_id/list_by_account/list_recent) with JSON serialization and RFC3339 timestamps; ActionRepository (create/get/list, status update helpers, transition validation allowing queued→executing/completed/failed, etc., with mark_executing/mark_completed/mark_failed convenience); ActionLinkRepository CRUD. Added row parsing helpers with enum validation and error enums mirroring existing patterns. Tests in repositories.rs seed accounts/threads/messages using existing repos, cover CRUD paths, status transitions (including invalid transitions), failure updates, and action link relationships. Updated lib.rs to expose decisions module types/repositories/errors. Ran cargo fmt and cargo test -p ashford-core (all 117 tests now pass).

Addressed reviewer fixes for tasks 5-11 around the decisions/action repositories. In server/crates/ashford-core/src/decisions/repositories.rs I now validate initial action states via is_valid_initial_status and return a new InvalidInitialStatus error so callers cannot insert completed/failed/rejected actions directly; list_recent now requires an account id and scopes the SQL to that tenant to avoid cross-account leaks. update_status performs an atomic UPDATE ... WHERE status check, preserving optimistic concurrency, and uses COALESCE together with executed_at_to_set to keep the first execution timestamp instead of overwriting it when completing or failing. Added a helper seed_account_with_email plus new tests to cover account-scoped recent listing and rejection of terminal initial states; test suite bumped to 122 cases and cd server && cargo test -p ashford-core passes. These changes integrate with existing repositories via the same Database plumbing and keep ActionRepository public API type-safe while tightening lifecycle invariants.
