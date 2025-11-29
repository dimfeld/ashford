---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: Gmail API Client & Account Management
goal: Implement Gmail REST API client with OAuth2 token management and account
  CRUD operations
id: 10
uuid: a0d2a8da-0146-4e99-9f3b-8c526bad5524
generatedBy: agent
status: done
priority: high
container: false
temp: false
dependencies: []
parent: 3
references: {}
issue: []
pullRequest: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T07:43:19.347Z
promptsGeneratedAt: 2025-11-29T07:43:19.347Z
createdAt: 2025-11-29T07:42:47.217Z
updatedAt: 2025-11-29T09:10:51.222Z
progressNotes:
  - timestamp: 2025-11-29T08:03:12.239Z
    text: Added Gmail core modules (types, oauth, client) with reqwest dependency
      and exports; tokens auto-refresh with refresh lock and optional
      persistence
    source: "implementer: tasks 1-5,9"
  - timestamp: 2025-11-29T08:10:40.900Z
    text: Added overridable token endpoint for Gmail client/oauth to enable local
      mocking; implemented wiremock-based unit tests covering token refresh
      success/error paths and 401 retry behavior.
    source: "tester: Gmail client tests"
  - timestamp: 2025-11-29T08:37:32.620Z
    text: Created accounts repository module with config/state structs, CRUD
      helpers, refresh_tokens with optimistic locking, and exported from lib.rs.
      Tests still to run.
    source: "implementer: tasks 6-8,11"
  - timestamp: 2025-11-29T08:41:02.302Z
    text: "Added missing-path coverage: Gmail client now checks token store failure;
      OAuth refresh retains existing refresh token; Account repo returns
      NotFound for absent records. All tests passing (cargo test -p
      ashford-core)."
    source: "tester: task10"
  - timestamp: 2025-11-29T08:49:07.477Z
    text: Built gmail-oauth loopback script (tokio listener + browser launch) that
      exchanges authorization code for tokens and prints AccountConfig JSON;
      uses existing OAuth types.
    source: "implementer: task12"
  - timestamp: 2025-11-29T08:52:36.999Z
    text: Added gmail-oauth binary unit tests for auth URL, state validation, and
      token exchange error paths; cargo test -p ashford-core passing.
    source: "tester: task10"
  - timestamp: 2025-11-29T09:06:02.229Z
    text: Added Gmail client coverage for thread fetch and query params; verified
      fresh tokens avoid refresh. cargo test -p ashford-core passing.
    source: "tester: task10"
tasks:
  - title: Add Gmail dependencies to Cargo.toml
    done: true
    description: Add reqwest (with json feature), base64, and update serde
      dependencies in ashford-core/Cargo.toml for Gmail API communication.
  - title: Create Gmail API types module
    done: true
    description: "Create server/crates/ashford-core/src/gmail/types.rs with Rust
      structs for Gmail API responses: Message, Thread, MessagePart, Header,
      History, HistoryRecord, etc. Use serde for JSON deserialization."
  - title: Implement OAuth2 token management
    done: true
    description: "Create server/crates/ashford-core/src/gmail/oauth.rs with:
      OAuthTokens struct (access_token, refresh_token, expires_at), token
      refresh function using Google's token endpoint, automatic refresh when
      token expires within 5 minutes."
  - title: Build Gmail REST API client
    done: true
    description: "Create server/crates/ashford-core/src/gmail/client.rs with
      GmailClient struct wrapping reqwest::Client. Implement methods:
      get_message(id), get_thread(id), list_history(start_history_id),
      list_messages(query). Handle auth header injection and token refresh on
      401."
  - title: Create Gmail module root
    done: true
    description: Create server/crates/ashford-core/src/gmail/mod.rs exporting
      client, oauth, and types submodules. Export main types for external use.
  - title: Implement Account repository
    done: true
    description: "Create server/crates/ashford-core/src/accounts.rs with
      AccountRepository struct. Implement: create(email, config), get_by_id(id),
      get_by_email(email), update_config(id, config), update_state(id, state),
      list_all(), delete(id). Use existing DB patterns from queue.rs."
  - title: Define Account config and state types
    done: true
    description: "In accounts.rs, define AccountConfig struct (oauth: OAuthTokens,
      pubsub settings) and AccountState struct (history_id, last_sync_at,
      sync_status). These serialize to config_json and state_json columns."
  - title: Add token refresh with optimistic locking
    done: true
    description: Implement refresh_tokens_if_needed() in accounts.rs that checks
      expires_at, refreshes if needed, and uses optimistic locking (WHERE
      updated_at = old_value) to prevent race conditions when multiple jobs
      refresh simultaneously.
  - title: Export new modules from lib.rs
    done: true
    description: Update server/crates/ashford-core/src/lib.rs to export gmail and
      accounts modules.
  - title: Write unit tests for Gmail client
    done: true
    description: "Add tests in gmail/client.rs for: API response parsing, token
      refresh flow, error handling (401, 404, 429). Use mock responses where
      appropriate."
  - title: Write unit tests for Account repository
    done: true
    description: "Add tests in accounts.rs for: CRUD operations, config/state
      serialization, optimistic locking behavior. Use temporary SQLite
      database."
  - title: Create OAuth token acquisition script
    done: true
    description: "Create a standalone script (scripts/gmail-oauth.rs or similar)
      that performs \"Desktop app\" style OAuth flow: opens browser for consent,
      runs local HTTP server to receive callback, exchanges code for tokens, and
      outputs the token JSON for manual entry into the app. Use Google's
      installed app flow with localhost redirect."
    files: []
    docs: []
    steps: []
