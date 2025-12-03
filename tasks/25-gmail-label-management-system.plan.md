---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: Gmail Label Management System
goal: Implement local label table with Gmail sync, descriptions for LLM context,
  and ID-based action parameters with name translation for classify job
id: 25
uuid: 0402f4e3-9063-4655-b42d-cef6910a6827
generatedBy: agent
status: done
priority: medium
container: false
temp: false
dependencies: []
parent: 5
references:
  "5": 66785b19-e85d-4135-bbca-9d061a0394c7
issue: []
pullRequest: []
docs: []
planGeneratedAt: 2025-12-03T07:43:31.619Z
promptsGeneratedAt: 2025-12-03T07:43:31.619Z
createdAt: 2025-12-03T02:38:59.861Z
updatedAt: 2025-12-03T08:40:18.591Z
progressNotes:
  - timestamp: 2025-12-03T07:51:04.193Z
    text: "Completed foundation tasks: Created migration 006_add_labels_table.sql
      with labels table and disabled_reason column for deterministic_rules.
      Added Gmail Label API types (Label, LabelColor, ListLabelsResponse). Added
      list_labels() method to GmailClient with test. Created LabelRepository
      with upsert, get_by_account, get_by_provider_id, get_by_name,
      get_available_for_classifier, delete_not_in_provider_ids,
      find_deleted_label_ids methods. All 354 tests pass. Committed as
      05ece7a3."
    source: "implementer: Tasks 1-4"
  - timestamp: 2025-12-03T07:55:34.821Z
    text: "Added 11 additional edge case tests: labels.rs now has 23 tests covering
      empty results, special characters in names, multi-account scenarios, and
      scoping. GmailClient now has 4 list_labels tests covering full response,
      empty response, minimal labels, and partial color. Fixed one clippy
      warning (useless format). All 365 tests pass."
    source: "tester: Tasks 1-4"
  - timestamp: 2025-12-03T08:09:10.194Z
    text: Completed Task 5 (disabled_reason field) and Task 6 (labels.sync.gmail
      job). Added disabled_reason field to
      DeterministicRule/NewDeterministicRule structs, updated repository SQL
      queries, added disable_rule_with_reason and find_rules_referencing_label
      methods. Created labels_sync_gmail.rs job handler that syncs Gmail labels,
      detects deleted labels, disables dependent rules with explanation, and
      removes deleted labels. All 383 tests pass.
    source: "implementer: Tasks 5-6"
  - timestamp: 2025-12-03T08:14:07.683Z
    text: "Added 8 new edge case tests: 4 for Task 5 (disabled_reason field:
      clearing via update, label ID matching, partial match documentation,
      nested condition search) and 4 for Task 6 (labels sync: multiple rules
      disabled for same deleted label, idempotency verification, dual reference
      rule handling, user field preservation on sync). All 391 tests pass."
    source: "tester: Tasks 5-6"
  - timestamp: 2025-12-03T08:22:13.534Z
    text: Completed Tasks 5 and 6. Task 5 added disabled_reason field to
      DeterministicRule struct with disable_rule_with_reason() and
      find_rules_referencing_label() repository methods. Task 6 implemented
      labels.sync.gmail job that syncs labels from Gmail API, detects deleted
      labels, and disables dependent rules. Fixed reviewer-identified issue with
      partial match false positives by using quoted JSON pattern in LIKE search.
      All 409 tests pass.
    source: "orchestrator: Tasks 5-6"
  - timestamp: 2025-12-03T08:29:53.206Z
    text: "Completed Task 7 (LLM prompt enhancement) and Task 8 (label name-to-ID
      translation). Changes: 1) Added available_labels parameter to
      PromptBuilder::build() with new build_available_labels_section() function
      that formats labels as '- {name}' or '- {name}: {description}'. 2) Updated
      run_llm_classification() to load available labels via
      LabelRepository::get_available_for_classifier() and pass to prompt
      builder. 3) Added translate_label_name_in_decision() and
      translate_label_name_to_id() functions for case-insensitive label name to
      provider_label_id translation in apply_label actions. 4) Added 6 new tests
      for prompt labels section and 10 new tests for translation logic. All 407
      tests pass."
    source: "implementer: Tasks 7-8"
  - timestamp: 2025-12-03T08:33:46.055Z
    text: "Added 21 additional edge case tests for Tasks 7 and 8. For Task 7
      (prompt.rs): 7 new tests covering special characters in label names,
      unicode labels, long descriptions, system labels (where ID matches name),
      single label case, and embedded newlines in descriptions. For Task 8
      (classify.rs): 14 new tests covering special character translation,
      unicode translation, system labels, duplicate case-insensitive names
      (first match wins), empty names, null/array/object label values,
      whitespace-only labels, partial match prevention, non-apply_label action
      behavior, and very long label names. Also fixed integration test in
      llm_prompt_decision_flow.rs to pass the new available_labels parameter.
      All 426 tests pass."
    source: "tester: Tasks 7-8"
  - timestamp: 2025-12-03T08:36:09.799Z
    text: Reviewed Tasks 7 and 8 implementation. All 426 tests pass. Found potential
      security issue with prompt injection via label names/descriptions that
      could allow attackers to inject instructions into LLM prompts. Also
      identified missing integration test for label translation in full classify
      flow.
    source: "reviewer: Tasks 7-8"
  - timestamp: 2025-12-03T08:37:18.498Z
    text: Completed Tasks 7 and 8. Task 7 enhanced the LLM prompt builder to include
      an AVAILABLE LABELS section with label names and descriptions, loaded via
      LabelRepository::get_available_for_classifier(). Task 8 added
      translate_label_name_to_id() and translate_label_name_in_decision()
      functions to convert label names in LLM responses back to
      provider_label_ids before persisting. Both tasks integrated into
      run_llm_classification() in classify.rs. All 426 tests pass including 21
      new edge case tests for special characters, unicode, system labels, and
      various JSON types.
    source: "orchestrator: Tasks 7-8"
