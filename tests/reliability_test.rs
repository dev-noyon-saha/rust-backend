use std::{
    io,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{Router, extract::State, routing::get};

use backend_project::{
    app::{
        server::{ShutdownOutcome, serve_with_shutdown},
        state::AppState,
    },
    config::{RedisConfig, Settings},
    http,
    infrastructure::{cache::redis as redis_cache, database::postgres},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Notify, oneshot},
    time::timeout,
};

/// Shared test state for deliberately slow requests.
#[derive(Clone)]
struct SlowRequestState {
    started: Arc<Notify>,
    release: Arc<Notify>,
}

/// Handler that waits until the test releases it.
async fn slow_handler(State(state): State<SlowRequestState>) -> &'static str {
    // Notify the test that the handler has started.
    state.started.notify_one();

    // Remain active until explicitly released.
    state.release.notified().await;

    "request completed"
}

/// Make a real HTTP/1.1 request using a TCP socket.
///
/// Connection: close ensures we can read the response
/// until EOF without waiting for keep-alive.
async fn http_get(address: SocketAddr, path: &str) -> io::Result<String> {
    let mut stream = TcpStream::connect(address).await?;

    let request = format!(
        "GET {path} HTTP/1.1\r\n\
         Host: localhost\r\n\
         Connection: close\r\n\
         \r\n"
    );

    stream.write_all(request.as_bytes()).await?;

    let mut response = Vec::new();

    stream.read_to_end(&mut response).await?;

    Ok(String::from_utf8_lossy(&response).into_owned())
}

/// Load the local configuration for integration tests.
fn local_settings() -> Settings {
    dotenvy::dotenv().ok();

    Settings::from_env().expect("Valid .env configuration is required")
}

// ------------------------------------------------------
// Test 01 — Graceful Shutdown
// ------------------------------------------------------

