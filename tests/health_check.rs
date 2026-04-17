use diesel::{prelude::*, sql_query};
use diesel_async::{
    AsyncConnection,
    AsyncPgConnection,
    RunQueryDsl,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use tokio::net::TcpListener;
use uuid::Uuid;
use zero2prod::{
    DbPool,
    configuration::{DatabaseSettings, get_configuration},
    schema::subscriptions,
    startup::run,
};

pub struct TestApp {
    pub address: String,
    pub db_pool: DbPool,
}

/// Spawns the application and returns the `TestApp` instance.
///
/// The application is spawned on a random available port to avoid conflicts with other tests or
/// applications.
///
/// # Returns
///
/// A `TestApp` instance containing the address of the spawned application and the database
/// connection pool.
async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to address");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let pool = configure_database(&configuration.database).await;

    let server = run(listener, pool.clone()).expect("Failed to start server");
    let _server_handle = tokio::spawn(server.into_future());

    TestApp { address, db_pool: pool }
}


pub async fn configure_database(config: &DatabaseSettings) -> DbPool {
    // Create Database
    let maintenance_settings = DatabaseSettings {
        database_name: "postgres".to_string(),
        username: "postgres".to_string(),
        password: "password".to_string(),
        ..config.clone()
    };

    let mut connection = AsyncPgConnection::establish(&maintenance_settings.connection_string())
        .await
        .expect("Failed to connect to Postgres");

    sql_query(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .execute(&mut connection)
        .await
        .expect("Failed to create database.");


    // Migrate database using synchronous connection.
    // Here it is somewhat acceptable, since we do need to run migration before anything else
    // happens with database. And by default tokio runs each tests in a single thread, so there
    // is no other process anyway to be blocked on this thread while running migration.
    {
        let mut connection = PgConnection::establish(&config.connection_string())
            .expect("Failed to connect to Postgres");
        connection
            .run_pending_migrations(
                FileBasedMigrations::find_migrations_directory()
                    .expect("Failed to find migration directory."),
            )
            .expect("Failed to run database migrations.");
    }


    // Create the connection pool and return it
    let connection_pool =
        AsyncDieselConnectionManager::<AsyncPgConnection>::new(config.connection_string());
    Pool::builder(connection_pool)
        .build()
        .expect("Failed to create connection pool.")
}

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let test_app = spawn_app().await;

    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(format!("{}/health_check", &test_app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}


#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(200, response.status().as_u16());

    // Get saved subscriber for database
    let mut conn = app
        .db_pool
        .get()
        .await
        .expect("Failed to get a connection from the pool.");

    let (email, name) = subscriptions::table
        .select((subscriptions::email, subscriptions::name))
        .first::<(String, String)>(&mut conn)
        .await
        .expect("Failed to get saved subscription.");

    assert_eq!(email, "ursula_le_guin@gmail.com");
    assert_eq!(name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");
        // Assert
        assert_eq!(
            422,
            response.status().as_u16(),
            // Additional customized error message on test failure
            "The API did not fail with 422 Unprocessable Entity when the payload was {}.",
            error_message
        );
    }
}
