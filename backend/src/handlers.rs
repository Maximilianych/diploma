use actix_web::{web, HttpResponse};
use sqlx::SqlitePool;
use crate::errors::AppError;
use crate::models::{CreateUserRequest, CreateTaskRequest, UpdateTaskRequest};
use crate::services;

// ============ Users ============

pub async fn create_user(
    pool: web::Data<SqlitePool>,
    req: web::Json<CreateUserRequest>,
) -> Result<HttpResponse, AppError> {
    let user = services::create_user(pool.get_ref(), req.into_inner()).await?;
    Ok(HttpResponse::Created().json(user))
}

pub async fn get_all_users(
    pool: web::Data<SqlitePool>,
) -> Result<HttpResponse, AppError> {
    let users = services::get_all_users(pool.get_ref()).await?;
    Ok(HttpResponse::Ok().json(users))
}

pub async fn get_user(
    pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let user = services::get_user_by_id(pool.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(user))
}

// ============ Tasks ============

pub async fn create_task(
    pool: web::Data<SqlitePool>,
    req: web::Json<CreateTaskRequest>,
) -> Result<HttpResponse, AppError> {
    // TODO: получать created_by из JWT токена
    let created_by: i64 = 1; // заглушка

    let task = services::create_task(pool.get_ref(), req.into_inner(), created_by).await?;
    Ok(HttpResponse::Created().json(task))
}

pub async fn get_all_tasks(
    pool: web::Data<SqlitePool>,
) -> Result<HttpResponse, AppError> {
    let tasks = services::get_all_tasks(pool.get_ref()).await?;
    Ok(HttpResponse::Ok().json(tasks))
}

pub async fn get_task(
    pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    let task = services::get_task_by_id(pool.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(task))
}

pub async fn update_task(
    pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    req: web::Json<UpdateTaskRequest>,
) -> Result<HttpResponse, AppError> {
    let task = services::update_task(pool.get_ref(), path.into_inner(), req.into_inner()).await?;
    Ok(HttpResponse::Ok().json(task))
}

pub async fn delete_task(
    pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
) -> Result<HttpResponse, AppError> {
    services::delete_task(pool.get_ref(), path.into_inner()).await?;
    Ok(HttpResponse::NoContent().finish())
}

// ============ Routes ============

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // Users (создание пользователя только для админа будет)
            .route("/users", web::post().to(create_user))
            .route("/users", web::get().to(get_all_users))
            .route("/users/{id}", web::get().to(get_user))
            // Tasks
            .route("/tasks", web::post().to(create_task))
            .route("/tasks", web::get().to(get_all_tasks))
            .route("/tasks/{id}", web::get().to(get_task))
            .route("/tasks/{id}", web::put().to(update_task))
            .route("/tasks/{id}", web::delete().to(delete_task))
    );
}