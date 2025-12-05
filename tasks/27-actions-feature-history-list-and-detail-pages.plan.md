---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Actions Feature: History List and Detail Pages"
goal: Build the actions history page with filters and action detail page with
  undo support, including all required Rust API endpoints
id: 27
uuid: 18da2db8-7523-467d-9f54-5f8a73896df7
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
planGeneratedAt: 2025-12-04T18:26:48.714Z
promptsGeneratedAt: 2025-12-04T18:26:48.714Z
createdAt: 2025-12-03T09:46:54.701Z
updatedAt: 2025-12-06T23:31:32.659Z
progressNotes:
  - timestamp: 2025-12-05T09:27:02.369Z
    text: "Completed backend implementation: (1) Added ActionListItem, ActionDetail,
      UndoActionResponse, and ActionListFilter types to
      ashford-core/src/api/types.rs with ts-rs derives; (2) Added
      list_filtered() and get_detail() methods to ActionRepository with SQL
      JOINs across actions, messages, decisions, and accounts tables; (3)
      Created server/crates/ashford-server/src/api/ module with mod.rs and
      actions.rs containing GET /api/actions, GET /api/actions/:id, and POST
      /api/actions/:id/undo endpoints; (4) Updated main.rs to nest the API
      router under /api; (5) Generated TypeScript types and updated index.ts
      exports. All tests pass."
    source: "implementer: Tasks 1-5 Backend API"
  - timestamp: 2025-12-05T09:30:41.640Z
    text: "Backend implementation verified: All 581 tests pass (1 pre-existing flaky
      test due to SQLite parallelism unrelated to Actions API). API types
      (ActionListItem, ActionDetail, UndoActionResponse), repository methods
      (list_filtered, get_detail), and API handlers (GET/POST /api/actions) are
      implemented correctly. TypeScript types regenerated successfully."
    source: "tester: Tasks 1-5 verification"
  - timestamp: 2025-12-05T09:49:08.753Z
    text: "Completed frontend implementation for actions list page: (1) Created
      actions.remote.ts with listActions query, getAction query, and undoAction
      command using Valibot schemas; (2) Created actions.constants.ts for shared
      constants (TIME_WINDOWS, ACTION_STATUSES) since remote files can only
      export remote functions; (3) Created /actions/+page.svelte with table,
      filters (time window toggle, sender input, action type multi-select,
      status checkboxes, confidence range inputs), pagination, and 10s polling
      with visibility change detection; (4) Created placeholder
      /actions/[id]/+page.svelte for detail page; (5) Enabled
      experimental.remoteFunctions in svelte.config.js. All lint and build
      checks pass."
    source: "implementer: Tasks 6-9"
  - timestamp: 2025-12-05T09:57:04.573Z
    text: "Frontend verification completed: Build succeeds, 33 tests pass (14 new
      tests for actions API logic), lint passes, typecheck passes. Fixed
      pre-existing type issues: (1) LogicalCondition.ts missing LeafCondition
      import, (2) sidebar-menu-button.svelte Record type missing args (added to
      .prettierignore due to Prettier bug). Implementation review confirms Tasks
      6-9 are complete: remote functions with Valibot schemas, actions list page
      with table/filters/pagination, URL param persistence, 10s polling with
      visibility detection and proper cleanup."
    source: "tester: Tasks 6-9 verification"
  - timestamp: 2025-12-05T10:18:22.665Z
    text: "Completed action detail page and undo functionality. Created
      web/src/routes/actions/[id]/+page.svelte with: (1) Header showing
      action_type Badge and status Badge, (2) Summary Card with timestamp,
      confidence %, subject, sender, executed_at, and error display, (3)
      Rationale section from decision or 'No rationale provided', (4)
      Collapsible Decision JSON with prettified JSON, (5) Collapsible Action
      Parameters section, (6) 'Undone by' link if has_been_undone with link to
      undo action, (7) 'Open in Gmail' button when gmail_link present, (8) Undo
      button visible only when can_undo is true with loading state and toast
      notifications. Also fixed pre-existing bug in LogicalCondition.ts where
      LeafCondition import was missing. All verifications pass: svelte-check,
      build, lint, and 33 unit tests."
    source: "implementer: Tasks 10-11"
  - timestamp: 2025-12-05T10:24:18.160Z
    text: Verification completed for action detail page and undo functionality. All
      requirements met. Added 26 new tests for helper functions by extracting
      them to helpers.ts for testability. Fixed pre-existing TypeScript error in
      LogicalCondition.ts (missing LeafCondition import). Renamed
      confusingly-named state variables (jsonCollapsed/paramsCollapsed to
      isJsonOpen/isParamsOpen). All 59 frontend tests pass. Build, lint, and
      typecheck all pass.
    source: "tester: Tasks 10-11 verification"
  - timestamp: 2025-12-05T10:29:59.455Z
    text: "Completed Tasks 10-11: Action detail page with full undo functionality.
      Files created/modified: +page.svelte (detail page), helpers.ts (extracted
      functions), helpers.spec.ts (26 tests), errors.ts (extracted ApiError
      class). Fixed build error by separating ApiError from server-only
      client.ts. Fixed reviewer issue with better API error message extraction.
      All checks pass: build, typecheck, lint, 59 server tests."
    source: "orchestrator: Tasks 10-11"
  - timestamp: 2025-12-05T21:58:10.047Z
    text: "Code review complete. Implementation is solid overall: SQL uses
      parameterized queries preventing injection, proper error handling, correct
      action link semantics for undo tracking. All 59 frontend server tests and
      all Rust tests pass. Minor issues found: 1) fetchAccounts effect has no
      dependencies so will re-run unnecessarily, 2) API defaults to offset=0 for
      list but frontend defaults to page=1 calculation. Pre-existing issue with
      LogicalCondition.ts missing LeafCondition import is not related to this
      PR."
    source: "reviewer: Final Review"
  - timestamp: 2025-12-05T22:01:02.755Z
    text: Identified critical bugs in actions list API parameter binding and
      confidence filtering; additional undo endpoint validation gaps noted
      during review.
    source: "reviewer: review"
  - timestamp: 2025-12-06T01:59:55.045Z
    text: "Fixed two code review issues: (1) Extracted duplicate formatting
      functions to shared module web/src/lib/formatting/actions.ts with
      formatTimestampShort and formatTimestampFull variants, updated helpers.ts
      to re-export for backwards compatibility. (2) Added 300ms debounce for
      sender input field to avoid API calls on every keystroke. Tests, type
      check, and linting all pass."
    source: "implementer: autofix review issues"
  - timestamp: 2025-12-06T23:22:48.720Z
    text: Fixed list_filtered placeholder ordering and confidence filtering;
      tightened undo inverse_action validation and error handling; updated
      frontend confidence param handling and tests; added regression test for
      filtered listing.
    source: "implementer: review-fixes"
  - timestamp: 2025-12-06T23:31:32.654Z
    text: Added backend undo_action tests for invalid inverse_action and link
      insertion failure cleanup.
    source: "tester: undo endpoint tests"
