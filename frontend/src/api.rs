use crate::models::*;
use gloo_storage::{LocalStorage, Storage};

const API_URL: &str = "http://localhost:8888/api";
const TOKEN_KEY: &str = "auth_token";

pub fn get_token() -> Option<String> {
    LocalStorage::get(TOKEN_KEY).ok()
}

pub fn set_token(token: &str) {
    let _ = LocalStorage::set(TOKEN_KEY, token);
}

pub fn clear_token() {
    LocalStorage::delete(TOKEN_KEY);
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn auth_header() -> Result<String, String> {
    let token = get_token().ok_or("Not authenticated")?;
    Ok(format!("Bearer {}", token))
}

// ============ Auth ============

pub async fn login(email: String, password: String) -> Result<AuthResponse, String> {
    let response = client()
        .post(format!("{}/login", API_URL))
        .json(&LoginRequest { email, password })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let auth: AuthResponse = response.json().await.map_err(|e| e.to_string())?;
        set_token(&auth.token);
        Ok(auth)
    } else {
        Err("Invalid credentials".to_string())
    }
}

// ============ Users ============

pub async fn get_users() -> Result<Vec<User>, String> {
    let response = client()
        .get(format!("{}/users", API_URL))
        .header("Authorization", auth_header()?)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to fetch users".to_string())
    }
}

pub async fn create_user(req: CreateUserRequest) -> Result<User, String> {
    let response = client()
        .post(format!("{}/users", API_URL))
        .header("Authorization", auth_header()?)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to create user".to_string())
    }
}

pub async fn delete_user(id: i64) -> Result<(), String> {
    let response = client()
        .delete(format!("{}/users/{}", API_URL, id))
        .header("Authorization", auth_header()?)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err("Failed to delete user".to_string())
    }
}

// ============ Tasks ============

pub async fn get_tasks() -> Result<Vec<Task>, String> {
    let response = client()
        .get(format!("{}/tasks", API_URL))
        .header("Authorization", auth_header()?)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to fetch tasks".to_string())
    }
}

pub async fn get_my_tasks() -> Result<Vec<Task>, String> {
    let response = client()
        .get(format!("{}/tasks/my", API_URL))
        .header("Authorization", auth_header()?)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to fetch my tasks".to_string())
    }
}

pub async fn create_task(
    title: String,
    description: Option<String>,
    assignee_id: Option<i64>,
) -> Result<Task, String> {
    let response = client()
        .post(format!("{}/tasks", API_URL))
        .header("Authorization", auth_header()?)
        .json(&CreateTaskRequest {
            title,
            description,
            assignee_id,
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to create task".to_string())
    }
}

pub async fn update_task(id: i64, req: UpdateTaskRequest) -> Result<Task, String> {
    let response = client()
        .put(format!("{}/tasks/{}", API_URL, id))
        .header("Authorization", auth_header()?)
        .json(&req)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to update task".to_string())
    }
}

pub async fn delete_task(id: i64) -> Result<(), String> {
    let response = client()
        .delete(format!("{}/tasks/{}", API_URL, id))
        .header("Authorization", auth_header()?)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err("Failed to delete task".to_string())
    }
}

pub async fn update_task_status(id: i64, status: String) -> Result<Task, String> {
    update_task(id, UpdateTaskRequest {
        title: None,
        description: None,
        status: Some(status),
        assignee_id: None,
        actual_hours: None,
    }).await
}

// ============ Analytics ============

pub async fn get_analytics() -> Result<AnalyticsResponse, String> {
    let response = client()
        .get(format!("{}/analytics", API_URL))
        .header("Authorization", auth_header()?)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to fetch analytics".to_string())
    }
}

// ============ Admin ============

pub async fn archive_completed() -> Result<ArchiveResponse, String> {
    let response = client()
        .post(format!("{}/admin/archive", API_URL))
        .header("Authorization", auth_header()?)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to archive".to_string())
    }
}

pub async fn get_ml_status() -> Result<MlStatusResponse, String> {
    let response = client()
        .get(format!("{}/admin/ml/status", API_URL))
        .header("Authorization", auth_header()?)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to fetch ML status".to_string())
    }
}

pub async fn trigger_retrain() -> Result<MlRetrainResponse, String> {
    let response = client()
        .post(format!("{}/admin/ml/retrain", API_URL))
        .header("Authorization", auth_header()?)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        response.json().await.map_err(|e| e.to_string())
    } else {
        Err("Failed to trigger retrain".to_string())
    }
}