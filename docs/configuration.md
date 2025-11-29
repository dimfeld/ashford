
- Config via TOML file, environment overrides.

- Database path points to libsql db file or remote endpoint.
- Secrets:

    - OAuth tokens stored in OS keychain or encrypted store.

    - TOML may refer to env: indirections.

Example:

```toml
    
    
    [app]
    service_name = "ashford"
    port = 17800              # web UI
    env = "dev"               # dev|prod
    
    [paths]
    database = "~/Library/Application Support/ai-mail-agent/app.db"
    
    [telemetry]
    otlp_endpoint = "http://localhost:4318"
    export_traces = true
    
    [model]
    provider = "vercel"
    model = "gemini-1.5-pro"
    temperature = 0.2
    max_output_tokens = 1024
    
    [discord]
    bot_token = "env:DISCORD_BOT_TOKEN"
    channel_id = "1234567890"
    whitelist = ["Daniel#1234"]
    
    [gmail]
    use_pubsub = true
    project_id = "your-gcp-project"
    subscription = "gmail-sub"
    
    [imap]
    idle = true
    backfill_days = 30
    archive_folder = "Archive"
    snooze_folder = "Snoozed"
    
    [policy]
    approval_always = ["delete","forward","auto_reply","escalate"]
    confidence_default = 0.7
    

**Env overrides (examples)**
    
    
    APP_PORT=17800
    OTLP_ENDPOINT=https://api.honeycomb.io:443
    MODEL="gemini-1.5-pro"
    DISCORD_BOT_TOKEN=...
    

