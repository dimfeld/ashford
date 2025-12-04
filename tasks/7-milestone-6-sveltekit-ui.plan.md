---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 6: SvelteKit UI"
goal: Build SvelteKit web application with actions history, rules management,
  and settings pages
id: 7
uuid: 5a952985-9ed4-4035-8fef-479f3f7e2010
generatedBy: agent
status: in_progress
priority: high
container: true
temp: false
dependencies:
  - 30
  - 27
  - 28
  - 29
parent: 1
references:
  "1": 076d03b1-833c-4982-b0ca-1d8868d40e31
  "27": 18da2db8-7523-467d-9f54-5f8a73896df7
  "28": 22994868-219e-4c67-affd-53b36c2248f7
  "29": 1a24434a-f5be-45ff-97bd-d50026e0869e
  "30": 64c00252-4c84-4b02-8fc2-68559edf27a9
issue: []
pullRequest: []
docs:
  - docs/web_ui.md
planGeneratedAt: 2025-11-29T01:23:12.568Z
promptsGeneratedAt: 2025-11-29T01:23:12.568Z
createdAt: 2025-11-29T01:21:27.056Z
updatedAt: 2025-12-03T10:23:52.295Z
progressNotes: []
tasks:
  - title: Create Rust API client
    done: false
    description: Build typed HTTP client for Rust backend at localhost:17801. Create
      fetch wrapper with error handling. Define TypeScript types matching Rust
      API responses. Use the `ts_rs` crate to generate TypeScript types from
      Rust
  - title: Implement remote functions pattern
    done: false
    description: Set up query() and command() helpers in *.remote.ts files.
      Configure SvelteKit for transparent client-server RPC.
  - title: Build actions list page
    done: false
    description: "Create /actions route with filterable table: time window, account,
      sender, action type, status, confidence. Paginated results with periodic
      polling refresh."
  - title: Add action filters UI
    done: false
    description: "Filter controls: date range picker, account dropdown,
      sender/domain search, action type multi-select, status checkboxes,
      confidence slider."
  - title: Build action detail page
    done: false
    description: "Create /actions/[id] route showing: decision JSON, rationale,
      before/after state, approval status, trace link, Gmail link. Add undo
      button."
  - title: Implement undo action
    done: false
    description: Undo button calls undoAction command. Show loading state, handle
      errors, refresh action detail on completion.
  - title: Build rules list page
    done: false
    description: Create /rules route with tabs for Deterministic and LLM rules. Show
      name, scope, enabled state, conditions summary. Enable/disable toggle.
  - title: Add rule reordering
    done: false
    description: Drag-and-drop or up/down buttons to change rule priority. Persist
      new priorities via PATCH endpoint.
  - title: Create rule edit forms
    done: false
    description: Forms for creating/editing deterministic rules (conditions builder,
      action selector) and LLM rules (rule_text textarea). Validate before save.
  - title: Build settings page
    done: false
    description: "Create /settings route showing read-only configuration: accounts,
      model settings, Discord channel, Gmail config. Redact secrets."
  - title: Add layout and navigation
    done: false
    description: "Create app shell with sidebar navigation: Actions, Rules, Rules
      Assistant, Settings. Add breadcrumbs, page titles."
tags:
  - frontend
  - sveltekit
  - ui
---

SvelteKit web application:
- Remote functions pattern for Rust API communication
- Actions history page (/actions) with filters
- Action detail page (/actions/:id) with undo support
- Rules list page (/rules) with tabbed deterministic/LLM rules
- Rule create/edit forms
- Settings page (/settings) with redacted secrets view
- Periodic polling for updates

## Research

### Summary
- The SvelteKit project exists at `web/` with a comprehensive UI component library (shadcn-svelte) already installed, but no application pages implemented yet.
- The Rust backend at `server/` has extensive domain models and repository layers but only one API endpoint (`/healthz`) - all other endpoints documented in `docs/web_ui.md` need to be built.
- This is a full-stack feature requiring both Rust API endpoints and SvelteKit pages to be implemented together.
- TypeScript type generation from Rust types (ts_rs) is not yet set up.

### Findings

#### Existing SvelteKit UI Structure

**Project Location**: `web/`

