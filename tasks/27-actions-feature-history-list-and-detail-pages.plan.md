---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Actions Feature: History List and Detail Pages"
goal: Build the actions history page with filters and action detail page with
  undo support, including all required Rust API endpoints
id: 27
uuid: 18da2db8-7523-467d-9f54-5f8a73896df7
generatedBy: agent
status: pending
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
updatedAt: 2025-12-04T18:26:48.714Z
progressNotes: []
tasks:
  - title: Create API types and repository methods
    done: false
    description: "Add ActionListItem and ActionDetail types to ashford-core with
      ts-rs derives. Add list_filtered() method to ActionRepository with:
      time_window, account_id, sender (smart match), action_type[], status[],
      confidence range, pagination. Add get_detail() method that JOINs action +
      decision + message. Run type generation."
  - title: Create actions API module and router
    done: false
    description: Create server/crates/ashford-server/src/api/mod.rs and actions.rs.
      Set up Axum router with /api/actions routes. Update main.rs to nest the
      API router under /api prefix.
  - title: Implement GET /api/actions endpoint
    done: false
    description: "Create list_actions handler accepting query params: time_window
      (24h/7d/30d), account_id, sender, action_type (comma-separated), status
      (comma-separated), min_confidence, max_confidence, limit (default 20, max
      100), offset. Parse time_window to datetime. Return
      PaginatedResponse<ActionListItem>."
  - title: Implement GET /api/actions/{id} endpoint
    done: false
    description: Create get_action handler using get_detail() repository method.
      Return ActionDetail with joined decision/message data, computed can_undo
      (status=Completed, has undo_hint, no existing undo link), gmail_link
      (constructed from account email + provider_message_id), has_been_undone
      flag.
  - title: Implement POST /api/actions/{id}/undo endpoint
    done: false
    description: "Create undo_action handler: validate can_undo eligibility, create
      new Action from undo_hint_json, create ActionLink with undo_of relation,
      enqueue job via JobQueue. Return {undo_action_id, status: 'queued',
      message} or 400/404 errors."
  - title: Create actions remote functions
    done: false
    description: Create web/src/lib/api/actions.remote.ts. Add listActions query
      with Valibot schema for all filter params. Add getAction query by ID. Add
      undoAction command. Use generated ActionListItem, ActionDetail types.
  - title: Build actions list page with table
    done: false
    description: "Create web/src/routes/actions/+page.svelte. Use shadcn Table with
      columns: Timestamp (formatted), Subject (truncated), Sender, Action Type,
      Confidence (percentage with color), Status (Badge). Rows link to
      /actions/[id]. Show Spinner during load, Empty state when no results."
  - title: Add action filters UI
    done: false
    description: "Add filter bar above table: time window buttons (24h/7d/30d/All),
      account Select dropdown, sender Input (placeholder: 'Email or domain'),
      action type multi-Select, status checkbox group, confidence min/max
      Inputs. Read initial values from URL params, update URL on change."
  - title: Add pagination and polling
    done: false
    description: Add Pagination component below table bound to page state. Add
      items-per-page Select (10/25/50). Show total count. Add $effect for 10s
      polling with visibilitychange listener to pause when hidden. Return
      cleanup function to clear interval.
  - title: Build action detail page
    done: false
    description: Create web/src/routes/actions/[id]/+page.svelte. Header with
      action_type Badge and status Badge. Card with timestamp, confidence %,
      subject, sender. Rationale section (or 'No rationale' if null).
      Collapsible 'Decision JSON' with pre-formatted JSON. 'Open in Gmail'
      Button (href=gmail_link). Show 'Undone by [link]' if has_been_undone.
  - title: Implement undo functionality
    done: false
    description: "Add Undo button to detail page (visible only if can_undo). On
      click: set loading state, call undoAction command, on success show
      toast('Action undo queued') and refresh data, on error show
      toast.error(message). Button disabled during loading with Spinner."
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
