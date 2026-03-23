use tokio::net::TcpListener;

pub async fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind random port");

    let port = listener.local_addr().unwrap().port();
    let server = lettro::run(listener);

    tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}
