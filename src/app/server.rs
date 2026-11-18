use std::{
    future::{Future, IntoFuture},
    io,
    time::Duration,
};

use axum::Router;

use tokio::{net::TcpListener, sync::oneshot, time::timeout};

/// Describes how the HTTP server stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownOutcome {
    /// The server stopped without an external signal.
    ServerStopped,

    /// Graceful shutdown completed successfully.
    Graceful,

    /// The graceful shutdown deadline was exceeded.
    DeadlineExceeded,
}

/// Run the HTTP server with a configurable
/// graceful shutdown deadline.
///
/// This function is shared by production and tests.
pub async fn serve_with_shutdown<S>(
    listener: TcpListener,
    app: Router,
    shutdown_signal: S,
    graceful_timeout: Duration,
) -> io::Result<ShutdownOutcome>
where
    S: Future<Output = ()> + Send,
{
    // Channel used to initiate Axum graceful shutdown.
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    // Axum 0.8 exposes IntoFuture here.
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .into_future();

    tokio::pin!(server);

    tokio::select! {
        // Server stopped before shutdown signal.
        result = &mut server => {
            result?;

            Ok(ShutdownOutcome::ServerStopped)
        }

        // External shutdown signal received.
        _ = shutdown_signal => {
            // Notify Axum to stop accepting requests.
            let _ = shutdown_tx.send(());

            // Wait for active requests, up to deadline.
            match timeout(
                graceful_timeout,
                &mut server,
            )
            .await
            {
                Ok(result) => {
                    result?;

                    Ok(ShutdownOutcome::Graceful)
                }

                Err(_) => {
                    Ok(ShutdownOutcome::DeadlineExceeded)
                }
            }
        }
    }
}
