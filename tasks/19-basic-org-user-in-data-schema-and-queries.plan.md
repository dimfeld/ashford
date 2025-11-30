---
# yaml-language-server: $schema=https://raw.githubusercontent.com/dimfeld/llmutils/main/schema/rmplan-plan-schema.json
title: basic org/user in data schema and queries
goal: ""
id: 19
uuid: 13f3a268-8ee5-4354-a336-f22820baf179
simple: false
status: pending
priority: high
temp: false
createdAt: 2025-11-30T08:36:03.718Z
updatedAt: 2025-11-30T08:36:03.718Z
tasks: []
tags: []
---

Although the application currently only supports a single user, we should add organization ID and user ID columns to all the relevant tables.

We don't really need to support this properly at this point, but building all the queries to filter on these columns will make it much easier to add the support in the future.

For now, we should add the columns, set all the values to 1, and hardcode the current organization and current user in the application to ID 1.

We don't need actual tables for the organizations and users yet. That can be added in the future if we ever actually need to make this system multi-user.
