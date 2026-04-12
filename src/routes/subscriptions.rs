use axum::{Form, http::StatusCode};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(_form: Form<FormData>) -> StatusCode {
    StatusCode::OK
}
