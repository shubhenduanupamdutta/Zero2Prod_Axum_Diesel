use std::sync::Arc;

use axum::{Form, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use diesel::prelude::Insertable;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::schema::subscriptions;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

type DbConnection = Arc<Mutex<AsyncPgConnection>>;

#[derive(Insertable)]
#[diesel(table_name = subscriptions)]
struct InsertSubscription {
    id: Uuid,
    email: String,
    name: String,
    subscribed_at: DateTime<Utc>,
}


pub async fn subscribe(
    State(connection): State<DbConnection>,
    Form(form): Form<FormData>,
) -> StatusCode {
    let mut connection = connection.lock().await;
    let new_subscriber = InsertSubscription {
        id: Uuid::new_v4(),
        email: form.email,
        name: form.name,
        subscribed_at: Utc::now(),
    };
    diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut *connection)
        .await
        .expect("Failed to execute query.");
    StatusCode::OK
}
