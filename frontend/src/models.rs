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
    pub is_archived: bool,
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

#[derive(Debug, Clone, Serialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalyticsResponse {
    pub tasks_by_status: TasksByStatus,
    pub tasks_by_user: Vec<UserTaskStats>,
    pub prediction_accuracy: Vec<PredictionPoint>,
    pub avg_time_by_user: Vec<UserAvgTime>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TasksByStatus {
    pub todo: i64,
    pub in_progress: i64,
    pub done: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserTaskStats {
    pub user_id: i64,
    pub user_name: String,
    pub todo: i64,
    pub in_progress: i64,
    pub done: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PredictionPoint {
    pub task_id: i64,
    pub predicted_hours: f64,
    pub actual_hours: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserAvgTime {
    pub user_id: i64,
    pub user_name: String,
    pub avg_hours: f64,
    pub task_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MlStatusResponse {
    pub active_model: Option<String>,
    pub model_candidate_id: Option<i64>,
    pub activated_at: Option<String>,
    pub r2_val: Option<f64>,
    pub medae_val: Option<f64>,
    pub mdape_val: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MlRetrainResponse {
    pub status: String,
    pub run_id: i64,
    pub best_model: Option<String>,
    pub metrics_val: Option<serde_json::Value>,
    pub metrics_test: Option<serde_json::Value>,
    pub activated: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArchiveResponse {
    pub archived: u64,
}