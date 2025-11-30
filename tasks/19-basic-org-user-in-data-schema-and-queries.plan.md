---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: basic org/user in data schema and queries
goal: Add org_id and user_id columns to all relevant database tables and update
  queries to filter on these columns, preparing for future multi-tenancy support
id: 19
uuid: 13f3a268-8ee5-4354-a336-f22820baf179
simple: false
status: done
priority: high
container: false
temp: false
dependencies: []
references: {}
issue: []
pullRequest: []
docs: []
createdAt: 2025-11-30T08:36:03.718Z
updatedAt: 2025-11-30T10:40:09.573Z
progressNotes:
  - timestamp: 2025-11-30T08:58:21.047Z
    text: Created migration 004_add_org_user_columns.sql adding org_id/user_id
      columns with defaults and composite indexes across accounts, threads,
      messages, decisions, actions, deterministic_rules, llm_rules, directions,
      rules_chat_sessions, and rules_chat_messages.
    source: "implementer: Task 1"
  - timestamp: 2025-11-30T08:58:44.213Z
    text: Added constants module exporting DEFAULT_ORG_ID and DEFAULT_USER_ID=1 and
      re-exported via ashford-core lib for callers.
    source: "implementer: Task 2"
  - timestamp: 2025-11-30T09:01:11.078Z
    text: Added migration coverage tests ensuring org_id/user_id columns and indexes
      exist with correct nullability; included 004 migration in runner; cargo
      test now passes 126/126.
    source: "tester: Tasks 1-2"
  - timestamp: 2025-11-30T09:14:57.613Z
    text: Implemented org_id/user_id support for Account, Thread, and Message
      repositories plus tests; updated dependent jobs and ingest flow to pass
      constants and filters; cargo test now green.
    source: "implementer: Tasks 3-5"
  - timestamp: 2025-11-30T09:27:20.570Z
    text: Added org/user scoping tests for accounts, threads, and messages; cargo
      test now includes new isolation coverage.
    source: "tester: Tasks 3-5"
  - timestamp: 2025-11-30T09:55:49.983Z
    text: Updated rules repositories (deterministic, LLM, directions) to carry
      org_id/user_id and scope queries accordingly; added org/user-aware tests
      validating org-wide NULL handling and tenant isolation; cargo test -p
      ashford-core now passing.
    source: "implementer: Tasks 8-10"
  - timestamp: 2025-11-30T09:59:23.140Z
    text: Added user-specific filtering and user-scope enforcement tests for
      deterministic, LLM, and direction repositories; cargo test -p ashford-core
      now passes 138 tests.
    source: "tester: tasks 8-10"
  - timestamp: 2025-11-30T10:19:42.227Z
    text: Added rules chat session and message repositories with org/user columns
      and scoped queries, updated types/exports, and verified with cargo test.
    source: "implementer: Tasks 11-14"
  - timestamp: 2025-11-30T10:20:50.844Z
    text: Ran workspace cargo test after rules chat scoping; all suites (140 core
      tests + bin/integration) passing with org/user coverage.
    source: "tester: tasks 11-14"
  - timestamp: 2025-11-30T10:33:22.431Z
    text: Added migration backfill test ensuring existing rows get org_id=1 and
      nullable user_id stays NULL; cargo test -p ashford-core now 143/143
      passing.
    source: "tester: migration coverage"
