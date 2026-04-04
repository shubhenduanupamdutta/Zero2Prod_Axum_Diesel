use axum::{Router, http::StatusCode, response::IntoResponse, routing::get};
use tokio::net::TcpListener;


async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let app = Router::new().route("/health_check", get(health_check));
    let listener = TcpListener::bind("127.0.0.1:8000").await?;
    axum::serve(listener, app).await
}
