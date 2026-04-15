use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use tokio::net::TcpListener;
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Panic if we can't read configuration
    let configuration = get_configuration().expect("Failed to read configuration.");
    let db_config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(
        configuration.database.connection_string(),
    );
    let pool = Pool::builder(db_config)
        .build()
        .expect("Failed to create connection pool.");

    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address).await?;
    run(listener, pool)?.await
}
