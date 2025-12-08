# Web UI Architecture

The web UI is built with SvelteKit and provides a user interface for viewing and managing email actions, rules, and settings.

## Architecture

### TypeScript Types

TypeScript types are automatically generated from Rust types using `ts-rs`. The generated types live in `web/src/lib/types/generated/` and include:

- **Action types**: `Action`, `ActionStatus`, `ActionLink`, `ActionLinkRelationType`
- **Action API types**: `ActionListItem` (enriched list item with message/confidence data), `ActionDetail` (full detail with computed fields), `UndoActionResponse`
- **Decision types**: `Decision`, `DecisionSource`
- **Rule types**: `DeterministicRule`, `LlmRule`, `RuleScope`, `SafeMode`, `Condition`
- **Summary types**: `AccountSummary`, `LabelSummary`, `MessageSummary` (optimized for API responses)
- **Pagination**: `PaginatedResponse<T>` generic wrapper

To regenerate types after changing Rust types:

```bash
cd server
cargo test --test export_ts_types -- --ignored
```

### API Client

The API client (`web/src/lib/api/client.ts`) provides typed fetch helpers for communicating with the Rust backend:

- `get<T>(path, options?)` - GET requests
- `post<T>(path, body?, options?)` - POST requests
- `patch<T>(path, body?, options?)` - PATCH requests
- `put<T>(path, body?, options?)` - PUT requests
- `del<T>(path, options?)` - DELETE requests
- `buildQueryString(params)` - Builds query strings from objects

The client includes:
- Automatic JSON serialization/deserialization
- Error handling with `ApiError` class (includes status code and response body)
- Configurable timeout (default 30s)
- Support for custom abort signals

Configure the backend URL via the `BACKEND_URL` environment variable (defaults to `http://127.0.0.1:17800`).

### Remote Functions Pattern

The SvelteKit application uses a **remote functions** pattern where:

1. **Frontend Components** (Svelte pages/components) call **SvelteKit remote functions**
2. **Remote Functions** (server-side TypeScript in `+page.server.ts` or `+server.ts` files) communicate with the **Rust backend API**
3. **Rust Backend** exposes REST endpoints on `http://127.0.0.1:17800`

```
┌─────────────────┐
│ Svelte Pages    │
│ (Browser)       │
└────────┬────────┘
         │ Call remote functions
         ▼
┌─────────────────┐
│ SvelteKit       │
│ Remote Functions│ (Server-side)
└────────┬────────┘
         │ HTTP REST calls
         ▼
┌─────────────────┐
│ Rust Backend    │
│ :17800          │
└─────────────────┘
```

## Pages

### 1. Actions History `/actions`

**Purpose:** View and filter historical email actions taken by the system.

**UI Components:**
- Filters:
  - Time window
  - Account
  - Sender/domain
  - Action type
  - Status (executed, queued, approved, rejected, failed)
  - Confidence range
- Table columns:
  - Timestamp
  - Subject
  - Sender
  - Action type
  - Confidence
  - Approval status
  - Trace link (to external tracing UI)

**Data Flow:**
1. Page component calls remote function (e.g., `load` function in `+page.server.ts`)
2. Remote function makes `GET /api/actions` request to Rust backend with filter parameters
3. Rust backend returns paginated list of actions
4. Remote function returns data to page component for rendering

### 2. Action Detail `/actions/:id`

**Purpose:** View detailed information about a specific action and optionally undo it.

**UI Components:**
- Decision JSON (prettified)
- Rationale and explanations
- Before/after state summary (labels, folder, read state)
- Approval/undo status
- Links:
  - "Open in Gmail"
  - "Open in Discord message" (optional)
- Optional undo button

**Data Flow:**
1. Page load: Remote function calls `GET /api/actions/{id}` to fetch action details
2. Undo action: Form action calls remote function → `POST /api/actions/{id}/undo` → enqueues undo job

### 3. Rules List `/rules`

**Purpose:** View, manage, and configure email processing rules.

**UI Components:**
- Tabbed layout:
  - Deterministic rules
  - LLM rules
- List each rule with:
  - Name, scope, enabled state
  - Conditions summary (for deterministic rules)
  - Example effect / description
- Controls:
  - Enable/disable toggle (optimistic UI with revert on error)
  - Reorder priorities (up/down arrow buttons for deterministic rules)
  - Edit button (navigates to form page)
  - Delete button with confirmation dialog

