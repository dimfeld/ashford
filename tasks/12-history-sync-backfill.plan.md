---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: History Sync & Backfill
goal: Implement History API sync for catchup and backfill job for initial
  account setup
id: 12
uuid: 58b733a3-2f88-4b04-890f-23d394b9550b
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies:
  - 11
parent: 3
issue: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
  - docs/job_queue.md
planGeneratedAt: 2025-11-29T07:44:18.186Z
promptsGeneratedAt: 2025-11-29T07:44:18.186Z
createdAt: 2025-11-29T07:42:47.306Z
updatedAt: 2025-11-29T07:44:18.186Z
tasks:
  - title: Add History API method to Gmail client
    done: false
    description: Extend GmailClient in gmail/client.rs with
      list_history(start_history_id, page_token) method. Parse History API
      response with messagesAdded, labelsAdded, labelsRemoved events. Handle
      pagination via nextPageToken.
  - title: Add messages.list method to Gmail client
    done: false
    description: Extend GmailClient with list_messages(query, page_token) method for
      search-based sync. Returns list of message IDs matching query (e.g.,
      'newer_than:30d'). Handle pagination.
  - title: Implement history.sync.gmail job handler
    done: false
    description: "Create server/crates/ashford-core/src/jobs/history_sync_gmail.rs.
      Handler: parse payload (account_id, start_history_id), call History API,
      for each messagesAdded enqueue ingest.gmail job, update account state_json
      with new historyId. Use idempotency key:
      history.sync.gmail:{account_id}:{start_history_id}."
  - title: Handle History API pagination
    done: false
    description: In history.sync.gmail handler, loop through all pages using
      nextPageToken. Process all messagesAdded events before updating account's
      historyId to ensure consistency.
  - title: Detect and handle history gaps
    done: false
    description: When History API returns 404 (historyId too old), trigger fallback
      sync. Enqueue backfill.gmail job with query based on last known sync time,
      or use default (newer_than:7d). Log warning about gap.
  - title: Implement backfill.gmail job handler
    done: false
    description: "Create server/crates/ashford-core/src/jobs/backfill_gmail.rs.
      Handler: parse payload (account_id, query, page_token), call messages.list
      API, enqueue ingest.gmail job for each message ID, if nextPageToken exists
      enqueue another backfill.gmail job for next page."
  - title: Use lower priority for backfill jobs
    done: false
    description: When enqueueing backfill.gmail jobs (both initial and pagination),
      use priority 0 or negative. This ensures real-time ingest.gmail jobs
      (higher priority) are processed first.
  - title: Add backfill trigger on account creation
    done: false
    description: In account creation flow (accounts.rs or a setup endpoint), after
      account is created with valid OAuth tokens, enqueue initial backfill.gmail
      job with configured query (e.g., newer_than:30d from config).
  - title: Update account state after successful sync
    done: false
    description: "After history.sync.gmail completes all pages, update account's
      state_json with: new historyId, last_sync_at timestamp, sync_status =
      'caught_up'."
  - title: Register new job handlers in dispatcher
    done: false
    description: Update jobs/mod.rs JobDispatcher to route history.sync.gmail and
      backfill.gmail job types to their respective handlers.
  - title: Write tests for history sync job
    done: false
    description: "Add tests for: normal History API flow, pagination handling, gap
      detection (404 response), historyId state update. Use mock Gmail
      responses."
  - title: Write tests for backfill job
    done: false
    description: "Add tests for: messages.list pagination, job enqueueing for each
      message, priority setting, handling empty results."
tags:
  - gmail
  - rust
---

This plan implements catchup and historical sync:

- history.sync.gmail job handler using Gmail History API
- Detection and handling of history gaps (404 when historyId too old)
- Fallback to search-based sync when history unavailable
- backfill.gmail job for initial account setup (last N days)
- Pagination handling for large result sets
- Lower priority for backfill jobs to not block real-time ingestion

Depends on: Message Ingestion plan (uses ingest.gmail job)

<!-- rmplan-generated-start -->
This plan implements catchup and historical sync:

- history.sync.gmail job handler using Gmail History API
- Detection and handling of history gaps (404 when historyId too old)
- Fallback to search-based sync when history unavailable
- backfill.gmail job for initial account setup (last N days)
- Pagination handling for large result sets
- Lower priority for backfill jobs to not block real-time ingestion

Depends on: Plan 11 (Message Ingestion) - uses ingest.gmail job and job dispatcher

## Acceptance Criteria

- [ ] history.sync.gmail job calls History API with correct startHistoryId
- [ ] All messagesAdded events result in ingest.gmail jobs being enqueued
- [ ] Pagination is handled correctly (all pages processed)
- [ ] Account historyId is updated after successful sync
- [ ] History gap (404) triggers backfill.gmail fallback
- [ ] backfill.gmail job imports messages using search query
- [ ] Backfill pagination creates follow-up jobs for next pages
- [ ] Backfill jobs use lower priority than real-time jobs
- [ ] All new code paths are covered by tests

## Files to Create

- `server/crates/ashford-core/src/jobs/history_sync_gmail.rs`
- `server/crates/ashford-core/src/jobs/backfill_gmail.rs`

## Files to Modify

- `server/crates/ashford-core/src/gmail/client.rs` - Add list_history, list_messages
- `server/crates/ashford-core/src/jobs/mod.rs` - Register new handlers
- `server/crates/ashford-core/src/accounts.rs` - Add backfill trigger on creation
<!-- rmplan-generated-end -->
