use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
    serve::Serve,
};
use diesel_async::AsyncPgConnection;
use tokio::net::TcpListener;

use crate::routes::{health_check, subscribe};


pub fn run(
    listener: TcpListener,
    connection: AsyncPgConnection,
) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    // Wrap the connection in an `Arc` to share it across multiple handlers
    let connection = Arc::new(connection);

    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        // Add the connection to the application state so it can be accessed in handlers
        .with_state(connection);
    let server = axum::serve(listener, app);
    Ok(server)
}
