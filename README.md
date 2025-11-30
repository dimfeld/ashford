# Ashford

An intelligent email automation agent for Gmail that automatically classifies and acts on incoming emails using deterministic rules and LLM-powered intelligence.

## Overview

Ashford is a self-hosted, single-user email automation system that:

- Automatically classifies incoming Gmail messages using rule-based logic and AI
- Executes configured actions (archive, label, delete, etc.) with approval workflows
- Provides a web interface for managing rules and viewing action history
- Integrates with Discord for notifications and interactive approvals
- Maintains full traceability and undo capabilities for all actions

## Architecture

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Gmail API     │────▶│   Rust Agent     │────▶│    Discord      │
│   (Pub/Sub)     │     │   (axum + queue) │     │   (approvals)   │
└─────────────────┘     └────────┬─────────┘     └─────────────────┘
                                 │
                                 ▼
                        ┌──────────────────┐
                        │   SvelteKit UI   │
                        │   (web interface)│
                        └──────────────────┘
```

### Components

- **Rust Agent Service** (`server/`) - Core backend with job queue, Gmail integration, rule engine, and Discord bot
- **SvelteKit Web App** (`web/`) - Web interface for rules management, action history, and settings
- **libsql Database** - Durable storage for messages, rules, decisions, and job queue

## Tech Stack

**Backend:**
- Rust (2024 edition)
- Axum HTTP server
- libsql (SQLite fork)
- Tokio async runtime
- OpenTelemetry for observability

**Frontend:**
- SvelteKit 2
- TypeScript
- Tailwind CSS
- Bits UI components

## Getting Started

### Prerequisites

- Rust (1.85+)
- Node.js 20+ / Bun
- pnpm
- Gmail API credentials
- Discord bot token (optional, for approvals)

### Configuration

Create a configuration file (see `docs/configuration.md` for all options):

```toml
database_path = "./ashford.db"

[gmail]
credentials_path = "./credentials.json"
token_path = "./token.json"

[discord]
bot_token = "your-discord-bot-token"
channel_id = "your-channel-id"
```

### Running the Server

```bash
cd server
cargo run --release
```

### Running the Web UI

```bash
cd web
pnpm install
pnpm dev
```

## Project Structure

```
ashford/
├── server/                 # Rust backend
│   ├── crates/
│   │   ├── ashford-core/   # Core domain logic, queue, database
│   │   └── ashford-server/ # HTTP API server
│   └── migrations/         # SQL migrations
├── web/                    # SvelteKit frontend
│   └── src/
│       ├── routes/         # Pages and layouts
│       └── lib/            # Shared components
├── docs/                   # Architecture documentation
└── tasks/                  # Development milestones (rmplan)
```

## Documentation

See the `docs/` directory for detailed documentation:

- [Overview](docs/overview.md) - High-level architecture
- [Data Model](docs/data_model.md) - Database schema
- [Decision Engine](docs/decision_engine.md) - Classification logic
- [Rules Engine](docs/rules_engine.md) - Rule evaluation
- [Gmail Integration](docs/gmail_integration.md) - Gmail API setup
- [Discord](docs/discord.md) - Discord bot integration
- [Web UI](docs/web_ui.md) - Frontend specification
- [Configuration](docs/configuration.md) - All config options

### Milestones

1. Rust skeleton & queue
2. Gmail ingest (Pub/Sub + History)
3. Rule engine & decision pipeline
4. Gmail actions
5. Discord bot integration
6. SvelteKit web UI
7. Rules assistant (chat interface)

## License

MIT