tasks:
  - title: Create database migration for org_id/user_id columns
    done: true
    description: >
      Create migration `004_add_org_user_columns.sql` that:

      1. Adds `org_id INTEGER NOT NULL DEFAULT 1` and `user_id INTEGER NOT NULL
      DEFAULT 1` to: accounts, threads, messages, decisions, actions,
      rules_chat_sessions, rules_chat_messages

      2. Adds `org_id INTEGER NOT NULL DEFAULT 1` and `user_id INTEGER`
      (nullable) to: deterministic_rules, llm_rules, directions

      3. Creates composite indexes `(org_id, user_id)` on all modified tables

      4. Updates any existing data to have org_id=1, user_id=1 (handled by
      DEFAULT)
    files:
      - server/migrations/004_add_org_user_columns.sql
  - title: Add hardcoded org/user ID constants
    done: true
    description: >
      Create a constants module with DEFAULT_ORG_ID and DEFAULT_USER_ID set to
      1.

      Export these from the ashford-core lib.rs.
    files:
      - server/crates/ashford-core/src/constants.rs
      - server/crates/ashford-core/src/lib.rs
  - title: Update AccountRepository for org_id/user_id
    done: true
    description: |
      Update AccountRepository in accounts.rs:
      1. Add org_id and user_id fields to Account struct
      2. Update ACCOUNT_COLUMNS constant to include org_id, user_id
      3. Update row_to_account to extract new fields
      4. Update create() to accept and insert org_id, user_id
      5. Update list methods to filter by org_id and optionally user_id
      6. Update existing tests to include org_id/user_id
    files:
      - server/crates/ashford-core/src/accounts.rs
  - title: Update ThreadRepository for org_id/user_id
    done: true
    description: |
      Update ThreadRepository in threads.rs:
      1. Add org_id and user_id fields to Thread struct
      2. Update THREAD_COLUMNS constant
      3. Update row_to_thread function
      4. Update create/upsert to accept and insert org_id, user_id
      5. Update list methods to filter by org_id, user_id
      6. Update existing tests
    files:
      - server/crates/ashford-core/src/threads.rs
  - title: Update MessageRepository for org_id/user_id
    done: true
    description: |
      Update MessageRepository in messages.rs:
      1. Add org_id and user_id fields to Message struct
      2. Update MESSAGE_COLUMNS constant
      3. Update row_to_message function
      4. Update create/upsert to accept and insert org_id, user_id
      5. Update list methods to filter by org_id, user_id
      6. Update existing tests
    files:
      - server/crates/ashford-core/src/messages.rs
  - title: Update DecisionRepository for org_id/user_id
    done: true
    description: |
      Update DecisionRepository in decisions/repositories.rs:
      1. Add org_id and user_id fields to Decision struct
      2. Update DECISION_COLUMNS constant
      3. Update row_to_decision function
      4. Update create to accept and insert org_id, user_id
      5. Update list methods to filter by org_id, user_id
      6. Update existing tests
    files:
      - server/crates/ashford-core/src/decisions/repositories.rs
      - server/crates/ashford-core/src/decisions/mod.rs
  - title: Update ActionRepository for org_id/user_id
    done: true
    description: |
      Update ActionRepository in decisions/repositories.rs:
      1. Add org_id and user_id fields to Action struct
      2. Update ACTION_COLUMNS constant
      3. Update row_to_action function
      4. Update create to accept and insert org_id, user_id
      5. Update list methods to filter by org_id, user_id
      6. Update existing tests
    files:
      - server/crates/ashford-core/src/decisions/repositories.rs
  - title: Update DeterministicRuleRepository for org_id/user_id
    done: true
    description: >
      Update DeterministicRuleRepository in rules/repositories.rs:

      1. Add org_id (required) and user_id (Option) fields to DeterministicRule
      struct

      2. Update DETERMINISTIC_RULE_COLUMNS constant

      3. Update row_to_deterministic_rule function

      4. Update create to accept org_id (required), user_id (optional)

      5. Update list methods to filter by org_id, optionally by user_id (NULL
      matches org-wide)

      6. Update existing tests in rules/test_repositories.rs
    files:
      - server/crates/ashford-core/src/rules/repositories.rs
      - server/crates/ashford-core/src/rules/test_repositories.rs
  - title: Update LlmRuleRepository for org_id/user_id
    done: true
    description: |
      Update LlmRuleRepository in rules/repositories.rs:
      1. Add org_id (required) and user_id (Option) fields to LlmRule struct
      2. Update LLM_RULE_COLUMNS constant
      3. Update row_to_llm_rule function
      4. Update create to accept org_id (required), user_id (optional)
      5. Update list methods to filter by org_id, optionally by user_id
      6. Update existing tests
    files:
      - server/crates/ashford-core/src/rules/repositories.rs
      - server/crates/ashford-core/src/rules/test_repositories.rs
  - title: Update DirectionsRepository for org_id/user_id
    done: true
    description: >
      Update DirectionsRepository in rules/repositories.rs:

      1. Add org_id (required) and user_id (Option) fields to Direction struct

      2. Update DIRECTION_COLUMNS constant

      3. Update row_to_direction function

      4. Update create to accept org_id (required), user_id (optional)

      5. Update list_enabled to filter by org_id, include both NULL and matching
      user_id

      6. Update existing tests
    files:
      - server/crates/ashford-core/src/rules/repositories.rs
      - server/crates/ashford-core/src/rules/test_repositories.rs
  - title: Update RulesChatSessionRepository for org_id/user_id
    done: true
    description: |
      Update RulesChatSessionRepository:
      1. Add org_id and user_id fields to RulesChatSession struct
      2. Update column constants and row conversion
      3. Update create/list methods
      4. Update tests
    files:
      - server/crates/ashford-core/src/rules/repositories.rs
  - title: Update RulesChatMessageRepository for org_id/user_id
    done: true
    description: |
      Update RulesChatMessageRepository:
      1. Add org_id and user_id fields to RulesChatMessage struct
      2. Update column constants and row conversion
      3. Update create/list methods
      4. Update tests
    files:
      - server/crates/ashford-core/src/rules/repositories.rs
  - title: Update callers to pass org_id/user_id from constants
    done: true
    description: >
      Find all places that call repository create/list methods and update them
      to pass

      the hardcoded DEFAULT_ORG_ID and DEFAULT_USER_ID constants. This includes:

      - Gmail sync logic

      - Decision/action creation

      - Rule management

      - Any HTTP handlers
    files:
      - server/crates/ashford-core/src/gmail/
      - server/crates/ashford-core/src/decisions/
      - server/crates/ashford-server/src/
  - title: Run all tests and fix any failures
    done: true
    description: >
      Run the full test suite with `cargo test` and fix any compilation errors
      or test failures.

      Ensure all existing functionality still works with the new org_id/user_id
      columns.
