use axum::{routing::get, Router};
use tokio::net::TcpListener;

async fn root() -> &'static str {
    "Hello, World!"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Router::new().route("/", get(root));

    // try to first get a socket from listenfd, if that does not give us
    // one (eg: no systemd or systemfd), open on port 3000 instead.
    let mut listenfd = listenfd::ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0).unwrap() {
        Some(listener) => TcpListener::from_std(listener),
        None => TcpListener::bind("0.0.0.0:3000").await,
    }?;
    axum::serve(listener, app).await?;
    Ok(())
}