**Data Flow:**
1. Load rules: Remote function calls `GET /api/rules/deterministic` and `GET /api/rules/llm`
2. Toggle enable/disable: Form action calls `PATCH /api/rules/{type}/{id}`
3. Reorder: Uses `POST /api/rules/deterministic/swap-priorities` endpoint to atomically swap priorities between adjacent rules (prevents race conditions that could corrupt priority ordering)
4. Create/Edit: Form action calls `POST /api/rules/{type}` or `PATCH /api/rules/{type}/{id}`

### 4. Deterministic Rule Form `/rules/deterministic/[id]`

**Purpose:** Create or edit deterministic rules with structured conditions and actions.

**URL Patterns:**
- `/rules/deterministic/new` - Create a new deterministic rule
- `/rules/deterministic/[id]` - Edit an existing deterministic rule

**UI Components:**
- **Basic Information Card:**
  - Name (required text input)
  - Description (optional textarea)
  - Enabled toggle (Switch component)
  - Priority number input (lower = earlier execution)
- **Scope Card:**
  - Scope type dropdown: global, account, sender, domain
  - Scope reference input (conditional, shown for non-global scopes)
- **Conditions Card:**
  - ConditionBuilder component (see below)
- **Action Card:**
  - Action type dropdown with grouped options:
    - Safe: archive, apply_label, remove_label, mark_read, mark_unread, move, trash, restore, none
    - Reversible: star, unstar, snooze, add_note, create_task
    - Dangerous: delete, forward, auto_reply, escalate
  - Dynamic parameter fields based on action type:
    - `apply_label`/`remove_label`: Label dropdown selector
    - `forward`: Email address input
    - `auto_reply`: Body textarea
    - `snooze`: DateTime picker
  - Safe mode dropdown: default, always_safe, dangerous_override
- Save and Cancel buttons

**Data Flow:**
1. Page load (edit mode): Fetch rule via `GET /api/rules/deterministic/{id}`
2. Page load (new mode): Initialize empty form state
3. Save: Call `POST /api/rules/deterministic` (new) or `PATCH /api/rules/deterministic/{id}` (edit)
4. Validation: Client-side required field and action parameter validation

### 5. Condition Builder Component

**Location:** `web/src/lib/components/ConditionBuilder.svelte`

**Purpose:** Visual builder for deterministic rule matching conditions.

**UI Components:**
- **Logical Operator Toggle:** Match ALL (AND) or ANY (OR) conditions
  - Only shown when 2+ conditions exist
- **Condition Rows:** Each row contains:
  - Type dropdown:
    - `sender_email` - Exact or wildcard (*@domain.com) email match
    - `sender_domain` - Domain-only match
    - `subject_contains` - Case-insensitive substring match
    - `subject_regex` - Full regex pattern
    - `header_match` - Regex on specific header (two inputs: header name, pattern)
    - `label_present` - Check for Gmail label (dropdown populated from API)
  - Value input(s) appropriate to the condition type
  - Remove button (trash icon)
- **Add Condition Button**

**Output Format:**
```json
// Single condition
{ "type": "sender_domain", "value": "amazon.com" }

// Multiple conditions with AND
{
  "op": "and",
  "children": [
    { "type": "sender_domain", "value": "amazon.com" },
    { "type": "subject_contains", "value": "order" }
  ]
}
```

**Utility Functions:** `web/src/lib/components/condition-builder-utils.ts`
- `parseConditionsJson(json)` - Parse API conditions into UI rows
- `buildConditionsJson(operator, rows)` - Build API-compatible conditions from UI state
- `leafToRow(leaf)` / `rowToLeaf(row)` - Convert between API and UI representations
- `hasNestedLogicalConditions(children)` - Detect nested logical groups in conditions
- `flattenConditionToLeaves(condition)` - Recursively extract all leaf conditions from nested trees
- `isLeafCondition(obj)` / `isLogicalCondition(obj)` - Type guards for condition discrimination

**Nested Condition Handling:**

The condition builder UI supports only flat condition lists (one level of AND/OR). When editing a rule with nested logical conditions (created via API), the utility functions handle this gracefully:

1. **Detection**: `hasNestedLogicalConditions()` checks if any children are LogicalConditions
2. **Flattening**: Nested conditions are recursively flattened to extract all leaf conditions
3. **Warnings**: The component emits warnings via `onwarnings` callback when flattening occurs:
   - NOT conditions are converted to their positive equivalents (may invert logic)
   - Original nested structure will be lost when saving
   - Users are advised to use the API directly for complex nested conditions

