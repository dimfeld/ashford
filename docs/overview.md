## **1) Goals & Non-Goals**
  

### **Goals**

- Automatically classify and act on incoming email for at least **Gmail**.
- Support **two rule types**:

    1. **Deterministic rules**: match on email address, domain, subject regex, headers, etc. and perform configured actions.

    2. **LLM rules**: natural-language, prompt-powered rules for emails not matched by deterministic filters.

- Allow **rule configuration via a chat-style interface** in the web app:

    - User describes what they want ("All receipts from X → archive after labeling 'Receipts'").

    - The system proposes concrete rules (deterministic or LLM-backed) for approval.
- Provide a **SvelteKit (Bun) web UI** to:

    - View history of actions taken.

    - Manage both deterministic rules and LLM rules.

    - Run the conversational "rules assistant."
- Provide a **Rust backend service**:

    - libsql-backed durable queue for ingest, classification, and actions.

    - Gmail Pub/Sub ingestion + History API for catchup.

    - Discord bot integration for approvals and logging.

    - OpenTelemetry traces end-to-end.
- Ensure **durable, idempotent execution** of all jobs:

    - Message ingest.

    - LLM classification.

    - Action execution.

    - Approvals / undo.

  

### **Non-Goals (for v1)**

- Multi-tenant SaaS or external auth; assume **single local user**.
- Complex analytics dashboard; use tracing backend for deep dives.
- IMAP support
- Rich WYSIWYG editor for rules; the primary UX is structured forms + chat assistant.
* * *

## **2) High-Level Architecture**

  

### **2.1 Components**

1. **Rust Agent Service** (core backend):

    - HTTP REST API (localhost-only by default).

    - Durable job queue using **libsql**.

    - Gmail integration (Pub/Sub + History).

    - LLM integration via a provider-agnostic crate (e.g., genai).

    - Rule engine (deterministic + LLM rules).

    - Discord bot client (logging + approvals).

    - OpenTelemetry tracing.

2. **SvelteKit Web App (Bun)**:

    - Runs on Bun (Node-compatible).

    - Uses SvelteKit "remote functions" (server actions/endpoints) to call Rust REST API.

    - Pages:

        - Actions history / detail.

        - Rules (deterministic + LLM).

        - Rules assistant chat.

        - Settings (read-only view of config).

3. **External Services**:

    - **Gmail**: Pub/Sub notifications + History API for gap filling + REST for actions.

    - **Discord**: Bot in a single channel:

        - Logs all actions (including "safe" ones).

        - Provides interactive **approval** for "dangerous" actions.

    - **OpenTelemetry** collector (Jaeger/Honeycomb or similar).

4. **Storage**:

    - **libsql** database holding:

        - Accounts, threads, messages.

        - Rules (deterministic + LLM rule definitions).

        - AI decisions.

        - Actions and undo links.

        - Jobs and job steps.

        - Discord whitelist and metadata.

        - Chat history for rules assistant (optional but recommended).

  

### **2.2 Key Flows**

1. **Message Ingestion**:

    - Gmail Pub/Sub notifies of new mail.

    - Rust service pulls details via Gmail API and persists message/ thread data.

    - Enqueues a classify job.

2. **Classification & Rule Evaluation**:

    - Queue worker pulls classify job.

    - Loads all applicable deterministic rules → evaluates:

        - If any deterministic rule matches → produce deterministic action(s).

    - If no deterministic rule matches:

        - Evaluate **LLM rules**:

            - Construct LLM prompt (directions + rule definitions + email context).

            - Receive JSON decision.

    - Persist decisions record.

3. **Policy & Approvals**:

    - Derived actions are tagged as:

        - **Safe**: reversible and/or explicitly allowed by deterministic rule.

        - **Dangerous**: irreversible and not explicitly allowed by a deterministic rule.

    - For safe actions above a confidence threshold (for LLM rules) → enqueue action.gmail job directly.

    - For dangerous actions (or low confidence) → enqueue approval.notify job.

4. **Discord Interaction**:

    - approval.notify job posts an embed to Discord with:

        - Proposed action(s), parameters, confidence, rationale.

    - Discord buttons:

        - Approve → enqueue corresponding action job(s).

        - Reject → mark action as rejected; no execution.

    - All actions, whether auto-executed or approved/rejected, are logged to Discord.

5. **Action Execution & Undo**:

    - action.gmail job performs Gmail operation(s).

    - Records outcome in actions table and links to decision.

    - Undo triggered from Discord or web UI:

        - Enqueue undo job → derive inverse action from stored undo_hint and/or Gmail pre-images → attempt reversal.


## **3) Development & Milestones (Rust + SvelteKit)**

1. **Milestone 1 -- Rust Skeleton & Queue******

    - libsql connection + migrations.

    - Jobs / job_steps tables.

    - Basic queue runner + /healthz endpoint.

    - OpenTelemetry initialization.

2. **Milestone 2 -- Gmail ingest (Pub/Sub + History)******

    - Account configuration.

    - Pub/Sub message handler → ingest.gmail jobs.

    - History-based catchup.

3. **Milestone 3 -- Rule Engine & Decision pipeline******

    - Deterministic rule evaluation.

    - LLM decision engine wired via genai.

    - Dangerous action policy implementation.

4. **Milestone 4 -- Actions (Gmail)******

    - Implement archive, labels, read/unread, delete, snooze, star, restore, forward/reply.

    - Record actions and decisions.

    - Undo support for reversible operations.

5. **Milestone 5 -- Discord bot******

    - Logging all actions.

    - Approval embeds with buttons.

    - Undo commands.

6. **Milestone 6 -- SvelteKit UI******

    - Actions history and detail pages.

    - Rules list page.

    - Settings page.

    - Remote functions to Rust APIs.

7. **Milestone 7 -- Rules Assistant******

    - Backend endpoints for assistant.

    - SvelteKit chat UI.

    - Round-trip: message → proposed rules → apply.


