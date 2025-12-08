---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Rules Feature: List, Edit, and Reorder"
goal: Build the rules management pages with tabbed list view, create/edit forms,
  and priority reordering, including all required Rust API endpoints
id: 28
uuid: 22994868-219e-4c67-affd-53b36c2248f7
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies:
  - 30
parent: 7
references:
  "7": 5a952985-9ed4-4035-8fef-479f3f7e2010
  "30": 64c00252-4c84-4b02-8fc2-68559edf27a9
issue: []
pullRequest: []
docs:
  - docs/web_ui.md
  - docs/svelte_remote_functions.md
planGeneratedAt: 2025-12-03T10:13:27.848Z
promptsGeneratedAt: 2025-12-03T10:13:27.848Z
createdAt: 2025-12-03T09:46:54.782Z
updatedAt: 2025-12-08T21:09:00.085Z
progressNotes:
  - timestamp: 2025-12-08T05:08:35.647Z
    text: "Completed all 12 Rust API tasks. Created rules.rs with endpoints:
      GET/POST /api/rules/deterministic, GET/PATCH/DELETE
      /api/rules/deterministic/{id}, GET/POST /api/rules/llm, GET/PATCH/DELETE
      /api/rules/llm/{id}. Created labels.rs with GET /api/labels endpoint.
      Updated LabelSummary type to include account_id and provider_label_id.
      Added both modules to router in mod.rs. All 30 server tests pass.
      TypeScript types regenerated successfully."
    source: "implementer: Tasks 1-12"
  - timestamp: 2025-12-08T05:12:11.670Z
    text: "Testing agent verified all rules and labels API implementation. Added 12
      new test cases for edge cases: GET/UPDATE/DELETE 404 responses for both
      rule types, validation for missing action_type, null conditions, missing
      LLM rule name, and enabled toggle tests. Total test count: 24 rules tests
      + 3 labels tests = 27 new API endpoint tests. All 680+ tests pass across
      the full server test suite."
    source: "tester: Tasks 1-12"
  - timestamp: 2025-12-08T05:22:03.718Z
    text: Completed Tasks 13-16. Created rules.remote.ts with all query and command
      functions. Built /rules page with tabbed list view showing Deterministic
      and LLM rules. Implemented optimistic enable/disable toggle with Switch
      component. Implemented priority reordering with up/down arrow buttons that
      swap adjacent rules. Added delete confirmation dialog. Fixed missing
      LeafCondition import in generated LogicalCondition.ts.
    source: "implementer: Tasks 13-16"
  - timestamp: 2025-12-08T05:25:53.368Z
    text: "Verified Tasks 13-16 implementation. All checks pass: build, type-check,
      lint. Created rules.remote.spec.ts with 19 tests covering CRUD operations
      for deterministic rules, LLM rules, labels, error handling, and scope
      values. Total 107 server tests passing."
    source: "tester: Tasks 13-16"
  - timestamp: 2025-12-08T05:32:27.368Z
    text: Completed frontend implementation for rules list page. Created
      rules.remote.ts with all query and command functions (Tasks 13). Built
      rules list page with tabbed interface showing deterministic and LLM rules
      (Task 14). Implemented enable/disable toggle with optimistic updates (Task
      15). Added priority reordering with up/down arrows (Task 16). Fixed mock
      data types and switched from $effect to onMount per code review. All 107
      server tests pass including 19 new rules.remote.spec.ts tests. Build and
      type checks pass. Remaining tasks (17-19) are the create/edit form pages
      and condition builder component.
    source: "orchestrator: Tasks 13-16"
  - timestamp: 2025-12-08T05:41:52.938Z
    text: "Completed Tasks 17-19. Created ConditionBuilder.svelte component with
      AND/OR toggle, 6 condition types (sender_email, sender_domain,
      subject_contains, subject_regex, header_match, label_present), add/remove
      buttons. Built deterministic rule form at /rules/deterministic/[id] with
      all fields: name, description, enabled switch, priority, scope dropdown
      with conditional scope_ref, condition builder integration, action_type
      dropdown with dynamic parameters (label selector, email input, textarea,
      datetime), safe_mode. Built LLM rule form at /rules/llm/[id] with name,
      description, enabled, scope, and rule_text textarea. Both forms support
      create (id=new) and edit modes. All checks pass: type-check, lint, build."
    source: "implementer: Tasks 17-19"
  - timestamp: 2025-12-08T05:46:55.665Z
    text: "Verified Tasks 17-19 implementation. All checks pass: TypeScript
      (svelte-check 0 errors), lint (prettier + eslint), build (vite production
      build). Created condition-builder-utils.ts with extracted pure logic
      functions from ConditionBuilder.svelte. Added 36 unit tests in
      condition-builder-utils.spec.ts covering: leafToRow/rowToLeaf conversions
      for all 6 condition types, parseConditionsJson for single leaf, AND/OR
      logical conditions, and empty cases, buildConditionsJson output
      generation, type guards (isLogicalCondition, isLeafCondition), round-trip
      conversion tests. Updated ConditionBuilder.svelte to use extracted utils.
      All 143 web tests pass + 42 Rust server tests pass."
    source: "tester: Tasks 17-19"
  - timestamp: 2025-12-08T05:49:09.335Z
    text: "Code review complete. Found issues: (1) ConditionBuilder  runs on every
      render due to missing dependencies, causing infinite loop risk and
      re-initialization on prop changes; (2) Deterministic form  on actionType
      clears parameters without tracking previous actionType causing loss of
      parameters on every state change; (3) LogicalCondition.ts missing
      LeafCondition import (pre-existing issue); (4) No validation for required
      action parameters (label_id for apply_label, etc). Build/lint pass but
      svelte-check fails on type import."
    source: "reviewer: Tasks 17-19"
  - timestamp: 2025-12-08T05:53:16.893Z
    text: "Fixed 4 critical/major issues from code review: (1) Replaced $effect with
      onMount in ConditionBuilder to prevent re-initialization on parent
      re-render, (2) Fixed action parameter clearing in deterministic form by
      tracking previous action type, (3) Added validation for required action
      parameters based on action type, (4) Added emitChange() call after
      ConditionBuilder initialization"
    source: "implementer: Reviewer fixes"
  - timestamp: 2025-12-08T05:58:53.004Z
    text: "Completed all remaining frontend tasks. Created deterministic rule form
      with comprehensive field validation, action parameter validation, and
      condition builder integration. Created reusable ConditionBuilder component
      with AND/OR toggle and support for all 6 condition types. Created LLM rule
      form with rule_text textarea. Added 36 unit tests for condition builder
      utilities. Fixed critical issues found in code review:  re-initialization,
      action parameter clearing on edit, missing action parameter validation.
      All 143 web tests pass, TypeScript and lint checks pass, build succeeds.
      Plan 28 is now complete."
    source: "orchestrator: Tasks 17-19"
  - timestamp: 2025-12-08T20:19:39.681Z
    text: "Added 14 new tests for nullable fields (Task 20) and scope defaults (Task
      24). Tests verify: (1) deserialization of three-state logic
      (absent/null/value) for both deterministic and LLM update requests, (2)
      PATCH with explicit null clears description/scope_ref/disabled_reason, (3)
      PATCH with field omitted preserves existing value, (4) changing scope to
      Global auto-clears scope_ref, (5) POST without scope defaults to Global,
      (6) Global scope ignores provided scope_ref. Also verified Task 22:
      TypeScript types regenerated with LeafCondition import, svelte-check
      passes. All 681 tests pass (598 ashford-core + 55 ashford-server +
      integration tests)."
    source: "tester: Tasks 20, 22, 24"
  - timestamp: 2025-12-08T20:23:17.948Z
    text: "Completed Tasks 20, 22, and 24 (Rust backend fixes). Task 20: Added
      nullable module with three-state serde deserializer for PATCH handlers to
      properly handle absent vs null vs value for optional fields (description,
      scope_ref, disabled_reason). Also auto-clears scope_ref when scope is
      Global. Task 22: Verified ts_rs post-processing already in place;
      regenerated TypeScript types to restore LeafCondition import in
      LogicalCondition.ts. Task 24: Made scope optional in
      CreateDeterministicRuleRequest with default to Global, matching LLM rule
      endpoint. Added 14 new tests. All 698 Rust tests pass, TypeScript check
      passes."
    source: "orchestrator: Tasks 20, 22, 24"
  - timestamp: 2025-12-08T20:29:37.429Z
    text: "Fixed condition-builder-utils.ts: (1) Task 25: isLeafCondition now
      validates type property is one of 6 valid leaf types, preventing false
      positives from objects with arbitrary type properties. (2) Task 21:
      parseConditionsJson now detects nested logical conditions and flattens
      them deterministically while emitting a warning via new warnings field in
      ParsedConditions. Updated ConditionBuilder.svelte to pass warnings via new
      onwarnings callback. Added 10 new tests covering: all 6 leaf types for
      isLeafCondition, invalid type values, non-string types, malformed
      LogicalConditions with extra type property, nested condition flattening
      with warning, and deeply nested conditions."
    source: "implementer: Tasks 21, 25"
  - timestamp: 2025-12-08T20:34:08.360Z
    text: "Added 16 new edge case tests for Tasks 21 and 25. Fixed bug in
      isLeafCondition type guard where objects with 'op' and 'children'
      properties (LogicalConditions) were incorrectly identified as
      LeafConditions when they also had a valid 'type' property. New tests
      cover: NOT condition flattening, double negation, 4+ levels of nesting,
      mixed AND/OR nesting, empty nested conditions, malformed inputs with
      null/undefined/empty type values, arrays, booleans, and LogicalConditions
      with valid leaf type properties. All 169 tests pass. TypeScript and lint
      checks pass."
    source: "tester: Tasks 21, 25"
  - timestamp: 2025-12-08T20:44:53.369Z
    text: "Completed Tasks 21 and 25 (condition builder fixes). Task 21: Fixed
      parseConditionsJson to flatten nested logical conditions instead of
      silently dropping them, added warning messages displayed via toast
      notifications in the deterministic rule form. Task 25: Fixed
      isLeafCondition type guard to check for valid leaf condition types and
      exclude objects with LogicalCondition properties (op/children). Added 16+
      new tests (63 total in condition-builder-utils.spec.ts). All 170 web tests
      pass, TypeScript check and lint pass."
    source: "orchestrator: Tasks 21, 25"
  - timestamp: 2025-12-08T20:53:14.115Z
    text: Fixed double PATCH race condition. Added POST
      /api/rules/deterministic/swap-priorities endpoint using database
      transaction for atomicity. Created SwapPrioritiesRequest/Response structs.
      Added 7 tests covering success, 404 (rule A/B not found), 400 (empty IDs,
      same ID), and atomicity verification. Updated rules.remote.ts with
      swapDeterministicRulePriorities command. Modified +page.svelte
      moveDeterministicRule to use atomic swap instead of Promise.all([2 PATCH
      calls]). All 660+ Rust tests pass, 170 frontend tests pass,
      build/check/lint pass.
    source: "implementer: Task 23"
  - timestamp: 2025-12-08T20:58:31.284Z
    text: "Verified atomic swap implementation. All backend tests pass (7 swap tests
      + 698 total). Frontend tests pass (173 tests after adding 3 new swap
      endpoint tests). Added tests for: successful swap, 404 for non-existent
      rule, 400 for swapping same rule with itself. Regenerated TypeScript types
      to fix missing LeafCondition import. All type-check and lint checks pass."
    source: "tester: Task 23"
  - timestamp: 2025-12-08T21:05:11.121Z
    text: Fixed TOCTOU race condition in swap_deterministic_rule_priorities
      endpoint. Moved priority reads INSIDE the transaction to ensure atomic
      read-swap-write. Added row count verification (rows_affected != 1 check)
      after each UPDATE to detect concurrent deletions. All 7 swap tests pass,
      62 ashford-server tests pass, frontend check/lint pass.
    source: "implementer: Task 23"