tasks:
  - title: Create API types and repository methods
    done: true
    description: "Add ActionListItem and ActionDetail types to ashford-core with
      ts-rs derives. Add list_filtered() method to ActionRepository with:
      time_window, account_id, sender (smart match), action_type[], status[],
      confidence range, pagination. Add get_detail() method that JOINs action +
      decision + message. Run type generation."
  - title: Create actions API module and router
    done: true
    description: Create server/crates/ashford-server/src/api/mod.rs and actions.rs.
      Set up Axum router with /api/actions routes. Update main.rs to nest the
      API router under /api prefix.
  - title: Implement GET /api/actions endpoint
    done: true
    description: "Create list_actions handler accepting query params: time_window
      (24h/7d/30d), account_id, sender, action_type (comma-separated), status
      (comma-separated), min_confidence, max_confidence, limit (default 20, max
      100), offset. Parse time_window to datetime. Return
      PaginatedResponse<ActionListItem>."
  - title: Implement GET /api/actions/{id} endpoint
    done: true
    description: Create get_action handler using get_detail() repository method.
      Return ActionDetail with joined decision/message data, computed can_undo
      (status=Completed, has undo_hint, no existing undo link), gmail_link
      (constructed from account email + provider_message_id), has_been_undone
      flag.
  - title: Implement POST /api/actions/{id}/undo endpoint
    done: true
    description: "Create undo_action handler: validate can_undo eligibility, create
      new Action from undo_hint_json, create ActionLink with undo_of relation,
      enqueue job via JobQueue. Return {undo_action_id, status: 'queued',
      message} or 400/404 errors."
  - title: Create actions remote functions
    done: true
    description: Create web/src/lib/api/actions.remote.ts. Add listActions query
      with Valibot schema for all filter params. Add getAction query by ID. Add
      undoAction command. Use generated ActionListItem, ActionDetail types.
  - title: Build actions list page with table
    done: true
    description: "Create web/src/routes/actions/+page.svelte. Use shadcn Table with
      columns: Timestamp (formatted), Subject (truncated), Sender, Action Type,
      Confidence (percentage with color), Status (Badge). Rows link to
      /actions/[id]. Show Spinner during load, Empty state when no results."
  - title: Add action filters UI
    done: true
    description: "Add filter bar above table: time window buttons (24h/7d/30d/All),
      account Select dropdown, sender Input (placeholder: 'Email or domain'),
      action type multi-Select, status checkbox group, confidence min/max
      Inputs. Read initial values from URL params, update URL on change."
  - title: Add pagination and polling
    done: true
    description: Add Pagination component below table bound to page state. Add
      items-per-page Select (10/25/50). Show total count. Add $effect for 10s
      polling with visibilitychange listener to pause when hidden. Return
      cleanup function to clear interval.
  - title: Build action detail page
    done: true
    description: Create web/src/routes/actions/[id]/+page.svelte. Header with
      action_type Badge and status Badge. Card with timestamp, confidence %,
      subject, sender. Rationale section (or 'No rationale' if null).
      Collapsible 'Decision JSON' with pre-formatted JSON. 'Open in Gmail'
      Button (href=gmail_link). Show 'Undone by [link]' if has_been_undone.
  - title: Implement undo functionality
    done: true
    description: "Add Undo button to detail page (visible only if can_undo). On
      click: set loading state, call undoAction command, on success show
      toast('Action undo queued') and refresh data, on error show
      toast.error(message). Button disabled during loading with Spinner."