changedFiles:
  - server/Cargo.lock
  - server/Cargo.toml
  - server/crates/ashford-core/Cargo.toml
  - server/crates/ashford-core/src/accounts.rs
  - server/crates/ashford-core/src/bin/gmail-oauth.rs
  - server/crates/ashford-core/src/gmail/client.rs
  - server/crates/ashford-core/src/gmail/mod.rs
  - server/crates/ashford-core/src/gmail/oauth.rs
  - server/crates/ashford-core/src/gmail/types.rs
  - server/crates/ashford-core/src/lib.rs
  - server/crates/ashford-core/src/worker.rs
  - web/CLAUDE.md
tags:
  - gmail
  - oauth
  - rust
---

This plan implements the foundational Gmail integration layer:

- Gmail REST API client using reqwest
- OAuth2 token storage in accounts.config_json
- Automatic token refresh before expiration
- Account repository with CRUD operations
- State management for sync tracking (historyId)

This is a prerequisite for all other Gmail functionality (ingestion, sync, backfill).

<!-- rmplan-generated-start -->
This plan implements the foundational Gmail integration layer:

- Gmail REST API client using reqwest
- OAuth2 token storage in accounts.config_json
- Automatic token refresh before expiration
- Account repository with CRUD operations
- State management for sync tracking (historyId)
- Standalone OAuth script for "Desktop app" style token acquisition

This is a prerequisite for all other Gmail functionality (ingestion, sync, backfill).

## OAuth Flow

Token acquisition is handled by a standalone script (not in-app):
1. User runs `scripts/gmail-oauth` (or `cargo run --bin gmail-oauth`)
2. Script opens browser to Google consent screen
3. Script runs temporary localhost HTTP server to receive OAuth callback
4. Script exchanges authorization code for access/refresh tokens
5. Script outputs JSON with tokens that user copies into account creation

This keeps OAuth complexity separate from the main application.

## Acceptance Criteria

- [ ] OAuth script successfully obtains access and refresh tokens
- [ ] Gmail API client can fetch messages and threads with valid OAuth token
- [ ] Tokens are automatically refreshed before expiration (5 minute buffer)
- [ ] Account CRUD operations work correctly
- [ ] Token refresh uses optimistic locking to prevent race conditions
- [ ] All new code paths are covered by tests

## Files to Create

- `server/crates/ashford-core/src/gmail/mod.rs`
- `server/crates/ashford-core/src/gmail/client.rs`
- `server/crates/ashford-core/src/gmail/oauth.rs`
- `server/crates/ashford-core/src/gmail/types.rs`
- `server/crates/ashford-core/src/accounts.rs`
- `server/scripts/gmail-oauth.rs` (or separate binary crate)

## Files to Modify

- `server/crates/ashford-core/Cargo.toml` - Add reqwest, base64
- `server/crates/ashford-core/src/lib.rs` - Export new modules
<!-- rmplan-generated-end -->

# Implementation Notes

Completed tasks: Add Gmail dependencies to Cargo.toml, Create Gmail API types module, Implement OAuth2 token management, Build Gmail REST API client, Create Gmail module root, and Export new modules from lib.rs. Added reqwest (json/gzip/rustls) and base64 dependencies plus chrono serde support in the workspace manifest to enable DateTime serialization for token storage. Introduced gmail/types.rs with serde models that mirror Gmail REST payloads (Message, Thread, MessagePart*, History*, List* responses) using camelCase field renames and defaults for optional arrays. Implemented gmail/oauth.rs with OAuthTokens (access/refresh/expires_at), needs_refresh helper, refresh_access_token hitting Google's token endpoint, and a NoopTokenStore that satisfies the TokenStore trait for persistence hooks; refresh buffer constant exported for reuse. Built gmail/client.rs wrapping reqwest::Client with per-user_id base URL handling and authenticated helpers that inject bearer tokens, refresh opportunistically within the 5 minute buffer, retry once on 401, and serialize responses into the new types; refresh synchronization uses RwLock for cached tokens and a Mutex to serialize refreshes before persisting via TokenStore. Added gmail/mod.rs to re-export client, oauth, and types, and updated lib.rs to expose GmailClient, OAuth tokens/errors, TokenStore/NoopTokenStore, and the refresh buffer for other crates. Ran cargo fmt and cargo test -p ashford-core to validate the new code compiles and existing tests remain green.

