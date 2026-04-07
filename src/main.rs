use lettro::configuration::get_configuration;
use lettro::startup::Application;
use lettro::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = get_subscriber("lettro".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let configuration = get_configuration().expect("Failed to read configuration.");
    let application = Application::build(configuration)
        .await
        .expect("Failed to build application.");
    application.run_until_stopped().await?;
    Ok(())
}