changedFiles:
  - docs/data_model.md
  - docs/web_ui.md
  - server/Cargo.lock
  - server/crates/ashford-core/src/api/mod.rs
  - server/crates/ashford-core/src/api/types.rs
  - server/crates/ashford-core/src/decisions/mod.rs
  - server/crates/ashford-core/src/decisions/repositories.rs
  - server/crates/ashford-core/src/gmail/mime_builder.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/tests/export_ts_types.rs
  - server/crates/ashford-server/Cargo.toml
  - server/crates/ashford-server/src/api/accounts.rs
  - server/crates/ashford-server/src/api/actions.rs
  - server/crates/ashford-server/src/api/mod.rs
  - server/crates/ashford-server/src/main.rs
  - web/.prettierignore
  - web/src/lib/api/accounts.remote.ts
  - web/src/lib/api/actions.constants.ts
  - web/src/lib/api/actions.remote.spec.ts
  - web/src/lib/api/actions.remote.ts
  - web/src/lib/api/client.ts
  - web/src/lib/api/errors.ts
  - web/src/lib/components/ui/sidebar/sidebar-menu-button.svelte
  - web/src/lib/types/generated/ActionDetail.ts
  - web/src/lib/types/generated/ActionListItem.ts
  - web/src/lib/types/generated/LogicalCondition.ts
  - web/src/lib/types/generated/UndoActionResponse.ts
  - web/src/lib/types/generated/index.ts
  - web/src/routes/actions/+page.svelte
  - web/src/routes/actions/[id]/+page.svelte
  - web/src/routes/actions/[id]/helpers.spec.ts
  - web/src/routes/actions/[id]/helpers.ts
  - web/svelte.config.js
tags:
  - actions
  - backend
  - frontend
---

Complete actions feature spanning Rust API and SvelteKit UI:

**Rust API Endpoints:**
- GET /api/actions - List actions with filters (timeWindow, account, sender, actionType, status, confidence range, pagination)
- GET /api/actions/{id} - Get action detail with decision, message info, and undo hint
- POST /api/actions/{id}/undo - Enqueue undo job

**SvelteKit Pages:**
- /actions - Filterable table with columns: timestamp, subject, sender, action type, confidence, status
- /actions/[id] - Detail view showing decision JSON, rationale, before/after state, Gmail link, undo button

**Features:**
- Pagination for large result sets
- Periodic polling for updates (10s interval)
- Filter persistence in URL params
- Loading and error states

## Research

### Summary

The Actions feature implements a history viewing and management system spanning Rust API endpoints and SvelteKit UI pages. The codebase has solid foundations: existing repository patterns for Actions/Decisions, auto-generated TypeScript types via ts-rs, a remote functions pattern for API calls, and shadcn-svelte UI components. The main work involves creating new API routes in the minimal server (currently only `/healthz`), adding a filtered list query to the ActionRepository, and building two new SvelteKit pages with existing UI primitives.

Key discoveries:
- The ActionRepository already has `get_by_id`, `list_by_status`, and `list_by_message_id` methods but lacks a general filtered list with pagination
- Types `Action`, `ActionStatus`, `Decision`, `PaginatedResponse<T>`, and `MessageSummary` already exist and are auto-generated
- The server's router is minimal (only healthz endpoint) - will need to establish the `/api` routing pattern
- Navigation sidebar already has `/actions` route defined but no page exists
- Remote function patterns in `example.remote.ts` provide exact templates for the new queries/commands

### Findings

#### Rust Backend Structure

**Server Entry Point**: `server/crates/ashford-server/src/main.rs`
- Currently only has `/healthz` GET endpoint
- Uses Axum framework with `Router::new().route(...).with_state(state)` pattern
- `AppState` struct contains `db: Database`
- Will need to create `api/` module structure and register routes

**ActionRepository**: `server/crates/ashford-core/src/decisions/repositories.rs`
- Existing methods:
  - `create(new_action)` - Create action
  - `get_by_id(org_id, user_id, id)` - Get single action
  - `get_by_decision_id(org_id, user_id, decision_id)` - Actions for a decision
  - `list_by_message_id(org_id, user_id, message_id)` - Actions for a message
  - `list_by_status(org_id, user_id, status, account_id)` - Filter by status
  - `update_status(...)`, `mark_executing(...)`, `mark_completed(...)`, `mark_failed(...)`
- **Missing**: A general `list_filtered()` method with pagination, time window, sender, action_type, confidence range filters
- Pattern uses `const ACTION_COLUMNS` for SELECT statements and `row_to_action()` for parsing

**DecisionRepository**: Same file
- Existing methods: `create`, `get_by_id`, `get_by_message_id`, `list`, `list_recent`
- Action detail page will need to JOIN with decisions to get rationale and decision_json

**ActionLinkRepository**: Same file
- `get_by_cause_action_id()` and `get_by_effect_action_id()` - needed to check if action has been undone

**Database Schema**: `server/migrations/001_initial.sql`
- `actions` table has: id, account_id, message_id, decision_id, action_type, parameters_json, status, error_message, executed_at, undo_hint_json, trace_id, timestamps, org_id, user_id
- Indexes: `actions_message_idx ON actions(message_id, created_at)`, `actions_status_idx ON actions(status, created_at)`
- May need additional index for time-based filtering: `actions_created_idx ON actions(org_id, user_id, created_at)`

