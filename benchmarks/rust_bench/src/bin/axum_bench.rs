use axum::{routing::get, Router};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(|| async { "Hello from Native Axum" }));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8182").await.unwrap();
    println!("Native Axum serving on http://127.0.0.1:8182");
    axum::serve(listener, app).await.unwrap();
}
