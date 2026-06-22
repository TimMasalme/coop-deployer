use std::sync::Arc;

use coop_app::{http::router, infra::fake_ports};

#[tokio::main]
async fn main() {
    let ports = Arc::new(fake_ports());
    let app = router(ports);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("0.0.0.0:{port}");

    println!("faf-coop-deployer listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