Added Task 10 coverage for Gmail client error handling and decoding. Updated gmail/client.rs send_json to read the response body and map JSON parse failures into GmailClientError::Decode instead of bubbling through reqwest::Error, keeping http errors unchanged. Updated gmail/oauth.rs refresh_access_token_with_endpoint to parse via serde_json::from_str so malformed token responses now raise OAuthError::Decode; kept token endpoint error handling intact by capturing the status and text first. Added wiremock-based tests covering 404 and 429 responses, parsing of list_history and list_messages payloads, and malformed JSON decode paths in gmail/client.rs; added a decode-error test for oauth refresh. All changes target Task 10’s missing error-path coverage, and tests now exercise list/history parsing plus 401/404/429 and decode scenarios to ensure diagnostics are correct.

Implemented Account repository, config/state models, optimistic token refresh, and tests (Tasks: Implement Account repository; Define Account config and state types; Add token refresh with optimistic locking; Write unit tests for Account repository). Added new module server/crates/ashford-core/src/accounts.rs defining Account/AccountConfig/AccountState/PubsubConfig with serde JSON storage for config_json/state_json and client_id/client_secret+OAuth token bundle for refresh. Repository supports create/get_by_id/get_by_email/update_config/update_state/list_all/delete plus refresh_tokens_if_needed helpers that call Gmail token endpoint (overrideable in tests) and enforce optimistic locking by matching updated_at before writing refreshed tokens. Helper to refresh using already-fetched Account enables conflict detection; updates use RFC3339 millisecond timestamps like queue.rs. Exported Account types and repository from lib.rs. Added comprehensive tests using temporary libsql db + migrations and wiremock token endpoint covering CRUD, serialization, refresh skip when fresh, refresh update path, and optimistic-lock conflict behavior. Ran cargo fmt and cargo test -p ashford-core (worker tests are flaky but all pass on final run).

Implemented the 'Create OAuth token acquisition script' task by adding a bin target at server/crates/ashford-core/src/bin/gmail-oauth.rs. The script launches an installed-app OAuth flow: it binds a loopback TcpListener on an ephemeral port, builds the Google consent URL with offline access and prompt=consent, generates a random URL-safe state, and tries to open the browser (open/xdg-open/cmd start) while printing the fallback URL. It waits up to five minutes for the callback, parses the GET request manually with a state check, and replies with a friendly HTML page before exchanging the code at oauth2.googleapis.com/token via reqwest. The response is converted into the existing OAuthTokens type and printed as pretty JSON along with a ready-to-paste AccountConfig (including client_id/client_secret and default PubsubConfig). All logic reuses existing dependencies (reqwest, chrono, rand, base64) and avoids new crates. Verified with cargo test -p ashford-core after formatting.

Addressed review feedback on the Gmail OAuth helper (Create OAuth token acquisition script). Updated server/crates/ashford-core/src/bin/gmail-oauth.rs to shell-escape the Windows browser launch by invoking cmd /C start with an empty title and a quoted URL so '&' in query strings no longer break. Hardened wait_for_code to send a 400 response when the callback URL is malformed or missing the authorization code, returning clear errors instead of leaving the browser hanging. Added wait_for_code_returns_error_for_missing_code test to cover the new response path. Ran cargo test -p ashford-core to verify the binary and core crate tests pass.

Implemented Gmail dependency wiring and core client stack: ashford-core/Cargo.toml now pulls reqwest with json/rustls/gzip plus base64/chrono serde to support Gmail REST calls; gmail/types.rs defines Message/Thread/MessagePart/Header and History/List response structs with camelCase serde defaults; gmail/oauth.rs introduces OAuthTokens (access/refresh/expires_at), needs_refresh, refresh_access_token (configurable endpoint), TokenStore trait and NoopTokenStore; gmail/client.rs wraps reqwest with auto token refresh (5 minute buffer), RwLock cache + Mutex guard, 401 retry path, explicit decode error handling, methods for get_message/get_thread/list_history/list_messages, and overridable api_base/token_endpoint for testing; gmail/mod.rs re-exports client/oauth/types and lib.rs exports GmailClient, OAuth tokens/errors, TokenStore/NoopTokenStore and DEFAULT_REFRESH_BUFFER. Tasks covered: Add Gmail dependencies to Cargo.toml; Create Gmail API types module; Build Gmail REST API client; Create Gmail module root; Export new modules from lib.rs. Validation: cargo test -p ashford-core exercises oauth/client decode paths, refresh flow, and API parsing.