changedFiles:
  - .rmfilter/config/rmplan.yml
  - server/crates/ashford-core/src/accounts.rs
  - server/crates/ashford-core/src/constants.rs
  - server/crates/ashford-core/src/decisions/repositories.rs
  - server/crates/ashford-core/src/decisions/types.rs
  - server/crates/ashford-core/src/jobs/backfill_gmail.rs
  - server/crates/ashford-core/src/jobs/history_sync_gmail.rs
  - server/crates/ashford-core/src/jobs/ingest_gmail.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/messages.rs
  - server/crates/ashford-core/src/migrations.rs
  - server/crates/ashford-core/src/pubsub_listener.rs
  - server/crates/ashford-core/src/rules/mod.rs
  - server/crates/ashford-core/src/rules/repositories.rs
  - server/crates/ashford-core/src/rules/types.rs
  - server/crates/ashford-core/src/threads.rs
  - server/crates/ashford-core/tests/ingest_flow.rs
  - server/migrations/004_add_org_user_columns.sql
tags: []
---

Although the application currently only supports a single user, we should add organization ID and user ID columns to all the relevant tables.

We don't really need to support this properly at this point, but building all the queries to filter on these columns will make it much easier to add the support in the future.

For now, we should add the columns, set all the values to 1, and hardcode the current organization and current user in the application to ID 1.

We don't need actual tables for the organizations and users yet. That can be added in the future if we ever actually need to make this system multi-user.

## Expected Behavior/Outcome

- All relevant database tables have `org_id` and `user_id` INTEGER columns
- Tables with user-owned data (accounts, threads, messages, decisions, actions, chat sessions/messages) have NOT NULL constraints on both columns
- Tables with potentially org-wide data (deterministic_rules, llm_rules, directions) have NOT NULL org_id and nullable user_id
- All repository queries filter by org_id and user_id where appropriate
- Existing functionality continues to work with hardcoded values of 1 for both IDs
- All existing tests pass with updated seed data

## Acceptance Criteria

- [ ] Migration `004_add_org_user_columns.sql` adds columns to all 10 tables with appropriate nullability
- [ ] Composite indexes `(org_id, user_id)` created on all modified tables
- [ ] Constants module exports `DEFAULT_ORG_ID = 1` and `DEFAULT_USER_ID = 1`
- [ ] All repository structs include org_id and user_id fields
- [ ] All repository create methods accept org_id/user_id parameters
- [ ] All repository list methods filter by org_id (required) and user_id (where applicable)
- [ ] For nullable user_id tables, queries return both org-wide (NULL) and user-specific records
- [ ] All callers updated to pass hardcoded constants
- [ ] All existing tests updated and passing
- [ ] `cargo test` passes with no failures
- [ ] `cargo build` compiles without warnings related to new code

## Dependencies & Constraints

