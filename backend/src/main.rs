use actix_cors::Cors;
use actix_web::{App, HttpResponse, HttpServer, middleware::Logger, web};
use sqlx::sqlite::SqlitePool;

mod auth;
mod config;
mod errors;
mod handlers;
mod ml_client;
mod models;
mod repository;
mod services;

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

    services::init_admin(&pool, &config.admin_email, &config.admin_password)
        .await
        .expect("Failed to initialize admin");

    let ml_client = ml_client::MlClient::new(config.ml_service_url.clone());
        
    tracing::info!("Starting server at http://{}:{}", config.host, config.port);

    let config_data = config.clone();

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config_data.clone()))
            .app_data(web::Data::new(ml_client.clone()))
            .route("/health", web::get().to(health))
            .configure(handlers::configure)
            .wrap(Logger::new("%a | %r | %s").log_target("http_log"))
            .wrap(cors)
    })
    .bind((config.host.as_str(), config.port))?
    .run()
    .await
}