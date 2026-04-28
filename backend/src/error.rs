use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::openrouter::ChatProviderError;

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub error: ApiErrorDetail,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid json: {0}")]
    InvalidJson(String),
    #[error("database error: {0}")]
    Database(#[source] sqlx::Error),
    #[error("provider error: {0}")]
    Provider(#[from] ChatProviderError),
    #[error("configuration error: {0}")]
    Config(String),
}

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn invalid_json(rejection: JsonRejection) -> Self {
        Self::InvalidJson(rejection.body_text())
    }

    pub fn database(error: sqlx::Error) -> Self {
        Self::Database(error)
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    pub fn detail(&self) -> ApiErrorDetail {
        match self {
            Self::Validation(message) => ApiErrorDetail {
                code: "validation_error".to_string(),
                message: message.clone(),
                details: None,
            },
            Self::NotFound(message) => ApiErrorDetail {
                code: "not_found".to_string(),
                message: message.clone(),
                details: None,
            },
            Self::InvalidJson(message) => ApiErrorDetail {
                code: "invalid_json".to_string(),
                message: message.clone(),
                details: None,
            },
            Self::Database(_) => ApiErrorDetail {
                code: "database_error".to_string(),
                message: "database operation failed".to_string(),
                details: None,
            },
            Self::Provider(error) => ApiErrorDetail {
                code: "provider_error".to_string(),
                message: error.public_message(),
                details: None,
            },
            Self::Config(message) => ApiErrorDetail {
                code: "configuration_error".to_string(),
                message: message.clone(),
                details: None,
            },
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Validation(_) | Self::InvalidJson(_) => StatusCode::BAD_REQUEST,
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Provider(_) => StatusCode::BAD_GATEWAY,
            Self::Database(_) | Self::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = ApiErrorBody {
            error: self.detail(),
        };
        (status, Json(body)).into_response()
    }
}
