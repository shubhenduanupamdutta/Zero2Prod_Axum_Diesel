use axum::{
    Router,
    routing::{get, post},
    serve::Serve,
};
use tokio::net::TcpListener;

use crate::routes::{health_check, subscribe};


pub fn run(listener: TcpListener) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe));
    let server = axum::serve(listener, app);
    Ok(server)
}
