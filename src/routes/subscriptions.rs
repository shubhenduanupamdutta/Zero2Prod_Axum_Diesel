use axum::{Form, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use diesel::prelude::Insertable;
use diesel_async::RunQueryDsl;
use serde::Deserialize;
use tracing::Instrument;
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

    let request_span = tracing::info_span!(
        "Adding a new subscriber.",
        %request_id,
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    );

    let _request_span_guard = request_span.enter();

    // We do not call `.enter` on query_span!
    // `.instrument` takes care of it at the right moments
    // in the query lifetime
    let query_span = tracing::info_span!("Saving new subscriber details in the database");

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

    match diesel::insert_into(subscriptions::table)
        .values(&new_subscriber)
        .execute(&mut conn)
        .instrument(query_span)
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            // Yes, this error log falls outside of `query_span`
            // We'll rectify it later, pinky swear!
            tracing::error!("Failed to execute query: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        },
    }
}
