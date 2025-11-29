---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: Gmail API Client & Account Management
goal: Implement Gmail REST API client with OAuth2 token management and account
  CRUD operations
id: 10
uuid: a0d2a8da-0146-4e99-9f3b-8c526bad5524
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies: []
parent: 3
issue: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T07:43:19.347Z
promptsGeneratedAt: 2025-11-29T07:43:19.347Z
createdAt: 2025-11-29T07:42:47.217Z
updatedAt: 2025-11-29T07:49:49.485Z
tasks:
  - title: Add Gmail dependencies to Cargo.toml
    done: false
    description: Add reqwest (with json feature), base64, and update serde
      dependencies in ashford-core/Cargo.toml for Gmail API communication.
  - title: Create Gmail API types module
    done: false
    description: "Create server/crates/ashford-core/src/gmail/types.rs with Rust
      structs for Gmail API responses: Message, Thread, MessagePart, Header,
      History, HistoryRecord, etc. Use serde for JSON deserialization."
  - title: Implement OAuth2 token management
    done: false
    description: "Create server/crates/ashford-core/src/gmail/oauth.rs with:
      OAuthTokens struct (access_token, refresh_token, expires_at), token
      refresh function using Google's token endpoint, automatic refresh when
      token expires within 5 minutes."
  - title: Build Gmail REST API client
    done: false
    description: "Create server/crates/ashford-core/src/gmail/client.rs with
      GmailClient struct wrapping reqwest::Client. Implement methods:
      get_message(id), get_thread(id), list_history(start_history_id),
      list_messages(query). Handle auth header injection and token refresh on
      401."
  - title: Create Gmail module root
    done: false
    description: Create server/crates/ashford-core/src/gmail/mod.rs exporting
      client, oauth, and types submodules. Export main types for external use.
  - title: Implement Account repository
    done: false
    description: "Create server/crates/ashford-core/src/accounts.rs with
      AccountRepository struct. Implement: create(email, config), get_by_id(id),
      get_by_email(email), update_config(id, config), update_state(id, state),
      list_all(), delete(id). Use existing DB patterns from queue.rs."
  - title: Define Account config and state types
    done: false
    description: "In accounts.rs, define AccountConfig struct (oauth: OAuthTokens,
      pubsub settings) and AccountState struct (history_id, last_sync_at,
      sync_status). These serialize to config_json and state_json columns."
  - title: Add token refresh with optimistic locking
    done: false
    description: Implement refresh_tokens_if_needed() in accounts.rs that checks
      expires_at, refreshes if needed, and uses optimistic locking (WHERE
      updated_at = old_value) to prevent race conditions when multiple jobs
      refresh simultaneously.
  - title: Export new modules from lib.rs
    done: false
    description: Update server/crates/ashford-core/src/lib.rs to export gmail and
      accounts modules.
  - title: Write unit tests for Gmail client
    done: false
    description: "Add tests in gmail/client.rs for: API response parsing, token
      refresh flow, error handling (401, 404, 429). Use mock responses where
      appropriate."
  - title: Write unit tests for Account repository
    done: false
    description: "Add tests in accounts.rs for: CRUD operations, config/state
      serialization, optimistic locking behavior. Use temporary SQLite
      database."
  - title: Create OAuth token acquisition script
    done: false
    description: "Create a standalone script (scripts/gmail-oauth.rs or similar)
      that performs \"Desktop app\" style OAuth flow: opens browser for consent,
      runs local HTTP server to receive callback, exchanges code for tokens, and
      outputs the token JSON for manual entry into the app. Use Google's
      installed app flow with localhost redirect."
    files: []
    docs: []
    steps: []
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
