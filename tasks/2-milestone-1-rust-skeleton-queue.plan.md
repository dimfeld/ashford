---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: "Milestone 1: Rust Skeleton & Queue"
goal: Set up Rust project structure, libsql database with migrations, job queue,
  health endpoint, and OpenTelemetry
id: 2
uuid: 85389e56-6e82-4b14-b6ab-153a10439a6e
generatedBy: agent
status: pending
priority: high
container: false
temp: false
dependencies: []
parent: 1
issue: []
docs:
  - docs/job_queue.md
  - docs/data_model.md
  - docs/configuration.md
  - docs/opentelemetry.md
planGeneratedAt: 2025-11-29T01:23:11.754Z
promptsGeneratedAt: 2025-11-29T01:23:11.754Z
createdAt: 2025-11-29T01:21:26.633Z
updatedAt: 2025-11-29T01:23:11.754Z
tasks:
  - title: Initialize Cargo workspace
    done: false
    description: "Create Cargo workspace with crates for: core library, queue runner
      binary, API server binary. Set up shared dependencies and workspace-level
      settings."
  - title: Set up libsql connection
    done: false
    description: Add libsql dependency, create database connection pool, implement
      connection helper with retry logic. Support both local file and remote
      Turso endpoints.
  - title: Create database migrations
    done: false
    description: "Implement migration system and create initial migration with all
      tables from data_model.md: accounts, threads, messages, decisions,
      actions, action_links, jobs, job_steps, discord_whitelist,
      deterministic_rules, llm_rules, directions, rules_chat_sessions,
      rules_chat_messages."
  - title: Implement TOML configuration
    done: false
    description: Create config structs matching configuration.md schema. Support
      TOML file loading, environment variable overrides (APP_PORT,
      OTLP_ENDPOINT, etc.), and 'env:' prefix for secrets.
  - title: Build job queue core
    done: false
    description: "Implement queue operations: enqueue job with idempotency key,
      claim job (atomic state transition to 'running'), heartbeat updates,
      complete/fail job, retry with exponential backoff and jitter."
  - title: Create queue worker loop
    done: false
    description: Build worker that polls for queued jobs, claims and executes them,
      handles panics gracefully, updates heartbeat during long operations, and
      respects not_before scheduling.
  - title: Add job step tracking
    done: false
    description: "Implement job_steps recording for observability: start_step(),
      finish_step(), store step results. Link steps to parent job."
  - title: Set up OpenTelemetry
    done: false
    description: Initialize OpenTelemetry with OTLP exporter. Configure resource
      attributes (service.name, version, environment). Create span helpers for
      email.receive, email.classify, email.action, queue.job.
  - title: Implement /healthz endpoint
    done: false
    description: Create HTTP server with axum. Add /healthz endpoint that checks
      database connectivity and returns service status. Include version info in
      response.
  - title: Add structured logging
    done: false
    description: Set up tracing subscriber with JSON formatting. Attach trace IDs to
      log lines. Configure log levels via environment.
tags:
  - libsql
  - otel
  - queue
  - rust
---

Initial Rust project setup with core infrastructure:
- Cargo workspace structure
- libsql connection and migrations for all tables
- Jobs and job_steps tables for durable queue
- Basic queue runner with claim/heartbeat/complete flow
- /healthz endpoint
- OpenTelemetry tracing initialization
- TOML configuration loading with env overrides