tasks:
  - title: Create labels table migration and add disabled_reason to
      deterministic_rules
    done: true
    description: >-
      Create migration `006_add_labels_table.sql` with:

      - Labels table: id (TEXT PK), account_id (TEXT FK), provider_label_id
      (TEXT), name (TEXT), label_type (TEXT), description (TEXT nullable),
      available_to_classifier (INTEGER default 1), message_list_visibility
      (TEXT), label_list_visibility (TEXT), background_color (TEXT), text_color
      (TEXT), created_at, updated_at, org_id, user_id

      - Unique index on (account_id, provider_label_id)

      - Index on (org_id, user_id)

      - Add `disabled_reason TEXT` column to deterministic_rules table

      - Register migration in migrations.rs MIGRATIONS array


      Files: server/migrations/006_add_labels_table.sql (new),
      server/crates/ashford-core/src/migrations.rs
  - title: Add Gmail Label API types
    done: true
    description: >-
      Add types to gmail/types.rs for the labels.list API response:

      - `Label` struct with fields: id, name, type (rename to label_type),
      message_list_visibility, label_list_visibility, color (optional nested
      struct with background_color, text_color)

      - `LabelColor` struct for the nested color object

      - `ListLabelsResponse` struct with labels: Vec<Label>

      - Use appropriate serde renames for camelCase fields


      Files: server/crates/ashford-core/src/gmail/types.rs
  - title: Add list_labels method to GmailClient
    done: true
    description: |-
      Add `list_labels()` method to GmailClient in gmail/client.rs:
      - Call GET /users/me/labels endpoint
      - Return ListLabelsResponse
      - Follow existing patterns for authentication and error handling
      - Add unit test with mock response

      Files: server/crates/ashford-core/src/gmail/client.rs
  - title: Create Label model and LabelRepository
    done: true
    description: >-
      Create new labels.rs module with:

      - `Label` struct matching database schema (id, account_id,
      provider_label_id, name, label_type, description, available_to_classifier,
      message_list_visibility, label_list_visibility, background_color,
      text_color, timestamps, org_id, user_id)

      - `NewLabel` struct for creating labels

      - `LabelRepository` with methods:
        - `upsert()` - insert or update by (account_id, provider_label_id)
        - `get_by_account()` - list all labels for an account
        - `get_by_provider_id()` - lookup by account_id + provider_label_id
        - `get_available_for_classifier()` - labels where available_to_classifier=true
        - `delete_by_provider_ids()` - bulk delete labels not in provided list (for sync)
        - `find_deleted_label_ids()` - compare local vs API labels to find deletions
      - Export from lib.rs

      - Add tests for repository methods


      Files: server/crates/ashford-core/src/labels.rs (new),
      server/crates/ashford-core/src/lib.rs
  - title: Update DeterministicRule to include disabled_reason field
    done: true
    description: >-
      Update rules system to support disabled_reason:

      - Add `disabled_reason: Option<String>` field to DeterministicRule struct
      in types.rs

      - Update NewDeterministicRule and UpdateDeterministicRule structs

      - Update repository SQL queries to include disabled_reason column

      - Update row_to_deterministic_rule function to parse the new column

      - Add method `disable_rule_with_reason(rule_id, reason)` to repository

      - Add method `find_rules_referencing_label(account_id, label_id)` to find
      rules that use a label in conditions or action parameters

      - Add tests


      Files: server/crates/ashford-core/src/rules/types.rs,
      server/crates/ashford-core/src/rules/repositories.rs
  - title: Implement labels.sync.gmail job
    done: true
    description: >-
      Create new job handler for label synchronization:

      - Create labels_sync_gmail.rs with `handle_labels_sync_gmail()` function

      - Payload: `LabelsSyncPayload { account_id: String }`

      - Job flow:
        1. Load account and create GmailClient
        2. Call list_labels() API
        3. Upsert all labels to database
        4. Detect deleted labels (labels in DB but not in API response)
        5. For deleted labels, find and disable dependent rules with reason
        6. Remove deleted labels from database
      - Register job type in jobs/mod.rs dispatcher

      - Add idempotency key format: `labels.sync.gmail:{account_id}`

      - Add tests with mocked API responses


      Files: server/crates/ashford-core/src/jobs/labels_sync_gmail.rs (new),
      server/crates/ashford-core/src/jobs/mod.rs
  - title: Enhance LLM prompt with available labels
    done: true
    description: >-
      Update prompt builder to include available labels for the LLM:

      - Modify `PromptBuilder::build()` signature to accept labels parameter
      (Vec<Label> or similar)

      - Add new section 'AVAILABLE LABELS' after MESSAGE CONTEXT

      - Format each label as: `- {name}` or `- {name}: {description}` if
      description exists

      - Only include labels where available_to_classifier=true

      - Update classify job to load labels and pass to prompt builder

      - Add tests for new prompt section


      Files: server/crates/ashford-core/src/llm/prompt.rs,
      server/crates/ashford-core/src/jobs/classify.rs
  - title: Add label name-to-ID translation in classify job
    done: true
    description: >-
      Add translation layer for LLM responses that use label names:

      - Create helper function `translate_label_name_to_id(account_id,
      label_name, labels) -> Option<String>`

      - In classify job, after parsing LLM decision:
        - If action is apply_label, extract label name from parameters
        - Look up label ID by name (case-insensitive match)
        - Replace label name with label ID in action parameters before persisting
      - Handle case where label name not found (log warning, skip translation)

      - For deterministic rules with apply_label action, parameters should
      already store IDs

      - Add tests for translation logic


      Files: server/crates/ashford-core/src/jobs/classify.rs,
      server/crates/ashford-core/src/labels.rs (add lookup helper)
