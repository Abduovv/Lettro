use lettro::email_client::EmailClient;
use lettro::run;
use lettro::telemetry;
use secrecy::ExposeSecret;
use sqlx::PgPool;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber("lettro".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);

    let config = lettro::configuration::get_configuration().expect("Faild to get the Config");

    let pool = PgPool::connect(config.database.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", config.application_port)).await?;
    let sender_email = config
        .email_client
        .sender()
        .expect("Invalid sender email address");
    let timeout = config.email_client.timeout();
    let email_client = EmailClient::new(
        config.email_client.base_url,
        sender_email,
        config.email_client.authorization_token,
        timeout,
    );
    run(listener, pool, email_client)
        .await
        .expect("Failed to run the server");
    Ok(())
}
