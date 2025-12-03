---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Phase 4.2: Deterministic Rule Engine"
goal: Implement condition tree parsing and evaluation, and deterministic rule
  execution
id: 14
uuid: 4faa40e3-cbc5-4d8c-a596-225ab64a50d9
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies:
  - 13
parent: 4
references:
  "4": 5cf4cc37-3eb8-4f89-adae-421a751d13a1
  "13": 38766c9f-5711-40f8-b264-292a865ef49e
issue: []
pullRequest: []
docs:
  - docs/rules_engine.md
planGeneratedAt: 2025-12-01T18:32:35.452Z
promptsGeneratedAt: 2025-12-01T18:32:35.452Z
createdAt: 2025-11-30T01:14:18.896Z
updatedAt: 2025-12-01T21:35:46.324Z
progressNotes:
  - timestamp: 2025-12-01T18:41:19.221Z
    text: Implemented condition parsing/evaluation module with logical operators,
      leaf matchers, regex cache, and comprehensive unit tests covering
      matching, logical nesting, and parsing; added regex dependency.
    source: "implementer: deterministic-rule-engine"
  - timestamp: 2025-12-01T18:44:44.931Z
    text: Added rule loader and executor with scope aggregation and first-match
      evaluation, plus integration tests covering scopes, priority ordering, and
      error handling; ran cargo build/test/clippy (clippy reports pre-existing
      warnings outside this change).
    source: "implementer: deterministic-rule-engine"
  - timestamp: 2025-12-01T18:46:42.949Z
    text: "Initial state: ran 'cargo test -p ashford-core'; all 192 unit tests and 2
      integration tests passed. No changes yet."
    source: "tester: coverage"
  - timestamp: 2025-12-01T18:48:22.784Z
    text: "Expanded tests for conditions and deterministic rule executor: added
      edge-case coverage (case-insensitive wildcard sender, malformed domains,
      missing subjects, non-string headers, NOT with multiple children, empty
      condition tree) plus executor cases for case-insensitive domain rules and
      missing sender email. Ran cargo fmt, cargo test -p ashford-core (all 200
      tests pass), cargo clippy -p ashford-core --tests (only pre-existing
      warnings)."
    source: "tester: coverage"
  - timestamp: 2025-12-01T21:21:02.417Z
    text: "Code review: flagged header_match never matches because headers stored as
      Gmail header array but evaluator expects object map; sender-scope rule
      lookup is case-sensitive so mixed-case From addresses skip sender rules;
      AND with empty children currently returns true, making empty condition
      tree match everything instead of erroring."
    source: "reviewer: deterministic-rule-engine"
  - timestamp: 2025-12-01T21:28:08.393Z
    text: "Addressed review findings: added validation so AND/OR with no children
      are rejected at parse/eval time; sender-scoped rules now load
      case-insensitively by lowercasing scope_ref on write and querying with
      LOWER(scope_ref), with new mixed-case sender integration tests. Ran cargo
      fmt and cargo test -p ashford-core (all pass)."
    source: "implementer: review-fixes"
  - timestamp: 2025-12-01T21:29:40.363Z
    text: Ran cargo test -p ashford-core from server/; all tests passed (205 unit +
      10 bin + 2 integration).
    source: "tester: Phase 4.2 deterministic rule engine"
  - timestamp: 2025-12-01T21:31:07.710Z
    text: Added repository-level case-insensitive scope_ref tests for domain/sender
      and reran cargo test -p ashford-core; 207 unit/bin/integration tests
      passing.
    source: "tester: Phase 4.2 deterministic rule engine"