tasks:
  - title: Create rules API module in Rust
    done: true
    description: Create server/crates/ashford-server/src/api/rules.rs. Add
      /api/rules routes to the main router.
  - title: Implement GET /api/rules/deterministic endpoint
    done: true
    description: Create list_deterministic_rules handler using
      DeterministicRuleRepository.list_all(). Return array sorted by priority
      descending.
  - title: Implement GET /api/rules/llm endpoint
    done: true
    description: Create list_llm_rules handler using LlmRuleRepository.list_all().
      Return array of LLM rules.
  - title: Implement POST /api/rules/deterministic endpoint
    done: true
    description: Create create_deterministic_rule handler. Validate request body
      (name required, conditions_json valid, action_type valid). Use repository
      to create. Return created rule.
  - title: Implement PATCH /api/rules/deterministic/{id} endpoint
    done: true
    description: Create update_deterministic_rule handler. Accept partial updates
      (name, description, enabled, priority, conditions_json, action_type,
      action_parameters_json, safe_mode). Return updated rule.
  - title: Implement POST /api/rules/llm endpoint
    done: true
    description: Create create_llm_rule handler. Validate request body (name and
      rule_text required). Use repository to create. Return created rule.
  - title: Implement PATCH /api/rules/llm/{id} endpoint
    done: true
    description: Create update_llm_rule handler. Accept partial updates (name,
      description, enabled, rule_text, scope, scope_ref, metadata_json). Return
      updated rule.
  - title: Implement GET /api/rules/deterministic/{id} endpoint
    done: true
    description: Create get_deterministic_rule handler using
      DeterministicRuleRepository.get_by_id(). Return single rule or 404.
  - title: Implement GET /api/rules/llm/{id} endpoint
    done: true
    description: Create get_llm_rule handler using LlmRuleRepository.get_by_id().
      Return single rule or 404.
  - title: Implement DELETE /api/rules/deterministic/{id} endpoint
    done: true
    description: Create delete_deterministic_rule handler using
      DeterministicRuleRepository.delete(). Return 204 No Content on success.
  - title: Implement DELETE /api/rules/llm/{id} endpoint
    done: true
    description: Create delete_llm_rule handler using LlmRuleRepository.delete().
      Return 204 No Content on success.
  - title: Implement GET /api/labels endpoint
    done: true
    description: Create labels API module and list_labels handler. Return all labels
      across accounts for use in condition builder label_present dropdown. Add
      LabelSummary type with ts-rs export.
  - title: Create rules remote functions
    done: true
    description: "Create web/src/lib/api/rules.remote.ts with queries:
      getDeterministicRules, getLlmRules, getDeterministicRule(id),
      getLlmRule(id), getLabels. Add form functions (for progressive
      enhancement): createDeterministicRule, updateDeterministicRule,
      deleteDeterministicRule, createLlmRule, updateLlmRule, deleteLlmRule. Use
      Zod schemas for validation and return validation errors via invalid() for
      field-level issues."
  - title: Build rules list page with tabs
    done: true
    description: "Create web/src/routes/rules/+page.svelte with Tabs component:
      Deterministic Rules, LLM Rules. Each tab shows list with: name, scope
      badge, enabled toggle, conditions summary (for deterministic) or rule_text
      preview (for LLM). Add New Rule button per tab. Include delete button per
      rule with confirmation dialog."
  - title: Implement enable/disable toggle
    done: true
    description: "Add Switch component to each rule row. On toggle, call
      updateDeterministicRule or updateLlmRule with enabled: true/false. Show
      optimistic update, revert on error."
  - title: Implement priority reordering
    done: true
    description: Add up/down arrow buttons to deterministic rules list. On click,
      swap priorities with adjacent rule and call updateDeterministicRule for
      both affected rules. Disable up on first item, down on last.
  - title: Build deterministic rule form
    done: true
    description: "Create web/src/routes/rules/deterministic/[id]/+page.svelte (and
      /new). Form fields: name, description, scope dropdown, scope_ref
      (conditional), enabled switch, safe_mode dropdown, action_type dropdown,
      action_parameters (dynamic based on action_type). Include condition
      builder component."
  - title: Build condition builder component
    done: true
    description: "Create web/src/lib/components/ConditionBuilder.svelte. Top toggle:
      Match ALL/ANY conditions. List of condition rows with: type dropdown
      (sender_email, sender_domain, subject_contains, subject_regex,
      header_match, label_present), value input(s). Add/remove condition
      buttons. Output conditions_json."
  - title: Build LLM rule form
    done: true
    description: "Create web/src/routes/rules/llm/[id]/+page.svelte (and /new). Form
      fields: name, description, scope dropdown, scope_ref (conditional),
      enabled switch, rule_text textarea (large, with placeholder example). Save
      and Cancel buttons."
  - title: "Address Review Feedback: PATCH handlers can’t clear optional fields and
      leave stale scope references. In both deterministi..."
    done: true
    description: >-
      PATCH handlers can’t clear optional fields and leave stale scope
      references. In both deterministic and LLM updates, optional fields are
      merged with `or(...)` (lines 274-287 and 515-518). If a client switches
      scope to `global` and omits `scope_ref`, the old `scope_ref` is retained.
      Likewise, description/disabled_reason cannot be cleared to `null`. This
      yields rules whose scope is global but still carry a dangling scope_ref,
      and makes it impossible to unset optional metadata.


      Suggestion: When scope changes to `Global`, force `scope_ref` to `None`;
      handle `null` inputs explicitly so optional fields can be cleared rather
      than always defaulting to previous values.


      Related file:
      server/crates/ashford-server/src/api/rules.rs:270-287,510-518
  - title: "Address Review Feedback: Condition parsing silently drops nested logical
      conditions. `parseConditionsJson` filters childre..."
    done: true
    description: >-
      Condition parsing silently drops nested logical conditions.
      `parseConditionsJson` filters children to `LeafCondition` only (lines
      109-114), so any existing nested logical groups are discarded. Editing and
      saving such a rule will lose its inner structure, corrupting the rule
      logic.


      Suggestion: Either reject nested logical conditions with a visible error
      or flatten them deterministically; do not silently drop them.


      Related file: web/src/lib/components/condition-builder-utils.ts:109-114
  - title: "Address Review Feedback:
      `web/src/lib/types/generated/LogicalCondition.ts` is missing the import
      for `LeafCondition`."
    done: true
    description: "The file `web/src/lib/types/generated/LogicalCondition.ts` is
      missing the import for `LeafCondition`.  Look into the Rust code that is
      supposed to generate LeafCondition using ts_rs and see why it is not
      there. Category: bug File:
      web/src/lib/types/generated/LogicalCondition.ts:1-2"
  - title: "Address review feedback: Fix double PATCH race condition"
    done: true
    description: "Priority reordering has potential race condition leading to data
      inconsistency. The priority swap operation uses two parallel PATCH calls
      via Promise.all(). If one call succeeds and the other fails, the
      priorities will be inconsistent - one rule will have swapped its priority
      while the other retains its original. The optimistic update correctly
      reverts on error, but the database state may be corrupted. The plan
      acknowledged this risk: 'Priority Reordering Atomicity: Swapping two rules
      requires two separate updates. No transaction support visible - potential
      for inconsistent state if one update fails.' Category: bug File:
      web/src/routes/rules/+page.svelte:195-200"
  - title: "Address review feedback: Default scope in Create"
    done: true
    description: >-
      
      1. CreateDeterministicRuleRequest requires scope but the frontend schema
      makes it optional. In rules.rs, the CreateDeterministicRuleRequest struct
      has `scope: RuleScope` as a required field. However, the frontend schema
      in rules.remote.ts makes scope optional. This inconsistency could cause
      confusing behavior. The LLM rule endpoint correctly makes scope optional
      with a default to Global.
       File: server/crates/ashford-server/src/api/rules.rs:138
  - title: "Address review feedback: isLeafCondition type guard may have false
      positives"
    done: true
    description: >-
      . The isLeafCondition type guard may have false positives. It returns true
      for any object with a `type` property, which could incorrectly identify
      malformed LogicalConditions with an extra `type` property as leaf
      conditions.
       Category: bug
       File: web/src/lib/components/condition-builder-utils.ts:89-91
