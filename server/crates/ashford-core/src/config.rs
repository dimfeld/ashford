use serde::Deserialize;
use std::{env, path::Path, path::PathBuf};
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub app: AppConfig,
    pub paths: PathsConfig,
    pub telemetry: TelemetryConfig,
    pub model: ModelConfig,
    pub discord: DiscordConfig,
    pub gmail: GmailConfig,
    pub imap: ImapConfig,
    pub policy: PolicyConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    pub service_name: String,
    pub port: u16,
    pub env: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PathsConfig {
    pub database: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TelemetryConfig {
    pub otlp_endpoint: Option<String>,
    pub export_traces: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelConfig {
    pub provider: String,
    pub model: String,
    pub temperature: f32,
    pub max_output_tokens: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DiscordConfig {
    pub bot_token: String,
    pub channel_id: String,
    #[serde(default)]
    pub whitelist: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GmailConfig {
    pub use_pubsub: bool,
    pub project_id: String,
    pub subscription: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ImapConfig {
    pub idle: bool,
    pub backfill_days: u32,
    pub archive_folder: String,
    pub snooze_folder: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PolicyConfig {
    #[serde(default)]
    pub approval_always: Vec<String>,
    pub confidence_default: f32,
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to read configuration file: {0}")]
    ConfigBuild(config::ConfigError),
    #[error("failed to parse configuration: {0}")]
    Deserialize(config::ConfigError),
    #[error("missing required environment variable {0}")]
    MissingEnvVar(String),
    #[error("invalid APP_PORT override: {0}")]
    InvalidPort(std::num::ParseIntError),
}

impl Config {
    /// Load configuration from the provided path, apply environment overrides, and
    /// resolve any `env:` indirections.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let raw = config::Config::builder()
            .add_source(config::File::from(path.as_ref()))
            .build()
            .map_err(ConfigError::ConfigBuild)?;

        let mut cfg: Config = raw.try_deserialize().map_err(ConfigError::Deserialize)?;
        cfg.apply_env_overrides()?;
        cfg.resolve_env_markers()?;
        cfg.expand_paths();
        Ok(cfg)
    }

    fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        if let Ok(port) = env::var("APP_PORT") {
            let port: u16 = port.parse().map_err(ConfigError::InvalidPort)?;
            self.app.port = port;
        }

        if let Ok(otlp) = env::var("OTLP_ENDPOINT") {
            self.telemetry.otlp_endpoint = Some(otlp);
        }

        if let Ok(model) = env::var("MODEL") {
            self.model.model = model;
        }

        if let Ok(token) = env::var("DISCORD_BOT_TOKEN") {
            self.discord.bot_token = token;
        }

        Ok(())
    }

    fn resolve_env_markers(&mut self) -> Result<(), ConfigError> {
        apply_env_marker(&mut self.app.service_name)?;
        apply_env_marker(&mut self.app.env)?;
        apply_env_marker(&mut self.model.provider)?;
        apply_env_marker(&mut self.model.model)?;
        apply_env_marker(&mut self.discord.bot_token)?;
        apply_env_marker(&mut self.discord.channel_id)?;
        for entry in &mut self.discord.whitelist {
            apply_env_marker(entry)?;
        }
        apply_env_marker(&mut self.gmail.project_id)?;
        apply_env_marker(&mut self.gmail.subscription)?;
        apply_env_marker(&mut self.imap.archive_folder)?;
        apply_env_marker(&mut self.imap.snooze_folder)?;
        apply_env_marker_path(&mut self.paths.database)?;
        if let Some(endpoint) = &mut self.telemetry.otlp_endpoint {
            apply_env_marker(endpoint)?;
        }
        Ok(())
    }

    fn expand_paths(&mut self) {
        let database_string = self.paths.database.to_string_lossy().to_string();
        let database = shellexpand::tilde(&database_string);
        self.paths.database = PathBuf::from(database.as_ref());
    }
}

fn apply_env_marker(value: &mut String) -> Result<(), ConfigError> {
    if let Some(rest) = value.strip_prefix("env:") {
        let resolved = env::var(rest).map_err(|_| ConfigError::MissingEnvVar(rest.to_string()))?;
        *value = resolved;
    }
    Ok(())
}

fn apply_env_marker_path(path: &mut PathBuf) -> Result<(), ConfigError> {
    let mut value = path.to_string_lossy().to_string();
    apply_env_marker(&mut value)?;
    *path = PathBuf::from(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::{fs, sync::Mutex};
    use tempfile::TempDir;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn write_config(contents: &str) -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().expect("temp dir");
        let path = dir.path().join("config.toml");
        fs::write(&path, contents).expect("write config");
        (dir, path)
    }

    fn with_env(vars: &[(&str, Option<&str>)], f: impl FnOnce()) {
        let _guard = ENV_LOCK.lock().expect("lock env");
        let saved: Vec<(String, Option<String>)> = vars
            .iter()
            .map(|(k, _)| (k.to_string(), env::var(k).ok()))
            .collect();

        for (key, value) in vars {
            match value {
                Some(v) => unsafe { env::set_var(key, v) },
                None => unsafe { env::remove_var(key) },
            }
        }

        f();

        for (key, value) in saved {
            match value {
                Some(v) => unsafe { env::set_var(&key, v) },
                None => unsafe { env::remove_var(&key) },
            }
        }
    }

    fn full_config_body(database_path: &str) -> String {
        format!(
            r#"
[app]
service_name = "ashford"
port = 17800
env = "dev"

[paths]
database = "{database_path}"

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
channel_id = "env:DISCORD_CHANNEL"
whitelist = ["env:WHITELIST_USER"]

[gmail]
use_pubsub = true
project_id = "env:GMAIL_PROJECT"
subscription = "env:GMAIL_SUB"

[imap]
idle = true
backfill_days = 30
archive_folder = "Archive"
snooze_folder = "Snoozed"

[policy]
approval_always = ["delete","forward"]
confidence_default = 0.7
"#
        )
    }

    #[test]
    fn load_config_expands_tilde_and_resolves_env_markers() {
        let (dir, path) = write_config(&full_config_body("env:DB_PATH"));
        let home_dir = dir.path().join("home");
        fs::create_dir_all(&home_dir).expect("create home dir");

        let expected_db = home_dir.join("db/ashford.db");
        with_env(
            &[
                ("APP_PORT", None),
                ("OTLP_ENDPOINT", None),
                ("MODEL", None),
                ("HOME", Some(home_dir.to_str().unwrap())),
                ("DB_PATH", Some("~/db/ashford.db")),
                ("DISCORD_BOT_TOKEN", Some("secret-token")),
                ("DISCORD_CHANNEL", Some("channel-123")),
                ("WHITELIST_USER", Some("user#1")),
                ("GMAIL_PROJECT", Some("project-1")),
                ("GMAIL_SUB", Some("sub-1")),
            ],
            || {
                let cfg = Config::load(&path).expect("config loads");
                assert_eq!(cfg.app.service_name, "ashford");
                assert_eq!(cfg.app.port, 17800);
                assert_eq!(cfg.paths.database, expected_db);
                assert_eq!(
                    cfg.telemetry.otlp_endpoint.as_deref(),
                    Some("http://localhost:4318")
                );
                assert_eq!(cfg.discord.bot_token, "secret-token");
                assert_eq!(cfg.discord.channel_id, "channel-123");
                assert_eq!(cfg.discord.whitelist, vec!["user#1".to_string()]);
                assert_eq!(cfg.gmail.project_id, "project-1");
                assert_eq!(cfg.gmail.subscription, "sub-1");
            },
        );
    }

    #[test]
    fn env_overrides_take_precedence() {
        let (_dir, path) = write_config(
            r#"
[app]
service_name = "ashford"
port = 12000
env = "dev"

[paths]
database = "/tmp/db.sqlite"

[telemetry]
otlp_endpoint = "http://example.com"
export_traces = false

[model]
provider = "vercel"
model = "file-model"
temperature = 0.1
max_output_tokens = 50

[discord]
bot_token = "file-token"
channel_id = "chan"
whitelist = []

[gmail]
use_pubsub = false
project_id = "proj"
subscription = "sub"

[imap]
idle = false
backfill_days = 1
archive_folder = "Archive"
snooze_folder = "Snoozed"

[policy]
approval_always = []
confidence_default = 0.5
"#,
        );

        with_env(
            &[
                ("APP_PORT", Some("19000")),
                ("OTLP_ENDPOINT", Some("http://override.local:4318")),
                ("MODEL", Some("env-model")),
                ("DISCORD_BOT_TOKEN", Some("env-token")),
            ],
            || {
                let cfg = Config::load(&path).expect("config loads");
                assert_eq!(cfg.app.port, 19000);
                assert_eq!(
                    cfg.telemetry.otlp_endpoint.as_deref(),
                    Some("http://override.local:4318")
                );
                assert_eq!(cfg.model.model, "env-model");
                assert_eq!(cfg.discord.bot_token, "env-token");
            },
        );
    }

    #[test]
    fn env_marker_without_variable_errors() {
        let (_dir, path) = write_config(
            r#"
[app]
service_name = "ashford"
port = 12000
env = "dev"

[paths]
database = "/tmp/db.sqlite"

[telemetry]
otlp_endpoint = "http://example.com"
export_traces = false

[model]
provider = "vercel"
model = "file-model"
temperature = 0.1
max_output_tokens = 50

[discord]
bot_token = "env:NEEDS_TOKEN"
channel_id = "chan"
whitelist = []

[gmail]
use_pubsub = false
project_id = "proj"
subscription = "sub"

[imap]
idle = false
backfill_days = 1
archive_folder = "Archive"
snooze_folder = "Snoozed"

[policy]
approval_always = []
confidence_default = 0.5
"#,
        );

        with_env(
            &[
                ("APP_PORT", None),
                ("OTLP_ENDPOINT", None),
                ("MODEL", None),
                ("DISCORD_BOT_TOKEN", None),
                ("NEEDS_TOKEN", None),
            ],
            || {
                let err = Config::load(&path).expect_err("missing env var should error");
                match err {
                    ConfigError::MissingEnvVar(name) => assert_eq!(name, "NEEDS_TOKEN"),
                    other => panic!("unexpected error: {other}"),
                }
            },
        );
    }

    #[test]
    fn invalid_port_override_is_reported() {
        let (_dir, path) = write_config(
            r#"
[app]
service_name = "ashford"
port = 12000
env = "dev"

[paths]
database = "/tmp/db.sqlite"

[telemetry]
otlp_endpoint = "http://example.com"
export_traces = false

[model]
provider = "vercel"
model = "file-model"
temperature = 0.1
max_output_tokens = 50

[discord]
bot_token = "token"
channel_id = "chan"
whitelist = []

[gmail]
use_pubsub = false
project_id = "proj"
subscription = "sub"

[imap]
idle = false
backfill_days = 1
archive_folder = "Archive"
snooze_folder = "Snoozed"

[policy]
approval_always = []
confidence_default = 0.5
"#,
        );

        with_env(&[("APP_PORT", Some("not-a-number"))], || {
            let err = Config::load(&path).expect_err("invalid port should error");
            assert!(matches!(err, ConfigError::InvalidPort(_)));
        });
    }
}
