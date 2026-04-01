use crate::startup::AppState;
use axum::{
    extract::{Form, State},
    http::{StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::Instrument;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(state),
    fields(
        request_id = %Uuid::new_v4(),
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {
    match insert_subscriber(&state, &form).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(name = "Saving subscriber to database", skip(state, form))]
async fn insert_subscriber(state: &AppState, form: &FormData) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO subscriptions (id, name, email, subscribed_at) VALUES ($1, $2, $3, $4)",
        Uuid::new_v4(),
        form.name,
        form.email,
        time::OffsetDateTime::now_utc(),
    )
    .execute(&state.connection)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}