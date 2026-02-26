use serde::Deserialize;
use std::path::Path;

use libp2p::{Multiaddr, PeerId, multiaddr::Protocol};

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub log: LogConfig,
    #[serde(default)]
    pub database: Option<DatabaseConfig>,
    #[serde(default)]
    pub metrics: Option<MetricsConfig>,
    #[serde(default)]
    pub backup: Option<BackupConfig>,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub avatar: AvatarConfig,
    #[serde(default)]
    pub p2p: P2pConfig,
    #[serde(default)]
    pub email: EmailConfig,
}

impl Config {
    pub fn metrics_enabled(&self) -> bool {
        self.metrics.as_ref().is_some_and(|m| m.enabled)
    }

    pub fn validate(&self) -> Result<(), ConfigValidationError> {
        if self.server.host.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "server.host",
                "must not be empty",
            ));
        }

        if self.server.port == 0 {
            return Err(ConfigValidationError::new(
                "server.port",
                "must be between 1 and 65535",
            ));
        }

        tracing_subscriber::EnvFilter::try_new(&self.log.level).map_err(|err| {
            ConfigValidationError::new("log.level", format!("invalid filter: {err}"))
        })?;

        let db = self.database.as_ref().ok_or_else(|| {
            ConfigValidationError::new(
                "database.url",
                "required — set database.url in config or DISCOOL_DATABASE__URL env var",
            )
        })?;

        if db.url.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "database.url",
                "must not be empty",
            ));
        }

        if !db.url.starts_with("postgres://")
            && !db.url.starts_with("postgresql://")
            && !db.url.starts_with("sqlite://")
            && !db.url.starts_with("sqlite:")
        {
            return Err(ConfigValidationError::new(
                "database.url",
                "must start with postgres://, postgresql://, sqlite://, or sqlite:",
            ));
        }

        if db.max_connections == 0 {
            return Err(ConfigValidationError::new(
                "database.max_connections",
                "must be >= 1",
            ));
        }

        if self.auth.session_ttl_hours == 0 {
            return Err(ConfigValidationError::new(
                "auth.session_ttl_hours",
                "must be >= 1",
            ));
        }
        if self.auth.challenge_ttl_seconds == 0 {
            return Err(ConfigValidationError::new(
                "auth.challenge_ttl_seconds",
                "must be >= 1",
            ));
        }

        if self.avatar.max_size_bytes == 0 {
            return Err(ConfigValidationError::new(
                "avatar.max_size_bytes",
                "must be >= 1",
            ));
        }

        if self.avatar.upload_dir.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "avatar.upload_dir",
                "must not be empty",
            ));
        }
        if let Err(err) = std::fs::create_dir_all(&self.avatar.upload_dir) {
            return Err(ConfigValidationError::new(
                "avatar.upload_dir",
                format!("failed to create directory: {err}"),
            ));
        }

        if self.p2p.enabled {
            if self.p2p.listen_host.trim().is_empty() {
                return Err(ConfigValidationError::new(
                    "p2p.listen_host",
                    "must not be empty",
                ));
            }
            if self.p2p.listen_port == 0 {
                return Err(ConfigValidationError::new(
                    "p2p.listen_port",
                    "must be between 1 and 65535",
                ));
            }
            if self.p2p.listen_port == self.server.port {
                return Err(ConfigValidationError::new(
                    "p2p.listen_port",
                    "must be different from server.port",
                ));
            }
            let identity_key_path = self.p2p.identity_key_path.trim();
            if identity_key_path.is_empty() {
                return Err(ConfigValidationError::new(
                    "p2p.identity_key_path",
                    "must not be empty",
                ));
            }
            let parent_dir = Path::new(identity_key_path)
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            if let Err(err) = std::fs::create_dir_all(parent_dir) {
                return Err(ConfigValidationError::new(
                    "p2p.identity_key_path",
                    format!("failed to create parent directory: {err}"),
                ));
            }
            if self.p2p.discovery_retry_initial_secs == 0 {
                return Err(ConfigValidationError::new(
                    "p2p.discovery_retry_initial_secs",
                    "must be >= 1",
                ));
            }
            if self.p2p.discovery_retry_max_secs < self.p2p.discovery_retry_initial_secs {
                return Err(ConfigValidationError::new(
                    "p2p.discovery_retry_max_secs",
                    "must be >= p2p.discovery_retry_initial_secs",
                ));
            }
            if self.p2p.discovery_refresh_interval_secs == 0 {
                return Err(ConfigValidationError::new(
                    "p2p.discovery_refresh_interval_secs",
                    "must be >= 1",
                ));
            }
            if self.p2p.gossip_mesh_n_low == 0 {
                return Err(ConfigValidationError::new(
                    "p2p.gossip_mesh_n_low",
                    "must be >= 1",
                ));
            }
            if self.p2p.gossip_mesh_n == 0 {
                return Err(ConfigValidationError::new(
                    "p2p.gossip_mesh_n",
                    "must be >= 1",
                ));
            }
            if self.p2p.gossip_mesh_n_high == 0 {
                return Err(ConfigValidationError::new(
                    "p2p.gossip_mesh_n_high",
                    "must be >= 1",
                ));
            }
            if !(self.p2p.gossip_mesh_n_low <= self.p2p.gossip_mesh_n
                && self.p2p.gossip_mesh_n <= self.p2p.gossip_mesh_n_high)
            {
                return Err(ConfigValidationError::new(
                    "p2p.gossip_mesh_n",
                    "must satisfy p2p.gossip_mesh_n_low <= p2p.gossip_mesh_n <= p2p.gossip_mesh_n_high",
                ));
            }
            for bootstrap_peer in &self.p2p.bootstrap_peers {
                let trimmed = bootstrap_peer.trim();
                if trimmed.is_empty() {
                    return Err(ConfigValidationError::new(
                        "p2p.bootstrap_peers",
                        "must not contain empty entries",
                    ));
                }

                let multiaddr: Multiaddr = trimmed.parse().map_err(|err| {
                    ConfigValidationError::new(
                        "p2p.bootstrap_peers",
                        format!("invalid multiaddr '{trimmed}': {err}"),
                    )
                })?;
                let mut has_peer_id = false;
                for protocol in multiaddr.iter() {
                    if let Protocol::P2p(peer_id) = protocol {
                        let _ = PeerId::from_bytes(&peer_id.to_bytes()).map_err(|err| {
                            ConfigValidationError::new(
                                "p2p.bootstrap_peers",
                                format!("invalid peer id in '{trimmed}': {err}"),
                            )
                        })?;
                        has_peer_id = true;
                    }
                }

                if !has_peer_id {
                    return Err(ConfigValidationError::new(
                        "p2p.bootstrap_peers",
                        format!("multiaddr '{trimmed}' must include /p2p/<peer-id>"),
                    ));
                }
            }
        }

        if self.email.smtp_host.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "email.smtp_host",
                "must not be empty",
            ));
        }
        if self.email.smtp_port == 0 {
            return Err(ConfigValidationError::new(
                "email.smtp_port",
                "must be between 1 and 65535",
            ));
        }
        if self.email.from_address.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "email.from_address",
                "must not be empty",
            ));
        }
        if self.email.from_name.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "email.from_name",
                "must not be empty",
            ));
        }
        if self.email.verification_url_base.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "email.verification_url_base",
                "must not be empty",
            ));
        }
        if self.email.recovery_url_base.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "email.recovery_url_base",
                "must not be empty",
            ));
        }
        if self.email.token_ttl_seconds == 0 {
            return Err(ConfigValidationError::new(
                "email.token_ttl_seconds",
                "must be >= 1",
            ));
        }
        if self.email.start_rate_limit_per_hour == 0 {
            return Err(ConfigValidationError::new(
                "email.start_rate_limit_per_hour",
                "must be >= 1",
            ));
        }
        if self.email.verify_rate_limit_per_hour == 0 {
            return Err(ConfigValidationError::new(
                "email.verify_rate_limit_per_hour",
                "must be >= 1",
            ));
        }
        if self.email.server_secret.trim().is_empty() {
            return Err(ConfigValidationError::new(
                "email.server_secret",
                "must not be empty",
            ));
        }
        if self.email.smtp_username.is_some() ^ self.email.smtp_password.is_some() {
            return Err(ConfigValidationError::new(
                "email.smtp_username/email.smtp_password",
                "must be set together",
            ));
        }

        if let Some(output_dir) = self
            .backup
            .as_ref()
            .and_then(|b| b.output_dir.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            && let Err(err) = std::fs::create_dir_all(output_dir)
        {
            tracing::warn!(
                error = %err,
                output_dir = %output_dir,
                "Failed to create backup output directory"
            );
        }

        Ok(())
    }

    pub fn log_summary(&self) {
        let db_url = self
            .database
            .as_ref()
            .map(|db| redact_secret(&db.url))
            .unwrap_or("[not configured]");

        tracing::info!(
            host = %self.server.host,
            port = self.server.port,
            log_level = %self.log.level,
            log_format = %self.log.format,
            database_url = %db_url,
            p2p_enabled = self.p2p.enabled,
            p2p_listen_host = %self.p2p.listen_host,
            p2p_listen_port = self.p2p.listen_port,
            p2p_identity_key_path = %self.p2p.identity_key_path,
            p2p_bootstrap_peer_count = self.p2p.bootstrap_peers.len(),
            p2p_discovery_retry_initial_secs = self.p2p.discovery_retry_initial_secs,
            p2p_discovery_retry_max_secs = self.p2p.discovery_retry_max_secs,
            p2p_discovery_retry_jitter_millis = self.p2p.discovery_retry_jitter_millis,
            p2p_discovery_refresh_interval_secs = self.p2p.discovery_refresh_interval_secs,
            p2p_gossip_mesh_n_low = self.p2p.gossip_mesh_n_low,
            p2p_gossip_mesh_n = self.p2p.gossip_mesh_n,
            p2p_gossip_mesh_n_high = self.p2p.gossip_mesh_n_high,
            "Configuration loaded"
        );
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct MetricsConfig {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BackupConfig {
    #[serde(default)]
    pub output_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_session_ttl_hours")]
    pub session_ttl_hours: u64,
    #[serde(default = "default_challenge_ttl_seconds")]
    pub challenge_ttl_seconds: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            session_ttl_hours: default_session_ttl_hours(),
            challenge_ttl_seconds: default_challenge_ttl_seconds(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AvatarConfig {
    #[serde(default = "default_avatar_upload_dir")]
    pub upload_dir: String,
    #[serde(default = "default_avatar_max_size_bytes")]
    pub max_size_bytes: usize,
}

impl Default for AvatarConfig {
    fn default() -> Self {
        Self {
            upload_dir: default_avatar_upload_dir(),
            max_size_bytes: default_avatar_max_size_bytes(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct P2pConfig {
    #[serde(default = "default_p2p_enabled")]
    pub enabled: bool,
    #[serde(default = "default_p2p_listen_host")]
    pub listen_host: String,
    #[serde(default = "default_p2p_listen_port")]
    pub listen_port: u16,
    #[serde(default = "default_p2p_identity_key_path")]
    pub identity_key_path: String,
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,
    #[serde(default = "default_p2p_discovery_retry_initial_secs")]
    pub discovery_retry_initial_secs: u64,
    #[serde(default = "default_p2p_discovery_retry_max_secs")]
    pub discovery_retry_max_secs: u64,
    #[serde(default = "default_p2p_discovery_retry_jitter_millis")]
    pub discovery_retry_jitter_millis: u64,
    #[serde(default = "default_p2p_discovery_refresh_interval_secs")]
    pub discovery_refresh_interval_secs: u64,
    #[serde(default = "default_p2p_gossip_mesh_n_low")]
    pub gossip_mesh_n_low: usize,
    #[serde(default = "default_p2p_gossip_mesh_n")]
    pub gossip_mesh_n: usize,
    #[serde(default = "default_p2p_gossip_mesh_n_high")]
    pub gossip_mesh_n_high: usize,
}

impl Default for P2pConfig {
    fn default() -> Self {
        Self {
            enabled: default_p2p_enabled(),
            listen_host: default_p2p_listen_host(),
            listen_port: default_p2p_listen_port(),
            identity_key_path: default_p2p_identity_key_path(),
            bootstrap_peers: Vec::new(),
            discovery_retry_initial_secs: default_p2p_discovery_retry_initial_secs(),
            discovery_retry_max_secs: default_p2p_discovery_retry_max_secs(),
            discovery_retry_jitter_millis: default_p2p_discovery_retry_jitter_millis(),
            discovery_refresh_interval_secs: default_p2p_discovery_refresh_interval_secs(),
            gossip_mesh_n_low: default_p2p_gossip_mesh_n_low(),
            gossip_mesh_n: default_p2p_gossip_mesh_n(),
            gossip_mesh_n_high: default_p2p_gossip_mesh_n_high(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailConfig {
    #[serde(default = "default_email_smtp_host")]
    pub smtp_host: String,
    #[serde(default = "default_email_smtp_port")]
    pub smtp_port: u16,
    #[serde(default)]
    pub smtp_username: Option<String>,
    #[serde(default)]
    pub smtp_password: Option<String>,
    #[serde(default = "default_email_from_address")]
    pub from_address: String,
    #[serde(default = "default_email_from_name")]
    pub from_name: String,
    #[serde(default = "default_email_verification_url_base")]
    pub verification_url_base: String,
    #[serde(default = "default_email_recovery_url_base")]
    pub recovery_url_base: String,
    #[serde(default = "default_email_token_ttl_seconds")]
    pub token_ttl_seconds: u64,
    #[serde(default = "default_email_start_rate_limit_per_hour")]
    pub start_rate_limit_per_hour: u32,
    #[serde(default = "default_email_verify_rate_limit_per_hour")]
    pub verify_rate_limit_per_hour: u32,
    #[serde(default = "default_email_server_secret")]
    pub server_secret: String,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            smtp_host: default_email_smtp_host(),
            smtp_port: default_email_smtp_port(),
            smtp_username: None,
            smtp_password: None,
            from_address: default_email_from_address(),
            from_name: default_email_from_name(),
            verification_url_base: default_email_verification_url_base(),
            recovery_url_base: default_email_recovery_url_base(),
            token_ttl_seconds: default_email_token_ttl_seconds(),
            start_rate_limit_per_hour: default_email_start_rate_limit_per_hour(),
            verify_rate_limit_per_hour: default_email_verify_rate_limit_per_hour(),
            server_secret: default_email_server_secret(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String, // Required — no default. Validation catches missing.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

fn default_max_connections() -> u32 {
    5
}

fn default_session_ttl_hours() -> u64 {
    168
}

fn default_challenge_ttl_seconds() -> u64 {
    300
}

fn default_avatar_upload_dir() -> String {
    "./data/avatars".to_string()
}

fn default_avatar_max_size_bytes() -> usize {
    2 * 1024 * 1024
}

fn default_p2p_enabled() -> bool {
    true
}

fn default_p2p_listen_host() -> String {
    "0.0.0.0".to_string()
}

fn default_p2p_listen_port() -> u16 {
    4001
}

fn default_p2p_identity_key_path() -> String {
    "./data/p2p/identity.key".to_string()
}

fn default_p2p_discovery_retry_initial_secs() -> u64 {
    2
}

fn default_p2p_discovery_retry_max_secs() -> u64 {
    60
}

fn default_p2p_discovery_retry_jitter_millis() -> u64 {
    500
}

fn default_p2p_discovery_refresh_interval_secs() -> u64 {
    15
}

fn default_p2p_gossip_mesh_n_low() -> usize {
    5
}

fn default_p2p_gossip_mesh_n() -> usize {
    6
}

fn default_p2p_gossip_mesh_n_high() -> usize {
    12
}

fn default_email_smtp_host() -> String {
    "stub".to_string()
}

fn default_email_smtp_port() -> u16 {
    1025
}

fn default_email_from_address() -> String {
    "no-reply@discool.local".to_string()
}

fn default_email_from_name() -> String {
    "Discool".to_string()
}

fn default_email_verification_url_base() -> String {
    "http://localhost:3000/api/v1/auth/recovery-email/verify".to_string()
}

fn default_email_recovery_url_base() -> String {
    "http://localhost:3000/".to_string()
}

fn default_email_token_ttl_seconds() -> u64 {
    900
}

fn default_email_start_rate_limit_per_hour() -> u32 {
    5
}

fn default_email_verify_rate_limit_per_hour() -> u32 {
    20
}

fn default_email_server_secret() -> String {
    "change-me-in-production".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default)]
    pub format: LogFormat,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: LogFormat::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    #[default]
    Json,
    Pretty,
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFormat::Json => f.write_str("json"),
            LogFormat::Pretty => f.write_str("pretty"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigValidationError {
    field: &'static str,
    message: String,
}

impl ConfigValidationError {
    fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ConfigValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ConfigValidationError {}

#[allow(dead_code)]
pub(crate) fn redact_secret(_value: &str) -> &'static str {
    "[REDACTED]"
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_log_level() -> String {
    "info".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::{Config as ConfigBuilder, File, FileFormat};

    #[test]
    fn log_format_deserializes() {
        #[derive(Deserialize)]
        struct Wrapper {
            format: LogFormat,
        }

        let cfg: Wrapper = ConfigBuilder::builder()
            .add_source(File::from_str("format = \"json\"", FileFormat::Toml))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        assert_eq!(cfg.format, LogFormat::Json);

        let cfg: Wrapper = ConfigBuilder::builder()
            .add_source(File::from_str("format = \"pretty\"", FileFormat::Toml))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        assert_eq!(cfg.format, LogFormat::Pretty);
    }

    #[test]
    fn log_format_rejects_invalid_value() {
        let err = ConfigBuilder::builder()
            .add_source(File::from_str(
                "[log]\nformat = \"invalid\"",
                FileFormat::Toml,
            ))
            .build()
            .unwrap()
            .try_deserialize::<Config>()
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("invalid") || msg.contains("unknown variant"),
            "unexpected error message: {msg}"
        );
    }

    #[test]
    fn default_config_validates() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn config_loads_from_empty_toml_with_defaults() {
        let cfg: Config = ConfigBuilder::builder()
            .add_source(File::from_str("", FileFormat::Toml))
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();

        assert_eq!(cfg.server.host, "0.0.0.0");
        assert_eq!(cfg.server.port, 3000);
        assert_eq!(cfg.log.level, "info");
        assert_eq!(cfg.log.format, LogFormat::Json);
        assert!(cfg.database.is_none());
        assert_eq!(cfg.avatar.upload_dir, "./data/avatars");
        assert_eq!(cfg.avatar.max_size_bytes, 2 * 1024 * 1024);
        assert!(cfg.p2p.enabled);
        assert_eq!(cfg.p2p.listen_host, "0.0.0.0");
        assert_eq!(cfg.p2p.listen_port, 4001);
        assert_eq!(cfg.p2p.identity_key_path, "./data/p2p/identity.key");
        assert!(cfg.p2p.bootstrap_peers.is_empty());
        assert_eq!(cfg.p2p.discovery_retry_initial_secs, 2);
        assert_eq!(cfg.p2p.discovery_retry_max_secs, 60);
        assert_eq!(cfg.p2p.discovery_retry_jitter_millis, 500);
        assert_eq!(cfg.p2p.discovery_refresh_interval_secs, 15);
        assert_eq!(cfg.p2p.gossip_mesh_n_low, 5);
        assert_eq!(cfg.p2p.gossip_mesh_n, 6);
        assert_eq!(cfg.p2p.gossip_mesh_n_high, 12);
    }

    #[test]
    fn metrics_config_defaults_disabled() {
        assert!(!MetricsConfig::default().enabled);
    }

    #[test]
    fn validate_rejects_port_0() {
        let mut cfg = Config::default();
        cfg.server.port = 0;
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("server.port"));
    }

    #[test]
    fn validate_rejects_p2p_port_0() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        cfg.p2p.listen_port = 0;
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("p2p.listen_port"));
    }

    #[test]
    fn validate_rejects_p2p_port_matching_server_port() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        cfg.p2p.listen_port = cfg.server.port;
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("p2p.listen_port"));
    }

    #[test]
    fn validate_skips_p2p_checks_when_disabled() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        cfg.p2p.enabled = false;
        cfg.p2p.listen_host = String::new();
        cfg.p2p.listen_port = 0;
        cfg.p2p.identity_key_path = String::new();
        cfg.p2p.discovery_retry_initial_secs = 0;
        cfg.p2p.discovery_retry_max_secs = 0;
        cfg.p2p.discovery_refresh_interval_secs = 0;
        cfg.p2p.bootstrap_peers = vec!["not-a-multiaddr".to_string()];
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_creates_p2p_identity_parent_dir() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut root = std::env::temp_dir();
        root.push(format!("discool-config-p2p-{nanos}"));
        let nested = root.join("nested");
        let identity_path = nested.join("identity.key");
        cfg.p2p.identity_key_path = identity_path.to_string_lossy().to_string();

        assert!(cfg.validate().is_ok());
        assert!(nested.is_dir());
        let _ = std::fs::remove_dir_all(root);
    }

    fn valid_bootstrap_peer() -> String {
        let peer_id = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        format!("/ip4/127.0.0.1/tcp/4001/p2p/{peer_id}")
    }

    #[test]
    fn validate_rejects_bootstrap_peer_without_peer_id() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        cfg.p2p.bootstrap_peers = vec!["/ip4/127.0.0.1/tcp/4001".to_string()];

        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("p2p.bootstrap_peers"));
    }

    #[test]
    fn validate_accepts_bootstrap_peers_with_peer_id() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        cfg.p2p.bootstrap_peers = vec![valid_bootstrap_peer()];

        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn validate_rejects_discovery_retry_max_less_than_initial() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        cfg.p2p.discovery_retry_initial_secs = 10;
        cfg.p2p.discovery_retry_max_secs = 5;

        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("p2p.discovery_retry_max_secs"));
    }

    #[test]
    fn validate_rejects_invalid_gossip_mesh_bounds() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        cfg.p2p.gossip_mesh_n_low = 7;
        cfg.p2p.gossip_mesh_n = 6;
        cfg.p2p.gossip_mesh_n_high = 12;

        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("p2p.gossip_mesh_n"));
    }

    #[test]
    fn redact_secret_hides_value() {
        assert_eq!(
            redact_secret("postgres://user:pass@localhost/db"),
            "[REDACTED]"
        );
    }

    #[test]
    fn validate_rejects_missing_database() {
        let cfg = Config::default();
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("database.url"));
    }

    #[test]
    fn validate_rejects_empty_database_url() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "   ".to_string(),
            max_connections: 5,
        });
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("database.url"));
    }

    #[test]
    fn validate_rejects_invalid_database_url_scheme() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "mysql://localhost/db".to_string(),
            max_connections: 5,
        });
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("database.url"));
    }

    #[test]
    fn validate_accepts_postgres_and_sqlite_urls() {
        for url in ["postgres://localhost/db", "sqlite::memory:"] {
            let mut cfg = Config::default();
            cfg.database = Some(DatabaseConfig {
                url: url.to_string(),
                max_connections: 5,
            });
            assert!(cfg.validate().is_ok(), "expected url to validate: {url}");
        }
    }

    #[test]
    fn validate_rejects_max_connections_0() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 0,
        });
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("database.max_connections"));
    }

    #[test]
    fn validate_rejects_zero_avatar_max_size() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        cfg.avatar.max_size_bytes = 0;

        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("avatar.max_size_bytes"));
    }

    #[test]
    fn validate_rejects_empty_avatar_upload_dir() {
        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            max_connections: 5,
        });
        cfg.avatar.upload_dir = " ".to_string();

        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("avatar.upload_dir"));
    }

    #[test]
    fn log_summary_redacts_database_url() {
        use std::io::Write;
        use std::sync::{Arc, Mutex};

        use tracing_subscriber::Layer;
        use tracing_subscriber::layer::SubscriberExt;

        #[derive(Clone)]
        struct Buf(Arc<Mutex<Vec<u8>>>);

        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Buf {
            type Writer = BufGuard;

            fn make_writer(&'a self) -> Self::Writer {
                BufGuard(self.0.clone())
            }
        }

        struct BufGuard(Arc<Mutex<Vec<u8>>>);

        impl Write for BufGuard {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(Buf(buffer.clone()))
                .with_filter(tracing_subscriber::filter::LevelFilter::INFO),
        );

        let mut cfg = Config::default();
        cfg.database = Some(DatabaseConfig {
            url: "postgres://user:pass@localhost/db".to_string(),
            max_connections: 5,
        });

        tracing::subscriber::with_default(subscriber, || {
            cfg.log_summary();
        });

        let output = String::from_utf8(buffer.lock().unwrap().clone()).unwrap();
        assert!(output.contains("[REDACTED]"), "unexpected output: {output}");
        assert!(
            !output.contains("user:pass"),
            "log output leaked secret: {output}"
        );
    }
}
