use actix_web::{web, App, HttpServer, HttpResponse, get};
use sqlx::sqlite::SqlitePool;

mod config;
mod errors;
mod models;
mod repository;

#[get("/health")]
async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    let config = config::Config::from_env();

    tracing::info!("Connecting to database...");
    let pool = SqlitePool::connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    tracing::info!("Starting server at http://{}:{}", config.host, config.port);

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(health)
    })
    .bind((config.host.as_str(), config.port))?
    .run()
    .await
}