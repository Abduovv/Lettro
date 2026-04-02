use crate::domain::{subscriber_name::SubscriberName, new_subscriber::NewSubscriber};
use crate::startup::AppState;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
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
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    State(state): State<AppState>,
    Form(form): Form<FormData>,
) -> impl IntoResponse {

    let name = match SubscriberName::parse(form.name) {
        Ok(name) => name,
        Err(_) => return StatusCode::BAD_REQUEST,
    };

    let new_subscriber = NewSubscriber {
        email: form.email,
        name,  
    };

    match insert_subscriber(&state, &new_subscriber).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tracing::instrument(name = "Saving subscriber to database", skip(state, new_subscriber))]
async fn insert_subscriber(
    state: &AppState,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO subscriptions (id, name, email, subscribed_at) VALUES ($1, $2, $3, $4)",
        Uuid::new_v4(),
        new_subscriber.name.as_ref(), 
        new_subscriber.email,
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