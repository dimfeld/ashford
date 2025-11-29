## General Code Guidance

- Do not use `any` in Typescript code, except in tests. It defeats the purpose of Typescript. If you actually don't know the type of something, use `unknown` instead.
- Comments in code should describe the current state of the code, but not how you changed it from before.

### SvelteKit

- Use Svelte 5 runes (`$state`) not old `$:` syntax
- Use `href` instead of onClick for navigation. Our `Button` component supports this as well.
- `redirect()` throws an error to work - use OUTSIDE try/catch blocks

### Remote Query Functions

- See docs/svelte_remote_functions.md for guidelines on using forms and remote functions in Svelte.
- SvelteKit now supports its own RPC layer with query functions in \*.remote.ts files.
- Remove functions can be transparently imported and called on the client, but always run on the server.
- Import these helpers directly and call them inside components or server files. SvelteKit transparently makes calls from the client work.

### Svelte Runes Guidelines

- Use `$derived` for expressions that can be written in one statement:
  - Ternary operators: `$derived(condition ? valueA : valueB)`
  - Property access: `$derived(object.property)`
  - Simple calculations: `$derived(a + b * 2)`
  - Method calls that return values: `$derived(array.find(item => item.id === id))`
- Use `$derived.by(() => {})` when you need:
  - Multi-line logic with if/else statements
  - Loops (for, while, etc.)
  - Multiple intermediate variables
  - Complex object/array construction
- Examples:
  ```typescript
  // Good - use $derived for simple expressions
  const isActive = $derived(status === 'active');
  const fullName = $derived(`${firstName} ${lastName}`);
  const selectedItem = $derived(items.find((i) => i.id === selectedId));
  // Good - use $derived.by for complex logic
  const processedData = $derived.by(() => {
  	const filtered = data.filter((item) => item.visible);
  	const sorted = filtered.sort((a, b) => a.name.localeCompare(b.name));
  	return sorted.map((item) => ({
  		...item,
  		displayName: `${item.name} (${item.count})`
  	}));
  });
  ```

`const value = $derived(() => { logic })` is WRONG. You must never put a function in `$derived`, use a simple expression
or `$derived.by` instead.
