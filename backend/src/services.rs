use sqlx::SqlitePool;
use crate::auth;
use crate::errors::AppError;
use crate::ml_client::MlClient;
use crate::models::{
    AuthResponse, AuthenticatedUser, ChangePasswordRequest, CreateTaskRequest,
    CreateUserRequest, LoginRequest, Task, UpdateTaskRequest, User,
};
use crate::repository;

// ============ Init ============

pub async fn init_admin(
    pool: &SqlitePool,
    email: &str,
    password: &str,
) -> Result<(), AppError> {
    let user_count = repository::count_users(pool).await?;

    if user_count > 0 {
        tracing::info!("Database already has users, skipping admin creation");
        return Ok(());
    }

    tracing::info!("Creating initial admin user: {}", email);
    
    let password_hash = auth::hash_password(password)?;
    repository::create_user(pool, email, &password_hash, "Admin", "admin").await?;

    tracing::info!("Admin user created successfully");
    Ok(())
}

// ============ Auth ============

pub async fn login(
    pool: &SqlitePool,
    req: LoginRequest,
    jwt_secret: &str,
) -> Result<AuthResponse, AppError> {
    let user = repository::get_user_by_email(pool, &req.email)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !auth::verify_password(&req.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    let token = auth::create_token(user.id, &user.role, jwt_secret)?;

    Ok(AuthResponse { token, user })
}

pub async fn change_password(
    pool: &SqlitePool,
    user_id: i64,
    req: ChangePasswordRequest,
) -> Result<(), AppError> {
    let user = repository::get_user_by_id(pool, user_id).await?;

    if !auth::verify_password(&req.current_password, &user.password_hash)? {
        return Err(AppError::BadRequest("Current password is incorrect".to_string()));
    }

    let new_hash = auth::hash_password(&req.new_password)?;
    repository::update_password(pool, user_id, &new_hash).await
}

// ============ Users ============

pub async fn create_user(
    pool: &SqlitePool,
    req: CreateUserRequest,
) -> Result<User, AppError> {
    let password_hash = auth::hash_password(&req.password)?;
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
    ml_client: &MlClient,
    req: CreateTaskRequest,
    created_by: i64,
) -> Result<Task, AppError> {
    let predicted_hours = ml_client
        .predict_time_safe(&req.title, req.description.as_deref())
        .await;

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
    if let Some(ref status) = req.status {
        if !["todo", "in_progress", "done"].contains(&status.as_str()) {
            return Err(AppError::BadRequest(
                "Status must be: todo, in_progress, done".to_string(),
            ));
        }
    }

    repository::update_task(pool, id, &req).await
}

pub async fn delete_task(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    repository::delete_task(pool, id).await
}