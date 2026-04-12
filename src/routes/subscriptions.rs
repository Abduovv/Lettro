use crate::domain::{
    new_subscriber::NewSubscriber, subscriber_email::SubscriberEmail,
    subscriber_name::SubscriberName,
};
use crate::email_client::EmailClient;
use crate::startup::AppState;
use anyhow::Context;
use axum::{Json, response::Response};
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::IntoResponse,
};
use rand::distributions::Alphanumeric;
use rand::{Rng, thread_rng};
use serde::Deserialize;
use serde_json::json;
use sqlx::{Postgres, Transaction};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Email error: {0}")]
    EmailError(#[from] reqwest::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::DatabaseError(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            AppError::UnexpectedError(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            AppError::EmailError(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        };

        let body = Json(json!({ "error": message }));
        (status, body).into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct FormData {
    pub name: String,
    pub email: String,
}

/// Generate a random 25-characters-long case-sensitive subscription token.
fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(value.name)?;
        let email = SubscriberEmail::parse(value.email)?;
        Ok(Self { email, name })
    }
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(app_state),
    fields(
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    State(app_state): State<AppState>,
    Form(form): Form<FormData>,
) -> Result<impl IntoResponse, AppError> {
    let new_subscriber: NewSubscriber = match form.try_into() {
        Ok(subscriber) => subscriber,
        Err(_) => return Err(AppError::ValidationError("Invalid subscriber data".into())),
    };

    let mut transaction = app_state
        .connection
        .begin()
        .await
        .context("Failed to acquire a Postgres connection from the pool")?;

    let subscriber_id = insert_subscriber(&mut transaction, &new_subscriber)
        .await
        .context("Failed to insert subscriber into the database")?;

    let subscription_token = generate_subscription_token();

    store_token(&mut transaction, subscriber_id, &subscription_token)
        .await
        .context("Failed to store subscription token in the database")?;
    send_confirmation_email(
        &app_state.email_client,
        new_subscriber,
        &app_state.base_url,
        &subscription_token,
    )
    .await
    .context("Failed to send confirmation email")?;

    transaction
        .commit()
        .await
        .context("Failed to commit transaction")?;

    Ok(StatusCode::OK)
}

#[tracing::instrument(
    name = "Store subscription token in the database",
    skip(subscription_token, transaction)
)]
pub async fn store_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO subscription_token (subscription_token, subscriber_id) VALUES ($1, $2)"#,
        subscription_token,
        subscriber_id
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    confirmation_token: &str,
) -> Result<(), AppError> {
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, confirmation_token
    );
    let plain_body = format!(
        "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
        confirmation_link
    );
    let html_body = format!(
        "Welcome to our newsletter!<br/>\
        Click <a href=\"{}\">here</a> to confirm your subscription.",
        confirmation_link
    );
    email_client
        .send_email(&new_subscriber.email, "Welcome!", &html_body, &plain_body)
        .await?;

    Ok(())
}

#[tracing::instrument(
    name = "Saving subscriber to database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, AppError> {
    let subscriber_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO subscriptions (id, name, email, subscribed_at, status) VALUES ($1, $2, $3, $4, 'pending_confirmation')",
        subscriber_id,
        new_subscriber.name.as_ref(),
        new_subscriber.email.as_ref(),
        time::OffsetDateTime::now_utc(),
    )
    .execute(&mut **transaction)
    .await?;

    Ok(subscriber_id)
}
