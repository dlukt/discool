use discool_server::{AppState, handlers};
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

    // Initialize tracing before validation so warn-level validation messages (e.g. backup output_dir)
    // are actually visible.
    init_tracing(&config.log);

    if let Err(err) = config.validate() {
        eprintln!("ERROR: Invalid configuration: {err}");
        std::process::exit(1);
    }

    config.log_summary();

    let config = std::sync::Arc::new(config);

    let Some(db_config) = config.database.as_ref() else {
        tracing::error!("Config validation bug: database section missing after validate()");
        std::process::exit(1);
    };
    let pool = match discool_server::db::init_pool(db_config).await {
        Ok(pool) => pool,
        Err(err) => {
            let err_msg = redact_db_url_in_error(&err, &db_config.url);
            tracing::error!(error = %err_msg, "Failed to connect to database");
            std::process::exit(1);
        }
    };

    if let Err(err) = discool_server::db::run_migrations(&pool).await {
        let err_msg = redact_db_url_in_message(&err.to_string(), &db_config.url);
        tracing::error!(error = %err_msg, "Failed to run database migrations");
        std::process::exit(1);
    }

    let backend = discool_server::db::DatabaseBackend::from_url(&db_config.url)
        .map(|b| format!("{b:?}"))
        .unwrap_or_else(|e| e);
    tracing::info!(
        backend = %backend,
        pool_size = pool.size(),
        pool_idle = pool.num_idle(),
        "Database connected and migrations complete"
    );

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
            pool.close().await;
            std::process::exit(1);
        }
    };

    let state = AppState {
        config: config.clone(),
        pool: pool.clone(),
        start_time: std::time::Instant::now(),
    };
    let app = handlers::router(state.clone());

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
        pool.close().await;
        std::process::exit(1);
    }

    pool.close().await;
    tracing::info!("Database pool closed");
}

fn redact_db_url_in_error(err: &sqlx::Error, db_url: &str) -> String {
    redact_db_url_in_message(&err.to_string(), db_url)
}

fn redact_db_url_in_message(msg: &str, db_url: &str) -> String {
    if msg.contains(db_url) {
        msg.replace(db_url, "[REDACTED_DATABASE_URL]")
    } else {
        msg.to_string()
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