- **Dependencies**: None - this is foundational work
- **Technical Constraints**:
  - Cannot add foreign key constraints for org_id/user_id since org/user tables don't exist yet
  - Must maintain backward compatibility with existing data (DEFAULT 1 handles this)
  - SQLite/LibSQL doesn't support adding NOT NULL columns without defaults to existing tables

## Implementation Notes

**Recommended Approach:**
1. Start with the migration to add all columns at once
2. Add constants module
3. Update repositories one at a time, running tests after each
4. Update callers last, as this will require all repositories to be updated first

**Potential Gotchas:**
- Row conversion functions use column indices - adding org_id/user_id will shift indices if placed early in column list. Consider adding them at the end of column lists to minimize changes.
- The `(?1 IS NULL OR column = ?1)` pattern works for optional filtering but for nullable user_id columns, need special handling: `(user_id IS NULL OR user_id = ?1)` to get org-wide + user-specific records

## Research

### Summary

This feature adds `org_id` and `user_id` columns to all relevant database tables in preparation for future multi-tenancy support. The current system is single-user with multiple email accounts, so all data will use hardcoded values of `1` for both IDs. The primary goal is to update queries now so that future multi-user support is easier to add.

**Key Discoveries:**
- Database uses LibSQL (SQLite-compatible) with TEXT-based UUIDs for all IDs
- 11 tables need modification, with existing `account_id` filtering patterns to follow
- Repository pattern is used consistently with parameterized queries
- The `(?1 IS NULL OR column = ?1)` pattern is used for optional filtering
- All tables have `created_at` and `updated_at` timestamps

### Findings

#### Database System & Migration Structure

**Database:** LibSQL (SQLite-compatible, via Turso)
- Connection management: `server/crates/ashford-core/src/db.rs`
- Configuration: `server/crates/ashford-core/src/config.rs`
- Foreign keys enforced via `PRAGMA foreign_keys = ON`

**Migrations Location:** `server/migrations/`
- `001_initial.sql` - Main schema (281 lines)
- `002_add_job_completion_fields.sql` - Job completion tracking
- `003_add_thread_message_unique_indices.sql` - Uniqueness constraints
- Migration runner: `server/crates/ashford-core/src/migrations.rs`
- Tracked in `schema_migrations` table with version numbers

**ID Pattern:** TEXT primary keys containing UUID v4 strings
- Generated via `Uuid::new_v4().to_string()` in Rust

#### Tables Requiring org_id/user_id Columns

Based on analysis of `001_initial.sql`, these tables need modification:

1. **`accounts`** - Email accounts (Gmail currently)
   - Columns: id, provider, email, display_name, config_json, state_json, created_at, updated_at

2. **`threads`** - Email conversation threads
   - Already has: account_id (FK)
   - Other columns: id, provider_thread_id, subject, snippet, last_message_at, metadata_json, raw_json

3. **`messages`** - Individual email messages
   - Already has: account_id (FK), thread_id (FK)
   - Other columns: id, provider_message_id, from_email, from_name, to_json, cc_json, bcc_json, subject, snippet, received_at, internal_date, labels_json, headers_json, body_plain, body_html, raw_json

4. **`decisions`** - AI or deterministic decisions about messages
   - Already has: account_id (FK), message_id (FK)
   - Other columns: id, source, decision_json, action_type, confidence, needs_approval, rationale, telemetry_json

5. **`actions`** - Executed or queued actions on messages
   - Already has: account_id (FK), message_id (FK), decision_id (FK)
   - Other columns: id, action_type, parameters_json, status, error_message, executed_at, undo_hint_json, trace_id

6. **`deterministic_rules`** - Hard-coded rule conditions
   - Currently uses scope-based filtering (global, account, sender, domain)
   - Columns: id, name, description, scope, scope_ref, priority, enabled, conditions_json, action_type, action_parameters_json, safe_mode
   - Note: org_id required, user_id nullable (NULL = org-wide, set = user-specific)

7. **`llm_rules`** - LLM-based rules with natural language
   - Same scope pattern as deterministic_rules
   - Columns: id, name, description, scope, scope_ref, rule_text, enabled, metadata_json
   - Note: org_id required, user_id nullable (NULL = org-wide, set = user-specific)

8. **`directions`** - System-wide LLM instructions
   - Columns: id, content, enabled
   - Note: org_id required, user_id nullable (NULL = org-wide, set = user-specific)

