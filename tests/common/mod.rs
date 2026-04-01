use lettro::{DatabaseSettings, get_configuration, startup::run};
use sqlx::{PgConnection, PgPool};
use tokio::net::TcpListener;
use uuid::Uuid;
use sqlx::Connection;
use lettro::telemetry::{init_subscriber, get_subscriber};
use once_cell::sync::Lazy;

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

static TRACER: Lazy<()> = Lazy::new(|| {
    let tracer = get_subscriber("test".into(), "debug".into());
    init_subscriber(tracer);
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
    let server = run(listener, connection_pool.clone());

    tokio::spawn(server);

    TestApp {
        address: format!("http://127.0.0.1:{}", port),
        db_pool: connection_pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    //Createdatabase
    let mut connection = PgConnection::connect(&config.connection_string_with_db())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::query(&format!(r#"CREATE DATABASE "{}";"#, config.database_name))
        .execute(&mut connection)
        .await
        .expect("Failed to create database.");
    
    //Migratedatabase
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to Postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}