**Status Transitions**: Enforced in `is_valid_transition()`:
- Queued → {Executing, Canceled, Rejected, ApprovedPending, Failed}
- Executing → {Completed, Failed, Canceled}
- ApprovedPending → {Queued, Canceled, Rejected}
- Terminal: Completed, Failed, Canceled, Rejected

**Undo Pattern**: Actions track `undo_hint_json` with inverse action info. `action_links` table tracks relationships with `undo_of` relation type.

#### SvelteKit Frontend Structure

**Directory Structure**: `web/src/`
- `routes/` - SvelteKit pages
- `lib/api/` - API client and remote functions
- `lib/components/ui/` - shadcn-svelte components
- `lib/types/generated/` - Auto-generated TS types from Rust

**Layout**: `web/src/routes/+layout.svelte`
- Sidebar with navigation items already includes `/actions` route
- Uses `Sidebar` components from shadcn-svelte
- Dark mode toggle via `mode-watcher`
- `Toaster` component available for notifications

**API Client**: `web/src/lib/api/client.ts`
- `get<T>(path, options?)`, `post<T>(path, body?, options?)`
- `buildQueryString(params)` - Builds query strings from objects
- `ApiError` class with status, statusText, body
- Backend URL via `BACKEND_URL` env var (defaults to `http://127.0.0.1:17800`)

**Remote Functions Pattern**: `web/src/lib/api/example.remote.ts`
- Already has `listActions`, `getAction`, `undoAction` examples that can be moved/adapted
- Uses `query()` from `$app/server` for read operations
- Uses `command()` from `$app/server` for write operations
- Valibot schemas for input validation (e.g., `v.picklist([...] as const satisfies readonly ActionStatus[])`)

**Generated Types**: `web/src/lib/types/generated/`
- `Action`: id, org_id, user_id, account_id, message_id, decision_id, action_type, parameters_json, status, error_message, executed_at, undo_hint_json, trace_id, timestamps
- `ActionStatus`: 'queued' | 'executing' | 'completed' | 'failed' | 'canceled' | 'rejected' | 'approved_pending'
- `Decision`: id, account_id, message_id, source, decision_json, action_type, confidence, needs_approval, rationale, telemetry_json, timestamps
- `PaginatedResponse<T>`: items, total, limit, offset, has_more
- `MessageSummary`: id, account_id, subject, snippet, from_email, from_name, received_at, labels

**UI Components Available** (in `lib/components/ui/`):
- `table/` - Table, TableHeader, TableBody, TableRow, TableCell, etc.
- `pagination/` - Pagination component with page binding
- `select/` - Select dropdowns for filters
- `input/` - Input fields for text search
- `button/` - Button with variants (default, destructive, outline, secondary, ghost, link)
- `badge/` - Status badges
- `empty/` - Empty state with title/description
- `card/` - Card layouts
- `spinner/` - Loading spinner
- `sonner/` - Toast notifications

**Svelte Patterns** (from CLAUDE.md):
- Use Svelte 5 runes (`$state`, `$derived`, `$effect`)
- `$derived` for simple expressions, `$derived.by(() => {})` for complex logic
- Use `href` instead of `onClick` for navigation
- URL params via `page.url.searchParams`

#### API Response Types (New types needed)

**ActionListItem** (enriched for list view):
```typescript
{
  id: string;
  account_id: string;
  action_type: string;
  status: ActionStatus;
  confidence: number | null;
  created_at: string;
  executed_at: string | null;
  // Joined from message
  message_subject: string | null;
  message_from_email: string | null;
  message_from_name: string | null;
  // Computed
  can_undo: boolean;
}
```

**ActionDetail** (enriched for detail view):
```typescript
{
  ...Action;
  // Joined from decision
  decision: Decision | null;
  // Joined from message
  message_subject: string | null;
  message_from_email: string | null;
  message_from_name: string | null;
  message_snippet: string | null;
  // Computed
  can_undo: boolean;
  gmail_link: string | null;
  has_been_undone: boolean;
}
```

#### Undo Implementation

**Undo Eligibility Criteria**:
1. Action status must be `Completed`
2. `undo_hint_json` must contain valid inverse action info
3. No existing `action_links` with `undo_of` relation pointing to this action

**Undo Flow**:
1. POST `/api/actions/{id}/undo` validates eligibility
2. Creates new `Action` with inverse operation from `undo_hint_json`
3. Creates `ActionLink` with `cause_action_id=original`, `effect_action_id=undo_action`, `relation_type=undo_of`
4. Enqueues job to execute the undo action
5. Returns `{undoActionId, status: 'queued', message}`

### Risks & Constraints

1. **Performance with Large Datasets**: The actions list may grow large. The time-based index `(org_id, user_id, created_at DESC)` should be added to support efficient pagination. Consider adding LIMIT defaults and maximum page sizes.

2. **Filter Complexity**: Multiple optional filters in SQL requires careful query building. Pattern in codebase uses `(?N IS NULL OR column = ?N)` which works but may not use indexes optimally for all filter combinations.

3. **Message/Decision JOINs**: Action detail needs data from `messages` and `decisions` tables. Should use single query with JOINs rather than multiple roundtrips.

4. **Confidence as Percentage**: The `confidence` field is stored as 0.0-1.0 float but should display as percentage (0-100%) in UI.

5. **Time Window Parsing**: Need to parse relative time strings like "24h", "7d", "30d" into datetime comparisons. Could use a simple regex + calculation approach.

