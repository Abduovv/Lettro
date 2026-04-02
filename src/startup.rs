use crate::routes::{health_check, subscribe};
use anyhow::Result;
use axum::{
    Router,
    http::Request,
    routing::{get, post},
};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::request_id::{
    MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub connection: PgPool,
}

#[derive(Clone)]
struct MakeRequestUuid;

impl MakeRequestId for MakeRequestUuid {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        let id = Uuid::new_v4().to_string();
        let header_value = id.parse().unwrap();
        Some(RequestId::new(header_value))
    }
}

pub async fn run(listener: TcpListener, connection: PgPool) -> Result<()> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(AppState { connection })
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(
                    TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                        let request_id = request
                            .headers()
                            .get("x-request-id")
                            .and_then(|v: &axum::http::HeaderValue| v.to_str().ok())
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
        );

    axum::serve(listener, app).await?;
    Ok(())
}
