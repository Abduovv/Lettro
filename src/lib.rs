use anyhow::Result;
use axum::{
    Form, Router, extract::{Path, Query}, http::StatusCode, response::IntoResponse, routing::{get, post}
};
use tokio::net::TcpListener;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct FormData {
    name: String,
    email: String,
}

pub async fn run(listener: TcpListener) -> Result<()> {
    axum::serve(listener, app()).await?;
    Ok(())
}
pub fn app() -> Router {
    Router::new()
        .route("/health_check", get(health_check))
        .route("/greet/{name}", get(greet))
        .route("/subscriptions", post(subscribe))
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

async fn greet(Path(name): Path<String>) -> impl IntoResponse {
    let name = if name.is_empty() {
        "stranger".to_string()
    } else {
        name
    };
    format!("Hello, {}!", name)
}

async fn subscribe(_from: Form<FormData>) -> impl IntoResponse {
    StatusCode::OK
}
