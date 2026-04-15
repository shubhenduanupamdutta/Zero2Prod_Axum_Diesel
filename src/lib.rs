use diesel_async::{AsyncPgConnection, pooled_connection::deadpool};

pub mod configuration;
pub mod routes;
pub mod schema;
pub mod startup;

pub type DbPool = deadpool::Pool<AsyncPgConnection>;
