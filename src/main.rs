use backend_project::{
    app::state::AppState,
    config::Settings,
    http,
    infrastructure::{cache::redis as redis_cache, database::postgres},
};

use tokio::net::TcpListener;
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize structured logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    // 2. Load .env file
    dotenvy::dotenv().ok();

    // 3. Load application configuration
    let settings = Settings::from_env()?;
    info!("Application configuration loaded");

    // 4. Connect PostgreSQL
    let db_pool = postgres::connect(&settings.database_url).await?;

    info!("PostgreSQL connected successfully");

    // 5. Connect Redis
    let redis_manager = redis_cache::connect(&settings.redis_url).await?;

    info!("Redis connected successfully");

    // 6. Initialize centralized AppState
    let app_state = AppState::new(db_pool.clone(), redis_manager);

    info!("Application state initialized");

    // 7. Register production HTTP router
    let app = http::router_with_state(app_state);

    // 8. Configure server address
    let address = "127.0.0.1:8000";

    // 9. Start TCP listener
    let listener = TcpListener::bind(address).await?;

    info!("Dwar AI Backend started");
    info!("Listening on http://{}", address);

    // 10. Start Axum server
    let server_result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;

    // 11. Close PostgreSQL pool
    info!("Closing PostgreSQL connection pool");
    db_pool.close().await;

    // 12. Propagate server errors
    server_result?;

    info!("Dwar AI Backend stopped");

    Ok(())
}

// Handle graceful shutdown
async fn shutdown_signal() {
    match tokio::signal::ctrl_c().await {
        Ok(()) => info!("Shutdown signal received"),
        Err(error) => {
            tracing::error!("Failed to listen for shutdown: {}", error);
        }
    }
}