**Technology Stack**:
- **Svelte 5** (v5.43.8) with runes syntax (`$state`, `$derived`, `$props`)
- **SvelteKit** (v2.48.5) with Node adapter
- **Tailwind CSS 4** (@tailwindcss/vite 4.1.17) with custom design tokens
- **shadcn-svelte** component library with 35+ components
- **bits-ui** (v2.14.4) for headless primitives
- **lucide-svelte** for icons
- **mode-watcher** for dark mode support
- **layerchart** for data visualization
- **svelte-sonner** for toast notifications
- **vaul-svelte** for drawer component

**Available UI Components** (`web/src/lib/components/ui/`):
- Layout: `sidebar/`, `card/`, `separator/`, `sheet/`, `drawer/`
- Forms: `input/`, `textarea/`, `select/`, `checkbox/`, `switch/`, `slider/`, `button/`, `toggle/`, `toggle-group/`
- Feedback: `alert/`, `alert-dialog/`, `badge/`, `spinner/`, `skeleton/`, `sonner/` (toasts)
- Navigation: `breadcrumb/`, `tabs/`, `dropdown-menu/`, `context-menu/`, `command/`
- Data Display: `table/`, `pagination/`, `chart/`, `avatar/`, `tooltip/`
- Overlay: `dialog/`, `sheet/`, `collapsible/`

**Sidebar System**: Full-featured with mobile responsiveness, collapse states, and keyboard shortcuts (Cmd/Ctrl+B). Components include `SidebarProvider`, `SidebarContent`, `SidebarMenu`, `SidebarMenuButton`, `SidebarMenuItem`, `SidebarGroup`, etc.

**Current Routes**: Only root layout (`+layout.svelte`) and placeholder home page (`+page.svelte`) exist. No application routes implemented.

**Remote Functions Pattern**: Documented in `docs/svelte_remote_functions.md`. Uses `query()`, `command()`, and `form()` helpers from `$app/server` in `*.remote.ts` files. These execute server-side but can be called transparently from client code.

**Testing Setup**:
- Vitest (v4.0.10) with browser mode for component tests
- Playwright (v1.56.1) for E2E tests
- Split test projects: client (browser) and server (node)
- Only placeholder demo tests exist currently

**Package Manager**: pnpm

---

#### Rust Backend API Analysis

**Server Location**: `server/crates/ashford-server/src/main.rs`

**Current Endpoints**:
- `GET /healthz` - Health check (only endpoint implemented)

**All Other Endpoints Need Implementation** (per `docs/web_ui.md`):

**Actions API**:
- `GET /api/actions` - List actions with filters (timeWindow, account, sender, actionType, status, minConfidence, maxConfidence, page, limit)
- `GET /api/actions/{id}` - Get action detail
- `POST /api/actions/{id}/undo` - Enqueue undo job

**Rules API**:
- `GET /api/rules/deterministic` - List deterministic rules
- `GET /api/rules/llm` - List LLM rules
- `POST /api/rules/deterministic` - Create deterministic rule
- `PATCH /api/rules/deterministic/{id}` - Update deterministic rule
- `POST /api/rules/llm` - Create LLM rule
- `PATCH /api/rules/llm/{id}` - Update LLM rule

**Rules Assistant API** (for chat UI):
- `POST /api/rules/assistant/message` - Non-streaming chat
- `POST /api/rules/assistant/stream` - SSE streaming chat
- `POST /api/rules/assistant/apply` - Apply proposed rule changes

**Settings API**:
- `GET /api/settings` - Get configuration (secrets redacted)

**Server Configuration**: Port 17801, localhost-only, no authentication required.

---

#### Domain Models and Types

