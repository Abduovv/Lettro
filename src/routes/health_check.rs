use axum::http::StatusCode;

pub async fn health_check() -> StatusCode {
    tracing::info!("Health check passed");
    StatusCode::OK
}
