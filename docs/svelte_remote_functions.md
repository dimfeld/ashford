## Remote Functions

SvelteKit's remote functions enable type-safe client-server communication. They are defined in `*.remote.ts` files and can be called from anywhere (client or server), but always execute on the server.


### Types of Remote Functions

**Query** - Read server data with caching support. Can be refreshed via `.refresh()` method.

```typescript
import { query } from '$app/server';
import { remoteGuard } from '$lib/server/decorators/serverGuard';
import { z } from 'zod';

const searchSchema = z.object({
  query: z.string().optional(),
  limit: z.number().int().min(1).max(50).optional(),
});

export const searchProducts = query(
  searchSchema,
  remoteGuard().wrap(async ({ input, organization }) => {
    // Server-only logic here
    return { products: await searchOrgProducts(input) };
  })
);
```

**Query.batch** - Collects multiple simultaneous calls into a single request, solving n+1 problems.

**Form** - Handles form submissions with progressive enhancement and validation.

```typescript
import { form } from '$app/server';
import { remoteGuard } from '$lib/server/decorators/serverGuard';

export const addManualEntry = form(
  addManualEntrySchema,
  remoteGuard().wrapForm(async ({ input, organization, db }) => {
    // Perform server-side mutation
    await addScanBatchManualEntry(db, { ...input });
    return { success: true };
  })
);
```

**Command** - Writes data without form binding, callable from event handlers.

**Prerender** - Executes at build time for static data, cached via Cache API.

### File Organization

- Remote functions live in `*.remote.ts` files anywhere in `src/` (except `src/lib/server`)
- Import and call them directly - SvelteKit handles the client/server boundary transparently
- `.remote.ts` files can only export remote functions, nothing else.
- Co-locate schemas in `*.types.ts` files for validation

### Usage in Components

**Query functions** can be awaited at the top level:

```svelte
<script lang="ts">
  import { searchProducts } from '$lib/queries/products.remote.ts';

  let products = await searchProducts({ query: 'widget', limit: 10 });
</script>
```

**Form functions** are used with the `.for()` and `.enhance()` methods:

```svelte
<script lang="ts">
  import { addManualEntry } from './manualEntry.remote.ts';

  const uid = $props.id();
  const addForm = addManualEntry.for(uid);

  let isSubmitting = $derived(!!addForm.pending);
</script>

<form {...addForm.preflight(schema).enhance()} >

  <label for="name">Name</label>
  <input id="name" {...addForm.fields.name.as('text')} />
  <button type="submit">Submit</button>
</form>
```

### Validation

Remote functions accepting arguments require Standard Schema validation (Zod recommended):

```typescript
import { z } from 'zod';

const schema = z.object({
  name: z.string().min(1),
  quantity: z.number().int().positive(),
});

export const myForm = form(schema, async (input) => {
  // input is type-safe and validated
});
```

### Error Handling

- Failed queries/commands trigger the nearest `<svelte:boundary>`
- Forms re-populate with submitted data on validation failure
- Use `invalid()` for programmatic field-level errors (see `docs/forms.md` for details)
- Server errors are automatically reported via `locals.reportError`

### Best Practices

1. **Prefer `form` for progressive enhancement** - Works without JavaScript
2. **Use batched queries** for related requests to avoid n+1 problems
3. **Always validate user input** with schemas
4. **Co-locate related functions** in the same `.remote.ts` file
5. **Return structured data** that components can easily consume

