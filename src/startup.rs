use axum::{
    Router,
    routing::{get, post},
    serve::Serve,
};
use tokio::net::TcpListener;

use crate::{
    DbPool,
    routes::{health_check, subscribe},
    telemetry::apply_tracing_with_req_id_middleware,
};

pub fn run(
    listener: TcpListener,
    pool: DbPool,
) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(pool);

    let app = apply_tracing_with_req_id_middleware(app);
    let server = axum::serve(listener, app);
    Ok(server)
}
