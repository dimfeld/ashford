---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 2: Gmail Ingest (Pub/Sub + History)"
goal: Implement Gmail integration with Pub/Sub notifications, History API sync,
  and message ingestion
id: 3
uuid: b93a0b33-fccb-4f57-8c97-002039917c44
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies:
  - 2
parent: 1
issue: []
docs:
  - docs/gmail_integration.md
  - docs/data_model.md
planGeneratedAt: 2025-11-29T01:23:11.905Z
promptsGeneratedAt: 2025-11-29T01:23:11.905Z
createdAt: 2025-11-29T01:21:26.709Z
updatedAt: 2025-11-29T01:23:11.905Z
tasks:
  - title: Add Gmail API client
    done: false
    description: Integrate google-gmail crate or build REST client for Gmail API.
      Implement authentication with OAuth2 tokens. Handle token refresh.
  - title: Implement OAuth token storage
    done: false
    description: Store OAuth credentials securely - either in OS keychain or
      encrypted in database. Support token refresh flow.
  - title: Create accounts management
    done: false
    description: API endpoints for account CRUD. Store config_json with
      provider-specific settings. Store state_json for sync state (historyId).
  - title: Build Pub/Sub endpoint
    done: false
    description: HTTP endpoint to receive Gmail Pub/Sub push notifications. Decode
      base64 message data, extract historyId or messageId, enqueue ingest.gmail
      job.
  - title: Implement ingest.gmail job handler
    done: false
    description: Job handler that fetches message details from Gmail API, parses
      headers/body, upserts thread and message records, enqueues classify job.
  - title: Add History API sync
    done: false
    description: Implement history.sync.gmail job that calls Gmail History API with
      startHistoryId, processes messagesAdded/labelsAdded events, updates
      state_json with new historyId.
  - title: Handle History gaps
    done: false
    description: Detect when historyId is too old (404 from History API). Fall back
      to full sync or incremental search-based sync.
  - title: Create backfill.gmail job
    done: false
    description: Job for initial account setup that fetches last N days of messages
      using Gmail search (newer_than:Xd). Batch process and enqueue classify
      jobs.
  - title: Parse email content
    done: false
    description: "Extract and store: from_email, from_name, to/cc/bcc lists,
      subject, snippet, headers, body_plain, body_html. Handle MIME parsing for
      multipart messages."
  - title: Implement thread management
    done: false
    description: Upsert threads table with provider_thread_id, update
      last_message_at, maintain snippet from most recent message.
tags:
  - gmail
  - pubsub
  - rust
---

Gmail integration for receiving and storing emails:
- Account configuration and OAuth token management
- Pub/Sub message handler endpoint
- ingest.gmail job handler
- History API integration for catchup/gap filling
- Message and thread storage (accounts, threads, messages tables)
- Backfill job for initial account setup
