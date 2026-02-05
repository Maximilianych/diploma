use sqlx::SqlitePool;
use crate::errors::AppError;
use crate::models::{User, Task, CreateUserRequest, CreateTaskRequest, UpdateTaskRequest};
use crate::repository;

// ============ Users ============

pub async fn create_user(
    pool: &SqlitePool,
    req: CreateUserRequest,
) -> Result<User, AppError> {
    // TODO: хеширование пароля
    let password_hash = format!("hash_{}", req.password); // заглушка

    repository::create_user(pool, &req.email, &password_hash, &req.name, &req.role).await
}

pub async fn get_all_users(pool: &SqlitePool) -> Result<Vec<User>, AppError> {
    repository::get_all_users(pool).await
}

pub async fn get_user_by_id(pool: &SqlitePool, id: i64) -> Result<User, AppError> {
    repository::get_user_by_id(pool, id).await
}

// ============ Tasks ============

pub async fn create_task(
    pool: &SqlitePool,
    req: CreateTaskRequest,
    created_by: i64,
) -> Result<Task, AppError> {
    // TODO: вызов ML-сервиса
    let predicted_hours: Option<f64> = None; // заглушка

    repository::create_task(pool, &req, created_by, predicted_hours).await
}

pub async fn get_all_tasks(pool: &SqlitePool) -> Result<Vec<Task>, AppError> {
    repository::get_all_tasks(pool).await
}

pub async fn get_task_by_id(pool: &SqlitePool, id: i64) -> Result<Task, AppError> {
    repository::get_task_by_id(pool, id).await
}

pub async fn update_task(
    pool: &SqlitePool,
    id: i64,
    req: UpdateTaskRequest,
) -> Result<Task, AppError> {
    // Валидация статуса
    if let Some(ref status) = req.status {
        if !["todo", "in_progress", "done"].contains(&status.as_str()) {
            return Err(AppError::BadRequest(
                "Status must be: todo, in_progress, done".to_string()
            ));
        }
    }

    repository::update_task(pool, id, &req).await
}

pub async fn delete_task(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    repository::delete_task(pool, id).await
}