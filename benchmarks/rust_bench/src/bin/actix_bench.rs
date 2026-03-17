use actix_web::{get, App, HttpServer, Responder};

#[get("/")]
async fn hello() -> impl Responder {
    "Hello from Native Actix"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let workers = std::env::var("ACTIX_WORKERS")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<usize>()
        .unwrap_or(1);
        
    println!("Native Actix serving on http://127.0.0.1:8183 with {} workers", workers);
    HttpServer::new(|| {
        App::new().service(hello)
    })
    .workers(workers)
    .bind(("127.0.0.1", 8183))?
    .run()
    .await
}