6. **Gmail Link Construction**: Need account's email and message's `provider_message_id` to construct Gmail deep link: `https://mail.google.com/mail/u/{email}/#inbox/{provider_message_id}`

7. **Polling Memory Leaks**: The `$effect` for polling must properly clear the interval on component destroy. Should also pause when tab is hidden using `document.visibilityState`.

8. **Type Generation**: After adding new Rust types (ActionListItem, ActionDetail, filter params), must run `cargo test --test export_ts_types -- --ignored` to regenerate TypeScript types.

9. **Multi-tenancy**: All queries must include `org_id` and `user_id` parameters. Currently hardcoded to defaults (1, 1) but pattern should be followed consistently.

10. **Sender Filter Ambiguity**: Resolved - using smart matching: if input contains `@`, match exact email; otherwise, match domain suffix (e.g., "example.com" matches "user@example.com" and "user@sub.example.com").

## Expected Behavior/Outcome

### Actions List Page (`/actions`)
- Displays a paginated table of all actions taken by the system
- Table columns: Timestamp, Subject, Sender, Action Type, Confidence (%), Status
- Each row is clickable, navigating to the action detail page
- Filters above the table allow narrowing results by:
  - Time window (preset buttons: 24h, 7d, 30d, All)
  - Account (dropdown of connected accounts)
  - Sender (text input - smart match: `@` = exact email, else domain suffix)
  - Action type (multi-select dropdown)
  - Status (checkboxes for each status)
  - Confidence range (min/max number inputs, 0-100%)
- Pagination controls below table with page navigation and items-per-page selector
- Filters persist in URL search params (shareable/bookmarkable)
- Auto-refreshes every 10 seconds (pauses when tab hidden)
- Shows loading spinner during data fetch
- Shows empty state when no results match filters

### Action Detail Page (`/actions/[id]`)
- Header with action type badge and status badge
- Summary card showing: timestamp, confidence, message subject, sender
- Rationale section displaying the LLM's reasoning (if available)
- Collapsible "Decision JSON" section with prettified JSON
- "Open in Gmail" button linking to the original message
- Undo button (visible only if action is undoable):
  - Shows loading state during undo request
  - Displays success toast and refreshes on completion
  - Displays error toast on failure
- Shows if action has already been undone (with link to undo action)

## Key Findings

### Product & User Story
As an Ashford user, I want to view a history of all automated email actions so I can monitor what the system has done, verify it's working correctly, and undo actions when needed.

### Design & UX Approach
- Use existing shadcn-svelte Table, Pagination, Select, Input, Button, Badge components
- Filter bar horizontally above table with responsive layout
- Status badges use semantic colors (green=completed, yellow=queued, red=failed)
- Confidence displayed as percentage with color coding (red <50%, yellow 50-80%, green >80%)
- Detail page uses Card components for logical groupings
- Toast notifications for undo success/failure via Sonner

### Technical Plan & Risks
- **Backend**: Add 3 new Axum routes, 1 new repository method, 2 new API types
- **Frontend**: Create 2 new pages, 1 remote function file, adapt example patterns
- **Risk**: Large result sets - mitigated by pagination with max 100 items per page
- **Risk**: Polling performance - mitigated by pausing on hidden tab

### Pragmatic Effort Estimate
This is a medium-complexity feature with well-established patterns in the codebase.

## Acceptance Criteria

### Functional Criteria
- [ ] User can view paginated list of actions at `/actions`
- [ ] User can filter actions by time window (24h, 7d, 30d, all)
- [ ] User can filter actions by account
- [ ] User can filter actions by sender (email or domain)
- [ ] User can filter actions by action type
- [ ] User can filter actions by status
- [ ] User can filter actions by confidence range
- [ ] Filters persist in URL and survive page refresh
- [ ] User can click an action row to view details at `/actions/[id]`
- [ ] Detail page shows action type, status, confidence, timestamp
- [ ] Detail page shows message subject and sender
- [ ] Detail page shows decision rationale when available
- [ ] Detail page shows collapsible decision JSON
- [ ] User can click "Open in Gmail" to view original message
- [ ] User can undo completed actions that have undo hints
- [ ] Undo button is hidden for non-undoable actions
- [ ] Success/error feedback shown after undo attempt

### UX Criteria
- [ ] List page shows loading spinner during initial load
- [ ] List page shows empty state when no actions match filters
- [ ] List auto-refreshes every 10 seconds when tab is visible
- [ ] Pagination shows total count and current page
- [ ] Status badges use semantic colors
- [ ] Confidence displays as percentage with color coding
- [ ] Undo button shows loading state during request
- [ ] Toast notifications appear for undo success/failure

### Technical Criteria
- [ ] GET `/api/actions` returns `PaginatedResponse<ActionListItem>` with filters
- [ ] GET `/api/actions/{id}` returns `ActionDetail` with joined data
- [ ] POST `/api/actions/{id}/undo` returns undo job info or error
- [ ] All endpoints return appropriate HTTP status codes (200, 400, 404, 500)
- [ ] All endpoints scope queries by org_id and user_id
- [ ] New Rust types derive `TS` for TypeScript generation
- [ ] Repository tests cover filtered list queries
- [ ] API handler tests verify filter parameter handling

## Dependencies & Constraints

