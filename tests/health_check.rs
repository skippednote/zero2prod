use sqlx::{PgPool};
use std::net::TcpListener;
use zero2prod::configuration::get_configuration;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

#[actix_rt::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let response = client
        .get(&format!("{}/health_check", app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

async fn spawn_app() -> TestApp {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let configuration = get_configuration().expect("Failed to read configuration");
    let connection_pool = PgPool::connect(&configuration.database.connection_string()).await.expect("Failed to connect to Postgres.");
    let server = zero2prod::startup::run(listener, connection_pool.clone()).expect("Failed to bind address");
    tokio::spawn(server);

    TestApp {
        address,
        db_pool: connection_pool,
    }
}

#[actix_rt::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let body = "name=Bassam%20Ismail&email=skippednote%40gmail.com";

    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());
    let saved = sqlx::query!("SELECT email, name from subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

        println!("{}", saved);

    assert_eq!(saved.email, "skippednote@gmail.com");
    assert_eq!(saved.name, "Bassam Ismail");
}

#[actix_rt::test]
async fn subscribe_returns_a_400_when_data_is_invalid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=bassam%20ismail", "missing the email"),
        ("email=skippednote%40@gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute to request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}",
            error_message
        );
    }
}
