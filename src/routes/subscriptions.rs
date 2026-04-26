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
pub struct InsertSubscription<'a> {
    id: Uuid,
    email: &'a str,
    name: &'a str,
    subscribed_at: DateTime<Utc>,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, pool),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(State(pool): State<DbPool>, Form(form): Form<FormData>) -> StatusCode {
    let new_subscriber = InsertSubscription {
        id: Uuid::new_v4(),
        email: &form.email,
        name: &form.name,
        subscribed_at: Utc::now(),
    };

    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => StatusCode::OK,
        Err(e) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}


#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(subscriber, pool)
)]
pub async fn insert_subscriber(
    pool: &DbPool,
    subscriber: &InsertSubscription<'_>,
) -> Result<(), diesel::result::Error> {
    let mut conn = pool
        .get()
        .await
        .expect("Failed to get a connection from the pool.");

    diesel::insert_into(subscriptions::table)
        .values(subscriber)
        .execute(&mut conn)
        .await
        .map_err(|e| {
            tracing::error!("Failed to execute query: {:?}", e);
            e
        })?;
    Ok(())
}