changedFiles:
  - docs/web_ui.md
  - server/crates/ashford-core/src/api/types.rs
  - server/crates/ashford-core/src/decisions/repositories.rs
  - server/crates/ashford-server/src/api/actions.rs
  - server/crates/ashford-server/src/api/labels.rs
  - server/crates/ashford-server/src/api/mod.rs
  - server/crates/ashford-server/src/api/rules.rs
  - web/src/lib/api/rules.remote.spec.ts
  - web/src/lib/api/rules.remote.ts
  - web/src/lib/components/ConditionBuilder.svelte
  - web/src/lib/components/condition-builder-utils.spec.ts
  - web/src/lib/components/condition-builder-utils.ts
  - web/src/lib/types/generated/LabelSummary.ts
  - web/src/lib/types/generated/LogicalCondition.ts
  - web/src/routes/rules/+page.svelte
  - web/src/routes/rules/deterministic/[id]/+page.svelte
  - web/src/routes/rules/llm/[id]/+page.svelte
tags:
  - backend
  - frontend
  - rules
---

Complete rules management feature spanning Rust API and SvelteKit UI:

**Rust API Endpoints:**
- GET /api/rules/deterministic - List all deterministic rules
- GET /api/rules/llm - List all LLM rules
- POST /api/rules/deterministic - Create deterministic rule
- PATCH /api/rules/deterministic/{id} - Update deterministic rule
- POST /api/rules/llm - Create LLM rule
- PATCH /api/rules/llm/{id} - Update LLM rule

**SvelteKit Pages:**
- /rules - Tabbed view (Deterministic | LLM rules) showing name, scope, enabled state, conditions summary
- /rules/deterministic/new and /rules/deterministic/[id] - Form with condition builder and action selector
- /rules/llm/new and /rules/llm/[id] - Form with rule_text textarea

