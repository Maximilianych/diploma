use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub predicted_hours: Option<f64>,
    pub actual_hours: Option<f64>,
    pub assignee_id: Option<i64>,
    pub created_by: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub assignee_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<Option<String>>,
    pub status: Option<String>,
    pub assignee_id: Option<Option<i64>>,
    pub actual_hours: Option<f64>,
}