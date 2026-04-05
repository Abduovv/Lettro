use crate::helpers::spawn_app;
use wiremock::matchers::{method, path};
use wiremock::{Mock,ResponseTemplate};

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = app.post_subscription(invalid_body.to_string()).await;

        assert_eq!(
            422,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() -> Result<(), sqlx::Error> {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscription(body.to_string()).await;

    assert_eq!(200, response.status().as_u16());

    // Query the database to check the subscription was saved

    let saved = sqlx::query!(
        "SELECT email, name FROM subscriptions WHERE email = $1",
        "ursula_le_guin@gmail.com"
    )
    .fetch_one(&app.db_pool)
    .await?;

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
    Ok(())
}

#[tokio::test]
async fn subscribe_returns_a_400_for_invalid_form_data() -> Result<(), sqlx::Error> {
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "emptyname"),
        ("name=Ursula&email=", "emptyemail"),
        ("name=Ursula&email=definitely-not-an-email", "invalidemail"),
    ];
    for (body, description) in test_cases {
        //Act
        let response = app.post_subscription(body.to_string()).await;
        //Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
    Ok(())
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_for_valid_data() -> Result<(), sqlx::Error> {
    let app = spawn_app().await;
    
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    
    Mock::given(path("/email"))
    .and(method("POST"))
    .respond_with(ResponseTemplate::new(200))
    .expect(1)
    .mount(&app.mock_server)
    .await;
    //Act
    app.post_subscription(body.into()).await;

    
    Ok(())
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() -> Result<(), sqlx::Error> {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;
    //Act
    app.post_subscription(body.into()).await;
    //Assert
    //Get thefirstinterceptedrequest
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    //Parse the body as JSON, starting from raw bytes
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
    
    Ok(())
}