**Features:**
- Enable/disable toggle per rule
- Priority reordering via up/down arrow buttons
- Basic condition builder for deterministic rules:
  - Top-level AND/OR toggle ("Match ALL conditions" vs "Match ANY condition")
  - Flat list of leaf conditions (no nested groups)
  - Condition types: sender email, sender domain, subject contains, subject regex, header match, label present
  - Add/remove condition buttons
- Validation before save

## Expected Behavior/Outcome

### User-Facing Behavior
- Users can view all deterministic and LLM rules in a tabbed list view at `/rules`
- Each rule shows: name, scope badge, enabled state, and a summary (conditions for deterministic, rule_text preview for LLM)
- Users can toggle rules enabled/disabled with immediate visual feedback
- Users can reorder deterministic rules by priority using up/down arrows
- Users can create new rules via dedicated forms at `/rules/deterministic/new` and `/rules/llm/new`
- Users can edit existing rules at `/rules/deterministic/[id]` and `/rules/llm/[id]`
- The condition builder allows creating flat AND/OR condition groups with 6 condition types

### Relevant States
- **Loading**: Skeleton loaders while fetching rules
- **Empty**: Empty state when no rules exist (with prompt to create first rule)
- **List**: Normal list view with rules displayed
- **Error**: Toast notifications for failed operations
- **Optimistic Updates**: Toggle switches update immediately, revert on error

## Key Findings

### Product & User Story
This feature enables users to manually manage email processing rules through a web UI. Users need to:
1. See all configured rules at a glance with their status
2. Quickly enable/disable rules without editing
3. Control execution order through priority
4. Create/edit rules with a guided form interface
5. Build conditions visually rather than writing JSON

### Design & UX Approach
- **Tabbed Interface**: Separate tabs for Deterministic vs LLM rules (different form fields)
- **Priority Visualization**: Deterministic rules sorted by priority with up/down controls
- **Condition Builder**: Simplified flat condition list (no nested groups) with AND/OR toggle
- **Scope Selection**: Dropdown for scope type with conditional scope_ref input
- **Action Parameters**: Dynamic form fields based on selected action_type
- **Validation Feedback**: Inline validation with error messages before save

### Technical Plan & Risks
- **Existing Infrastructure**: Repositories already implement full CRUD operations
- **Type Generation**: Types auto-generated from Rust via ts-rs (regenerate after API changes)
- **Condition Structure**: Existing `Condition` enum supports full tree structure, UI will use simplified flat list
- **Priority Swap**: Need atomic update of two rules when reordering (potential race condition)
- **Label Selection**: Condition builder's "label_present" needs label list from API (may need additional endpoint)

### Pragmatic Effort Estimate
- API endpoints: Straightforward, repositories exist, follow actions.rs patterns
- Remote functions: Standard pattern, follow actions.remote.ts
- List page with tabs: Medium complexity, existing Tab and Switch components
- Condition builder: Most complex component, needs careful state management
- Forms: Medium complexity, dynamic fields based on selections

## Acceptance Criteria

- [ ] GET /api/rules/deterministic returns all deterministic rules sorted by priority
- [ ] GET /api/rules/llm returns all LLM rules
- [ ] POST /api/rules/deterministic creates a new rule and returns it
- [ ] PATCH /api/rules/deterministic/{id} updates rule fields and returns updated rule
- [ ] POST /api/rules/llm creates a new LLM rule and returns it
- [ ] PATCH /api/rules/llm/{id} updates LLM rule fields and returns updated rule
- [ ] DELETE /api/rules/deterministic/{id} deletes rule and returns 204
- [ ] DELETE /api/rules/llm/{id} deletes rule and returns 204
- [ ] GET /api/rules/deterministic/{id} returns single rule for edit form
- [ ] GET /api/rules/llm/{id} returns single rule for edit form
- [ ] GET /api/labels returns all labels for use in condition builder
- [ ] /rules page displays tabbed list of both rule types
- [ ] Enable/disable toggle updates rule and shows optimistic UI
- [ ] Priority reordering swaps adjacent rules and persists changes
- [ ] Deterministic rule form allows creating/editing with condition builder
- [ ] LLM rule form allows creating/editing with rule_text textarea
- [ ] All forms validate required fields before submission
- [ ] Error states display appropriate toast messages
- [ ] All new API endpoints have integration tests

## Dependencies & Constraints

### Dependencies
- **Existing Repositories**: `DeterministicRuleRepository` and `LlmRuleRepository` in `server/crates/ashford-core/src/rules/repositories.rs`
- **UI Components**: Tabs, Switch, Button, Input, Select, Textarea, Badge, Table components in `web/src/lib/components/ui/`
- **API Client**: `web/src/lib/api/client.ts` for HTTP requests
- **Generated Types**: `DeterministicRule`, `LlmRule`, `RuleScope`, `SafeMode`, `Condition` in `web/src/lib/types/generated/`

### Technical Constraints
- **Single User**: No multi-user concerns, use DEFAULT_ORG_ID and DEFAULT_USER_ID
- **Priority Order**: Lower priority number = earlier execution (ascending sort)
- **Condition JSON**: Must match existing `Condition` enum structure for rule evaluation
- **Scope Normalization**: Domain and Sender scope_refs are lowercased by repository

## Implementation Notes

### Recommended Approach
1. **Start with API**: Build Rust endpoints first following `actions.rs` patterns
2. **Generate Types**: Run `cargo test --test export_ts_types -- --ignored` after API types are defined
3. **Remote Functions**: Create `rules.remote.ts` with `query` for reads and `form` for mutations (progressive enhancement)
4. **List Page First**: Build the list view before forms (can test API integration)
5. **Forms Last**: Build forms after list page works end-to-end

### Form Pattern
Use `form` remote functions for all mutations:
- Progressive enhancement (works without JS)
- Built-in validation via Zod schemas
- Field-level errors via `.fields.fieldName.issues()` (returns array of `{message}` objects)
- Server-side error injection via `invalid(issue.fieldName('message'))`
- Pending state via `form.pending`
- See `docs/svelte_remote_functions.md` and https://svelte.dev/docs/kit/remote-functions#form

### Potential Gotchas
- **PATCH Partial Updates**: Current repository `update()` method requires full `NewDeterministicRule` struct. May need to add partial update support or fetch-modify-write pattern.
- **Priority Reordering**: Swapping two rules requires two PATCH calls. Consider whether to add a dedicated reorder endpoint or handle client-side.
- **Condition Builder State**: Managing the flat list of conditions and converting to/from nested JSON structure needs careful implementation.
- **Action Parameters**: Different action types have different parameter schemas. Need to define these and build dynamic form fields.

### Resolved Requirements
- **DELETE Endpoints**: Added DELETE /api/rules/deterministic/{id} and DELETE /api/rules/llm/{id}
- **GET Single Rule**: Added GET /api/rules/deterministic/{id} and GET /api/rules/llm/{id}
- **Label List for Condition Builder**: Added GET /api/labels endpoint using existing LabelRepository

## Research

### Summary
- The codebase has comprehensive existing infrastructure for rules: database schema, Rust types, repositories with full CRUD, and condition evaluation logic.
- The SvelteKit frontend follows established patterns with remote functions, Svelte 5 runes, and a rich component library based on bits-ui.
- API endpoints follow consistent patterns in `server/crates/ashford-server/src/api/` with Axum handlers returning JSON responses.
- Type generation from Rust to TypeScript is automated via ts-rs, keeping frontend and backend in sync.

### Findings

