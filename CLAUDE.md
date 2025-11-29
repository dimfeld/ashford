# Coding Agent Rules

This file provides guidance to coding agents working with code in this repository.

For further in-depth guidance, search the "docs" directory for relevant files.

## rmplan

The tasks in this repository are using the rmplan task tracking system. If you are working on a task and discover
some other piece of work that needs to be done but isn't in scope, add it as a new plan using the create-plan MCP tool.

### Reading and Writing Files

- Always read files in full to get the entire context.
- Before making any code changes, start by finding & reading ALL of it
- Never make changes without reading the entire file


### Remote Query Functions

- SvelteKit now supports its own RPC layer with query functions in *.remote.ts files.
- Remove fuunctions can be transparently imported and called on the client, but always run on the server.
- Import these helpers directly and call them inside components or server files. SvelteKit transparently makes calls from the client work.

### Testing & Mocking

- Avoid mocks in backend tests, unless they call external services
- Mocking the backend is ok in frontend tests, but should be done only after careful consideration
- Prefer regular for loops over `it.each` for table-driven tests
- Use vi.waitFor any time an assertion may not be immediately met.
- If you get errors about a local monorepo package not found, run `pnpm install` to ensure everything is linked up properly.
- If you run into errors about tests not recognizing Svelte syntax, check if you may have deleted vite.config.ts.
- For full integration tests (database → load function → component rendering), see `docs/inbox-unit-test-tutorial.md` section "Full Integration Testing"



