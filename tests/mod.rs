mod common;
use crate::common::spawn_app;

#[tokio::test]
async fn health_check_works() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/health_check", app.address))
        .await
        .expect("failed to execute request");

    claim::assert!(response.status().is_success());
    claim::assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        claim::assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data_v2() -> Result<(), sqlx::Error> {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    claim::assert_eq!(200, response.status().as_u16());

    // Query the database to check the subscription was saved

    let saved = sqlx::query!(
        "SELECT email, name FROM subscriptions WHERE email = $1",
        "ursula_le_guin@gmail.com"
    )
    .fetch_one(&app.db_pool)
    .await?;

    claim::assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    claim::assert_eq!(saved.name, "le guin");
    Ok(())
}

#[tokio::test]
async fn subscribe_returns_a_200_when_fields_are_present_but_empty() {
    //Arrange
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "emptyname"),
        ("name=Ursula&email=", "emptyemail"),
        ("name=Ursula&email=definitely-not-an-email", "invalidemail"),
    ];
    for (body, description) in test_cases {
        //Act
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");
        //Assert
        claim::assert_eq!(
            200,
            response.status().as_u16(),
            "The API did not return a 200 OK when the payload was {}.",
            description
        );
    }
}