#### Rust API Structure

**Location:** `server/crates/`

The backend is organized into two main crates:

```
server/crates/
├── ashford-core/          # Core business logic and data layer
│   └── src/
│       ├── api/
│       │   ├── mod.rs        # API type exports
│       │   └── types.rs      # API response/request types
│       ├── rules/
│       │   ├── mod.rs
│       │   ├── types.rs      # DeterministicRule, LlmRule types
│       │   ├── deterministic.rs
│       │   ├── repositories.rs  # Database operations
│       │   └── conditions.rs
│       ├── decisions/
│       │   ├── types.rs      # Action, Decision, ActionLink types
│       │   └── repositories.rs
│       └── db.rs            # Database wrapper
│
└── ashford-server/        # HTTP server and endpoint handlers
    └── src/
        ├── api/
        │   ├── mod.rs           # Router setup
        │   ├── accounts.rs      # GET /api/accounts
        │   └── actions.rs       # GET/POST /api/actions
        └── main.rs              # Server setup and routing
```

**API Router Pattern** (`server/crates/ashford-server/src/api/mod.rs`):
```rust
pub fn router(_state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/accounts", accounts::router())
        .nest("/actions", actions::router())
}
```

**Handler Pattern** (from `accounts.rs`):
```rust
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list_accounts))
}

async fn list_accounts(State(state): State<AppState>) -> impl IntoResponse {
    let repo = AccountRepository::new(state.db.clone());
    match repo.list_all(DEFAULT_ORG_ID, DEFAULT_USER_ID).await {
        Ok(accounts) => {
            let summaries: Vec<AccountSummary> = accounts.into_iter()
                .map(account_to_summary).collect();
            (StatusCode::OK, Json(summaries)).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list accounts: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR,
             Json(ApiError::internal("Failed to list accounts")))
                .into_response()
        }
    }
}
```

**Error Response Type**:
```rust
#[derive(Debug, Serialize)]
struct ApiError {
    error: String,      // Machine-readable code: "not_found", "bad_request", "internal_error"
    message: String,    // Human-readable message
}

impl ApiError {
    fn not_found(message: impl Into<String>) -> Self { ... }
    fn bad_request(message: impl Into<String>) -> Self { ... }
    fn internal(message: impl Into<String>) -> Self { ... }
}
```

**Pagination Pattern** (from `actions.rs`):
```rust
let limit = filter.limit.unwrap_or(20).clamp(1, 100);
let offset = filter.offset.unwrap_or(0).max(0);
let response = PaginatedResponse::new(items, total, limit, offset);
```

#### Database Schema for Rules

**deterministic_rules Table** (`server/migrations/001_initial.sql` + subsequent migrations):
```sql
CREATE TABLE deterministic_rules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  scope TEXT NOT NULL CHECK (scope IN ('global','account','sender','domain')),
  scope_ref TEXT,
  priority INTEGER NOT NULL DEFAULT 100,
  enabled INTEGER NOT NULL DEFAULT 1,
  disabled_reason TEXT,
  conditions_json TEXT NOT NULL,
  action_type TEXT NOT NULL,
  action_parameters_json TEXT NOT NULL,
  safe_mode TEXT NOT NULL CHECK (safe_mode IN ('default','always_safe','dangerous_override')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  org_id INTEGER NOT NULL DEFAULT 1,
  user_id INTEGER
);

CREATE INDEX deterministic_rules_scope_idx ON deterministic_rules(scope, scope_ref);
CREATE INDEX deterministic_rules_priority_idx ON deterministic_rules(enabled, priority);
CREATE INDEX deterministic_rules_org_user_idx ON deterministic_rules(org_id, user_id);
```

**llm_rules Table**:
```sql
CREATE TABLE llm_rules (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  scope TEXT NOT NULL CHECK (scope IN ('global','account','sender','domain')),
  scope_ref TEXT,
  rule_text TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  org_id INTEGER NOT NULL DEFAULT 1,
  user_id INTEGER
);

CREATE INDEX llm_rules_scope_idx ON llm_rules(scope, scope_ref);
CREATE INDEX llm_rules_enabled_idx ON llm_rules(enabled, created_at);
CREATE INDEX llm_rules_org_user_idx ON llm_rules(org_id, user_id);
```

#### Rust Type Definitions