**Action** (`server/crates/ashford-core/src/decisions/types.rs`):
```rust
pub struct Action {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub decision_id: Option<String>,
    pub action_type: String,
    pub parameters_json: Value,
    pub status: ActionStatus,  // Queued, Executing, Completed, Failed, Canceled, Rejected, ApprovedPending
    pub error_message: Option<String>,
    pub executed_at: Option<DateTime<Utc>>,
    pub undo_hint_json: Value,
    pub trace_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Decision** (`server/crates/ashford-core/src/decisions/types.rs`):
```rust
pub struct Decision {
    pub id: String,
    pub org_id: i64,
    pub user_id: i64,
    pub account_id: String,
    pub message_id: String,
    pub source: DecisionSource,  // Llm, Deterministic
    pub decision_json: Value,
    pub action_type: Option<String>,
    pub confidence: Option<f64>,  // 0.0 to 1.0
    pub needs_approval: bool,
    pub rationale: Option<String>,
    pub telemetry_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**DeterministicRule** (`server/crates/ashford-core/src/rules/types.rs`):
```rust
pub struct DeterministicRule {
    pub id: String,
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,  // Global, Account, Sender, Domain
    pub scope_ref: Option<String>,
    pub priority: i64,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
    pub conditions_json: Value,
    pub action_type: String,
    pub action_parameters_json: Value,
    pub safe_mode: SafeMode,  // Default, AlwaysSafe, DangerousOverride
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**LlmRule** (`server/crates/ashford-core/src/rules/types.rs`):
```rust
pub struct LlmRule {
    pub id: String,
    pub org_id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub scope: RuleScope,
    pub scope_ref: Option<String>,
    pub rule_text: String,
    pub enabled: bool,
    pub metadata_json: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**Condition System** (`server/crates/ashford-core/src/rules/conditions.rs`):
```rust
pub enum Condition {
    Logical(LogicalCondition),  // And, Or, Not with children
    Leaf(LeafCondition),
}

pub enum LeafCondition {
    SenderEmail { value: String },
    SenderDomain { value: String },
    SubjectContains { value: String },
    SubjectRegex { value: String },
    HeaderMatch { header: String, pattern: String },
    LabelPresent { value: String },
}
```

**Action Types**: ApplyLabel, MarkRead, MarkUnread, Archive, Delete, Move, Star, Unstar, Forward, AutoReply, CreateTask, Snooze, AddNote, Escalate, None

**Action Danger Levels**: Safe (archive, label, read), Reversible (star, snooze), Dangerous (delete, forward, auto_reply, escalate)

**Repository Layer**: Comprehensive CRUD operations exist in `server/crates/ashford-core/src/decisions/repositories.rs` and `server/crates/ashford-core/src/rules/repositories.rs`.

---

#### TypeScript Type Generation

**Current State**: No ts_rs integration exists. TypeScript types will need to be either:
1. Manually created to match Rust types, or
2. ts_rs added to Rust types with `#[derive(TS)]` and exported to `web/src/lib/types/`

---

#### Testing Patterns

**Backend Testing**:
- Unit tests inline in source files with `#[cfg(test)] mod tests`
- Integration tests in `server/crates/ashford-core/tests/`
- Uses `tempfile` for temporary test databases
- Uses `wiremock` for mocking external HTTP APIs
- MockLLMClient in `server/crates/ashford-core/src/llm/mock.rs` for deterministic LLM testing
- Feature-gated `llm-integration` tests for real API calls

**Frontend Testing**:
- Vitest with browser mode for Svelte components
- Pattern: `*.svelte.{test,spec}.{js,ts}` for component tests
- Pattern: `*.{test,spec}.{js,ts}` for server/utility tests
- Playwright for E2E tests in `web/e2e/`

---

#### Related Files

- `docs/web_ui.md` - Full UI architecture documentation
- `docs/svelte_remote_functions.md` - Remote functions pattern guide
- `web/CLAUDE.md` - Frontend-specific coding guidelines (Svelte 5 runes, TypeScript rules)
- `server/CLAUDE.md` - Backend-specific guidelines (use `cargo add`)

### Risks & Constraints

1. **Backend API Gap**: The Rust backend has only a health endpoint. All documented API endpoints must be implemented before the UI can function. This is a significant amount of work.

2. **Type Synchronization**: Without ts_rs, TypeScript types must be manually kept in sync with Rust types. This is error-prone.

3. **Remote Functions Pattern**: The `*.remote.ts` pattern is relatively new in SvelteKit. Need to ensure it's properly configured and working.

4. **SSE Streaming**: The Rules Assistant requires SSE streaming from Rust through SvelteKit to the browser. This adds complexity.

5. **Condition Builder UI**: Building a visual condition editor for deterministic rules (AND/OR trees with multiple leaf conditions) is non-trivial UX.

6. **Rule Reordering**: Drag-and-drop or up/down priority reordering requires careful state management and optimistic updates.

7. **Pagination**: Need standardized pagination response types in Rust and corresponding UI patterns.

8. **No Existing Page Tests**: Frontend testing infrastructure exists but no real tests. New pages should include component tests.

9. **Single-User Assumption**: The system uses DEFAULT_ORG_ID=1 and DEFAULT_USER_ID=1 constants. API endpoints should use these constants.
