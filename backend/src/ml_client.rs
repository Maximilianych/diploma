use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::errors::AppError;
use crate::models::{MlStatusResponse, MlRetrainResponse};

#[derive(Clone)]
pub struct MlClient {
    base_url: String,
    client: Client,
}

#[derive(Serialize)]
struct PredictRequest {
    summary: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct PredictResponse {
    predicted_seconds: f64,
}

impl MlClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: Client::new(),
        }
    }

    pub async fn predict_time(
        &self,
        title: &str,
        description: Option<&str>,
    ) -> Result<f64, AppError> {
        let request = PredictRequest {
            summary: title.to_string(),
            description: description.map(|s| s.to_string()),
        };

        let response = self
            .client
            .post(format!("{}/predict", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("ML service unavailable: {}", e);
                AppError::Internal("ML service unavailable".to_string())
            })?;

        if !response.status().is_success() {
            tracing::warn!("ML service returned error: {}", response.status());
            return Err(AppError::Internal("ML service error".to_string()));
        }

        let result: PredictResponse = response.json().await.map_err(|e| {
            tracing::warn!("Failed to parse ML response: {}", e);
            AppError::Internal("Invalid ML response".to_string())
        })?;

        Ok(result.predicted_seconds)
    }

    pub async fn predict_time_safe(
        &self,
        title: &str,
        description: Option<&str>,
    ) -> Option<f64> {
        self.predict_time(title, description).await.ok()
    }

    pub async fn get_status(&self) -> Result<MlStatusResponse, AppError> {
        let response = self
            .client
            .get(format!("{}/status", self.base_url))
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("ML service unavailable: {}", e);
                AppError::Internal("ML service unavailable".to_string())
            })?;

        response.json().await.map_err(|e| {
            tracing::warn!("Failed to parse ML status: {}", e);
            AppError::Internal("Invalid ML response".to_string())
        })
    }

    pub async fn trigger_retrain(&self) -> Result<MlRetrainResponse, AppError> {
        let response = self
            .client
            .post(format!("{}/retrain", self.base_url))
            .send()
            .await
            .map_err(|e| {
                tracing::warn!("ML service unavailable: {}", e);
                AppError::Internal("ML service unavailable".to_string())
            })?;

        response.json().await.map_err(|e| {
            tracing::warn!("Failed to parse retrain response: {}", e);
            AppError::Internal("Invalid ML response".to_string())
        })
    }
}