use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    health_check, newsletter::publish_newsletter, subscribe, subscriptions_confirm::confirm,
};
use axum::{
    Router,
    http::Request,
    routing::{get, post},
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::TraceLayer;
use uuid::Uuid;
// ---------- State ----------

#[derive(Clone)]
pub struct AppState {
    pub connection: PgPool,
    pub email_client: EmailClient,
    pub base_url: String,
}

// ---------- Request ID ----------

#[derive(Clone)]
struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        let id = Uuid::new_v4().to_string();
        let header_value = id.parse().unwrap();
        Some(RequestId::new(header_value))
    }
}

// ---------- Application ----------

pub struct Application {
    port: u16,
    listener: TcpListener,
    app: Router,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);
        let email_client = configuration.email_client.client();

        let address = format!(
            "{}:{}",
            configuration.application.host, configuration.application.port
        );
        let listener = TcpListener::bind(address).await?;
        let port = listener.local_addr().unwrap().port();
        let app = run(
            connection_pool,
            email_client,
            configuration.application.base_url,
        );

        Ok(Self {
            port,
            listener,
            app,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        axum::serve(self.listener, self.app).await
    }
}

// ---------- Router ----------

fn run(connection: PgPool, email_client: EmailClient, base_url: String) -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .route("/subscriptions/confirm", get(confirm))
        .route("/newsletters", post(publish_newsletter))
        .with_state(AppState {
            connection,
            email_client,
            base_url,
        })
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(
                    TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                        let request_id = request
                            .headers()
                            .get("x-request-id")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("unknown");

                        tracing::info_span!(
                            "http_request",
                            method = %request.method(),
                            uri = %request.uri(),
                            request_id = %request_id,
                        )
                    }),
                )
                .layer(PropagateRequestIdLayer::x_request_id()),
        )
}

// ---------- Database ----------

pub fn get_connection_pool(config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.with_db())
}