### Dependencies
- Existing `ActionRepository`, `DecisionRepository`, `ActionLinkRepository` in ashford-core
- Existing `PaginatedResponse<T>` type
- shadcn-svelte Table, Pagination, Select, Input, Button, Badge components
- Remote functions pattern from `example.remote.ts`
- Toaster component for notifications

### Technical Constraints
- Must use existing `org_id=1, user_id=1` defaults (single-user system)
- Must follow existing repository patterns (column constants, row_to_* functions)
- Must regenerate TypeScript types after adding Rust types
- Pagination limited to max 100 items per page for performance
- Gmail link requires `provider_message_id` from messages table

## Implementation Notes

### Recommended Approach

**Phase 1: Backend API**
1. Create `server/crates/ashford-server/src/api/mod.rs` with router setup
2. Create `server/crates/ashford-server/src/api/actions.rs` with handlers
3. Add `list_filtered()` method to `ActionRepository` with:
   - Time window filter (created_at >= now - duration)
   - Account filter (account_id = ?)
   - Sender filter (smart match on messages.from_email)
   - Action type filter (action_type IN (?))
   - Status filter (status IN (?))
   - Confidence range (decisions.confidence BETWEEN ? AND ?)
   - Pagination (LIMIT/OFFSET with COUNT for total)
4. Add `get_detail()` method that JOINs action + decision + message
5. Create `ActionListItem` and `ActionDetail` types with ts-rs derives
6. Implement undo endpoint with eligibility validation

**Phase 2: Frontend Pages**
1. Create `web/src/lib/api/actions.remote.ts` with queries/commands
2. Create `web/src/routes/actions/+page.svelte` with:
   - Filter bar component
   - Table with clickable rows
   - Pagination controls
   - Polling effect with visibility check
3. Create `web/src/routes/actions/[id]/+page.svelte` with:
   - Summary card
   - Rationale section
   - Collapsible JSON viewer
   - Gmail link button
   - Conditional undo button

### Potential Gotchas

1. **SQL JOIN complexity**: The filtered list query joins actions → messages → decisions. Use LEFT JOINs since decision_id can be NULL.

2. **Sender smart matching**: Backend needs to detect `@` in sender param and use either `from_email = ?` or `from_email LIKE '%@' || ?` for domain matching.

3. **Confidence source**: Confidence comes from the `decisions` table, not `actions`. Need JOIN even for list view.

4. **Time window parsing**: Parse strings like "24h", "7d", "30d" in Rust. Use regex or simple suffix matching (h/d) with numeric prefix.

5. **Empty filter arrays**: When status or action_type filters are empty arrays, should return all (not none). Use `(?1 IS NULL OR ...)` pattern.

6. **Polling cleanup**: The `$effect` must return a cleanup function that clears the interval. Also add `visibilitychange` event listener.

7. **URL param serialization**: Arrays (status, action_type) need to be serialized as comma-separated or repeated params. Use consistent approach.

### Conflicting, Unclear, or Impossible Requirements

None identified - all requirements are clear and implementable with existing patterns

Backend API Implementation (Tasks 1-5) completed:

## Task 1: API Types and Repository Methods
Created ActionListItem, ActionDetail, and UndoActionResponse types in server/crates/ashford-core/src/api/types.rs with ts-rs derives for TypeScript generation. Added ActionListFilter struct for query parameters.

Added two new repository methods to ActionRepository in server/crates/ashford-core/src/decisions/repositories.rs:
- list_filtered(): Complex query supporting time_window, account_id, sender (smart match - @ for exact email, else domain suffix), action_type[], status[], confidence range, and pagination with LIMIT/OFFSET
- get_detail(): JOINs actions with decisions, messages, and accounts tables to return full ActionDetail including computed fields (can_undo, gmail_link, has_been_undone)

TypeScript types generated in web/src/lib/types/generated/: ActionListItem.ts, ActionDetail.ts, UndoActionResponse.ts, and updated index.ts exports.

## Task 2: API Module and Router
Created server/crates/ashford-server/src/api/mod.rs with api_router() function returning Axum Router with /actions routes.
Created server/crates/ashford-server/src/api/actions.rs with all handler implementations.
Updated server/crates/ashford-server/src/main.rs to nest API router under /api prefix.

## Task 3: GET /api/actions Endpoint
Implemented list_actions handler in actions.rs accepting query parameters:
- time_window (24h/7d/30d) parsed to DateTime
- account_id, sender (smart matching), action_type (comma-separated), status (comma-separated)
- min_confidence, max_confidence (0.0-1.0 range)
- limit (default 20, max 100), offset (default 0)
Returns PaginatedResponse<ActionListItem> with total count, has_more flag.

## Task 4: GET /api/actions/{id} Endpoint
Implemented get_action handler using get_detail() repository method.
Returns ActionDetail with:
- Full action data plus joined decision and message fields
- Computed can_undo (status=Completed, has undo_hint with inverse_action, no existing undo link)
- Computed gmail_link (from account email + provider_message_id)
- Computed has_been_undone flag
- undo_action_id linking to the undo action if one exists