This ensures existing rules with complex conditions can still be viewed and edited, while clearly communicating that the original structure cannot be preserved.

### 6. LLM Rule Form `/rules/llm/[id]`

**Purpose:** Create or edit LLM rules with natural language instructions.

**URL Patterns:**
- `/rules/llm/new` - Create a new LLM rule
- `/rules/llm/[id]` - Edit an existing LLM rule

**UI Components:**
- **Basic Information Card:**
  - Name (required text input)
  - Description (optional textarea)
  - Enabled toggle (Switch component)
- **Scope Card:**
  - Scope type dropdown: global, account, sender, domain
  - Scope reference input (conditional, shown for non-global scopes)
- **Rule Instructions Card:**
  - Large textarea for `rule_text`
  - Placeholder with example instructions
- Save and Cancel buttons

**Data Flow:**
1. Page load (edit mode): Fetch rule via `GET /api/rules/llm/{id}`
2. Page load (new mode): Initialize empty form state
3. Save: Call `POST /api/rules/llm` (new) or `PATCH /api/rules/llm/{id}` (edit)

### 7. Rules Assistant `/rules/assistant`

**Purpose:** Natural language interface for creating and modifying rules via AI assistance.

**UI Components:**
- Chat UI:
  - Conversation history in scrollable pane
  - Input textbox
- Each AI response shows:
  - Proposed rule changes as human-readable text
  - "Show JSON" toggle for technical details
- Buttons:
  - "Apply these changes"
  - "Discard"

**Data Flow:**
1. Send message (streaming):
   - Client calls SvelteKit endpoint `/api/assistant/stream`
   - SvelteKit endpoint forwards to `POST /api/rules/assistant/stream` on Rust backend
   - Rust backend streams SSE events back through SvelteKit to client
   - Client displays tokens as they arrive for real-time feedback
   - Final event includes proposed rule changes
2. Send message (non-streaming): Form action calls remote function → `POST /api/rules/assistant/message` → returns complete response with proposed rule changes
3. Apply changes: Form action calls remote function → `POST /api/rules/assistant/apply` → persists rule changes to database
4. Conversation context maintained on server between requests

### 8. Settings `/settings`

**Purpose:** View system configuration (read-only for now).

**UI Components:**
- Read-only display of:
  - Configured accounts
  - Model selection and confidence thresholds
  - Discord channel & whitelist
  - Gmail configuration
- Secrets redacted for security

**Data Flow:**
1. Page component calls remote function
2. Remote function calls `GET /api/settings`
3. Rust backend returns sanitized configuration (secrets redacted)
4. Remote function returns data to page component for display

## Remote Functions Implementation

SvelteKit remote functions are implemented as:
- **Load functions** in `+page.server.ts` files (for data loading)
- **Form actions** in `+page.server.ts` files (for mutations)
- **API routes** in `+server.ts` files (for programmatic endpoints)

All remote functions make HTTP requests to the Rust backend API at `http://127.0.0.1:17800` using the API client from `$lib/api/client.ts`.

### Streaming Responses

Some Rust endpoints return **Server-Sent Events (SSE)** for streaming data, particularly for LLM interactions. In these cases:

1. **Client** calls a SvelteKit API endpoint (e.g., `POST /api/assistant/stream`)
2. **SvelteKit endpoint** opens an SSE connection to the Rust backend
3. **SvelteKit endpoint** forwards SSE chunks back to the client as they arrive
4. **Client** processes each chunk in real-time (e.g., streaming LLM response tokens)

This ensures the UI can display streaming responses without buffering the entire response server-side.

### Periodic Polling for Updates

For non-streaming updates (e.g., action history, rule changes), use simple periodic polling with SvelteKit remote functions:

```typescript
// lib/queries.remote.ts
import { query } from '$app/server';
import * as v from 'valibot';
import { get, buildQueryString } from '$lib/api/client';
import type { PaginatedResponse, Action } from '$lib/types/generated';

export const getActions = query(
  v.object({
    timeWindow: v.optional(v.string()),
    account: v.optional(v.string()),
    // ... other filter parameters
  }),
  async (filters): Promise<PaginatedResponse<Action>> => {
    const queryString = buildQueryString(filters);
    return get<PaginatedResponse<Action>>(`/api/actions${queryString}`);
  }
);
```