tasks:
  - title: Add regex crate dependency
    done: true
    description: Add the `regex` crate to ashford-core's Cargo.toml dependencies.
      Use `cargo add regex` to get the latest version. This is needed for
      subject_regex and header_match condition evaluation.
  - title: Define condition types in conditions.rs
    done: true
    description: >-
      Create `server/crates/ashford-core/src/rules/conditions.rs` with:


      1. `LogicalOperator` enum: And, Or, Not (with serde rename_all snake_case)

      2. `LeafCondition` enum with tagged variants (serde tag="type"):
         - SenderEmail { value: String }
         - SenderDomain { value: String }
         - SubjectContains { value: String }
         - SubjectRegex { value: String }
         - HeaderMatch { header: String, pattern: String }
         - LabelPresent { value: String }
      3. `LogicalCondition` struct: { op: LogicalOperator, children:
      Vec<Condition> }

      4. `Condition` enum (untagged): Logical(LogicalCondition),
      Leaf(LeafCondition)

      5. `ConditionError` enum with variants: InvalidJson, InvalidRegex,
      InvalidNotChildCount, EmptyTree


      Include proper derives: Debug, Clone, PartialEq, Serialize, Deserialize.
      Add helper function `parse_condition(value: &serde_json::Value) ->
      Result<Condition, ConditionError>`.
  - title: Implement condition evaluator
    done: true
    description: >-
      In `conditions.rs`, implement the condition evaluation logic:


      1. Create `EvaluationContext` struct to hold cached compiled regexes:
      `struct EvaluationContext { regex_cache: HashMap<String, Regex> }`

      2. Implement `EvaluationContext::new()` and
      `EvaluationContext::get_or_compile_regex(&mut self, pattern: &str) ->
      Result<&Regex, ConditionError>`

      3. Implement `evaluate(condition: &Condition, message: &Message, ctx: &mut
      EvaluationContext) -> Result<bool, ConditionError>`

      4. Implement evaluation for each leaf condition:
         - `sender_email`: exact match or wildcard (*@domain.com) against message.from_email
         - `sender_domain`: extract domain from message.from_email using rfind('@'), compare case-insensitively
         - `subject_contains`: case-insensitive substring match against message.subject
         - `subject_regex`: regex match against message.subject
         - `header_match`: find header (case-insensitive name), regex match value
         - `label_present`: check if value exists in message.labels
      5. Implement logical operators: AND (all children true), OR (any child
      true), NOT (single child, invert result)

      6. Add helper `fn extract_domain(email: &str) -> Option<&str>` using
      rfind('@')
  - title: Add unit tests for leaf conditions
    done: true
    description: >-
      In `conditions.rs`, add `#[cfg(test)] mod tests` with unit tests for each
      leaf condition type:


      1. `sender_email_exact_match` - exact email matches

      2. `sender_email_wildcard` - *@domain.com pattern matches any local part

      3. `sender_email_no_match` - non-matching email returns false

      4. `sender_email_missing` - message.from_email is None returns false

      5. `sender_domain_matches` - domain extracted and matched
      case-insensitively

      6. `sender_domain_no_match` - different domain returns false

      7. `subject_contains_match` - substring found (case-insensitive)

      8. `subject_contains_no_match` - substring not found

      9. `subject_contains_missing` - message.subject is None returns false

      10. `subject_regex_match` - valid regex matches

      11. `subject_regex_no_match` - valid regex doesn't match

      12. `subject_regex_invalid` - invalid regex returns
      ConditionError::InvalidRegex

      13. `header_match_found` - header exists and value matches regex

      14. `header_match_header_not_found` - header doesn't exist returns false

      15. `header_match_value_no_match` - header exists but value doesn't match

      16. `header_match_case_insensitive_name` - header name matched
      case-insensitively

      17. `label_present_found` - label in message.labels returns true

      18. `label_present_not_found` - label not in list returns false

      19. `label_present_empty_labels` - empty labels vec returns false


      Create helper `fn sample_message() -> Message` that returns a Message with
      test data.
  - title: Add unit tests for logical operators and nesting
    done: true
    description: >-
      In `conditions.rs` tests module, add tests for logical operators:


      1. `and_all_true` - AND with all children true returns true

      2. `and_one_false` - AND with one false child returns false

      3. `and_empty_children` - AND with empty children returns true (identity)

      4. `or_one_true` - OR with one true child returns true

      5. `or_all_false` - OR with all children false returns false

      6. `or_empty_children` - OR with empty children returns false (identity)

      7. `not_inverts_true` - NOT with true child returns false

      8. `not_inverts_false` - NOT with false child returns true

      9. `not_wrong_child_count` - NOT with 0 or 2+ children returns
      InvalidNotChildCount error

      10. `nested_and_or` - AND containing OR conditions evaluates correctly

      11. `nested_or_and` - OR containing AND conditions evaluates correctly

      12. `deeply_nested` - 4+ levels of nesting evaluates correctly

      13. `complex_tree` - realistic condition tree from plan example
      (amazon.com AND (shipped OR delivered))


      Also test JSON parsing:

      14. `parse_leaf_condition` - JSON with type field parses to correct
      LeafCondition variant

      15. `parse_logical_condition` - JSON with op and children parses correctly

      16. `parse_nested_tree` - complex nested JSON parses correctly

      17. `parse_invalid_json` - malformed JSON returns InvalidJson error
  - title: Create deterministic.rs with RuleMatch and RuleLoader
    done: true
    description: >-
      Create `server/crates/ashford-core/src/rules/deterministic.rs` with:


      1. `RuleMatch` struct:
         ```rust
         pub struct RuleMatch {
             pub rule: DeterministicRule,
             pub action_type: String,
             pub action_parameters: Value,
             pub safe_mode: SafeMode,
         }
         ```

      2. `RuleLoaderError` enum with variants: Database, ConditionParse


      3. `RuleLoader` struct:
         ```rust
         pub struct RuleLoader {
             repo: DeterministicRuleRepository,
         }
         ```

      4. Implement `RuleLoader::new(repo: DeterministicRuleRepository)`


      5. Implement `RuleLoader::load_applicable_rules(&self, org_id, user_id,
      account_id, sender_email: Option<&str>) -> Result<Vec<DeterministicRule>,
      RuleLoaderError>`:
         - Load global rules (scope=Global, scope_ref=None)
         - Load account rules (scope=Account, scope_ref=account_id)
         - If sender_email provided, extract domain and load domain rules (scope=Domain, scope_ref=domain)
         - If sender_email provided, load sender rules (scope=Sender, scope_ref=sender_email)
         - Merge all rules and sort by priority ascending
         - Return combined list
  - title: Implement RuleExecutor
    done: true
    description: >-
      In `deterministic.rs`, add:


      1. `ExecutorError` enum with variants: RuleLoader(RuleLoaderError),
      Condition(ConditionError)


      2. `RuleExecutor` struct:
         ```rust
         pub struct RuleExecutor {
             loader: RuleLoader,
         }
         ```

      3. Implement `RuleExecutor::new(repo: DeterministicRuleRepository)`


      4. Implement `RuleExecutor::evaluate(&self, org_id, user_id, message:
      &Message) -> Result<Option<RuleMatch>, ExecutorError>`:
         - Call loader.load_applicable_rules() with message.account_id and message.from_email
         - Create EvaluationContext for regex caching
         - Iterate through rules in order (already sorted by priority)
         - For each rule:
           - Parse rule.conditions_json into Condition
           - Call evaluate(condition, message, ctx)
           - If match, construct RuleMatch and return Some(match)
         - If no rules match, return None
         - Propagate errors appropriately
  - title: Add integration tests for RuleExecutor
    done: true
    description: >-
      In `deterministic.rs`, add `#[cfg(test)] mod tests` with integration
      tests:


      1. Setup helper `async fn setup_executor() -> (RuleExecutor, Database,
      TempDir)` that creates DB, runs migrations, creates executor


      2. Helper `fn sample_message_for_executor(account_id: &str) -> Message`
      with realistic test data


      3. `executor_no_rules_returns_none` - empty database returns Ok(None)


      4. `executor_single_matching_rule` - one rule matches, returns RuleMatch
      with correct fields


      5. `executor_single_non_matching_rule` - one rule doesn't match, returns
      None


      6. `executor_priority_ordering` - multiple matching rules, lowest priority
      number wins


      7. `executor_first_match_stops` - after first match, later rules not
      evaluated (can verify by having later rule with invalid regex that would
      error if evaluated)


      8. `executor_global_scope_matches` - global rule matches any message


      9. `executor_account_scope_matches` - account-scoped rule matches only
      that account


      10. `executor_domain_scope_matches` - domain-scoped rule matches sender
      from that domain


      11. `executor_sender_scope_matches` - sender-scoped rule matches exact
      sender email


      12. `executor_scope_aggregation` - rules from multiple scopes are all
      considered, priority determines winner


      13. `executor_disabled_rules_skipped` - disabled rules not evaluated


      14. `executor_invalid_condition_returns_error` - rule with invalid regex
      in conditions returns ExecutorError
  - title: Update mod.rs exports and verify build
    done: true
    description: >-
      Update `server/crates/ashford-core/src/rules/mod.rs` to:


      1. Add module declarations:
         ```rust
         pub mod conditions;
         pub mod deterministic;
         ```

      2. Add public exports for new types:
         ```rust
         pub use conditions::{Condition, ConditionError, EvaluationContext, LeafCondition, LogicalCondition, LogicalOperator};
         pub use deterministic::{ExecutorError, RuleExecutor, RuleLoader, RuleLoaderError, RuleMatch};
         ```

      3. Run `cargo build -p ashford-core` to verify no compilation errors


      4. Run `cargo test -p ashford-core` to verify all tests pass


      5. Run `cargo clippy -p ashford-core` and fix any warnings
