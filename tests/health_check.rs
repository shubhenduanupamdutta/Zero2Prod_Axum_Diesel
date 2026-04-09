use tokio::net::TcpListener;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    spawn_app().await;

    let client = reqwest::Client::new();

    // Act
    let response = client
        .get("http://127.0.0.1:8000/health_check")
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length())
}

async fn spawn_app() {
    let listener = TcpListener::bind("127.0.0.1:8000")
        .await
        .expect("Failed to bind to address");
    let server = zero2prod::run(listener).expect("Failed to start server");
    let _server_handle = tokio::spawn(server.into_future());
}