## Task 5: POST /api/actions/{id}/undo Endpoint
Implemented undo_action handler with:
- Eligibility validation (status=Completed, has undo_hint_json.inverse_action, not already undone)
- Creates new Action from undo_hint_json inverse_action data
- Creates ActionLink with cause_action_id=original, effect_action_id=undo_action, relation_type=undo_of
- Enqueues job via JobQueue with JOB_TYPE_ACTION_GMAIL
- Returns UndoActionResponse with undo_action_id, status='queued', message
- Proper error handling with 400/404/500 status codes

## Bug Fix During Review
Fixed critical ActionLink semantics bug where cause_action_id and effect_action_id were inverted. The plan specified 'cause=original, effect=undo' but implementation had them reversed. This would have caused has_been_undone to always return false. Corrected in actions.rs lines 374-380.

## Verification
All 581 tests pass including 11 new API tests. Build compiles successfully.

Frontend Implementation for Actions List Page (Tasks 6-9) completed:

## Task 6: Create actions remote functions

Created web/src/lib/api/actions.remote.ts with three remote functions:
- listActions: Query function with Valibot schema for all filter params (timeWindow, accountId, sender, actionTypes, statuses, minConfidence, maxConfidence, limit, offset). Handles conversion of confidence percentages (0-100) to decimals (0.0-1.0) and array-to-comma-separated-string conversion for actionTypes and statuses.
- getAction: Query function to fetch action details by ID
- undoAction: Command function to trigger action undo via POST /api/actions/{id}/undo

Created web/src/lib/api/actions.constants.ts with shared constants:
- ACTION_STATUSES array with all valid status values
- TIME_WINDOWS array with valid time window options (24h, 7d, 30d)

## Task 7: Build actions list page with table

Created web/src/routes/actions/+page.svelte with:
- shadcn Table component with columns: Timestamp (formatted with date-fns), Subject (truncated to 50 chars), Sender (email with optional name), Action Type (capitalized badge), Confidence (percentage with color coding: red <50%, yellow 50-80%, green >80%), Status (Badge with semantic colors)
- Clickable table rows using anchor tags that navigate to /actions/[id]
- Spinner component displayed during initial load
- Empty state component when no results match filters

## Task 8: Add action filters UI

Added comprehensive filter bar above the table:
- Time window toggle buttons (24h/7d/30d/All) using ToggleGroup component
- Account Select dropdown that fetches accounts from GET /api/accounts (required creating new backend endpoint and accounts.remote.ts)
- Sender Input with placeholder 'Email or domain'
- Action type multi-select using DropdownMenu with checkboxes
- Status checkbox group for filtering by action status
- Confidence min/max number inputs (0-100 range)
- URL param persistence: filters read from URL on mount, URL updated on filter changes using replaceState for shareable/bookmarkable URLs

Bug fix during review: The 'All' time window button was passing empty string to Valibot schema which expected undefined. Fixed handleTimeWindowChange to convert empty string to undefined.

## Task 9: Add pagination and polling

Added pagination and auto-refresh:
- Pagination component below table with page navigation
- Items-per-page Select with options (10/25/50)
- Total count display showing 'Showing X-Y of Z actions'
- 10-second polling interval using $effect with proper cleanup
- visibilitychange event listener to pause polling when tab is hidden and resume when visible

## Additional Backend Work

Created GET /api/accounts endpoint (server/crates/ashford-server/src/api/accounts.rs) to support the account filter dropdown. Returns Vec<AccountSummary> with OAuth credentials stripped for security.

## Files Created/Modified

New files:
- web/src/lib/api/actions.remote.ts
- web/src/lib/api/actions.constants.ts
- web/src/lib/api/accounts.remote.ts
- web/src/lib/api/actions.remote.spec.ts (14 unit tests)
- web/src/routes/actions/+page.svelte
- web/src/routes/actions/[id]/+page.svelte (placeholder for Tasks 10-11)
- server/crates/ashford-server/src/api/accounts.rs

Modified files:
- web/svelte.config.js (enabled experimental.remoteFunctions)
- server/crates/ashford-server/src/api/mod.rs (added accounts router)

## Verification

All 581 backend tests pass, 33 frontend server tests pass, build succeeds, lint passes, typecheck passes.

## Tasks 10-11: Action Detail Page and Undo Functionality

### Task 10: Build action detail page

Implemented the complete action detail page at `web/src/routes/actions/[id]/+page.svelte` with the following features:

**Header Section:**
- Action type formatted as title with proper capitalization (e.g., 'Apply Label', 'Mark Read')
- Status Badge using semantic colors (default for completed, secondary for queued/executing, destructive for failed)
- Additional 'Undone' outline badge shown when action has been undone

**Summary Card:**
- Timestamp formatted with weekday, month, day, year, and time
- Confidence percentage with color coding (green >80%, yellow 50-80%, red <50%)
- Message subject (with 'No subject' fallback)
- Sender email and name display
- Executed at timestamp (conditional)
- Error message display (conditional, in red)

**Rationale Section:**
- Displays decision.rationale text
- Shows 'No rationale provided' italic text when null

**Collapsible Sections:**
- Decision JSON: Collapsible card showing prettified JSON of decision_json
- Action Parameters: Collapsible card showing prettified JSON of parameters_json
- Uses ChevronDown/ChevronRight icons to indicate open/closed state

**Gmail Integration:**
- 'Open in Gmail' button with external link icon
- Only shown when gmail_link is present
- Opens in new tab with proper rel attributes

**Undo Status:**
- Shows 'Undone by [link]' when has_been_undone is true
- Links to /actions/{undo_action_id} with truncated ID display