9. **`rules_chat_sessions`** - Conversations about rules
    - Columns: id, title, created_at, updated_at

10. **`rules_chat_messages`** - Messages within rule chat sessions
    - Already has: session_id (FK)
    - Other columns: id, role, content

**Tables that should NOT get org_id/user_id:**
- `action_links` - Junction table for action relationships; filtering via joins to `actions` table
- `jobs` - Background job queue (system-level, not user data)
- `job_steps` - Job execution steps (system-level)
- `discord_whitelist` - Discord access control (system-level)
- `schema_migrations` - Migration tracking (system-level)

#### Repository Files Requiring Updates

All repositories are in `server/crates/ashford-core/src/`:

1. **`accounts.rs`** - AccountRepository
   - Methods: create, get_by_id, get_by_email, list_all, update, delete, refresh_tokens_if_needed

2. **`messages.rs`** - MessageRepository
   - Methods: create, get_by_id, get_by_provider_id, upsert, list_by_thread, update_labels, delete

3. **`threads.rs`** - ThreadRepository
   - Methods: create, get_by_id, get_by_provider_id, upsert, list_by_account, update_snippet, delete

4. **`decisions/repositories.rs`** - DecisionRepository, ActionRepository
   - DecisionRepository methods: create, get_by_id, get_for_message, list, update
   - ActionRepository methods: create, get_by_id, list_by_status, list_by_message, update_status, list_pending_approval

5. **`rules/repositories.rs`** - DeterministicRuleRepository, LlmRuleRepository, DirectionsRepository
   - DeterministicRuleRepository methods: create, get_by_id, list_enabled_by_scope, list_all, update, delete
   - LlmRuleRepository methods: create, get_by_id, list_enabled_by_scope, list_all, update, delete
   - DirectionsRepository methods: create, get_by_id, list_enabled, list_all, update, delete

6. **`rules/chat_repository.rs`** (if exists, or in rules/repositories.rs)
   - RulesChatSessionRepository, RulesChatMessageRepository

#### Existing Query Patterns

**Column Definition Constants Pattern:**
```rust
const DECISION_COLUMNS: &str = "id, account_id, message_id, source, decision_json, ...";
```

**Optional Filtering Pattern:**
```rust
pub async fn list(&self, account_id: Option<&str>) -> Result<Vec<Decision>, DecisionError> {
    let conn = self.db.connection().await?;
    let mut rows = conn.query(
        &format!(
            "SELECT {DECISION_COLUMNS}
             FROM decisions
             WHERE (?1 IS NULL OR account_id = ?1)
             ORDER BY created_at DESC"
        ),
        params![account_id],
    ).await?;
}
```

**Row Conversion Pattern:**
```rust
fn row_to_decision(row: Row) -> Result<Decision, DecisionError> {
    let source: String = row.get(3)?;
    let decision_json: String = row.get(4)?;
    // ... parse fields by index
    Ok(Decision { ... })
}
```

**Index Patterns (from migrations):**
```sql
CREATE INDEX decisions_message_idx ON decisions(message_id);
CREATE INDEX actions_status_idx ON actions(status, created_at);
CREATE INDEX deterministic_rules_scope_idx ON deterministic_rules(scope, scope_ref);
```

#### Application Context Handling

**Config Location:** `server/crates/ashford-core/src/config.rs`

Config supports `env:` prefixes for environment variables. Example structure:
```toml
[app]
service_name = "ashford"
port = 17800
env = "dev"
```

**Best Location for Hardcoded IDs:**
Option 1: Add to config struct in `config.rs`:
```rust
pub struct AppConfig {
    pub default_org_id: i64,  // = 1
    pub default_user_id: i64, // = 1
}
```

Option 2: Create constants module `server/crates/ashford-core/src/constants.rs`:
```rust
pub const DEFAULT_ORG_ID: i64 = 1;
pub const DEFAULT_USER_ID: i64 = 1;
```

**Recommendation:** Option 2 is simpler and matches the "hardcode for now" requirement. Can be migrated to config later if needed.

#### Test Patterns

Tests are located in `server/crates/ashford-core/src/` alongside the modules:
- `rules/test_repositories.rs` (1077 lines of comprehensive tests)
- Tests use temp databases with full migration runs
- Pattern: seed data, execute operations, assert results

