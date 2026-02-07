#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub host: String,
    pub port: u16,
    pub ml_service_url: String,
    pub admin_email: String,
    pub admin_password: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string()),
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "super-secret-change-me".to_string()),
            host: std::env::var("HOST")
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .expect("PORT must be a number"),
            ml_service_url: std::env::var("ML_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
            admin_email: std::env::var("ADMIN_EMAIL")
                .unwrap_or_else(|_| "admin@example.com".to_string()),
            admin_password: std::env::var("ADMIN_PASSWORD")
                .unwrap_or_else(|_| "adminpass123".to_string()),
        }
    }
}