changedFiles:
  - server/Cargo.lock
  - server/crates/ashford-core/Cargo.toml
  - server/crates/ashford-core/src/rules/conditions.rs
  - server/crates/ashford-core/src/rules/deterministic.rs
  - server/crates/ashford-core/src/rules/mod.rs
  - server/crates/ashford-core/src/rules/repositories.rs
tags: []
---

Core deterministic rule evaluation engine that provides the "fast path" for email classification. This handles explicit, structured rules before LLM involvement.

## Key Components

### Condition Tree Schema & Parser
Define JSON schema for condition trees supporting:
- **Logical operators**: AND, OR, NOT
- **Leaf conditions**:
  - `sender_email` - exact match or wildcard (e.g., `*@amazon.com`)
  - `sender_domain` - domain match
  - `subject_contains` - substring match
  - `subject_regex` - regex pattern match
  - `header_match` - specific header value check
  - `label_present` - Gmail label exists

Example condition tree:
```json
{
  "op": "AND",
  "children": [
    { "type": "sender_domain", "value": "amazon.com" },
    { "op": "OR", "children": [
      { "type": "subject_contains", "value": "shipped" },
      { "type": "subject_contains", "value": "delivered" }
    ]}
  ]
}
```

### Condition Evaluator
- Recursive tree evaluation
- Pattern matching for different condition types
- Efficient regex caching if needed
- Returns `bool` for match result

