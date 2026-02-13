use crate::errors::AppError;
use crate::models::{CreateTaskRequest, Task, UpdateTaskRequest, User};
use sqlx::SqlitePool;

// ============ Users ============

pub async fn create_user(
    pool: &SqlitePool,
    email: &str,
    password_hash: &str,
    name: &str,
    role: &str,
) -> Result<User, AppError> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (email, password_hash, name, role)
        VALUES (?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(email)
    .bind(password_hash)
    .bind(name)
    .bind(role)
    .fetch_one(pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            AppError::BadRequest("Email already exists".to_string())
        } else {
            e.into()
        }
    })
}

pub async fn get_user_by_id(pool: &SqlitePool, id: i64) -> Result<User, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))
}

pub async fn get_user_by_email(pool: &SqlitePool, email: &str) -> Result<Option<User>, AppError> {
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn get_all_users(pool: &SqlitePool) -> Result<Vec<User>, AppError> {
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn count_users(pool: &SqlitePool) -> Result<i64, AppError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

pub async fn delete_user(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }
    Ok(())
}

// ============ Tasks ============

pub async fn create_task(
    pool: &SqlitePool,
    req: &CreateTaskRequest,
    created_by: i64,
    predicted_hours: Option<f64>,
) -> Result<Task, AppError> {
    Ok(sqlx::query_as::<_, Task>(
        r#"
        INSERT INTO tasks (title, description, assignee_id, created_by, predicted_hours)
        VALUES (?, ?, ?, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.assignee_id)
    .bind(created_by)
    .bind(predicted_hours)
    .fetch_one(pool)
    .await?)
}

pub async fn get_task_by_id(pool: &SqlitePool, id: i64) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Task not found".to_string()))
}

pub async fn get_all_tasks(pool: &SqlitePool) -> Result<Vec<Task>, AppError> {
    Ok(
        sqlx::query_as::<_, Task>("SELECT * FROM tasks ORDER BY created_at DESC")
            .fetch_all(pool)
            .await?,
    )
}

pub async fn get_tasks_by_status(pool: &SqlitePool, status: &str) -> Result<Vec<Task>, AppError> {
    Ok(
        sqlx::query_as::<_, Task>("SELECT * FROM tasks WHERE status = ? ORDER BY created_at DESC")
            .bind(status)
            .fetch_all(pool)
            .await?,
    )
}

pub async fn get_tasks_by_assignee(pool: &SqlitePool, user_id: i64) -> Result<Vec<Task>, AppError> {
    Ok(sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE assignee_id = ? ORDER BY created_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?)
}

pub async fn update_task(
    pool: &SqlitePool,
    id: i64,
    req: &UpdateTaskRequest,
) -> Result<Task, AppError> {
    let current = get_task_by_id(pool, id).await?;

    sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET title = ?, description = ?, status = ?,
            assignee_id = ?, actual_hours = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#,
    )
    .bind(req.title.as_ref().unwrap_or(&current.title))
    .bind(req.description.as_ref().or(current.description.as_ref()))
    .bind(req.status.as_ref().unwrap_or(&current.status))
    .bind(req.assignee_id.or(current.assignee_id))
    .bind(req.actual_hours.or(current.actual_hours))
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}

pub async fn delete_task(pool: &SqlitePool, id: i64) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Task not found".to_string()));
    }
    Ok(())
}

pub async fn update_password(
    pool: &SqlitePool,
    user_id: i64,
    new_password_hash: &str,
) -> Result<(), AppError> {
    let result = sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(new_password_hash)
        .bind(user_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("User not found".to_string()));
    }
    Ok(())
}