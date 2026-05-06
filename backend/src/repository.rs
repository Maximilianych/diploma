use sqlx::SqlitePool;
use chrono::Utc;
use crate::models::{
    User, Task, CreateTaskRequest, UpdateTaskRequest,
    TasksByStatus, UserTaskStats, PredictionPoint, UserAvgTime,
};
use crate::errors::AppError;

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
        "#
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
    Ok(sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await?)
}

pub async fn get_all_users(pool: &SqlitePool) -> Result<Vec<User>, AppError> {
    Ok(sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at")
        .fetch_all(pool)
        .await?)
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

// ============ Tasks ============

pub async fn create_task(
    pool: &SqlitePool,
    req: &CreateTaskRequest,
    created_by: i64,
    predicted_seconds: Option<f64>,
) -> Result<Task, AppError> {
    Ok(sqlx::query_as::<_, Task>(
        r#"
        INSERT INTO tasks (title, description, assignee_id, created_by, predicted_seconds)
        VALUES (?, ?, ?, ?, ?)
        RETURNING *
        "#
    )
    .bind(&req.title)
    .bind(&req.description)
    .bind(req.assignee_id)
    .bind(created_by)
    .bind(predicted_seconds)
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

pub async fn get_all_active_tasks(pool: &SqlitePool) -> Result<Vec<Task>, AppError> {
    Ok(sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE is_archived = 0 ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get_tasks_by_assignee(pool: &SqlitePool, user_id: i64) -> Result<Vec<Task>, AppError> {
    Ok(sqlx::query_as::<_, Task>(
        "SELECT * FROM tasks WHERE assignee_id = ? AND is_archived = 0 ORDER BY created_at DESC"
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

    let new_description = match &req.description {
        Some(desc) => desc.clone(),
        None => current.description,
    };

    let new_assignee = match &req.assignee_id {
        Some(id) => *id,
        None => current.assignee_id,
    };

    let new_status = req.status.as_ref().unwrap_or(&current.status);

    let new_actual_seconds = req.actual_hours
        .map(|h| h * 3600.0)
        .or(current.actual_spent_seconds);

    let new_completed_at = if new_status == "done" && current.status != "done" {
        Some(Utc::now())
    } else if new_status != "done" {
        None
    } else {
        current.completed_at
    };

    sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET title = ?, description = ?, status = ?,
            assignee_id = ?, actual_spent_seconds = ?,
            completed_at = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ?
        RETURNING *
        "#
    )
    .bind(req.title.as_ref().unwrap_or(&current.title))
    .bind(&new_description)
    .bind(new_status)
    .bind(new_assignee)
    .bind(new_actual_seconds)
    .bind(new_completed_at)
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

pub async fn archive_completed_tasks(pool: &SqlitePool) -> Result<u64, AppError> {
    let result = sqlx::query(
        "UPDATE tasks SET is_archived = 1 WHERE status = 'done' AND is_archived = 0"
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// ============ Analytics ============

pub async fn get_tasks_by_status_counts(pool: &SqlitePool) -> Result<TasksByStatus, AppError> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*) FROM tasks WHERE is_archived = 0 GROUP BY status"
    )
    .fetch_all(pool)
    .await?;

    let mut stats = TasksByStatus { todo: 0, in_progress: 0, done: 0 };
    for (status, count) in rows {
        match status.as_str() {
            "todo" => stats.todo = count,
            "in_progress" => stats.in_progress = count,
            "done" => stats.done = count,
            _ => {}
        }
    }
    Ok(stats)
}

pub async fn get_tasks_by_user_counts(pool: &SqlitePool) -> Result<Vec<UserTaskStats>, AppError> {
    let rows: Vec<(i64, String, String, i64)> = sqlx::query_as(
        r#"
        SELECT u.id, u.name, t.status, COUNT(*)
        FROM tasks t
        JOIN users u ON t.assignee_id = u.id
        WHERE t.is_archived = 0
        GROUP BY u.id, u.name, t.status
        ORDER BY u.name
        "#
    )
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<i64, UserTaskStats> = std::collections::HashMap::new();
    for (uid, name, status, count) in rows {
        let entry = map.entry(uid).or_insert(UserTaskStats {
            user_id: uid,
            user_name: name,
            todo: 0,
            in_progress: 0,
            done: 0,
        });
        match status.as_str() {
            "todo" => entry.todo = count,
            "in_progress" => entry.in_progress = count,
            "done" => entry.done = count,
            _ => {}
        }
    }
    Ok(map.into_values().collect())
}

pub async fn get_prediction_accuracy(pool: &SqlitePool) -> Result<Vec<PredictionPoint>, AppError> {
    Ok(sqlx::query_as::<_, (i64, f64, f64)>(
        r#"
        SELECT id, predicted_seconds, actual_spent_seconds
        FROM tasks
        WHERE status = 'done'
          AND predicted_seconds IS NOT NULL
          AND actual_spent_seconds IS NOT NULL
          AND actual_spent_seconds > 0
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(id, pred, actual)| PredictionPoint {
        task_id: id,
        predicted_hours: pred / 3600.0,
        actual_hours: actual / 3600.0,
    })
    .collect())
}

pub async fn get_avg_time_by_user(pool: &SqlitePool) -> Result<Vec<UserAvgTime>, AppError> {
    Ok(sqlx::query_as::<_, (i64, String, f64, i64)>(
        r#"
        SELECT u.id, u.name,
               AVG(t.actual_spent_seconds) / 3600.0,
               COUNT(*)
        FROM tasks t
        JOIN users u ON t.assignee_id = u.id
        WHERE t.status = 'done'
          AND t.actual_spent_seconds IS NOT NULL
          AND t.actual_spent_seconds > 0
        GROUP BY u.id, u.name
        "#
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|(id, name, avg, count)| UserAvgTime {
        user_id: id,
        user_name: name,
        avg_hours: avg,
        task_count: count,
    })
    .collect())
}

pub async fn update_prediction(
    pool: &SqlitePool,
    id: i64,
    predicted_seconds: Option<f64>,
) -> Result<Task, AppError> {
    sqlx::query_as::<_, Task>(
        r#"
        UPDATE tasks
        SET predicted_seconds = ?
        WHERE id = ?
        RETURNING *
        "#
    )
    .bind(predicted_seconds)
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.into())
}