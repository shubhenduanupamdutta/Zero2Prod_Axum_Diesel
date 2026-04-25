use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use tokio::net::TcpListener;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt};
use zero2prod::{configuration::get_configuration, startup::run};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    // Redirect all `log`'s events to our subscriber
    LogTracer::init().expect("Failed to set logger");

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,tower_http=debug"));
    let formatting_layer = BunyanFormattingLayer::new(
        "zero2prod".into(),
        // Output the formatted span to stdout
        std::io::stdout,
    );

    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Failed to get subscriber");

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
