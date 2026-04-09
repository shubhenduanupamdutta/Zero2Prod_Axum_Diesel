use axum::{Router, http::StatusCode, response::IntoResponse, routing::get, serve::Serve};
use tokio::net::TcpListener;


async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

pub fn run(listener: TcpListener) -> Result<Serve<TcpListener, Router, Router>, std::io::Error> {
    let app = Router::new().route("/health_check", get(health_check));
    let server = axum::serve(listener, app);
    Ok(server)
}
