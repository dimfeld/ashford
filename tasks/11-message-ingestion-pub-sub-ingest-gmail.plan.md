---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: Message Ingestion (Pub/Sub + ingest.gmail)
goal: Implement Pub/Sub webhook and ingest.gmail job handler for real-time
  message ingestion
id: 11
uuid: 5b35e65e-3d87-45e5-98bc-45312701e05b
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies:
  - 10
parent: 3
issue: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
  - docs/job_queue.md
planGeneratedAt: 2025-11-29T07:43:49.905Z
promptsGeneratedAt: 2025-11-29T07:43:49.905Z
createdAt: 2025-11-29T07:42:47.263Z
updatedAt: 2025-11-29T07:43:49.905Z
tasks:
  - title: Create Pub/Sub webhook types
    done: false
    description: "Create request/response types for Gmail Pub/Sub webhook:
      PubSubPushMessage (with message.data, message.attributes, subscription),
      decoded notification (emailAddress, historyId). Add to a new webhooks
      module or gmail module."
  - title: Implement Pub/Sub webhook endpoint
    done: false
    description: Add POST /webhooks/gmail-pubsub endpoint in
      ashford-server/src/main.rs. Decode base64 message data, look up account by
      email, enqueue history.sync.gmail job with the historyId. Return 200 to
      acknowledge.
  - title: Add JobQueue to AppState
    done: false
    description: Update AppState struct in main.rs to include JobQueue. Update
      router initialization to pass queue to handlers that need it.
  - title: Create job dispatcher infrastructure
    done: false
    description: Create server/crates/ashford-core/src/jobs/mod.rs with
      JobDispatcher struct implementing JobExecutor trait. Route jobs by
      job.job_type string to specific handlers. Start with ingest.gmail handler.
  - title: Create thread repository
    done: false
    description: "Create server/crates/ashford-core/src/threads.rs with
      ThreadRepository. Implement: upsert(account_id, provider_thread_id,
      subject, snippet, last_message_at), get_by_provider_id(account_id,
      provider_thread_id), update_last_message_at(id, timestamp)."
  - title: Create message repository
    done: false
    description: "Create server/crates/ashford-core/src/messages.rs with
      MessageRepository. Implement: upsert(message_data),
      get_by_provider_id(account_id, provider_message_id), exists(account_id,
      provider_message_id) for dedup checking."
  - title: Implement email content parser
    done: false
    description: "Create server/crates/ashford-core/src/gmail/parser.rs. Parse Gmail
      message payload: extract from_email/from_name from From header, parse
      to/cc/bcc lists, extract subject, find body_plain and body_html from
      payload.parts (handle multipart recursively)."
  - title: Implement ingest.gmail job handler
    done: false
    description: "Create server/crates/ashford-core/src/jobs/ingest_gmail.rs.
      Handler: parse payload (account_id, message_id), fetch message from Gmail
      API, parse content, upsert thread (create if needed), upsert message. Use
      idempotency key: ingest.gmail:{account_id}:{message_id}."
  - title: Wire job dispatcher in main.rs
    done: false
    description: Replace NoopExecutor with JobDispatcher in main.rs worker
      initialization. Pass required dependencies (Database, GmailClient factory)
      to dispatcher.
  - title: Handle Gmail API errors in ingest job
    done: false
    description: "In ingest_gmail.rs, handle: 404 (message deleted, mark as Fatal),
      401 (token expired, refresh and retry as Retryable), 429 (rate limit,
      Retryable with backoff), 5xx (Retryable)."
  - title: Export new modules from lib.rs
    done: false
    description: Update lib.rs to export jobs, threads, and messages modules.
  - title: Write integration tests for ingest flow
    done: false
    description: "Add tests for: webhook endpoint decoding, ingest.gmail job
      execution with mock Gmail responses, thread/message upsert behavior,
      idempotency (same message twice)."
tags:
  - gmail
  - pubsub
  - rust
---

This plan implements real-time message ingestion:

- Pub/Sub webhook endpoint to receive Gmail notifications
- Job dispatcher infrastructure to route jobs by type
- ingest.gmail job handler that fetches message from Gmail API
- Email content parsing (from, to, subject, body_plain, body_html)
- Thread and message upsert operations
- Idempotency handling to prevent duplicate processing

Depends on: Gmail API Client & Account Management plan

<!-- rmplan-generated-start -->
This plan implements real-time message ingestion:

- Pub/Sub webhook endpoint to receive Gmail notifications
- Job dispatcher infrastructure to route jobs by type
- ingest.gmail job handler that fetches message from Gmail API
- Email content parsing (from, to, subject, body_plain, body_html)
- Thread and message upsert operations
- Idempotency handling to prevent duplicate processing

Depends on: Plan 10 (Gmail API Client & Account Management)

## Acceptance Criteria

- [ ] Pub/Sub webhook receives notifications and returns 200
- [ ] Webhook enqueues history.sync.gmail job with correct historyId
- [ ] ingest.gmail job fetches message from Gmail API
- [ ] Email content (from, to, subject, bodies) is correctly parsed
- [ ] Thread records are created/updated with correct last_message_at
- [ ] Message records are upserted with all fields populated
- [ ] Duplicate messages are handled via idempotency
- [ ] Gmail API errors (401, 404, 429) are handled appropriately
- [ ] All new code paths are covered by tests

## Files to Create

- `server/crates/ashford-core/src/jobs/mod.rs`
- `server/crates/ashford-core/src/jobs/ingest_gmail.rs`
- `server/crates/ashford-core/src/gmail/parser.rs`
- `server/crates/ashford-core/src/threads.rs`
- `server/crates/ashford-core/src/messages.rs`

## Files to Modify

- `server/crates/ashford-server/src/main.rs` - Add webhook route, wire dispatcher
- `server/crates/ashford-core/src/lib.rs` - Export new modules
- `server/crates/ashford-core/src/gmail/mod.rs` - Export parser
<!-- rmplan-generated-end -->