**Navigation:**
- Back to Actions link at top of page

**State Management:**
- Loading state with centered Spinner
- Error state with retry button in Card format
- Data fetched via getAction remote function

### Task 11: Implement undo functionality

**Undo Button:**
- Only visible when action.can_undo is true
- Positioned in action buttons row alongside Gmail button
- Uses Undo icon from lucide-svelte

**Loading State:**
- Button disabled during undo operation
- Shows Spinner and 'Undoing...' text during request

**Success Handling:**
- Calls undoAction remote function with actionId
- Shows toast.success('Action undo queued') on success
- Automatically refreshes page data to show updated state

**Error Handling:**
- Extracts descriptive error message from API response body
- Falls back to generic error message if body not available
- Shows toast.error(message) with specific backend error text (e.g., 'Action has already been undone', 'Cannot undo action with status: Failed')

### Additional Changes

**New File: `web/src/lib/api/errors.ts`**
- Extracted ApiError class from client.ts to separate file
- Allows importing ApiError in browser code without triggering server-only module errors
- client.ts re-exports ApiError for backwards compatibility

**Helper Functions: `web/src/routes/actions/[id]/helpers.ts`**
- Extracted formatting functions for testability
- Functions: formatTimestamp, formatActionType, formatConfidence, getConfidenceColor, getStatusVariant, getStatusLabel, formatSender, formatJson

**Unit Tests: `web/src/routes/actions/[id]/helpers.spec.ts`**
- 26 unit tests covering all helper functions
- Tests edge cases for null values, various status types, confidence thresholds

**Bug Fix: `web/src/lib/types/generated/LogicalCondition.ts`**
- Fixed pre-existing TypeScript error by adding missing LeafCondition import

### Verification

- Build: Passes
- TypeCheck (svelte-check): 0 errors, 0 warnings
- Lint (prettier + eslint): Passes
- Server Tests: 59 tests pass (including 26 new helper tests)

## Autofix: Code Review Issues Resolution

### Issue 1: Duplicate Formatting Functions (Fixed)

Created a new shared formatting module at `web/src/lib/formatting/actions.ts` containing all the shared formatting functions that were previously duplicated between the actions list page and detail page helpers:

- `formatTimestampShort()` - Compact timestamp format for list views (month, day, hour, minute)
- `formatTimestampFull()` - Full timestamp format for detail views (weekday, month, day, year, hour, minute, second)
- `formatActionType()` - Converts snake_case to Title Case
- `formatConfidence()` - Converts 0.0-1.0 to percentage string
- `getConfidenceColor()` - Returns color class based on confidence level
- `getStatusVariant()` - Returns badge variant for action status
- `getStatusLabel()` - Returns display label for action status
- `formatSender()` - Formats sender with name and email
- `formatJson()` - Pretty-prints JSON

The detail page helpers file (`web/src/routes/actions/[id]/helpers.ts`) now re-exports all functions from the shared module for backwards compatibility with existing tests. A deprecated `formatTimestamp` alias points to `formatTimestampFull` to maintain compatibility.

The list page (`web/src/routes/actions/+page.svelte`) was updated to import from the shared module and uses `formatTimestampShort` for the compact list display. It also keeps a local `formatSenderShort` function for the more compact sender display in the list (just name or email, not both).

A comprehensive test file was added at `web/src/lib/formatting/actions.spec.ts` with 29 tests covering all shared formatting functions.

### Issue 2: Sender Input Debounce (Fixed)

Added debouncing to the sender input in `web/src/routes/actions/+page.svelte` to prevent API calls on every keystroke:

- Added `senderDebounced` state variable to hold the debounced value
- Created a `$effect` that watches the raw `sender` value and updates `senderDebounced` after a 300ms delay
- The effect returns a cleanup function that clears the timeout to prevent stale updates
- Updated the filter effect to watch `senderDebounced` instead of the raw `sender` value
- Updated `fetchActions()` and `updateUrl()` to use `senderDebounced`
- Updated `clearFilters()` to immediately reset `senderDebounced` (not just the raw `sender`) to avoid stale values

### Verification

All 88 frontend unit tests pass, all 581 backend tests pass, TypeScript check passes with 0 errors and 0 warnings, and the build succeeds.

Repaired review issues for the actions history feature. In ashford-core's ActionRepository.list_filtered I rebuilt the WHERE/parameter construction to use sequential '?' placeholders and only push params when a filter is present, preventing LIMIT/OFFSET misbinding when optional filters are absent. Confidence filtering now enforces non-null decision confidence whenever a range is supplied. Added regression test action_list_filtered_handles_defaults_and_confidence_range to cover the default path and to confirm actions with NULL confidence are excluded when min/max confidence are set. In the server actions API, undo_action now validates inverse_action is a non-empty string before proceeding and returns a BAD_REQUEST when invalid. The endpoint now treats action link creation as a critical step: on failure it cancels the newly created undo action and returns an internal error instead of silently continuing. On the frontend, removed the extra 0-100 -> 0-1 scaling in actions.remote.ts so the backend handles conversion once; updated actions.remote.spec.ts expectations accordingly. Tests executed: cargo test -p ashford-core action_list_filtered_handles_defaults_and_confidence_range (server) and pnpm vitest run src/lib/api/actions.remote.spec.ts (web).