changedFiles:
  - docs/data_model.md
  - docs/gmail_integration.md
  - docs/job_queue.md
  - docs/rules_engine.md
  - server/crates/ashford-core/src/gmail/client.rs
  - server/crates/ashford-core/src/gmail/types.rs
  - server/crates/ashford-core/src/jobs/action_gmail.rs
  - server/crates/ashford-core/src/jobs/approval_notify.rs
  - server/crates/ashford-core/src/jobs/classify.rs
  - server/crates/ashford-core/src/jobs/labels_sync_gmail.rs
  - server/crates/ashford-core/src/jobs/mod.rs
  - server/crates/ashford-core/src/labels.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/llm/prompt.rs
  - server/crates/ashford-core/src/migrations.rs
  - server/crates/ashford-core/src/rules/deterministic.rs
  - server/crates/ashford-core/src/rules/repositories.rs
  - server/crates/ashford-core/src/rules/types.rs
  - server/crates/ashford-core/tests/llm_prompt_decision_flow.rs
  - server/migrations/006_add_labels_table.sql
tags:
  - gmail
  - labels
  - rust
---

Implement a proper label management system for Gmail integration:

## Scope
- Create `labels` table mapping account_id + label_id to name, with optional description and `available_to_classifier` boolean
- Add periodic sync from Gmail API to local labels table
- Update deterministic_rules to store label IDs instead of names
- Translate label IDs to names when building classify job prompts (so LLM has semantic context)
- Translate label names back to IDs when processing classify job results
- Handle deleted labels by marking affected rules/actions as disabled

## Key Behaviors
- Labels table: account_id, provider_label_id, name, label_type (system/user), description (optional, empty by default), available_to_classifier (default true), plus Gmail metadata (color, visibility settings)
- Sync pulls labels via Gmail labels.list API
- Classify job receives label names + descriptions for better LLM decisions
- Action parameters store label IDs for stability across renames
- Deleted label detection marks dependent rules as disabled

## Out of Scope
- Label creation via the application (users create labels in Gmail)
- UI for managing label descriptions (can be added later)

## Research

### Summary
This plan implements a proper label management system that bridges Gmail's label IDs (stable identifiers) with human-readable names for LLM context. The key insight is that Gmail labels can be renamed, which would break rules that store label names directly. By storing label IDs in rules/actions and translating to names only when needed for LLM prompts, the system gains stability while preserving semantic context for classification decisions.