Example test setup:
```rust
async fn setup_db() -> (Database, TempDir) {
    let dir = TempDir::new().expect("temp dir");
    let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
    let db_path = dir.path().join(db_name);
    let db = Database::new(&db_path).await.expect("create db");
    run_migrations(&db).await.expect("migrations");
    (db, dir)
}
```

### Risks & Constraints

1. **Column Type Decision:** Should use INTEGER (i64) for org_id/user_id, not TEXT UUIDs, since:
   - IDs are explicitly hardcoded to `1`
   - Future auto-increment compatibility
   - More efficient indexing and joins

2. **Migration Order:** New migration must run after existing ones; all tables must have columns added before any query changes

3. **Index Considerations:** New indexes needed for efficient filtering:
   - `(org_id, user_id)` composite index on frequently queried tables
   - May need to update existing indexes to include org_id/user_id as leading columns

4. **Breaking Changes:** Existing data must have org_id=1, user_id=1 set during migration (UPDATE after ALTER)

5. **Query Parameter Ordering:** Adding new parameters to queries affects row_to_* functions - column indices will shift if org_id/user_id are added early in column lists

6. **Scope-Based Rules Complexity:** `deterministic_rules` and `llm_rules` use a scope system. Need to decide:
   - Should org_id/user_id be separate from scope?
   - Or should scope=global mean "global within org"?
   - Recommendation: Add org_id/user_id AND keep scope system; scope is within-org filtering

7. **Chat Sessions/Messages:** These represent rule discussions - should be scoped to org/user

8. **Directions Table:** Currently global instructions - probably should be org-scoped, user-optional

9. **Test Updates:** All existing tests will need updates to include org_id/user_id in seed data and assertions

10. **No Foreign Keys for org/user:** Since we're not creating org/user tables yet, we can't add FK constraints - just plain INTEGER columns

Implemented Task 1 (Create database migration for org_id/user_id columns) and Task 2 (Add hardcoded org/user ID constants). Added migration server/migrations/004_add_org_user_columns.sql that appends org_id/user_id columns to accounts, threads, messages, decisions, actions, deterministic_rules, llm_rules, directions, rules_chat_sessions, and rules_chat_messages. User-owned tables use NOT NULL DEFAULT 1 for both columns; org-wide tables keep user_id nullable but default to 1 for compatibility, and every modified table gains a composite (org_id, user_id) index. Created server/crates/ashford-core/src/constants.rs with DEFAULT_ORG_ID and DEFAULT_USER_ID set to 1 and re-exported them through lib.rs so application callers can adopt the hardcoded IDs. No additional code paths were changed yet; this lays foundation for subsequent repository/query updates.

Completed Task 3: Update AccountRepository for org_id/user_id, Task 4: Update ThreadRepository for org_id/user_id, and Task 5: Update MessageRepository for org_id/user_id. Accounts, threads, and messages now carry org_id/user_id fields and every repository query scopes by these identifiers. In server/crates/ashford-core/src/accounts.rs added org_id/user_id to Account struct and ACCOUNT_COLUMNS, required org_id/user_id parameters on create/get/update/delete/list/refresh token paths, and scoped all SQL WHERE clauses; tests now inject DEFAULT_ORG_ID/DEFAULT_USER_ID. In server/crates/ashford-core/src/threads.rs added org_id/user_id to Thread plus column list, updated upsert/get/update_last_message_at to accept/filter by org/user, and refreshed tests. In server/crates/ashford-core/src/messages.rs extended Message/NewMessage with org_id/user_id, expanded MESSAGE_COLUMNS and upsert/get/exists to set and filter on org/user, and updated message tests. Updated dependent flows to compile and honor scoping: pubsub supervisor list_all (server/crates/ashford-core/src/pubsub_listener.rs), Gmail ingest/history/backfill jobs (server/crates/ashford-core/src/jobs/ingest_gmail.rs, .../history_sync_gmail.rs, .../backfill_gmail.rs) now pass defaults through repository calls; decisions test scaffolding seeds org/user aware accounts/threads/messages (server/crates/ashford-core/src/decisions/repositories.rs); ingestion integration tests adjusted for new repo signatures (server/crates/ashford-core/tests/ingest_flow.rs). Formatted code with cargo fmt and verified with cargo test (all 126 unit tests plus integration suites passing).

