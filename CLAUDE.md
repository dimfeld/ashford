# Coding Agent Rules

This file provides guidance to coding agents working with code in this repository.

For further in-depth guidance, search the "docs" directory for relevant files.

## rmplan

The tasks in this repository are using the rmplan task tracking system. If you are working on a task and discover
some other piece of work that needs to be done but isn't in scope, add it as a new plan using the create-plan MCP tool.

This system is not a substitute for your own Todo tools and planning. You should continue to use your own Todo and/or planning tools to plan and track your work as you go.

## Accounts System

This application currently supports only a single user. Therefore there are no concerns around multi-tenancy, as all
data belongs to that user.

The accounts system as it exists right now is for a single user who has multiple mail accounts. We should not enforce strict separation between accounts as many operations from a UI perspective will find it useful to look at all accounts at once.


## Dependencies

When adding dependencies, always use `pnpm add` or `cargo add` to make sure you get the latest version of the package.

## Reading and Writing Files

- Always read files in full to get the entire context.
- Before making any code changes, start by finding & reading ALL of it
- Never make changes without reading the entire file

## Testing & Mocking

- Avoid mocks in backend tests, unless they call external services
- Mocking the backend is ok in frontend tests, but should be done only after careful consideration
- Prefer regular for loops over `it.each` for table-driven tests
- In Vitest tests, use vi.waitFor any time an assertion may not be immediately met.