### Rule Loader
- Load rules by scope (global → account → domain → sender)
- Sort by priority (ascending)
- Filter to enabled rules only

### Rule Executor
- Evaluate rules against message metadata
- First-match mode: stop after first matching rule (all-matches mode deferred to future work)
- Return matched rule with action (or None if no match)
- Handle `safe_mode` field for dangerous action policy

### File Organization
```
ashford-core/src/rules/
├── conditions.rs    # Condition types and evaluator
├── deterministic.rs # Rule loader and executor
```

### Testing
- Unit tests for each condition type
- Complex condition tree evaluation tests
- Priority ordering tests
- First-match behavior tests (stops at first matching rule)

## Research

### Summary

The deterministic rule engine builds on a solid foundation established in Phase 4.1 (Plan 13). The `rules` module already exists with `DeterministicRule` types and `DeterministicRuleRepository` for data access. The key work in this phase is defining structured condition types, implementing the condition tree evaluator, and creating the rule executor that loads rules by scope and evaluates them against email messages.

Key insights:
- The `conditions_json` field is currently stored as opaque `serde_json::Value` - this plan defines the structured types for it
- The `Message` struct provides all fields needed for condition matching (from_email, subject, labels, headers as JSON)
- Rules are already indexed and ordered by priority in the database
- The existing repository's `list_enabled_by_scope()` method provides the foundation for rule loading, but the executor needs to aggregate rules across multiple scopes (global → account → domain → sender)

### Findings

#### Existing Rules Module Structure

**File: `server/crates/ashford-core/src/rules/mod.rs`**
```rust
pub mod repositories;
pub mod types;

pub use repositories::{
    DeterministicRuleError, DeterministicRuleRepository, DirectionError, DirectionsRepository,
    LlmRuleError, LlmRuleRepository, RulesChatMessageError, RulesChatMessageRepository,
    RulesChatSessionError, RulesChatSessionRepository,
};
pub use types::{
    DeterministicRule, Direction, LlmRule, NewDeterministicRule, NewDirection, NewLlmRule,
    NewRulesChatMessage, NewRulesChatSession, RuleScope, RulesChatMessage, RulesChatRole,
    RulesChatSession, SafeMode,
};
```

The module is well-organized with types and repositories separated. New files (`conditions.rs`, `deterministic.rs`) will be added alongside existing files.

#### DeterministicRule Type (Existing)