Critical discoveries:
1. **Current label storage is by name, not ID** - Both `message.labels` in the database and `LabelPresent` conditions use label names/IDs interchangeably (Gmail's label IDs like "INBOX", "STARRED" are human-readable, but custom labels use opaque IDs like "Label_1234")
2. **No Gmail API types exist for labels.list** - The `gmail/types.rs` file needs new types for `Label` and `ListLabelsResponse`
3. **The prompt system needs enhancement** - Currently only shows current message labels, not available labels for the LLM to choose from
4. **Action parameters are untyped JSON** - No validation exists for `action_parameters_json` structure per action type

### Findings

#### Database Schema and Models

**File Locations:**
- Migrations: `server/migrations/`
- Database module: `server/crates/ashford-core/src/db.rs`
- Migration runner: `server/crates/ashford-core/src/migrations.rs`

**Current Label Storage:**
Messages store labels in `labels_json TEXT NOT NULL DEFAULT '[]'` as an array of label IDs from Gmail API. In the Message struct (`server/crates/ashford-core/src/messages.rs`), this is parsed as `labels: Vec<String>`.

**Standard Table Pattern for New Labels Table:**
```sql
CREATE TABLE table_name (
  id TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  -- other columns
  org_id INTEGER NOT NULL DEFAULT 1,
  user_id INTEGER NOT NULL DEFAULT 1,
  FOREIGN KEY (account_id) REFERENCES accounts(id)
);
CREATE INDEX table_org_user_idx ON table_name(org_id, user_id);
```

**Migration System:**
- SQL files embedded into binary using `include_str!`
- Migrations stored in MIGRATIONS array with version numbers
- `run_migrations()` function applies unapplied migrations atomically
- Current migrations: 001 (initial), 002 (job fields), 003 (unique indices), 004 (org_id/user_id), 005 (llm_calls)
- Next migration will be `006_add_labels_table.sql`

**Account State Structure:**
```rust
pub struct AccountState {
    pub history_id: Option<String>,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub sync_status: SyncStatus, // Active, NeedsBackfill, Paused
}
```

#### Gmail API Integration

**File Locations:**
- OAuth management: `server/crates/ashford-core/src/gmail/oauth.rs`
- API client: `server/crates/ashford-core/src/gmail/client.rs`
- Response types: `server/crates/ashford-core/src/gmail/types.rs`
- Message parsing: `server/crates/ashford-core/src/gmail/parser.rs`

**GmailClient Structure:**
- Generic over `TokenStore` implementation
- Auto-refreshes tokens before requests if needed
- Handles 401 Unauthorized by refreshing and retrying once
- Existing methods: `get_message()`, `get_thread()`, `list_history()`, `list_messages()`, `get_profile()`
- Need to add: `list_labels()` method

**Gmail Labels API Reference:**
The Gmail API provides `users.labels.list` endpoint that returns:
```json
{
  "labels": [
    {
      "id": "INBOX",
      "name": "INBOX",
      "type": "system",
      "messageListVisibility": "show",
      "labelListVisibility": "labelShow"
    },
    {
      "id": "Label_123456789",
      "name": "My Custom Label",
      "type": "user",
      "messageListVisibility": "show",
      "labelListVisibility": "labelShow",
      "color": { "backgroundColor": "#ffffff", "textColor": "#000000" }
    }
  ]
}
```

**Proposed Labels Table Schema:**
```sql
CREATE TABLE labels (
  id TEXT PRIMARY KEY,                    -- internal UUID
  account_id TEXT NOT NULL,
  provider_label_id TEXT NOT NULL,        -- Gmail's label ID (e.g., "INBOX" or "Label_123")
  name TEXT NOT NULL,                     -- Display name from Gmail
  label_type TEXT NOT NULL,               -- "system" or "user"
  description TEXT,                       -- Optional, user-provided for LLM context
  available_to_classifier INTEGER NOT NULL DEFAULT 1,
  message_list_visibility TEXT,           -- "show", "hide", etc.
  label_list_visibility TEXT,             -- "labelShow", "labelHide", etc.
  background_color TEXT,                  -- Hex color from Gmail
  text_color TEXT,                        -- Hex color from Gmail
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  org_id INTEGER NOT NULL DEFAULT 1,
  user_id INTEGER NOT NULL DEFAULT 1,
  FOREIGN KEY (account_id) REFERENCES accounts(id)
);
CREATE UNIQUE INDEX labels_account_provider_uidx ON labels(account_id, provider_label_id);
CREATE INDEX labels_org_user_idx ON labels(org_id, user_id);
```

**Sync Job Patterns:**
- Jobs use idempotency keys to prevent duplicates
- Error handling maps Gmail API errors to `JobError::Fatal` or `JobError::Retryable`
- Pagination handled via `nextPageToken` in while loops
- Label sync will be a standalone periodic job (`labels.sync.gmail`) following similar patterns to `history_sync_gmail.rs`
- Job payload: `{ account_id }` - simple since labels.list doesn't require pagination for typical accounts

#### Deterministic Rules System

**File Locations:**
- Rule types: `server/crates/ashford-core/src/rules/types.rs`
- Condition evaluation: `server/crates/ashford-core/src/rules/conditions.rs`
- Rule execution: `server/crates/ashford-core/src/rules/deterministic.rs`
- Database layer: `server/crates/ashford-core/src/rules/repositories.rs`

**Current Label Handling in Rules:**
The `LabelPresent` condition matches labels by exact string comparison:
```rust
LeafCondition::LabelPresent { value } => {
    Ok(message.labels.iter().any(|label| label == value))
}
```

Since Gmail provides label IDs in messages (`labelIds` field), and the condition checks against `message.labels`, this already works with label IDs. The issue is that:
1. Rules store label names in `action_parameters_json` for `apply_label` actions
2. The LLM receives label names in prompts and returns label names in responses
3. No translation layer exists between IDs and names

**DeterministicRule Structure:**
```rust
pub struct DeterministicRule {
    pub id: String,
    pub action_type: String,           // e.g., "apply_label"
    pub action_parameters_json: Value, // e.g., {"label": "Work"}
    pub safe_mode: SafeMode,
    pub enabled: bool,
    pub disabled_reason: Option<String>, // NEW: e.g., "Label 'Work' was deleted"
    // ... other fields
}
```

**RuleMatch Structure:**
```rust
pub struct RuleMatch {
    pub rule: DeterministicRule,
    pub action_type: String,
    pub action_parameters: Value, // Cloned from rule.action_parameters_json
    pub safe_mode: SafeMode,
}
```

#### Classify Job System

**File Locations:**
- Main job: `server/crates/ashford-core/src/jobs/classify.rs`
- Prompt building: `server/crates/ashford-core/src/llm/prompt.rs`
- Decision parsing: `server/crates/ashford-core/src/llm/decision.rs`
- Action execution: `server/crates/ashford-core/src/jobs/action_gmail.rs`

**How Labels Appear in LLM Prompts:**
Currently, labels are included as a simple JSON array of whatever is stored:
```rust
lines.push(format!(
    "Labels: {}",
    serde_json::to_string(&message.labels).unwrap_or_else(|_| "[]".to_string())
));
```

This outputs something like: `Labels: ["INBOX","Label_123","IMPORTANT"]`

**Key Gap:** The LLM has no information about:
1. What labels are available in the account
2. Human-readable names for labels with opaque IDs
3. Descriptions explaining label purposes

**ApplyLabel Action Flow:**
1. LLM calls `record_decision` tool with `action: "apply_label"` and `parameters: {"label": "Some Label"}`
2. Decision is persisted with `action_type` and `parameters_json`
3. Action is created with same parameters
4. `action_gmail.rs` would execute the Gmail API call (currently a stub)

**Translation Points Needed:**
1. **Prompt Building (ID → Name):** When building LLM prompt, translate label IDs to names+descriptions
2. **Decision Parsing (Name → ID):** When processing LLM response, translate label names back to IDs
3. **Action Execution:** Use label IDs when calling Gmail API

#### Key Files That Need Modification

| File | Changes Needed |
|------|----------------|
| `server/migrations/006_add_labels_table.sql` | New file: Create labels table |
| `server/crates/ashford-core/src/migrations.rs` | Add new migration |
| `server/crates/ashford-core/src/gmail/types.rs` | Add `Label` and `ListLabelsResponse` types |
| `server/crates/ashford-core/src/gmail/client.rs` | Add `list_labels()` method |
| `server/crates/ashford-core/src/labels.rs` | New file: Label model and LabelRepository |
| `server/crates/ashford-core/src/lib.rs` | Export labels module |
| `server/crates/ashford-core/src/jobs/mod.rs` | Register label sync job handler |
| `server/crates/ashford-core/src/jobs/labels_sync_gmail.rs` | New file: Label sync job |
| `server/crates/ashford-core/src/llm/prompt.rs` | Add available labels section with names+descriptions |
| `server/crates/ashford-core/src/jobs/classify.rs` | Add label name→ID translation for LLM responses |
| `server/crates/ashford-core/src/rules/repositories.rs` | Add methods to disable rules referencing deleted labels |
| `server/migrations/006_add_labels_table.sql` | Also add `disabled_reason` column to deterministic_rules table |

### Risks & Constraints

1. **Gmail System Labels**: System labels (INBOX, SENT, SPAM, etc.) have predictable IDs that match their names. Custom user labels have opaque IDs like `Label_123456789`. The system must handle both cases.

2. **Label Deletion Cascade**: When a label is deleted in Gmail:
   - Rules with `LabelPresent` conditions referencing it should be soft-disabled with `disabled_reason`
   - Rules with `apply_label` actions referencing it should be soft-disabled with `disabled_reason`
   - Pending actions referencing it should be cancelled or flagged
   - This requires careful transaction handling
   - Adding `disabled_reason` field to deterministic_rules table allows clear user feedback

3. **Sync Timing**: Label sync runs as a standalone periodic job (`labels.sync.gmail`):
   - Triggered on account setup (initial sync)
   - Runs periodically on a schedule (e.g., hourly)
   - Can be triggered manually/on-demand
   - Independent from history sync and classify jobs for clear separation of concerns

4. **LLM Prompt Size**: Including all available labels with descriptions could significantly increase prompt size. Consider:
   - Only including labels where `available_to_classifier = true`
   - Truncating descriptions if too long
   - Grouping/nesting labels if Gmail supports it

5. **Case Sensitivity**: Gmail label names are case-insensitive for matching but preserve case for display. Label ID matching should be case-sensitive.

6. **Rename Detection**: Gmail API doesn't provide rename history. When syncing, if a label ID exists locally but has a different name, update the local name. This is the correct behavior since IDs are stable.

7. **Race Conditions**: If a user deletes a label in Gmail while a classify job is processing:
   - The job might return a label name that no longer exists
   - Action execution should fail gracefully and mark the action as failed
   - Consider re-syncing labels on action execution failures related to labels

## Expected Behavior/Outcome

After implementation:
1. **Label Sync:** Labels are automatically synced from Gmail and stored locally with account_id, provider_label_id, name, description, and available_to_classifier fields
2. **LLM Context:** The classify job prompt includes available labels with names and descriptions, enabling the LLM to make semantically meaningful label choices
3. **Stable References:** Rules and actions store label IDs (not names), making them resilient to label renames
4. **Graceful Degradation:** Deleted labels are detected during sync and dependent rules are automatically disabled

## Key Findings

**Product & User Story:**
As a user with Gmail labels, I want the email classification system to understand my label taxonomy so it can accurately apply labels based on semantic meaning rather than guessing label names.

**Design & UX Approach:**
- Labels sync automatically in the background
- No UI changes required initially (descriptions can be added via future UI)
- Rules and actions continue to work even if labels are renamed in Gmail
- Clear error states when labels are deleted

**Technical Plan & Risks:**
- Database schema extension with foreign key to accounts
- New Gmail API integration for labels.list endpoint
- Translation layer between label IDs and names at prompt building and decision processing boundaries
- Risk: Prompt size growth with many labels (mitigated by available_to_classifier filter)
- Risk: Race conditions on label deletion (mitigated by graceful failure handling)

**Pragmatic Effort Estimate:**
This is a medium-complexity feature touching multiple layers (database, Gmail API, rules engine, classify job). The work can be parallelized into:
1. Database + model layer (independent)
2. Gmail API extension (independent)
3. Label sync job (depends on 1, 2)
4. Prompt enhancement (depends on 1)
5. Decision translation (depends on 1)
6. Deleted label handling (depends on 1, 3)

## Acceptance Criteria

- [ ] Labels table exists with account_id, provider_label_id, name, label_type, description, available_to_classifier, color, and visibility columns
- [ ] Label sync job fetches labels from Gmail API and upserts to local table
- [ ] Deleted labels are detected and dependent rules are soft-disabled with `disabled_reason` field explaining why
- [ ] LLM prompts include available label names and descriptions
- [ ] LLM responses with label names are translated to label IDs before action storage
- [ ] Rules with apply_label actions store label IDs in action_parameters_json
- [ ] All new code paths are covered by tests

## Dependencies & Constraints

**Dependencies:**
- Existing accounts table and AccountRepository
- Existing GmailClient with OAuth token management
- Existing rules system and DeterministicRuleRepository
- Existing classify job infrastructure

**Technical Constraints:**
- Must maintain backwards compatibility with existing rules (migration should update existing rules if feasible, or document manual migration)
- Label sync should not block message processing
- Must handle Gmail API rate limits appropriately

## Implementation Notes

**Recommended Approach:**
1. Start with database schema and model layer
2. Add Gmail API types and client method
3. Implement label sync job
4. Enhance prompt builder to include available labels
5. Add name→ID translation in classify job
6. Implement deleted label detection and rule disabling

**Potential Gotchas:**
- Gmail system labels (INBOX, SENT, etc.) have IDs that match their names, but user labels have opaque IDs
- The `LabelPresent` condition already works with label IDs since that's what Gmail returns on messages
- The `apply_label` action needs the translation because LLMs work better with semantic names
- Descriptions start empty by default; users can populate them later via future UI
- Store all Gmail label metadata (type, color, visibility) for future UI use

## Tasks 1-4: Labels Foundation Layer (completed)

### Task 1: Database Migration (006_add_labels_table.sql)
Created migration at server/migrations/006_add_labels_table.sql with:
- **labels table** with all required columns: id (TEXT PK), account_id (TEXT FK to accounts), provider_label_id (TEXT), name (TEXT), label_type (TEXT), description (TEXT nullable), available_to_classifier (INTEGER default 1), message_list_visibility (TEXT), label_list_visibility (TEXT), background_color (TEXT), text_color (TEXT), created_at (TEXT), updated_at (TEXT), org_id (INTEGER default 1), user_id (INTEGER default 1)
- **Unique index** labels_account_provider_uidx on (account_id, provider_label_id) to prevent duplicate labels per account
- **Standard indexes**: labels_org_user_idx on (org_id, user_id) and labels_account_idx on account_id
- **Added disabled_reason TEXT column** to deterministic_rules table via ALTER TABLE
- Registered migration in migrations.rs MIGRATIONS array as entry 6

### Task 2: Gmail API Types (gmail/types.rs)
Added three new types for the labels.list API response:
- **LabelColor**: Nested struct with background_color and text_color (both Option<String> for defensive parsing)
- **Label**: Full label struct with id, name, label_type (serde renamed from 'type'), message_list_visibility, label_list_visibility, and optional color field. All fields use appropriate serde(rename) for camelCase mapping.
- **ListLabelsResponse**: Top-level response struct with labels: Vec<Label>

### Task 3: GmailClient.list_labels() Method (gmail/client.rs)
Implemented list_labels() method following existing patterns:
- Calls GET /gmail/v1/users/me/labels endpoint
- Uses request_with_retry() for automatic token refresh and retry on 401
- Returns ListLabelsResponse parsed from JSON
- Added 4 tests: basic labels response, empty response, minimal label (only id/name), and partial color object

### Task 4: LabelRepository (labels.rs)
Created comprehensive repository at server/crates/ashford-core/src/labels.rs:
- **Label struct**: Matches database schema exactly with all columns
- **NewLabel struct**: For creating/updating labels from Gmail API data
- **LabelError enum**: Database, Sql, DateTimeParse, NotFound variants following existing patterns
- **Repository methods**:
  - upsert(): INSERT OR UPDATE by (account_id, provider_label_id), importantly preserves user-editable fields (description, available_to_classifier) on updates
  - get_by_account(): Returns all labels for an account ordered by name
  - get_by_provider_id(): Lookup by account_id + provider_label_id
  - get_by_name(): Case-insensitive lookup using LOWER() for LLM translation needs
  - get_available_for_classifier(): Labels where available_to_classifier=true for LLM prompt building
  - delete_not_in_provider_ids(): Bulk delete labels not in provided list (for sync cleanup)
  - find_deleted_label_ids(): Compare local vs API labels to identify deletions
- Exported as pub mod labels from lib.rs with re-exports of Label, NewLabel, LabelError, LabelRepository
- **23 comprehensive tests** covering all methods, edge cases (empty results, special characters, unicode), multi-tenancy isolation, and cross-account scenarios

### Test Results
All 365 tests pass. Added 11 new tests beyond the initial implementation covering edge cases like empty responses, special characters in label names, partial color objects, and account scoping verification.

### Design Decisions
1. **Defensive optional fields in LabelColor**: While Gmail API docs suggest both colors are always present, we handle partial/missing colors gracefully
2. **Preserve user fields on upsert**: The upsert() preserves description and available_to_classifier from existing rows since these are user-editable
3. **Case-insensitive name lookup**: get_by_name() uses SQL LOWER() for case-insensitive matching, critical for LLM label name translation
4. **find_deleted_label_ids returns Label objects**: Easier for callers to access label name when building disabled_reason messages

## Tasks 5-6: Rules Disabled Reason and Label Sync Job

### Task 5: Update DeterministicRule to include disabled_reason field

**Files Modified:**
- `server/crates/ashford-core/src/rules/types.rs` - Added `disabled_reason: Option<String>` to `DeterministicRule` and `NewDeterministicRule` structs
- `server/crates/ashford-core/src/rules/repositories.rs` - Updated all SQL queries and added new methods
- `server/crates/ashford-core/src/rules/deterministic.rs` - Updated evaluation function signature for new field
- `server/crates/ashford-core/src/jobs/classify.rs` - Updated call sites

**Key Changes:**
1. Added `disabled_reason` field to `DeterministicRule` struct (stores why a rule was disabled)
2. Added `disabled_reason` to `NewDeterministicRule` struct
3. Updated `DETERMINISTIC_RULE_COLUMNS` constant and all SQL queries (create, update, get)
4. Adjusted all column indices in `row_to_deterministic_rule()` function
5. Added `disable_rule_with_reason(org_id, user_id, id, reason)` method - atomically disables a rule and sets the reason
6. Added `find_rules_referencing_label(org_id, user_id, label_provider_id)` method - searches conditions_json and action_parameters_json using quoted JSON string pattern

**Design Decision:** The `find_rules_referencing_label` method uses a quoted JSON pattern (`%"Label_1"%`) for LIKE search to prevent false positives where one label ID is a prefix of another (e.g., Label_1 vs Label_10). This is more precise than a simple substring search while avoiding full JSON parsing overhead.

### Task 6: Implement labels.sync.gmail job

**Files Created:**
- `server/crates/ashford-core/src/jobs/labels_sync_gmail.rs` - New job handler module

**Files Modified:**
- `server/crates/ashford-core/src/jobs/mod.rs` - Registered `JOB_TYPE_LABELS_SYNC_GMAIL` constant and added to dispatcher

**Job Flow:**
1. Parse `LabelsSyncPayload { account_id }` from job payload
2. Load account and refresh OAuth tokens if needed
3. Create GmailClient and call `list_labels()` API
4. Detect deleted labels by comparing local DB labels with API response
5. For each deleted label:
   - Retrieve label name for user-friendly disabled reason
   - Find all rules referencing the deleted label
   - Disable each rule with reason: "Label 'X' was deleted from Gmail"
6. Delete the removed labels from local database
7. Upsert all labels from API response (preserves user-editable fields like description and available_to_classifier)

**Test Coverage (22 tests total for both tasks):**
- Task 5: 14 tests covering disabled_reason field storage, disable_rule_with_reason method, find_rules_referencing_label with exact matching, no partial matches, nested conditions
- Task 6: 8 tests covering new label insertion, existing label updates, deleted label handling, rule disabling for deleted labels, empty responses, rate limit retry, invalid payload, nonexistent account

**All 392 unit tests + 17 integration tests pass.**

## Tasks 7-8: LLM Prompt Labels and Name-to-ID Translation

### Task 7: Enhance LLM prompt with available labels

**Files Modified:**
- `server/crates/ashford-core/src/llm/prompt.rs` - Added labels support to prompt builder
- `server/crates/ashford-core/src/jobs/classify.rs` - Load labels and pass to prompt builder
- `server/crates/ashford-core/tests/llm_prompt_decision_flow.rs` - Updated integration test

**Implementation Details:**
1. Modified `PromptBuilder::build()` signature to accept `available_labels: &[Label]` parameter
2. Added `build_available_labels_section()` function that formats labels as:
   - `- {name}` for labels without description
   - `- {name}: {description}` for labels with description
   - Empty descriptions (empty string or whitespace only) are treated as no description
3. The AVAILABLE LABELS section appears after MESSAGE CONTEXT and before TASK in the prompt
4. Updated `run_llm_classification()` in classify.rs to load available labels using `LabelRepository::get_available_for_classifier()` and pass to the prompt builder

### Task 8: Add label name-to-ID translation in classify job

**Files Modified:**
- `server/crates/ashford-core/src/jobs/classify.rs` - Translation functions and integration

**Implementation Details:**
1. Added `translate_label_name_to_id(label_name, labels)` helper function for case-insensitive label lookup by name
2. Added `translate_label_name_in_decision(decision, labels)` function that:
   - Checks if action_type is 'apply_label'
   - Extracts the label name from parameters['label']
   - Performs case-insensitive matching against available labels
   - Replaces the label name with `provider_label_id` in the parameters
   - Logs warning if label not found (does not fail the job)
3. Integrated translation into `run_llm_classification()` after parsing LLM decision

### Test Coverage
All 426 tests pass, including:
- 6 initial tests for prompt label section formatting
- 10 initial tests for label translation logic
- 21 additional edge case tests covering:
  - Special characters (slashes, ampersands, quotes, colons)
  - Unicode characters (French, German, Polish accented text)
  - System labels where provider_label_id matches name (INBOX, STARRED, SENT)
  - Long descriptions (500+ characters)
  - Various JSON types (null, array, object)
  - Empty/whitespace label names
  - Partial match prevention
  - Non-apply_label action types (unaffected by translation)

### Design Decisions
1. **Label filtering at load time:** Labels are filtered by `available_to_classifier=true` when loaded via `get_available_for_classifier()`, so the prompt builder receives only relevant labels
2. **Case-insensitive matching:** Translation uses `.eq_ignore_ascii_case()` for matching label names, accommodating LLMs that may return different casing
3. **Graceful handling of unknown labels:** If the LLM returns a label name that doesn't exist, a warning is logged but the job continues with the original label name intact
4. **First match wins:** If multiple labels match case-insensitively (unlikely in practice), the first one in the list is used

### Reviewer Notes
The implementation was reviewed and approved. One security consideration was noted: label names/descriptions are interpolated directly into the prompt without sanitization, which could theoretically enable prompt injection. This is acceptable in the current single-user context where labels can only be created by the authenticated user via Gmail.