Addressed reviewer fixes for org/user scoping. Updated decisions domain types and repositories to carry org_id/user_id, insert those columns, and scope create/get/list/update queries by tenant; added helper seed functions plus new tests that verify decisions and actions are stored with the correct org/user and cannot be read or updated across tenants (server/crates/ashford-core/src/decisions/types.rs, .../decisions/repositories.rs). Hardened thread/message upserts by adding ON CONFLICT WHERE clauses that require matching org_id/user_id so conflicting inserts cannot reassigntenant ownership (server/crates/ashford-core/src/threads.rs, .../messages.rs). Corrected migration defaults for org-wide rule tables so user_id is nullable and existing rows are backfilled to NULL instead of defaulting to user 1 (server/migrations/004_add_org_user_columns.sql). Ran cargo fmt and cargo test -p ashford-core (workspace) to confirm all 131+ tests now pass.

Implemented org/user scoping for the rules data layer (Tasks 8-10). Added org_id and user_id (nullable for user-scoped data) to rule domain structs and builders in server/crates/ashford-core/src/rules/types.rs. Extended deterministic, LLM, and directions repositories to insert the new columns, scope create/get/list/update/delete queries by org_id plus user_id (returning both NULL and matching user rows where appropriate), and to persist user_id changes on updates while preventing cross-org access. Updated column lists and row mappers to read the new fields. Expanded repository tests to exercise tenant isolation and org-wide inclusion, ensuring list and scope filters return only current-org data while including org-wide records, and adjusted existing CRUD/scope tests to pass org/user filters. Ran cargo fmt -p ashford-core and cargo test -p ashford-core --manifest-path server/Cargo.toml to verify all 146 tests pass.

Implemented Tasks 11-12 (rules chat session/message repos) and Task 14 (test run) under the org/user scoping effort. Added multi-tenant aware chat role, session, and message domain types in server/crates/ashford-core/src/rules/types.rs including the RulesChatRole enum with as_str/from_str helpers plus NewRulesChatSession/NewRulesChatMessage builders; chat structs now carry org_id and user_id fields to match the new NOT NULL columns. Expanded server/crates/ashford-core/src/rules/repositories.rs with RULES_CHAT_SESSION_COLUMNS and RULES_CHAT_MESSAGE_COLUMNS constants, new error enums, and new repositories RulesChatSessionRepository and RulesChatMessageRepository whose create/get/list methods filter on org_id and user_id and return typed rows via new row_to_rules_chat_session/row_to_rules_chat_message helpers. Added sample builders and two isolation-focused tokio tests to the repository test module verifying sessions and messages cannot be fetched across org or user boundaries; updated exports in server/crates/ashford-core/src/rules/mod.rs and server/crates/ashford-core/src/lib.rs so consumers can access the new types and repositories. Ran cargo fmt and cargo test -p ashford-core --manifest-path server/Cargo.toml to confirm formatting and that all 140 tests (including the new chat tests) pass. No additional caller changes were needed for Task 13 because chat repositories were newly introduced and unused, but all new code defaults org/user to DEFAULT_ORG_ID/DEFAULT_USER_ID in test scaffolding.

Fixed reviewer feedback for org/user scoping of chat messages. RulesChatMessageRepository::create now verifies the target session belongs to the same org/user before inserting, returning NotFound when session ownership mismatches to prevent cross-tenant message injection. The method also updates rules_chat_sessions.updated_at to the message timestamp so list_for_user stays recency ordered. Added regression tests covering session ownership enforcement and updated_at bumping (with a small tokio sleep to ensure the timestamp advances). Changes are localized to server/crates/ashford-core/src/rules/repositories.rs and validated with cargo test -p ashford-core. Tasks: Update RulesChatSessionRepository for org_id/user_id; Update RulesChatMessageRepository for org_id/user_id; address reviewer issues on chat repositories. Design choices: reuse the existing NotFound error instead of a new variant, share one timestamp across insert and session bump, and rely on existing repository interfaces and migrations without schema changes.

Verified org/user multi-tenancy wiring is already present for accounts, threads, messages, and callers. Reviewed current repository code to ensure org_id/user_id columns and filters are in place, and that Gmail ingest jobs and pubsub listener pass DEFAULT_ORG_ID/DEFAULT_USER_ID. Ran full workspace tests with 'cargo test' from server/ (all 142 core tests and additional suites pass) confirming no regressions. No code changes were required today; the work validates Tasks 3-5 and caller updates are already implemented.
