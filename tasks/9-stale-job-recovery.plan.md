---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: Stale Job Recovery
goal: Implement background task to detect and recover jobs stuck in 'running'
  state due to worker crashes
id: 9
uuid: 62073031-b34a-4c7d-bfcf-d28c4f1695e7
status: pending
priority: medium
container: false
temp: false
dependencies:
  - 2
parent: 1
issue: []
docs: []
createdAt: 2025-11-29T01:31:38.085Z
updatedAt: 2025-11-29T01:31:38.085Z
tasks: []
tags:
  - queue
  - rust
---

Add a background sweeper task that periodically checks for stale jobs (jobs in 'running' state with old heartbeat timestamps) and recovers them by resetting to 'queued' state with appropriate backoff.

Key requirements:
- Run every 60 seconds
- Detect jobs where state='running' AND heartbeat_at < (now - 2 minutes)
- Reset stale jobs to state='queued', increment attempts, set not_before for backoff
- Jobs exceeding max_attempts should transition to 'failed' instead
- Log recovered jobs at warning level with job details
- Include unit tests for stale detection logic
