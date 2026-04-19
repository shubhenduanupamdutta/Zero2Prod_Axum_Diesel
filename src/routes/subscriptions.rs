use axum::{Form, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use diesel::prelude::Insertable;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use uuid::Uuid;

use crate::{DbPool, schema::subscriptions};

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}


#[derive(Insertable)]
#[diesel(table_name = subscriptions)]
struct InsertSubscription {
    id: Uuid,
    email: String,
    name: String,
    subscribed_at: DateTime<Utc>,
}


pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> StatusCode {
    let request_id = Uuid::new_v4();
    log::info!(
        "request_id {} - Adding '{}' '{}' as a new subscriber.",
        request_id,
        form.email,
        form.name
    );

    let new_subscriber = InsertSubscription {
        id: Uuid::new_v4(),
        email: form.email,
        name: form.name,
        subscribed_at: Utc::now(),
    };

    let mut conn = pool
        .get()
        .await
        .expect("Failed to get a connection from the pool.");

    log::info!(
        "request_id {} - Saving new subscriber details in the database",
        request_id
    );
    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .await
    {
        Ok(_) => {
            log::info!(
                "request_id {} - New subscriber details have been saved successfully.",
                request_id
            );
            StatusCode::OK
        },
        Err(e) => {
            log::error!(
                "request_id {} - Failed to execute query: {:?}",
                request_id,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        },
    }
}
