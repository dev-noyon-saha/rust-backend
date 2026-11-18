use backend_project::http;

use tokio::net::TcpListener;
use tracing::{Level, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // Register API routes
    let app = http::router();

    // Configure server address
    let address = "127.0.0.1:8000";

    // Create TCP listener
    let listener = TcpListener::bind(address).await?;

    info!("Dwar AI Backend started");
    info!("Listening on http://{}", address);

    // Start server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Dwar AI Backend stopped");

    Ok(())
}

// Handle Ctrl+C shutdown signal
async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_ok() {
        info!("Shutdown signal received");
    }
}
