use crate::auth;
use crate::config::Config;
use crate::errors::AppError;
use crate::ml_client::MlClient;
use crate::models::{
    AuthenticatedUser, ChangePasswordRequest, CreateTaskRequest, CreateUserRequest, LoginRequest,
    UpdateTaskRequest,
};
use crate::services;
use actix_web::{HttpRequest, HttpResponse, web};
use sqlx::SqlitePool;

fn extract_user(req: &HttpRequest, config: &Config) -> Result<AuthenticatedUser, AppError> {
    let header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or(AppError::Unauthorized)?;

    let claims = auth::verify_token(token, &config.jwt_secret)?;

    Ok(AuthenticatedUser {
        id: claims.sub,
        role: claims.role,
    })
}

fn require_admin(user: &AuthenticatedUser) -> Result<(), AppError> {
    if user.role != "admin" {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

// ============ Auth ============

pub async fn login(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    req: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let response = services::login(pool.get_ref(), req.into_inner(), &config.jwt_secret).await?;
    Ok(HttpResponse::Ok().json(response))
}

pub async fn change_password(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    http_req: HttpRequest,
    req: web::Json<ChangePasswordRequest>,
) -> Result<HttpResponse, AppError> {
    let user = extract_user(&http_req, &config)?;
    services::change_password(pool.get_ref(), user.id, req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Password changed"})))
}

// ============ Users ============

pub async fn create_user(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    http_req: HttpRequest,
    req: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, AppError> {
    let user = extract_user(&http_req, &config)?;
    require_admin(&user)?;

    let new_user = services::create_user(pool.get_ref(), req.into_inner()).await?;
    Ok(HttpResponse::Created().json(new_user))
}

pub async fn get_all_users(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let _ = extract_user(&http_req, &config)?;
    let users = services::get_all_users(pool.get_ref()).await?;
    Ok(HttpResponse::Ok().json(users))
}

pub async fn get_user(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    http_req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let _ = extract_user(&http_req, &config)?;
    let user = services::get_user_by_id(pool.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(user))
}

// ============ Tasks ============

pub async fn create_task(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    ml_client: web::Data<MlClient>,
    http_req: HttpRequest,
    req: web::Json<CreateTaskRequest>,
) -> Result<HttpResponse, AppError> {
    let user = extract_user(&http_req, &config)?;
    let task = services::create_task(
        pool.get_ref(),
        ml_client.get_ref(),
        req.into_inner(),
        user.id,
    )
    .await?;
    Ok(HttpResponse::Created().json(task))
}

pub async fn get_all_tasks(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    http_req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let _ = extract_user(&http_req, &config)?;
    let tasks = services::get_all_tasks(pool.get_ref()).await?;
    Ok(HttpResponse::Ok().json(tasks))
}

pub async fn get_task(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    http_req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let _ = extract_user(&http_req, &config)?;
    let task = services::get_task_by_id(pool.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(task))
}

pub async fn update_task(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    http_req: HttpRequest,
    path: web::Path<i64>,
    req: web::Json<UpdateTaskRequest>,
) -> Result<HttpResponse, AppError> {
    let _ = extract_user(&http_req, &config)?;
    let task = services::update_task(pool.get_ref(), path.into_inner(), req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(task))
}

pub async fn delete_task(
    pool: web::Data<SqlitePool>,
    config: web::Data<Config>,
    http_req: HttpRequest,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let _ = extract_user(&http_req, &config)?;
    services::delete_task(pool.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

// ============ Routes ============

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // Auth
            .route("/login", web::post().to(login))
            .route("/change-password", web::post().to(change_password))
            // Users
            .route("/users", web::post().to(create_user))
            .route("/users", web::get().to(get_all_users))
            .route("/users/{id}", web::get().to(get_user))
            // Tasks
            .route("/tasks", web::post().to(create_task))
            .route("/tasks", web::get().to(get_all_tasks))
            .route("/tasks/{id}", web::get().to(get_task))
            .route("/tasks/{id}", web::put().to(update_task))
            .route("/tasks/{id}", web::delete().to(delete_task)),
    );
}