**File: `server/crates/ashford-core/src/rules/types.rs` (lines 89-122)**
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeterministicRule {
    pub id: String,
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,
    pub scope_ref: Option<String>,
    pub priority: i64,
    pub enabled: bool,
    pub conditions_json: Value,  // Currently opaque - this plan defines structure
    pub action_type: String,
    pub action_parameters_json: Value,
    pub safe_mode: SafeMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

The `conditions_json` field uses `serde_json::Value` - it can remain that way in the database layer while we add typed parsing in the conditions module.

#### RuleScope Enum (Existing)

**File: `server/crates/ashford-core/src/rules/types.rs` (lines 5-33)**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleScope {
    Global,
    Account,
    Sender,
    Domain,
}
```

The scopes are already defined. The executor needs to load rules in order: Global → Account → Domain → Sender (for maximum specificity).

#### SafeMode Enum (Existing)

**File: `server/crates/ashford-core/src/rules/types.rs` (lines 35-60)**
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafeMode {
    Default,
    AlwaysSafe,
    DangerousOverride,
}
```

The rule executor will pass through `safe_mode` as part of matched rule results for downstream safety enforcement.

#### Existing Repository Methods

**File: `server/crates/ashford-core/src/rules/repositories.rs`**

Key methods available:
- `list_enabled_by_scope(org_id, user_id, scope: RuleScope, scope_ref: Option<&str>)` - Returns rules for a specific scope, ordered by priority ASC
- `list_all(org_id, user_id)` - Returns all rules

The `list_enabled_by_scope` method handles the WHERE clause differently based on whether `scope_ref` is provided:
- With scope_ref: `WHERE scope = ? AND scope_ref = ?`
- Without scope_ref: `WHERE scope = ? AND scope_ref IS NULL`

This is important for Global scope (no scope_ref) vs Account/Domain/Sender (has scope_ref).

#### Message Model - Fields Available for Matching

**File: `server/crates/ashford-core/src/messages.rs` (lines 18-42)**
```rust
pub struct Message {
    pub id: String,
    pub account_id: String,
    pub thread_id: String,
    pub provider_message_id: String,
    pub from_email: Option<String>,      // For sender_email, sender_domain conditions
    pub from_name: Option<String>,
    pub to: Vec<Mailbox>,
    pub cc: Vec<Mailbox>,
    pub bcc: Vec<Mailbox>,
    pub subject: Option<String>,          // For subject_contains, subject_regex
    pub snippet: Option<String>,
    pub received_at: Option<DateTime<Utc>>,
    pub internal_date: Option<DateTime<Utc>>,
    pub labels: Vec<String>,              // For label_present condition
    pub headers: Value,                   // For header_match condition (JSON object)
    pub body_plain: Option<String>,
    pub body_html: Option<String>,
    pub raw_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub org_id: i64,
    pub user_id: i64,
}
```

All fields needed for condition matching are present:
- `from_email` → sender_email, sender_domain (extract domain via `@` split)
- `subject` → subject_contains, subject_regex
- `labels` → label_present (Vec<String>, can use `.contains()`)
- `headers` → header_match (serde_json::Value, typically an object)

**Note:** There's no built-in domain extraction utility - the evaluator will need to implement `fn extract_domain(email: &str) -> Option<&str>`.

#### Database Schema for deterministic_rules

**File: `server/migrations/001_initial.sql` (lines 185-205)**
```sql
CREATE TABLE deterministic_rules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  scope TEXT NOT NULL CHECK (scope IN ('global','account','sender','domain')),
  scope_ref TEXT,
  priority INTEGER NOT NULL DEFAULT 100,
  enabled INTEGER NOT NULL DEFAULT 1,
  conditions_json TEXT NOT NULL,
  action_type TEXT NOT NULL,
  action_parameters_json TEXT NOT NULL,
  safe_mode TEXT NOT NULL CHECK (safe_mode IN ('default','always_safe','dangerous_override')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX deterministic_rules_scope_idx ON deterministic_rules(scope, scope_ref);
CREATE INDEX deterministic_rules_priority_idx ON deterministic_rules(enabled, priority);
```

The `deterministic_rules_priority_idx` index supports efficient ordering by priority for enabled rules.

#### Testing Patterns

**Inline tests with setup helpers** (from `server/crates/ashford-core/src/messages.rs`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{DEFAULT_ORG_ID, DEFAULT_USER_ID};
    use tempfile::TempDir;

    async fn setup_repo() -> (MessageRepository, Database, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let db_name = format!("db_{}.sqlite", uuid::Uuid::new_v4());
        let db_path = dir.path().join(db_name);
        let db = Database::new(&db_path).await.expect("create db");
        run_migrations(&db).await.expect("migrations");
        (MessageRepository::new(db.clone()), db, dir)
    }

    #[tokio::test]
    async fn test_name() {
        let (repo, db, _dir) = setup_repo().await;
        // test implementation
    }
}
```

For condition evaluation tests, no database is needed - these are pure functions that can be tested with constructed `Message` structs and condition trees.

#### Rules Engine Evaluation Flow (from docs/rules_engine.md)

The evaluation order:
1. Load all enabled deterministic rules relevant to account/domain/sender
2. Sort by priority (ascending - lower numbers = higher priority)
3. Evaluate conditions
4. If match found:
   - Produce deterministic action(s)
   - Apply safety gating based on safe_mode
   - Skip LLM evaluation entirely (unless all-matches mode and no terminal match)

**Evaluation mode:**
- First-match only (for this phase): Stop after first matching rule
- All-matches mode deferred to future work if needed

#### Proposed Condition Types

Based on the plan requirements and Message model:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Logical(LogicalCondition),
    Leaf(LeafCondition),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogicalCondition {
    pub op: LogicalOperator,
    pub children: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogicalOperator {
    And,
    Or,
    Not,  // expects exactly 1 child
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LeafCondition {
    SenderEmail { value: String },      // exact or wildcard (*@domain.com)
    SenderDomain { value: String },     // domain match
    SubjectContains { value: String },  // case-insensitive substring
    SubjectRegex { value: String },     // regex pattern
    HeaderMatch { header: String, pattern: String },  // header name (case-insensitive) + regex pattern
    LabelPresent { value: String },     // Gmail label ID
}
```

#### Regex Caching Consideration

For `subject_regex` conditions, consider caching compiled regex patterns. Options:
1. **Per-evaluation caching**: Use a HashMap in the evaluator context
2. **Rule-level caching**: Store compiled regex alongside the rule (would require changing rule storage)
3. **Global LRU cache**: Thread-safe cache shared across evaluations

Given the expected low cardinality of unique regex patterns, a simple per-evaluation `HashMap<String, Regex>` should suffice initially.

#### Error Handling Pattern

Following existing patterns (from `server/crates/ashford-core/src/rules/repositories.rs`):

```rust
#[derive(Debug, Error)]
pub enum ConditionError {
    #[error("invalid condition json: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("invalid regex pattern '{pattern}': {source}")]
    InvalidRegex { pattern: String, source: regex::Error },
    #[error("NOT condition requires exactly 1 child, got {0}")]
    InvalidNotChildCount(usize),
    #[error("empty condition tree")]
    EmptyTree,
}
```

### Risks & Constraints

1. **Regex Performance**: Malicious or complex regex patterns could cause performance issues. Consider:
   - Timeout limits for regex matching
   - Pattern complexity limits (character count, nesting depth)
   - Pre-validation of patterns during rule creation (not in this phase, but noted)

2. **Case Sensitivity**: Need to decide on case sensitivity for:
   - `sender_email` - typically case-insensitive for the local part, case-preserving for domain
   - `subject_contains` - likely case-insensitive
   - `header_match` - header names are case-insensitive per RFC 2822
   - `label_present` - Gmail label IDs are case-sensitive

3. **Wildcard Syntax**: The `*@domain.com` wildcard syntax needs careful implementation:
   - Only support prefix wildcards (e.g., `*@domain.com`)
   - Don't allow arbitrary glob patterns initially
   - Consider if `*` anywhere else should be an error

4. **Multi-scope Rule Loading**: The executor needs to efficiently load rules across all applicable scopes. Current approach:
   - Make 4 separate database calls (global, account, domain, sender)
   - Merge and sort by priority
   - Alternative: Add a new repository method that loads all applicable scopes in one query

5. **Headers JSON Structure**: The `headers` field is stored as `serde_json::Value`. Need to handle:
   - Object access: `headers["Header-Name"]`
   - Case-insensitive header name matching
   - Multiple values for same header (rare but possible)

6. **Scope Reference Extraction**: For domain-scoped rules, need to extract domain from `from_email`. For sender-scoped rules, use full `from_email`. Ensure graceful handling of missing/malformed email addresses.

7. **Empty Conditions**: Decide behavior for rules with empty condition trees:
   - Option A: Always match (dangerous - could apply to all emails)
   - Option B: Error during evaluation
   - Option C: Never match
   Recommend Option B - require at least one condition.

8. **Test Coverage Priority**: Focus testing on:
   - Each leaf condition type in isolation
   - Logical operators (AND, OR, NOT)
   - Nested conditions (depth 3+)
   - Edge cases (empty strings, missing fields, malformed data)
   - Priority ordering (lower priority number = evaluated first)
   - First-match behavior (stops at first matching rule)

#### Executor Return Type

```rust
pub struct RuleMatch {
    pub rule: DeterministicRule,      // The full rule that matched
    pub action_type: String,          // Convenience copy
    pub action_parameters: Value,     // Convenience copy
    pub safe_mode: SafeMode,          // For downstream safety enforcement
}
```

The executor returns `Result<Option<RuleMatch>, EvaluatorError>`:
- `Ok(Some(match))` - A rule matched
- `Ok(None)` - No rules matched (fall through to LLM)
- `Err(e)` - Evaluation failed (invalid regex, malformed condition, etc.)

<!-- rmplan-generated-start -->
## Expected Behavior/Outcome

This plan implements the deterministic rule evaluation engine - the "fast path" for email classification that runs before any LLM involvement. Upon completion:

- **Condition Parsing**: JSON condition trees can be parsed into strongly-typed Rust structures supporting AND, OR, NOT operators and 6 leaf condition types (sender_email, sender_domain, subject_contains, subject_regex, header_match, label_present)
- **Condition Evaluation**: Given a `Message` and a condition tree, the evaluator returns whether the message matches
- **Rule Loading**: Rules are loaded from the database across all applicable scopes (global → account → domain → sender), filtered to enabled rules, and sorted by priority
- **Rule Execution**: The executor evaluates rules in priority order and returns the first matching rule with its action details, or None if no rules match

## Acceptance Criteria

- [ ] Condition types are defined with proper serde derives for JSON parsing
- [ ] All 6 leaf condition types evaluate correctly against Message fields
- [ ] Logical operators (AND, OR, NOT) combine conditions correctly
- [ ] Nested condition trees (3+ levels deep) evaluate correctly
- [ ] Regex patterns compile and match correctly for subject_regex and header_match
- [ ] Wildcard email patterns (`*@domain.com`) match correctly for sender_email
- [ ] Rule loader aggregates rules from all applicable scopes
- [ ] Rules are evaluated in priority order (ascending)
- [ ] First matching rule stops evaluation and is returned
- [ ] Invalid conditions (bad regex, malformed JSON) return appropriate errors
- [ ] All new code paths are covered by tests
- [ ] Code compiles without warnings with `cargo build`
- [ ] All tests pass with `cargo test`

## Dependencies & Constraints

- **Dependencies**: Requires Phase 4.1 (Plan 13) completed - DeterministicRule types and repository
- **Technical Constraints**: Must use existing patterns (thiserror for errors, serde for JSON, regex crate for patterns)
- **Performance Constraint**: Regex compilation should be cached within a single evaluation run

## Implementation Notes

### Recommended Approach
1. Start with `conditions.rs` - define types and implement evaluator as pure functions
2. Add comprehensive tests for conditions before moving to executor
3. Implement `deterministic.rs` - rule loader and executor that uses the condition evaluator
4. Add integration tests that use real database with seeded rules

### Potential Gotchas
- **Serde untagged enum**: The `Condition` enum uses `#[serde(untagged)]` which tries variants in order - put `Logical` first since it has the distinctive `op` field
- **Header name case**: Header names must be matched case-insensitively per RFC 2822
- **Domain extraction**: Use `rfind('@')` not `find('@')` to handle edge cases like `"user@company"@example.com`
- **Empty labels vector**: `label_present` should return false if message has no labels, not error
- **Regex caching**: Use a `HashMap<String, Regex>` in the evaluator context to avoid recompiling same pattern
<!-- rmplan-generated-end -->

Implemented deterministic rule engine pieces (Tasks 1-9: add regex dependency, define condition types, implement evaluator, leaf/logical tests, create deterministic.rs with RuleMatch/RuleLoader, add RuleExecutor, integration tests, mod.rs export/build/test/clippy). Added regex dependency to ashford-core Cargo.toml and updated lockfile. New rules/conditions.rs defines LogicalOperator (with uppercase aliases), LogicalCondition/LeafCondition/Condition enums with serde, ConditionError, parse_condition (null -> EmptyTree), EvaluationContext with per-evaluation regex cache, and evaluate logic: sender email exact or *@domain wildcard, domain extraction via rfind, subject substring case-insensitive, regex-based subject/header matching (header names case-insensitive, arrays supported), label presence, logical short-circuit semantics (AND empty true, OR empty false, NOT enforces single child). New rules/deterministic.rs introduces RuleMatch, RuleLoader aggregating enabled rules across global/account/domain/sender scopes (domain normalized lowercase) sorted by priority/created_at/id, and RuleExecutor performing first-match evaluation with shared regex cache, returning matched rule/action/safe_mode or None while surfacing Condition errors. Comprehensive unit tests cover leaf conditions, logical composition, parsing, and integration tests validate scope aggregation, priority ordering, first-match short-circuiting (including invalid regex in later rule), disabled rule filtering, scope-specific matches, and error propagation. Updated rules/mod.rs to expose new modules. Ran cargo fmt, cargo build -p ashford-core, cargo test -p ashford-core. cargo clippy -p ashford-core still reports pre-existing warnings in other modules; no new warnings from the added code.

Fixed review item: domain-scope rule lookup case-insensitive. Normalized domain scope_ref values on write (create/update) and during list_enabled_by_scope queries in server/crates/ashford-core/src/rules/repositories.rs by lowercasing domain scope references and comparing with LOWER(scope_ref) to support legacy mixed-case records. Added normalize_scope_ref helper and used std::borrow::Cow for borrowed/owned handling. Updated integration tests in server/crates/ashford-core/src/rules/deterministic.rs with libsql::params import and new executor_domain_scope_matches_with_uppercase_scope_ref_in_db test that flips stored scope_ref to 'Amazon.COM' via direct SQL and confirms RuleExecutor still matches. Task: Fix domain rule scope_ref case-sensitivity per review. Design rationale: ensure consistent normalization on writes while keeping query case-insensitive to honor existing data; no broader refactors. Integration points: RuleLoader -> DeterministicRuleRepository list_enabled_by_scope; NewDeterministicRule create/update paths. Tests: cargo test -p ashford-core --tests after cargo fmt.

Implemented review fixes focusing on deterministic rule evaluation safety and sender scoping. Tasks: fix sender-scope case sensitivity and block empty logical trees. Updated server/crates/ashford-core/src/rules/conditions.rs with recursive validate_condition invoked by parse_condition and guard clauses in evaluate so AND/OR nodes with zero children return ConditionError::EmptyTree; NOT still enforces a single child via InvalidNotChildCount. Added new parse/evaluate tests for empty logical nodes and nested empties, and adjusted existing AND/OR tests to expect errors. Updated server/crates/ashford-core/src/rules/repositories.rs to normalize scope_ref for both Domain and Sender when creating/updating rules and to query sender/domain scopes case-insensitively using LOWER(scope_ref) with lowercase inputs (via Cow to avoid copies). Added deterministic executor tests for mixed-case sender emails and legacy uppercase sender scope_ref rows to verify case-insensitive loading (server/crates/ashford-core/src/rules/deterministic.rs). Design rationale: prevent empty condition trees from accidentally matching all messages and ensure sender-scoped rules run regardless of email casing or legacy data. Integration points: RuleLoader/RuleExecutor rely on DeterministicRuleRepository::list_enabled_by_scope, which now handles sender/domain normalization; parse_condition used throughout now rejects empty logical structures early. No deviations from plan beyond added validation depth. Tests: cargo fmt; cargo test -p ashford-core (all tests pass).

Handled reviewer issue: header_match now supports Gmail-style header arrays persisted by ingest. In server/crates/ashford-core/src/rules/conditions.rs I taught the HeaderMatch evaluator to scan both object maps and arrays of {name,value} entries, reusing existing header_value_matches so string/array values still work. Added unit test header_match_gmail_array_shape to cover the real Gmail payload layout and ensure case-insensitive matching succeeds while missing headers stay false. Ran cargo test -p ashford-core to verify the full ashford-core suite (208 tests + bin/integration) passes. Tasks: fix header_match for Gmail header array shape as flagged by review.
