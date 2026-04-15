use axum::{
    Router,
    routing::{get, post},
    serve::Serve,
};
use tokio::net::TcpListener;

use crate::{
    DbPool,
    routes::{health_check, subscribe},
};

pub fn run(
    listener: TcpListener,
    pool: DbPool,
) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        // Add the connection to the application state so it can be accessed in handlers
        .with_state(pool);
    let server = axum::serve(listener, app);
    Ok(server)
}
