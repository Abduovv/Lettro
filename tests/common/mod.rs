use fake::{Fake, Faker};
use lettro::email_client::EmailClient;
use lettro::telemetry::{get_subscriber, init_subscriber};
use lettro::{DatabaseSettings, get_configuration, startup::run};
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use secrecy::SecretString;
use sqlx::Connection;
use sqlx::{PgConnection, PgPool};
use tokio::net::TcpListener;
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
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

    let mut config = get_configuration().expect("Failedtoreadconfiguration.");
    config.database.database_name = Uuid::new_v4().to_string();

    let connection_pool = configure_database(&config.database).await;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind random port");

    let port = listener.local_addr().unwrap().port();
    let sender_email = config
        .email_client
        .sender()
        .expect("Invalid sender email address");
    let email_client = EmailClient::new(
        config.email_client.base_url.clone(),
        sender_email,
        SecretString::new(Faker.fake::<String>().into_boxed_str()),
        config.email_client.timeout(),
    );

    let server = run(listener, connection_pool.clone(), email_client);

    tokio::spawn(server);

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    //Createdatabase
    let mut connection = PgConnection::connect(&config.connection_string_with_db().expose_secret())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::query(&format!(r#"CREATE DATABASE "{}";"#, config.database_name))
        .execute(&mut connection)
        .await
        .expect("Failed to create database.");

    //Migratedatabase
    let connection_pool = PgPool::connect(&config.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}