```svelte
<!-- routes/actions/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { getActions } from '$lib/queries.remote';

  let filters = { timeWindow: '7d', account: 'all' };

  $effect(() => {
    // Poll for updates every 10 seconds by refreshing the query
    const interval = setInterval(() => {
      getActions(filters).refresh();
    }, 10000);

    return () => clearInterval(interval);
  });

  // The query automatically provides reactive data
  let result = $derived(await getActions(filters));
</script>

{#if result}
  <ActionsList actions={result} />
{/if}
```

This approach is:
- Simple and reliable
- Easy to debug
- Stateless (no connection management)
- Type-safe with Valibot schema validation
- Automatically reactive when filters change
- Sufficient for updates that don't need sub-second latency

For real-time streaming (LLM responses), use the SSE pattern described above.

### Example: Query Remote Function

See `$lib/api/actions.remote.ts`, `$lib/api/accounts.remote.ts`, and `$lib/api/rules.remote.ts` for complete working examples.

```typescript
// lib/actions.remote.ts
import { query } from '$app/server';
import * as v from 'valibot';
import { get, buildQueryString } from '$lib/api/client';
import type { Action, PaginatedResponse } from '$lib/types/generated';

// Define a query for fetching actions
export const getActions = query(
  v.object({
    timeWindow: v.optional(v.string()),
    account: v.optional(v.string()),
    sender: v.optional(v.string()),
    actionType: v.optional(v.string()),
    status: v.optional(v.string()),
  }),
  async (filters): Promise<PaginatedResponse<Action>> => {
    const queryString = buildQueryString(filters);
    return get<PaginatedResponse<Action>>(`/api/actions${queryString}`);
  }
);

// Define a query for fetching a single action
export const getAction = query(
  v.object({ id: v.string() }),
  async (input): Promise<Action> => {
    return get<Action>(`/api/actions/${input.id}`);
  }
);
```

```svelte
<!-- routes/actions/+page.svelte -->
<script lang="ts">
  import { getActions } from '$lib/actions.remote';
  import { page } from '$app/state';

  // Filters from URL search params
  $: filters = {
    timeWindow: $page.url.searchParams.get('timeWindow') ?? undefined,
    account: $page.url.searchParams.get('account') ?? undefined,
  };

  // Query automatically re-runs when filters change
  $: result = getActions(filters);
</script>

{#if result.data}
  <ActionsList actions={result.data} />
{:else if result.error}
  <Error message={result.error.message} />
{/if}
```

### Example: Form Remote Function (Preferred for Mutations)

**Prefer `form` over `command` for mutations.** Form remote functions provide:
- Progressive enhancement (works without JavaScript)
- Built-in validation with field-level error display via `.fields.fieldName.issues()`
- Pending state tracking via `form.pending`
- Automatic form data binding via `.fields.fieldName.as('type')`
- Server-side error injection via `invalid(issue.fieldName('message'))`

See `docs/svelte_remote_functions.md` and https://svelte.dev/docs/kit/remote-functions#form for complete documentation.

```typescript
// lib/rules.remote.ts
import { form, invalid } from '$app/server';
import { z } from 'zod';
import { post } from '$lib/api/client';
import { ApiError } from '$lib/api/errors';

// Schema for creating a rule
const createRuleSchema = z.object({
  name: z.string().min(1, 'Name is required'),
  description: z.string().optional(),
  enabled: z.boolean().default(true)
});

// Define a form for creating a rule
export const createRule = form(
  createRuleSchema,
  async (input, issue) => {
    try {
      const result = await post('/api/rules/deterministic', input);
      return { success: true, rule: result };
    } catch (e) {
      if (e instanceof ApiError && e.status === 400) {
        // Server returned validation error - mark field as invalid
        invalid(issue.name('A rule with this name already exists'));
      }
      throw e;
    }
  }
);
```

