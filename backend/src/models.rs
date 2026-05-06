use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============ User ============

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub name: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub name: String,
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "member".to_string()
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

// ============ Task ============

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub predicted_seconds: Option<f64>,
    pub actual_spent_seconds: Option<f64>,
    pub assignee_id: Option<i64>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub is_archived: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskResponse {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub predicted_hours: Option<f64>,
    pub actual_hours: Option<f64>,
    pub assignee_id: Option<i64>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub is_archived: bool,
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            title: task.title,
            description: task.description,
            status: task.status,
            predicted_hours: task.predicted_seconds.map(|s| s / 3600.0),
            actual_hours: task.actual_spent_seconds.map(|s| s / 3600.0),
            assignee_id: task.assignee_id,
            created_by: task.created_by,
            created_at: task.created_at,
            updated_at: task.updated_at,
            completed_at: task.completed_at,
            is_archived: task.is_archived,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub assignee_id: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub description: Option<Option<String>>,
    pub status: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub assignee_id: Option<Option<i64>>,
    pub actual_hours: Option<f64>,
}

fn deserialize_optional_field<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}

// ============ Auth context ============

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: i64,
    pub role: String,
}

// =========== Analytics ============

#[derive(Debug, Serialize)]
pub struct AnalyticsResponse {
    pub tasks_by_status: TasksByStatus,
    pub tasks_by_user: Vec<UserTaskStats>,
    pub prediction_accuracy: Vec<PredictionPoint>,
    pub avg_time_by_user: Vec<UserAvgTime>,
}

#[derive(Debug, Serialize)]
pub struct TasksByStatus {
    pub todo: i64,
    pub in_progress: i64,
    pub done: i64,
}

#[derive(Debug, Serialize)]
pub struct UserTaskStats {
    pub user_id: i64,
    pub user_name: String,
    pub todo: i64,
    pub in_progress: i64,
    pub done: i64,
}

#[derive(Debug, Serialize)]
pub struct PredictionPoint {
    pub task_id: i64,
    pub predicted_hours: f64,
    pub actual_hours: f64,
}

#[derive(Debug, Serialize)]
pub struct UserAvgTime {
    pub user_id: i64,
    pub user_name: String,
    pub avg_hours: f64,
    pub task_count: i64,
}

// =========== ML ============

#[derive(Debug, Serialize, Deserialize)]
pub struct MlStatusResponse {
    pub active_model: Option<String>,
    pub model_candidate_id: Option<i64>,
    pub activated_at: Option<String>,
    pub r2_val: Option<f64>,
    pub medae_val: Option<f64>,
    pub mdape_val: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MlRetrainResponse {
    pub status: String,
    pub run_id: i64,
    pub best_model: Option<String>,
    pub metrics_val: Option<serde_json::Value>,
    pub metrics_test: Option<serde_json::Value>,
    pub activated: bool,
    pub message: Option<String>,
}