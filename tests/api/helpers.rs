use lettro::startup::{Application, get_connection_pool};
use lettro::telemetry::{get_subscriber, init_subscriber};
use lettro::{DatabaseSettings, get_configuration};
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::Connection;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub mock_server: MockServer,
}

impl TestApp {
    pub async fn post_subscription(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("http://{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to send request")
    }
}

static TRACER: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "debug".to_string();
    let subscriber_name = "test".to_string();

    if std::env::var("TEST_LOG").is_ok() {
        let tracer = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(tracer);
    } else {
        let tracer = get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        init_subscriber(tracer);
    }
});

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACER);
    
    let mock_server = MockServer::start().await;

    let config = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application_port = 0;
        c.email_client.base_url = mock_server.uri();
        c
    };

    // ✅ Create & migrate the DB FIRST
    configure_database(&config.database).await;

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build server");

    let port = app.port();
    let _ = tokio::spawn(app.run_until_stopped());

    TestApp {
        address: format!("127.0.0.1:{}", port),
        db_pool: get_connection_pool(&config.database),
        mock_server,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Step 1: Connect WITHOUT the DB name to create it
    let mut connection =
        PgConnection::connect(&config.connection_string_without_db().expose_secret())
            .await
            .expect("Failed to connect to Postgres");

    sqlx::query(&format!(r#"CREATE DATABASE "{}";"#, config.database_name))
        .execute(&mut connection)
        .await
        .expect("Failed to create database.");

    // Step 2: Connect WITH the DB name to run migrations
    let connection_pool = PgPool::connect(&config.connection_string_with_db().expose_secret())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