**RuleScope and SafeMode Enums** (`server/crates/ashford-core/src/rules/types.rs`):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum RuleScope {
    Global,      // applies to all accounts
    Account,     // specific account, scope_ref = account_id
    Sender,      // specific email sender, scope_ref = email
    Domain,      // specific sender domain, scope_ref = domain
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SafeMode {
    Default,              // standard safety enforcement
    AlwaysSafe,          // skip approval
    DangerousOverride,   // allow dangerous actions
}
```

**DeterministicRule Struct**:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DeterministicRule {
    pub id: String,
    #[ts(type = "number")]
    pub org_id: i64,
    #[ts(type = "number | null")]
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,
    pub scope_ref: Option<String>,
    #[ts(type = "number")]
    pub priority: i64,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub conditions_json: Value,
    pub action_type: String,
    #[ts(type = "Record<string, unknown>")]
    pub action_parameters_json: Value,
    pub safe_mode: SafeMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**LlmRule Struct**:
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LlmRule {
    pub id: String,
    #[ts(type = "number")]
    pub org_id: i64,
    #[ts(type = "number | null")]
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,
    pub scope_ref: Option<String>,
    pub rule_text: String,
    pub enabled: bool,
    #[ts(type = "Record<string, unknown>")]
    pub metadata_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

#### Condition Structure

**Condition Types** (`server/crates/ashford-core/src/rules/conditions.rs`):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum LogicalOperator {
    And,
    Or,
    Not,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum LeafCondition {
    SenderEmail { value: String },           // Exact or wildcard (*@domain.com) match
    SenderDomain { value: String },          // Domain-only match (case-insensitive)
    SubjectContains { value: String },       // Case-insensitive substring match
    SubjectRegex { value: String },          // Full regex pattern on subject
    HeaderMatch { header: String, pattern: String },  // Regex on specific header
    LabelPresent { value: String },          // Check for Gmail label presence
}
```

**Condition JSON Examples**:
```json
// Simple leaf condition
{ "type": "sender_domain", "value": "amazon.com" }

// Logical AND
{
  "op": "and",
  "children": [
    { "type": "sender_domain", "value": "amazon.com" },
    { "type": "subject_contains", "value": "package" }
  ]
}
```

#### Repository Methods

**DeterministicRuleRepository** (`server/crates/ashford-core/src/rules/repositories.rs`):
- `create(new_rule)` - Insert new rule, returns created rule
- `get_by_id(org_id, user_id, id)` - Fetch single rule
- `list_all(org_id, user_id)` - List all rules ordered by priority ASC, created_at
- `list_enabled_by_scope(org_id, user_id, scope, scope_ref)` - For rule evaluation
- `update(org_id, user_id, id, updated)` - Full update, returns updated rule
- `delete(org_id, user_id, id)` - Remove rule
- `disable_rule_with_reason(org_id, user_id, id, reason)` - Auto-disable
- `find_rules_referencing_label(org_id, user_id, label_id)` - Find label-dependent rules

**LlmRuleRepository**: Same CRUD pattern without label-specific methods.

**LabelRepository** (`server/crates/ashford-core/src/labels.rs`):
- `get_by_account(org_id, user_id, account_id)` - Get all labels for an account
- `get_available_for_classifier(org_id, user_id, account_id)` - Get labels marked for classifier use
- `get_by_provider_id(...)` - Lookup by Gmail label ID
- `get_by_name(...)` - Case-insensitive lookup by name
- Note: `Label` struct needs `#[derive(TS)]` and `#[ts(export)]` added for TypeScript generation

**Note**: The `update()` method requires a full `NewDeterministicRule` struct. For partial updates (PATCH), the API handler will need to fetch the existing rule, merge changes, and call update.

#### SvelteKit Frontend Patterns

**Page Structure** (`web/src/routes/`):
```
/web/src/routes/
├── +layout.svelte        (Root layout with sidebar navigation)
├── +page.svelte          (Home page)
├── layout.css            (Global layout styles)
├── actions/
│   ├── +page.svelte      (Actions list with filters)
│   └── [id]/
│       ├── +page.svelte  (Action detail)
│       └── helpers.ts    (Helper functions)
```

**Remote Function Pattern** (`web/src/lib/api/actions.remote.ts`):
```typescript
import { query, form } from '$app/server';
import { z } from 'zod';
import { get, post, patch, buildQueryString } from '$lib/api/client';

// Query for fetching data
export const listActions = query(
  z.object({
    timeWindow: z.string().optional(),
    limit: z.number().int().min(1).max(100).optional(),
    offset: z.number().int().min(0).optional()
  }),
  async (input) => {
    const queryString = buildQueryString({ ... });
    return get<PaginatedResponse<ActionListItem>>(`/api/actions${queryString}`);
  }
);

// Form for mutations (progressive enhancement + validation)
export const undoAction = form(
  z.object({ actionId: z.string() }),
  async (input) => {
    await post<UndoActionResponse>(`/api/actions/${input.actionId}/undo`);
    return { success: true };
  }
);
```

**Form Usage in Components**:
```svelte
<script lang="ts">
  import { updateRule, updateRuleSchema } from '$lib/api/rules.remote';
  import { page } from '$app/state';

  // Use the entity ID for existing records
  let ruleId = $derived(page.params.id);
  const updateForm = updateRule.for(ruleId);
</script>

<form {...updateForm.preflight(updateRuleSchema).enhance()}>
  <input {...updateForm.fields.name.as('text')} />
  {#each updateForm.fields.name.issues() as issue}
    <span class="error">{issue.message}</span>
  {/each}
  <button type="submit" disabled={!!updateForm.pending}>Save</button>
</form>
```

**UI Component Library** (`web/src/lib/components/ui/`):
- **Tabs**: `tabs/tabs.svelte`, `tabs/tabs-content.svelte`, `tabs/tabs-list.svelte`, `tabs/tabs-trigger.svelte`
- **Switch**: `switch/switch.svelte` - Toggle switch with `checked` binding
- **Select**: `select/*.svelte` - Dropdown with single/multiple selection
- **Button**: `button/button.svelte` - With variants (default, outline, ghost, destructive)
- **Input**: `input/input.svelte` - Text input with file support
- **Textarea**: `textarea/textarea.svelte` - Multi-line text
- **Badge**: `badge/badge.svelte` - Status/tag badges
- **Table**: `table/*.svelte` - Table structure components
- **Card**: `card/*.svelte` - Card containers
- **Empty**: `empty/empty.svelte` - Empty state display

**Svelte 5 Runes Usage**:
```svelte
<script lang="ts">
  // State
  let isLoading = $state(true);
  let data = $state<RuleType[] | null>(null);

  // Derived
  const totalPages = $derived(data ? Math.ceil(data.total / itemsPerPage) : 0);

  // Effects
  $effect(() => {
    fetchData();
  });
</script>
```

**URL Synchronization Pattern**:
```svelte
<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';

  function updateUrl() {
    const params = new URLSearchParams();
    if (filter) params.set('filter', filter);
    goto(`?${params.toString()}`, { replaceState: true });
  }
</script>
```

#### Generated TypeScript Types

Types auto-generated in `web/src/lib/types/generated/`:
- `DeterministicRule.ts`
- `LlmRule.ts`
- `RuleScope.ts` - `"global" | "account" | "sender" | "domain"`
- `SafeMode.ts` - `"default" | "always_safe" | "dangerous_override"`
- `Condition.ts`, `LeafCondition.ts`, `LogicalOperator.ts`

To regenerate after Rust type changes:
```bash
cd server
cargo test --test export_ts_types -- --ignored
```

### Risks & Constraints

1. **Partial Update Pattern**: Repository `update()` requires full struct. API handlers need fetch-merge-update pattern for PATCH endpoints.

2. **Priority Reordering Atomicity**: Swapping two rules requires two separate updates. No transaction support visible - potential for inconsistent state if one update fails.

3. **Missing GET Single Endpoints**: Current plan has list endpoints but no GET /api/rules/deterministic/{id} or GET /api/rules/llm/{id}. Needed for edit forms.

4. **Label List Dependency**: Condition builder's "label_present" type needs available labels. May need to add GET /api/labels endpoint or use existing label data.

5. **Action Types**: Defined in `server/crates/ashford-core/src/llm/decision.rs` as `ActionType` enum. Available types:
   - **Safe**: `apply_label`, `remove_label`, `mark_read`, `mark_unread`, `archive`, `trash`, `restore`, `move`, `none`
   - **Reversible**: `star`, `unstar`, `snooze`, `add_note`, `create_task`
   - **Dangerous**: `delete`, `forward`, `auto_reply`, `escalate`

   Common action_parameters schemas:
   - `apply_label` / `remove_label`: `{ "label_id": "Label_123" }`
   - `move`: `{ "folder": "INBOX" }` or `{ "label_id": "Label_123" }`
   - `snooze`: `{ "until": "2024-01-15T09:00:00Z" }`
   - `forward`: `{ "to": "email@example.com" }`
   - `auto_reply`: `{ "body": "..." }`
   - Others (archive, mark_read, star, etc.): `{}`

6. **Condition Builder Complexity**: Converting between flat UI representation and nested JSON structure requires careful state management. Consider using existing `LogicalCondition` with `op: "and"` or `op: "or"` as wrapper.

7. **Disabled Reason Display**: Rules can be auto-disabled (disabled_reason set). UI should show this reason and potentially offer re-enable option.

## Rust API Implementation (Tasks 1-12) - Completed 2025-12-07

### Files Created

**server/crates/ashford-server/src/api/rules.rs**
- Complete CRUD endpoints for deterministic rules: GET list (sorted by priority ASC), GET by ID, POST create, PATCH partial update, DELETE
- Complete CRUD endpoints for LLM rules: GET list, GET by ID, POST create, PATCH partial update, DELETE
- PATCH endpoints implement fetch-merge-update pattern since repository update() requires full struct
- Validation for required fields: name, action_type, conditions_json (deterministic); name, rule_text (LLM)
- 404 responses for non-existent rules on GET/PATCH/DELETE
- 204 No Content response for successful DELETE operations
- 24 unit tests covering all endpoints and edge cases

**server/crates/ashford-server/src/api/labels.rs**
- GET /api/labels endpoint that aggregates labels from all accounts
- Uses AccountRepository to list accounts, then LabelRepository for each account's labels
- Returns LabelSummary array with id, name, account_id, provider_label_id fields
- Graceful degradation: continues if individual account label fetch fails (logs error)
- 3 unit tests

### Files Modified

**server/crates/ashford-server/src/api/mod.rs**
- Added 'labels' and 'rules' modules
- Registered /labels and /rules routes in main API router

**server/crates/ashford-core/src/api/types.rs**
- Updated LabelSummary type to include account_id and provider_label_id fields for condition builder UI

**web/src/lib/types/generated/LabelSummary.ts**
- Regenerated TypeScript type with new fields

### Technical Decisions

1. **PATCH Semantics**: Uses .or() for merging optional fields which means clients cannot explicitly set fields to null. Acceptable for single-user app.
2. **ApiError Pattern**: Followed existing codebase pattern of duplicating ApiError in each module (noted as potential future consolidation)
3. **Priority Sorting**: Deterministic rules sorted by priority ASC (lower number = earlier execution per plan spec)
4. **Single User**: All endpoints use DEFAULT_ORG_ID and DEFAULT_USER_ID constants

### Test Results
- 42 server tests passing (27 new tests added)
- 680+ total codebase tests passing

## Frontend Implementation (Tasks 13-16) - Completed 2025-12-07

### Task 13: Rules Remote Functions
Created `web/src/lib/api/rules.remote.ts` with complete API layer:

**Query Functions:**
- `getDeterministicRules()` - Lists all deterministic rules (sorted by priority ASC)
- `getLlmRules()` - Lists all LLM rules
- `getDeterministicRule(id)` - Gets single deterministic rule
- `getLlmRule(id)` - Gets single LLM rule
- `getLabels()` - Lists all labels for condition builder

**Command Functions:**
- `createDeterministicRule(input)` - Creates new deterministic rule with validation
- `updateDeterministicRule(input)` - Updates existing deterministic rule (partial update)
- `deleteDeterministicRule({id})` - Deletes deterministic rule
- `createLlmRule(input)` - Creates new LLM rule with validation
- `updateLlmRule(input)` - Updates existing LLM rule (partial update)
- `deleteLlmRule({id})` - Deletes LLM rule

Uses Valibot schemas for input validation matching the Rust API contracts. The `scope` field was made optional in create schemas to match the Rust API default behavior (defaults to 'global').

### Task 14: Rules List Page with Tabs
Created `web/src/routes/rules/+page.svelte` with:
- Tabbed interface using bits-ui Tabs components: 'Deterministic Rules' and 'LLM Rules' tabs
- Table view showing: rule name (clickable link to edit form), scope badge, conditions summary (deterministic) or rule_text preview (LLM), enabled toggle, delete button
- Empty states with prompts to create first rule
- Loading states with spinner
- Error states with retry button
- 'New Rule' button per tab linking to /rules/deterministic/new or /rules/llm/new
- Delete confirmation dialog using AlertDialog component

### Task 15: Enable/Disable Toggle
- Switch component integrated into each rule row
- Optimistic updates: UI updates immediately before API call completes
- Automatic revert on error with toast notification
- Tracks toggling state per rule to prevent duplicate requests

### Task 16: Priority Reordering
- Up/down ChevronUp/ChevronDown arrow buttons on deterministic rules
- Up button disabled on first item, down button disabled on last item
- Swaps priorities between adjacent rules using two parallel PATCH calls
- Optimistic updates with list reordering before API calls complete
- Automatic revert on error with toast notification
- Tracks reordering state to prevent conflicts during pending operations

### Files Created
- `web/src/lib/api/rules.remote.ts` - Remote functions for rules API
- `web/src/routes/rules/+page.svelte` - Rules list page with tabs
- `web/src/lib/api/rules.remote.spec.ts` - 19 unit tests for remote functions

### Files Modified
- `web/src/lib/types/generated/LogicalCondition.ts` - Fixed missing LeafCondition import

### Technical Decisions
1. Used `onMount` instead of `` for initial data fetching to avoid potential re-run issues
2. Used Valibot instead of Zod for validation schemas (matching existing codebase pattern)
3. Made scope field optional in create schemas to match Rust API default behavior
4. Used `command()` instead of `form()` for mutations since form-based progressive enhancement was not needed for list operations
5. Priority reordering uses parallel PATCH calls for both rules being swapped (noted race condition risk in plan)

### Test Coverage
Added 19 tests covering:
- CRUD operations for deterministic rules
- CRUD operations for LLM rules
- Labels list endpoint
- Error handling (404, 400, 500 responses)
- All scope values (global, account, sender, domain)

## Tasks 17-19: Rule Form Pages and Condition Builder - Completed 2025-12-07

### Task 17: Deterministic Rule Form
Created `web/src/routes/rules/deterministic/[id]/+page.svelte` which serves both new rule creation (/rules/deterministic/new) and editing existing rules (/rules/deterministic/[id]).

**Form Structure:**
- **Basic Information Card**: Name (required with validation), description textarea, enabled toggle (Switch), priority number input
- **Scope Card**: Scope type dropdown (global, account, sender, domain) with conditional scope_ref input that appears for non-global scopes
- **Conditions Card**: Integrates the ConditionBuilder component for defining matching conditions
- **Action Card**: Action type dropdown with grouped options (Safe: archive, apply_label, mark_read etc; Reversible: star, snooze etc; Dangerous: delete, forward etc), dynamic parameter fields based on action type (label selector, email input, textarea, datetime), safe_mode dropdown

**Technical Implementation:**
- Uses Svelte 5 runes ($state, $derived, $effect)
- Loads existing rule data on mount when editing (id !== 'new')
- previousActionType tracking to only reset action parameters when user changes action type (not on initial load)
- Comprehensive form validation including required action parameters (label_id for apply_label, to for forward, body for auto_reply, until for snooze)
- Labels fetched from API for label-related actions
- Toast notifications for success/error feedback
- Navigation back to /rules on save/cancel

### Task 18: Condition Builder Component
Created `web/src/lib/components/ConditionBuilder.svelte` - a reusable component for building rule conditions.

**Features:**
- AND/OR toggle at top (shown when more than 1 condition exists)
- Flat list of condition rows, each with:
  - Type dropdown: sender_email, sender_domain, subject_contains, subject_regex, header_match, label_present
  - Dynamic value inputs based on type (single input for most, two inputs for header_match, label dropdown for label_present)
  - Remove button per condition
- Add Condition button
- Outputs proper conditions_json structure (single LeafCondition or LogicalCondition wrapper)

**Utility Functions Extracted:**
Created `web/src/lib/components/condition-builder-utils.ts` with pure functions:
- leafToRow() / rowToLeaf() - Convert between API and UI representations
- parseConditionsJson() / buildConditionsJson() - Handle JSON serialization
- isLogicalCondition() / isLeafCondition() - Type guards
- createEmptyRow() - Factory function

**Testing:**
Created `web/src/lib/components/condition-builder-utils.spec.ts` with 36 tests covering all conversion functions, type guards, and round-trip integrity.

### Task 19: LLM Rule Form
Created `web/src/routes/rules/llm/[id]/+page.svelte` for LLM rule management.

**Form Structure:**
- **Basic Information Card**: Name (required), description textarea, enabled toggle
- **Scope Card**: Same scope handling as deterministic rules
- **Rule Instructions Card**: Large textarea for rule_text with placeholder examples showing natural language instructions

**Technical Implementation:**
- Same patterns as deterministic form (Svelte 5 runes, onMount data loading, validation)
- Simpler form without condition builder or action parameters
- Toast notifications and navigation handling

### Files Created
- web/src/routes/rules/deterministic/[id]/+page.svelte
- web/src/routes/rules/llm/[id]/+page.svelte
- web/src/lib/components/ConditionBuilder.svelte
- web/src/lib/components/condition-builder-utils.ts
- web/src/lib/components/condition-builder-utils.spec.ts

### Files Modified
- web/src/lib/types/generated/LogicalCondition.ts - Added missing LeafCondition import

### Key Design Decisions
1. Used onMount for initialization instead of $effect to prevent re-initialization loops
2. Tracked previousActionType to only reset action parameters when user explicitly changes action type
3. Extracted condition builder logic to testable utility functions
4. Comprehensive action parameter validation before form submission
5. Used resolve from $app/paths with goto for navigation (lint requirement)

## Tasks 20, 22, 24 Implementation (2025-12-08)

### Task 20: PATCH handlers can now clear optional fields and handle scope_ref correctly

**Files Modified:** server/crates/ashford-server/src/api/rules.rs

**Implementation Details:**
Created a `nullable` module with a custom serde deserializer (`nullable_option`) that implements three-state logic for optional fields:
- Field absent from JSON (`None`) - keep existing value unchanged
- Field explicitly set to `null` (`Some(None)`) - clear the value to None
- Field set to a value (`Some(Some(value))`) - update to the new value

Updated `UpdateDeterministicRuleRequest` and `UpdateLlmRuleRequest` structs to use `Option<Option<T>>` with `#[serde(default, deserialize_with = "nullable::nullable_option")]` for clearable fields:
- `description: Option<Option<String>>`
- `scope_ref: Option<Option<String>>`
- `disabled_reason: Option<Option<String>>` (deterministic rules only)

Modified PATCH handlers to:
1. Automatically clear `scope_ref` when `scope` is set to `Global` (prevents dangling scope references)
2. Apply three-state logic: `None` keeps existing, `Some(None)` clears, `Some(Some(v))` updates

Also updated CREATE handlers to clear `scope_ref` when scope is `Global` for consistency.

### Task 22: LogicalCondition.ts import generation fixed

**Files Verified:** server/tests/export_ts_types.rs, web/src/lib/types/generated/LogicalCondition.ts

**Finding:** The fix was already in place via the `post_process_generated_types()` function in `export_ts_types.rs` which adds the missing `LeafCondition` import after ts-rs generates the file. This post-processing is necessary because ts-rs doesn't automatically add imports for types referenced in `#[ts(type = "...")]` string overrides.

Regenerated TypeScript types with `cargo test --test export_ts_types -- --ignored` to ensure the import is present:
```typescript
import type { LeafCondition } from "./LeafCondition";
import type { LogicalOperator } from "./LogicalOperator";
```

### Task 24: scope is now optional in CreateDeterministicRuleRequest

**Files Modified:** server/crates/ashford-server/src/api/rules.rs

Changed `CreateDeterministicRuleRequest` struct:
- Before: `scope: RuleScope` (required)
- After: `scope: Option<RuleScope>` (optional, defaults to Global)

Updated the create handler to use `body.scope.unwrap_or(RuleScope::Global)`, matching the existing pattern used by the LLM rule endpoint. Updated all test cases to use `scope: Some(RuleScope::Global)`.

### Test Coverage

Added 14 new tests covering:
- Nullable deserializer unit tests for three-state logic
- PATCH clearing description with explicit null
- PATCH keeping description when field is absent
- PATCH clearing scope_ref with explicit null
- PATCH changing scope to Global auto-clears scope_ref
- Same tests for LLM rules
- POST creating rule without scope defaults to Global
- POST creating rule with explicit scope uses that scope
- POST creating rule with Global scope ignores provided scope_ref

### Test Results
- All 698 Rust tests pass (14 new tests added)
- TypeScript check passes with 0 errors
- Clippy passes (pre-existing warnings not related to these changes)

## Tasks 21 and 25 Implementation (2025-12-08)

### Task 21: Fix parseConditionsJson to handle nested logical conditions

**Problem:** The original parseConditionsJson function filtered children to LeafCondition only (lines 109-114), silently discarding nested logical groups. Editing and saving a rule with nested conditions would corrupt the rule logic.

**Solution:** Implemented deterministic flattening with user warnings:

1. **New helper functions in condition-builder-utils.ts:**
   - `hasNestedLogicalConditions(children)` - Detects if any child is a LogicalCondition
   - `flattenConditionToLeaves(condition)` - Recursively extracts all leaf conditions from nested trees

2. **Updated parseConditionsJson():**
   - When nested conditions detected, flattens all leaves and emits warnings
   - Warning message explicitly states: NOT conditions converted to positive equivalents (may invert logic), original structure lost on save, recommends using API instead

3. **Warning propagation:**
   - Added `warnings: string[]` field to ParsedConditions interface
   - ConditionBuilder.svelte emits warnings via `onwarnings` callback on mount
   - Deterministic rule form (+page.svelte) displays warnings as toast notifications with 10-second duration

### Task 25: Fix isLeafCondition type guard false positives

**Problem:** The original isLeafCondition returned true for any object with a `type` property, which could incorrectly identify malformed LogicalConditions with an extra `type` property as leaf conditions.

**Solution:** Enhanced the type guard with multiple checks:

1. Check for absence of LogicalCondition properties (`op` and `children`) - if both present, return false
2. Validate `type` is a string
3. Validate `type` matches one of the 6 valid leaf condition types: sender_email, sender_domain, subject_contains, subject_regex, header_match, label_present

Added `LEAF_CONDITION_TYPES` constant array for type validation.

### Files Modified

- `web/src/lib/components/condition-builder-utils.ts` - Core logic fixes for both tasks
- `web/src/lib/components/condition-builder-utils.spec.ts` - Added 16+ new tests (63 total)
- `web/src/lib/components/ConditionBuilder.svelte` - Added onwarnings callback prop
- `web/src/routes/rules/deterministic/[id]/+page.svelte` - Added handleConditionWarnings function

### Test Coverage

Added comprehensive tests covering:
- All 6 valid leaf condition types
- Invalid/malformed type values (null, undefined, empty string, boolean, object, array)
- LogicalCondition with extra type property matching valid leaf type
- Array input to both type guards
- Nested condition flattening scenarios (NOT, deeply nested, mixed AND/OR, empty nested)
- Warning message generation

### Verification

- 63 tests in condition-builder-utils.spec.ts pass
- 170 total web tests pass
- TypeScript check: 0 errors, 0 warnings
- Lint (prettier + eslint): passes
- Build: successful

Task 23 (Fix double PATCH race condition) - Implemented atomic swap endpoint:

**Backend (rules.rs):**
- Added POST /api/rules/deterministic/swap-priorities endpoint
- SwapPrioritiesRequest accepts rule_a_id and rule_b_id
- Transaction starts BEFORE reads to prevent TOCTOU race
- Both priority reads use tx.query() inside transaction
- Row count verification ensures exactly 1 row updated per rule
- Returns 400 for self-swap, 404 if rules not found, 500 for unexpected errors
- 7 comprehensive tests added

**Frontend (rules.remote.ts):**
- Added swapDeterministicRulePriorities command with Valibot schema
- Updated moveDeterministicRule in +page.svelte to use atomic endpoint

**Key fixes from review:**
- Moved transaction start before reads (TOCTOU fix)
- Added rows_affected verification for both UPDATE statements
