use discool_server::handlers;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() {
    let config = match discool_server::config::load() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("ERROR: Failed to load configuration: {err}");
            std::process::exit(1);
        }
    };

    if let Err(err) = config.validate() {
        eprintln!("ERROR: Invalid configuration: {err}");
        std::process::exit(1);
    }

    init_tracing(&config.log);
    config.log_summary();

    let config = std::sync::Arc::new(config);
    let listener = match TcpListener::bind((config.server.host.as_str(), config.server.port)).await
    {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(
                %err,
                host = %config.server.host,
                port = config.server.port,
                "Failed to bind TCP listener"
            );
            std::process::exit(1);
        }
    };

    let app = handlers::router(config.clone());

    let addr = if let Ok(ip) = config.server.host.parse::<std::net::IpAddr>() {
        std::net::SocketAddr::new(ip, config.server.port).to_string()
    } else {
        format!("{}:{}", config.server.host, config.server.port)
    };
    tracing::info!(%addr, "Starting server");

    if let Err(err) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        tracing::error!(%err, "Server error");
        std::process::exit(1);
    }
}

fn init_tracing(log_config: &discool_server::config::LogConfig) {
    // RUST_LOG always wins (debugging escape hatch).
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&log_config.level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    match log_config.format {
        discool_server::config::LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().json().with_target(true))
                .init();
        }
        discool_server::config::LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().pretty().with_target(true))
                .init();
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(%err, "Failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut sigterm) => {
                sigterm.recv().await;
            }
            Err(err) => {
                tracing::error!(%err, "Failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received");
}
