# Ashford Web UI

SvelteKit-based web interface for managing Ashford email automation.

## Features

- **Actions History** - View and filter historical email actions
- **Rules Management** - Create and manage deterministic and LLM rules
- **Rules Assistant** - Natural language interface for rule creation
- **Settings** - View system configuration
- **Dark Mode** - Toggle between light and dark themes

## Getting Started

### Prerequisites

- Node.js 20+
- pnpm

### Development

```bash
# Install dependencies
pnpm install

# Start development server
pnpm dev

# Open in browser
open http://localhost:5173
```

### Environment Variables

| Variable      | Description          | Default                  |
| ------------- | -------------------- | ------------------------ |
| `BACKEND_URL` | Rust backend API URL | `http://127.0.0.1:17800` |

### Building

```bash
# Create production build
pnpm build

# Preview production build
pnpm preview
```

## Project Structure

```
src/
├── routes/              # SvelteKit pages and layouts
│   ├── +layout.svelte   # App shell with sidebar navigation
│   ├── actions/         # Actions history pages
│   ├── rules/           # Rules management pages
│   └── settings/        # Settings page
├── lib/
│   ├── api/             # API client and remote functions
│   │   ├── client.ts    # Typed fetch wrapper for backend API
│   │   └── *.remote.ts  # Remote function definitions
│   ├── components/
│   │   └── ui/          # shadcn-svelte UI components
│   └── types/
│       └── generated/   # TypeScript types from Rust (ts-rs)
```

## TypeScript Types

Types are auto-generated from Rust using `ts-rs`. To regenerate after Rust type changes:

```bash
cd ../server
cargo test --test export_ts_types -- --ignored
```

Generated types include:

- `Action`, `ActionStatus`, `Decision`, `DecisionSource`
- `DeterministicRule`, `LlmRule`, `Condition`
- `AccountSummary`, `MessageSummary`, `LabelSummary`
- `PaginatedResponse<T>`

## Remote Functions

Remote functions in `*.remote.ts` files provide type-safe client-server communication. They execute on the server but can be called from client code.

```typescript
// lib/api/example.remote.ts
import { query, command } from '$app/server';
import * as v from 'valibot';
import { get } from './client';

export const getActions = query(v.object({ status: v.optional(v.string()) }), async (input) => {
	return get('/api/actions');
});
```

See `docs/svelte_remote_functions.md` and `src/lib/api/example.remote.ts` for full documentation and examples.
