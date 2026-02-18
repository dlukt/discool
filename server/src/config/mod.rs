mod settings;

pub use settings::{Config, DatabaseConfig, LogConfig, LogFormat, ServerConfig};

pub fn load() -> Result<Config, config::ConfigError> {
    use config::{Config as ConfigBuilder, Environment, File, FileFormat};

    let mut builder = ConfigBuilder::builder()
        .add_source(File::new("/etc/discool/config.toml", FileFormat::Toml).required(false))
        .add_source(File::new("config.toml", FileFormat::Toml).required(false));

    if let Ok(path) = std::env::var("DISCOOL_CONFIG") {
        builder = builder.add_source(File::new(&path, FileFormat::Toml).required(true));
    }

    builder
        .add_source(
            Environment::with_prefix("DISCOOL")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?
        .try_deserialize::<Config>()
}