#[tokio::test]
async fn graceful_shutdown_finishes_active_request() {
    let state = SlowRequestState {
        started: Arc::new(Notify::new()),
        release: Arc::new(Notify::new()),
    };

    let router = Router::new()
        .route("/slow", get(slow_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind test server");

    let address = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Run the same server lifecycle as production.
    let server_task = tokio::spawn(serve_with_shutdown(
        listener,
        router,
        async move {
            let _ = shutdown_rx.await;
        },
        Duration::from_secs(5),
    ));

    // Start an HTTP request that will remain active.
    let request_task = tokio::spawn(async move { http_get(address, "/slow").await });

    // Wait until the handler has started.
    timeout(Duration::from_secs(3), state.started.notified())
        .await
        .expect("Slow request did not start");

    // Trigger server shutdown.
    shutdown_tx
        .send(())
        .expect("Failed to send shutdown signal");

    // The server should still be waiting for the request.
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(
        !server_task.is_finished(),
        "Server stopped before the active request finished"
    );

    // Allow the active request to finish.
    state.release.notify_one();

    let response = timeout(Duration::from_secs(3), request_task)
        .await
        .expect("HTTP request timed out")
        .expect("HTTP task failed")
        .expect("HTTP connection failed");

    assert!(
        response.starts_with("HTTP/1.1 200"),
        "Unexpected HTTP response"
    );

    assert!(response.contains("request completed"));

    let outcome = timeout(Duration::from_secs(3), server_task)
        .await
        .expect("Server shutdown timed out")
        .expect("Server task failed")
        .expect("Server returned an error");

    assert_eq!(outcome, ShutdownOutcome::Graceful);
}

// ------------------------------------------------------
// Test 02 — Graceful Shutdown Deadline
// ------------------------------------------------------

#[tokio::test]
async fn shutdown_deadline_prevents_indefinite_waiting() {
    let state = SlowRequestState {
        started: Arc::new(Notify::new()),
        release: Arc::new(Notify::new()),
    };

    let router = Router::new()
        .route("/slow", get(slow_handler))
        .with_state(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind test server");

    let address = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Deliberately short shutdown deadline.
    let server_task = tokio::spawn(serve_with_shutdown(
        listener,
        router,
        async move {
            let _ = shutdown_rx.await;
        },
        Duration::from_millis(200),
    ));

    let request_task = tokio::spawn(async move { http_get(address, "/slow").await });

    // Ensure the handler is already running.
    timeout(Duration::from_secs(3), state.started.notified())
        .await
        .expect("Slow handler did not start");

    // Trigger shutdown while the request is blocked.
    shutdown_tx
        .send(())
        .expect("Failed to send shutdown signal");

    // Server must return when its deadline expires.
    let outcome = timeout(Duration::from_secs(3), server_task)
        .await
        .expect("Shutdown exceeded the test timeout")
        .expect("Server task failed")
        .expect("Server returned an error");

    assert_eq!(outcome, ShutdownOutcome::DeadlineExceeded);

    // Clean up the test's blocked request.
    state.release.notify_one();

    request_task.abort();

    let _ = request_task.await;
}

// ------------------------------------------------------
// Test 03 — PostgreSQL Pool Limits
// Requires running PostgreSQL.
// ------------------------------------------------------

#[tokio::test]
async fn postgres_pool_respects_acquire_timeout() {
    let settings = local_settings();

    // Use a separate pool for this test.
    let mut config = settings.database.clone();

    config.max_connections = 1;
    config.min_connections = 1;
    config.acquire_timeout = Duration::from_millis(200);

    let pool = postgres::connect_with_config(&config)
        .await
        .expect("PostgreSQL connection failed");

    // Verify max connection configuration.
    assert_eq!(pool.options().get_max_connections(), 1);

    // Acquire the only available connection.
    let held_connection = pool.acquire().await.expect("Failed to acquire connection");

    let started = Instant::now();

    // Attempting a second acquisition should time out.
    let result = timeout(Duration::from_secs(3), pool.acquire())
        .await
        .expect("Outer test deadline exceeded");

    assert!(
        matches!(result, Err(sqlx::Error::PoolTimedOut)),
        "Expected SQLx PoolTimedOut"
    );

    assert!(started.elapsed() < Duration::from_secs(3));

    // Release the held connection.
    drop(held_connection);

    // The pool should become usable again.
    let connection = pool
        .acquire()
        .await
        .expect("Pool did not recover after release");

    drop(connection);

    pool.close().await;
}

// ------------------------------------------------------
// Test 04 — Redis Unresponsive Endpoint
// Does not interrupt the real Redis container.
// ------------------------------------------------------

#[tokio::test]
async fn redis_connection_has_bounded_timeout() {
    // Reserve a local TCP port without responding
    // to the Redis protocol.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to create unresponsive endpoint");

    let address = listener.local_addr().unwrap();

    let redis_url = format!("redis://127.0.0.1:{}/0", address.port());

    let mut config = RedisConfig::new(redis_url);

    config.operation_timeout = Duration::from_millis(250);

    let started = Instant::now();

    // Redis initialization must fail without hanging.
    let result = timeout(
        Duration::from_secs(3),
        redis_cache::connect_with_config(&config),
    )
    .await
    .expect("Redis connector exceeded test deadline");

    assert!(result.is_err(), "Unresponsive Redis endpoint was accepted");

    assert!(started.elapsed() < Duration::from_secs(3));

    drop(listener);
}

// ------------------------------------------------------
// Test 05 — Real HTTP Health Endpoint
// Requires running PostgreSQL and Redis.
// ------------------------------------------------------

#[tokio::test]
async fn real_http_health_reports_dependency_failure() {
    let settings = local_settings();

    // Create isolated database pool.
    let db = postgres::connect_with_config(&settings.database)
        .await
        .expect("PostgreSQL connection failed");

    // Create Redis manager.
    let redis_manager = redis_cache::connect_with_config(&settings.redis)
        .await
        .expect("Redis connection failed");

    // Build production AppState.
    let state =
        AppState::with_health_timeout(db.clone(), redis_manager, settings.health.check_timeout);

    let app = http::router_with_state(state);

    // Use a random available local port.
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind HTTP test server");

    let address = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let server_task = tokio::spawn(serve_with_shutdown(
        listener,
        app,
        async move {
            let _ = shutdown_rx.await;
        },
        Duration::from_secs(5),
    ));

    // First request: both dependencies healthy.
    let healthy_response = timeout(Duration::from_secs(5), http_get(address, "/health"))
        .await
        .expect("Healthy request timed out")
        .expect("Healthy request failed");

    assert!(healthy_response.starts_with("HTTP/1.1 200"));

    assert!(healthy_response.contains("\"postgresql\":\"healthy\""));

    assert!(healthy_response.contains("\"redis\":\"healthy\""));

    // Simulate PostgreSQL unavailability by closing
    // only this test's pool, not the database server.
    db.close().await;

    // Second request: database is unavailable.
    let unhealthy_response = timeout(Duration::from_secs(5), http_get(address, "/health"))
        .await
        .expect("Unhealthy request timed out")
        .expect("Unhealthy request failed");

    assert!(unhealthy_response.starts_with("HTTP/1.1 503"));

    assert!(unhealthy_response.contains("\"postgresql\":\"unhealthy\""));

    assert!(unhealthy_response.contains("\"redis\":\"healthy\""));

    assert!(unhealthy_response.contains("SERVICE_UNAVAILABLE"));

    // Shut down the isolated test server.
    shutdown_tx
        .send(())
        .expect("Failed to send shutdown signal");

    let outcome = timeout(Duration::from_secs(5), server_task)
        .await
        .expect("Test server shutdown timed out")
        .expect("Test server task failed")
        .expect("Test server returned an error");

    assert_eq!(outcome, ShutdownOutcome::Graceful);
}
