# Web UI Architecture

The web UI is built with SvelteKit and provides a user interface for viewing and managing email actions, rules, and settings.

## Architecture

The SvelteKit application uses a **remote functions** pattern where:

1. **Frontend Components** (Svelte pages/components) call **SvelteKit remote functions**
2. **Remote Functions** (server-side TypeScript in `+page.server.ts` or `+server.ts` files) communicate with the **Rust backend API**
3. **Rust Backend** exposes REST endpoints on `http://127.0.0.1:17801`

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
│ :17801          │
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
  - Enable/disable toggle
  - Reorder priorities (e.g., drag/drop or up/down)
  - Edit button (opens form or navigates to Assistant with preloaded context)

**Data Flow:**
1. Load rules: Remote function calls `GET /api/rules/deterministic` and `GET /api/rules/llm`
2. Toggle enable/disable: Form action calls `PATCH /api/rules/{type}/{id}`
3. Reorder: Form action calls `PATCH /api/rules/{type}/{id}` with new priority
4. Create/Edit: Form action calls `POST /api/rules/{type}` or `PATCH /api/rules/{type}/{id}`

### 4. Rules Assistant `/rules/assistant`

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

### 5. Settings `/settings`

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

All remote functions make HTTP requests to the Rust backend API at `http://127.0.0.1:17801`.

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

export const getActions = query(
  v.object({
    timeWindow: v.optional(v.string()),
    account: v.optional(v.string()),
    // ... other filter parameters
  }),
  async (filters) => {
    const response = await fetch(
      'http://127.0.0.1:17801/api/actions?' + new URLSearchParams(filters).toString()
    );
    return await response.json();
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

```typescript
// lib/actions.remote.ts
import { query } from '$app/server';
import * as v from 'valibot';

// Define a query for fetching actions
export const getActions = query(
  v.object({
    timeWindow: v.optional(v.string()),
    account: v.optional(v.string()),
    sender: v.optional(v.string()),
    actionType: v.optional(v.string()),
    status: v.optional(v.string()),
  }),
  async (filters) => {
    const params = new URLSearchParams(
      Object.entries(filters).filter(([_, v]) => v !== undefined)
    );

    const response = await fetch(
      `http://127.0.0.1:17801/api/actions?${params}`
    );

    if (!response.ok) {
      throw new Error('Failed to fetch actions');
    }

    return await response.json();
  }
);

// Define a query for fetching a single action
export const getAction = query(v.string(), async (id) => {
  const response = await fetch(`http://127.0.0.1:17801/api/actions/${id}`);

  if (!response.ok) {
    throw new Error('Failed to fetch action');
  }

  return await response.json();
});
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

### Example: Command Remote Function

```typescript
// lib/actions.remote.ts (continued)
import { command } from '$app/server';

// Define a command for undoing an action
export const undoAction = command(v.string(), async (id) => {
  const response = await fetch(
    `http://127.0.0.1:17801/api/actions/${id}/undo`,
    { method: 'POST' }
  );

  if (!response.ok) {
    throw new Error(await response.text());
  }

  // Refresh the action detail after undo
  getAction(id).refresh();

  return await response.json();
});
```

```svelte
<!-- routes/actions/[id]/+page.svelte -->
<script lang="ts">
  import { getAction, undoAction } from '$lib/actions.remote';
  import { page } from '$app/state';

  $: actionId = $page.params.id;
  $: result = getAction(actionId);

  async function handleUndo() {
    try {
      await undoAction(actionId);
      // getAction automatically refreshes due to the .refresh() call in undoAction
    } catch (error) {
      console.error('Undo failed:', error);
    }
  }
</script>

{#if result.data}
  <ActionDetail action={result.data} />
  <button onclick={handleUndo}>Undo Action</button>
{/if}
```

### Example: Streaming SSE Endpoint

```typescript
// routes/api/assistant/stream/+server.ts
import type { RequestHandler } from './$types';

export const POST: RequestHandler = async ({ request, fetch }) => {
  const { message, conversationId } = await request.json();

  // Open SSE connection to Rust backend
  const response = await fetch('http://127.0.0.1:17801/api/rules/assistant/stream', {
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

The Rust backend exposes the following REST endpoints on `http://127.0.0.1:17801`:

### Actions
- `GET /api/actions` - List actions with optional filters
  - Query params: `timeWindow`, `account`, `sender`, `actionType`, `status`, `minConfidence`, `maxConfidence`, `page`, `limit`
  - Returns: Paginated list of actions
- `GET /api/actions/{id}` - Get action detail
  - Returns: Full action details including decision JSON, rationale, links
- `POST /api/actions/{id}/undo` - Enqueue undo job for an action
  - Returns: Job ID

### Rules
- `GET /api/rules/deterministic` - List all deterministic rules
  - Returns: Array of deterministic rules with conditions
- `GET /api/rules/llm` - List all LLM rules
  - Returns: Array of LLM rules with prompts/examples
- `POST /api/rules/deterministic` - Create new deterministic rule
  - Body: Rule configuration JSON
  - Returns: Created rule with ID
- `PATCH /api/rules/deterministic/{id}` - Update deterministic rule
  - Body: Partial rule configuration
  - Returns: Updated rule
- `POST /api/rules/llm` - Create new LLM rule
  - Body: Rule configuration JSON
  - Returns: Created rule with ID
- `PATCH /api/rules/llm/{id}` - Update LLM rule
  - Body: Partial rule configuration
  - Returns: Updated rule

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

### Settings
- `GET /api/settings` - Get system configuration
  - Returns: Configuration object with secrets redacted

## Authentication

All endpoints are internal (localhost only). Authentication uses:
- Simple bearer token in `Authorization` header, OR
- No authentication (since it's localhost-only)

The SvelteKit app should include the auth token in all requests to the Rust backend if configured.
