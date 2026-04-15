use axum::{Form, http::StatusCode};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

pub async fn subscribe(_form: Form<FormData>) -> StatusCode {
    StatusCode::OK
}
