use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub log: LogConfig,
}

impl Config {
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

        Ok(())
    }

    pub fn log_summary(&self) {
        tracing::info!(
            host = %self.server.host,
            port = self.server.port,
            log_level = %self.log.level,
            log_format = %self.log.format,
            "Configuration loaded"
        );
    }
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
        let cfg = Config::default();
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
    }

    #[test]
    fn validate_rejects_port_0() {
        let mut cfg = Config::default();
        cfg.server.port = 0;
        let err = cfg.validate().unwrap_err();
        assert!(err.to_string().contains("server.port"));
    }

    #[test]
    fn redact_secret_hides_value() {
        assert_eq!(
            redact_secret("postgres://user:pass@localhost/db"),
            "[REDACTED]"
        );
    }
}
