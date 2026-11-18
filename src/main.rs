use std::{io, time::Duration};

use backend_project::{
    app::{
        server::{ShutdownOutcome, serve_with_shutdown},
        state::AppState,
    },
    config::Settings,
    http,
    infrastructure::{cache::redis as redis_cache, database::postgres},
};

use tokio::{net::TcpListener, time::timeout};

use tracing::{Level, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize structured logging.
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // 2. Load .env.
    dotenvy::dotenv().ok();

    // 3. Load and validate configuration.
    let settings = Settings::from_env()?;

    info!(
        environment = %settings.environment,
        "Configuration loaded and validated"
    );

    // 4. Connect PostgreSQL using typed settings.
    let db_pool = postgres::connect_with_config(&settings.database)
        .await
        .map_err(|_| io::Error::other("PostgreSQL initialization failed"))?;

    info!(
        max_connections = settings.database.max_connections,
        min_connections = settings.database.min_connections,
        "PostgreSQL connected"
    );

    // 5. Initialize Redis.
    let redis_manager = redis_cache::connect_with_config(&settings.redis)
        .await
        .map_err(|_| io::Error::other("Redis initialization failed"))?;

    info!("Redis connected");

    // 6. Build centralized AppState.
    let app_state = AppState::with_health_timeout(
        db_pool.clone(),
        redis_manager,
        settings.health.check_timeout,
    );

    info!("Application state initialized");

    // 7. Create production router.
    let app = http::router_with_state(app_state);

    // 8. Bind configured server address.
    let address = settings.server.bind_address();

    let listener = TcpListener::bind(&address).await?;

    info!(
        address = %address,
        environment = %settings.environment,
        "Dwar AI Backend started"
    );

    // 9. Start HTTP server with shared
    // graceful shutdown lifecycle.
    let server_result = serve_with_shutdown(
        listener,
        app,
        shutdown_signal(),
        settings.server.graceful_shutdown_timeout,
    )
    .await;

    // 10. Log shutdown outcome.
    match &server_result {
        Ok(ShutdownOutcome::Graceful) => {
            info!("Graceful HTTP shutdown completed");
        }

        Ok(ShutdownOutcome::DeadlineExceeded) => {
            warn!("HTTP graceful shutdown deadline exceeded");
        }

        Ok(ShutdownOutcome::ServerStopped) => {
            info!("HTTP server stopped");
        }

        Err(_) => {
            warn!("HTTP server terminated with an error");
        }
    }

    // 11. Attempt bounded PostgreSQL cleanup.
    match timeout(Duration::from_secs(5), db_pool.close()).await {
        Ok(()) => {
            info!("PostgreSQL pool closed");
        }

        Err(_) => {
            warn!("PostgreSQL pool close deadline exceeded");
        }
    }

    // 12. Propagate server errors.
    server_result?;

    info!("Dwar AI Backend stopped");

    Ok(())
}

/// Handle Ctrl+C.
async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutdown signal received");
        }

        Err(_) => {
            warn!("Ctrl+C signal listener failed");
        }
    }
}
