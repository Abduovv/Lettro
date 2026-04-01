use anyhow::Result;
use tokio::net::TcpListener;
use axum::{
    Router,
    routing::{get, post},
};
use crate::routes::{health_check, subscribe};
use axum::{
    extract::{Form, State},
    response::IntoResponse,
};
use sqlx::PgPool;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub connection: PgPool,
}

pub async fn run(listener: TcpListener, connection: PgPool) -> Result<()> {
    let app = Router::new()
        .route("/health_check", get(health_check))
        .route("/subscriptions", post(subscribe))
        .with_state(AppState { connection })
        .layer(TraceLayer::new_for_http());

    axum::serve(listener, app).await?;
    Ok(())
}
