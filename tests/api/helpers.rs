use argon2::password_hash::SaltString;
use lettro::email_client::EmailClient;
use lettro::startup::{Application, get_connection_pool};
use lettro::telemetry::{get_subscriber, init_subscriber};
use lettro::{DatabaseSettings, get_configuration};
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use argon2::{Argon2, PasswordHasher, password_hash::ParamsString};
use sqlx::Connection;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }
    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::default()
            .hash_password(self.password.as_bytes(), &salt)
            .unwrap().to_string();
        
        sqlx::query!(
            "INSERT INTO users(user_id,username,password_hash)
             VALUES($1,$2,$3)",
            self.user_id,
            self.username,
            password_hash,
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub mock_server: MockServer,
    pub email_client: EmailClient,
    pub port: u16,
    pub test_user: TestUser,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscription(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to send request")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .json(&body)
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .send()
            .await
            .expect("Failed to execute request")
    }

    pub async fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Let's makesure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };
        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
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
        c.application.port = 0;
        c.email_client.base_url = mock_server.uri();
        c
    };

    configure_database(&config.database).await;

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build server");

    let port = app.port();
    let _ = tokio::spawn(app.run_until_stopped());

    let test_app = TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db_pool: get_connection_pool(&config.database),
        mock_server,
        email_client: config.email_client.client(),
        port,
        test_user: TestUser::generate(),
    };

    test_app.test_user.store(&test_app.db_pool).await;

    test_app
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