```svelte
<!-- routes/rules/deterministic/new/+page.svelte -->
<script lang="ts">
  import { createRule } from '$lib/api/rules.remote';

  // Use a stable ID - for new entities use $props.id(), for existing use the entity ID
  const uid = $props.id();
  const ruleForm = createRule.for(uid);
</script>

<form {...ruleForm.preflight(createRuleSchema).enhance()}>
  <label for="name">Name</label>
  <input id="name" {...ruleForm.fields.name.as('text')} />
  {#each ruleForm.fields.name.issues() as issue}
    <p class="error">{issue.message}</p>
  {/each}

  <label for="description">Description</label>
  <textarea id="description" {...ruleForm.fields.description.as('text')} />

  <label>
    <input type="checkbox" {...ruleForm.fields.enabled.as('checkbox')} />
    Enabled
  </label>

  <button type="submit" disabled={!!ruleForm.pending}>
    {ruleForm.pending ? 'Creating...' : 'Create Rule'}
  </button>

  {#each ruleForm.fields.allIssues() as issue}
    <p class="error">{issue.message}</p>
  {/each}
</form>
```

### Example: Streaming SSE Endpoint

```typescript
// routes/api/assistant/stream/+server.ts
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, fetch }) => {
  const { message, conversationId } = await request.json();

  // Open SSE connection to Rust backend
  const response = await fetch('http://127.0.0.1:17800/api/rules/assistant/stream', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ message, conversationId })
  });

  // Forward the SSE stream directly to the client
  return new Response(response.body, {
    headers: {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive'
    }
  });
};
```

```svelte
<!-- Client-side usage in Svelte component -->
<script lang="ts">
  async function sendMessage(message: string) {
    const response = await fetch('/api/assistant/stream', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message, conversationId })
    });

    const reader = response.body?.getReader();
    const decoder = new TextDecoder();

    while (true) {
      const { done, value } = await reader!.read();
      if (done) break;

      const chunk = decoder.decode(value);
      // Process SSE chunk (e.g., update UI with streaming token)
      handleStreamChunk(chunk);
    }
  }
</script>
```

## Rust Backend API Endpoints

The Rust backend exposes the following REST endpoints on `http://127.0.0.1:17800`:

### Actions
- `GET /api/actions` - List actions with optional filters
  - Query params:
    - `time_window` - Time filter: `24h`, `7d`, or `30d`
    - `account_id` - Filter by account UUID
    - `sender` - Smart match: contains `@` = exact email match, otherwise matches domain suffix
    - `action_type` - Comma-separated action types (e.g., `archive,apply_label`)
    - `status` - Comma-separated statuses (e.g., `completed,failed`)
    - `min_confidence`, `max_confidence` - Confidence range (0.0 to 1.0)
    - `limit` - Results per page (default 20, max 100)
    - `offset` - Pagination offset (default 0)
  - Returns: `PaginatedResponse<ActionListItem>` with `items`, `total`, `limit`, `offset`, `has_more`
- `GET /api/actions/{id}` - Get action detail with computed fields
  - Returns: `ActionDetail` including:
    - Full action data with joined decision and message fields
    - `can_undo` - Whether undo is available (status=Completed, has undo_hint, not already undone)
    - `gmail_link` - Constructed deep link to message in Gmail
    - `has_been_undone` - Whether this action was already undone
    - `undo_action_id` - ID of the undo action if one exists
- `POST /api/actions/{id}/undo` - Enqueue undo job for an action
  - Validates: status=Completed, has undo_hint with inverse_action, not already undone
  - Creates: New action from undo_hint, ActionLink with `undo_of` relation, enqueues job
  - Returns: `UndoActionResponse` with `undo_action_id`, `status: "queued"`, `message`
  - Errors: 400 (not eligible), 404 (not found), 500 (internal error)

### Rules
- `GET /api/rules/deterministic` - List all deterministic rules
  - Returns: Array of `DeterministicRule` sorted by priority ASC (lower number = earlier execution)
- `GET /api/rules/deterministic/{id}` - Get a single deterministic rule
  - Returns: `DeterministicRule` or 404 if not found
- `POST /api/rules/deterministic` - Create new deterministic rule
  - Body: `{ name: string (required), description?: string, scope?: RuleScope (default global), scope_ref?: string, priority?: number (default 100), enabled?: boolean (default true), conditions_json: object (required), action_type: string (required), action_parameters_json?: object, safe_mode?: SafeMode }`
  - Note: If `scope` is `global`, any provided `scope_ref` is ignored and set to null
  - Returns: Created `DeterministicRule` with generated ID (201)
  - Errors: 400 if name, action_type, or conditions_json is missing/empty
