use actix_web::{get, App, HttpServer, Responder};

#[get("/")]
async fn hello() -> impl Responder {
    "Hello from Native Actix"
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Native Actix serving on http://127.0.0.1:8183");
    HttpServer::new(|| {
        App::new().service(hello)
    })
    .bind(("127.0.0.1", 8183))?
    .run()
    .await
}
