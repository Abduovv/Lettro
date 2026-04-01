use anyhow::Result;
use lettro::run;
use lettro::telemetry;
use sqlx::PgPool;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let subscriber = telemetry::get_subscriber("lettro".into(), "info".into());
    telemetry::init_subscriber(subscriber);
    
    let pool = PgPool::connect(&std::env::var("DATABASE_URL")?)
        .await
        .expect("Failed to connect to Postgres");

    let config = lettro::configuration::get_configuration()?;

    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.application_port)).await?;
    run(listener, pool).await?;
    Ok(())
}