- `PATCH /api/rules/deterministic/{id}` - Update deterministic rule (partial)
  - Body: Any subset of `{ name, description, scope, scope_ref, priority, enabled, disabled_reason, conditions_json, action_type, action_parameters_json, safe_mode }`
  - **Three-state logic for clearable fields** (`description`, `scope_ref`, `disabled_reason`):
    - Field absent from JSON → keep existing value unchanged
    - Field explicitly set to `null` → clear the value to null
    - Field set to a value → update to the new value
  - **Automatic scope_ref clearing**: When `scope` is changed to `global`, `scope_ref` is automatically set to null to prevent dangling references
  - Returns: Updated `DeterministicRule`
  - Errors: 404 if not found
- `DELETE /api/rules/deterministic/{id}` - Delete deterministic rule
  - Returns: 204 No Content on success
  - Errors: 404 if not found
- `POST /api/rules/deterministic/swap-priorities` - Atomically swap priorities between two deterministic rules
  - Body: `{ rule_a_id: string, rule_b_id: string }`
  - Implementation: Uses a database transaction to prevent TOCTOU race conditions
    - Transaction starts before reading priorities
    - Both rules' priorities are read and swapped atomically
    - Row count verification ensures exactly 1 row updated per rule
  - Returns: 200 with `{ success: true }` on success
  - Errors: 400 (self-swap or missing IDs), 404 (rule not found), 500 (internal error)
  - Used by the UI for priority reordering to ensure data consistency
- `GET /api/rules/llm` - List all LLM rules
  - Returns: Array of `LlmRule`
- `GET /api/rules/llm/{id}` - Get a single LLM rule
  - Returns: `LlmRule` or 404 if not found
- `POST /api/rules/llm` - Create new LLM rule
  - Body: `{ name: string (required), description?: string, scope?: RuleScope (default global), scope_ref?: string, rule_text: string (required), enabled?: boolean (default true), metadata_json?: object }`
  - Note: If `scope` is `global`, any provided `scope_ref` is ignored and set to null
  - Returns: Created `LlmRule` with generated ID (201)
  - Errors: 400 if name or rule_text is missing/empty
- `PATCH /api/rules/llm/{id}` - Update LLM rule (partial)
  - Body: Any subset of `{ name, description, scope, scope_ref, rule_text, enabled, metadata_json }`
  - **Three-state logic for clearable fields** (`description`, `scope_ref`):
    - Field absent from JSON → keep existing value unchanged
    - Field explicitly set to `null` → clear the value to null
    - Field set to a value → update to the new value
  - **Automatic scope_ref clearing**: When `scope` is changed to `global`, `scope_ref` is automatically set to null to prevent dangling references
  - Returns: Updated `LlmRule`
  - Errors: 404 if not found
- `DELETE /api/rules/llm/{id}` - Delete LLM rule
  - Returns: 204 No Content on success
  - Errors: 404 if not found

### Labels
- `GET /api/labels` - List all labels across all accounts
  - Returns: Array of `LabelSummary` with `{ id, account_id, provider_label_id, name, label_type, description, colors: { background_color, text_color } }`
  - Used by the condition builder to populate the label_present dropdown
  - Gracefully handles per-account errors (logs and continues with other accounts)

### Rules Assistant
- `POST /api/rules/assistant/message` - Send chat message to assistant (non-streaming)
  - Body: `{ message: string, conversationId?: string }`
  - Returns: `{ response: string, proposedChanges: RuleChange[], conversationId: string }`
- `POST /api/rules/assistant/stream` - Send chat message to assistant with SSE streaming
  - Body: `{ message: string, conversationId?: string }`
  - Returns: Server-Sent Events stream with incremental response tokens
  - Event format: `data: {"type": "token", "content": "...", "conversationId": "..."}\n\n`
  - Final event: `data: {"type": "done", "proposedChanges": [...], "conversationId": "..."}\n\n`
- `POST /api/rules/assistant/apply` - Apply proposed rule changes
  - Body: `{ conversationId: string, changeIds: string[] }`
  - Returns: `{ success: boolean, appliedRules: Rule[] }`

### Accounts
- `GET /api/accounts` - List all accounts for the current user
  - Returns: `AccountSummary[]` with `id`, `email`, `display_name`, `provider`
  - Note: OAuth credentials are stripped for security

### Settings
- `GET /api/settings` - Get system configuration
  - Returns: Configuration object with secrets redacted

## Authentication

All endpoints are internal (localhost only). Authentication uses:
- Simple bearer token in `Authorization` header, OR
- No authentication (since it's localhost-only)

The SvelteKit app should include the auth token in all requests to the Rust backend if configured.
