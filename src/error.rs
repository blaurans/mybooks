use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;

pub enum AppError {
    NotFound,
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "non trouvé".to_string()).into_response(),
            AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m).into_response(),
        }
    }
}
pub type AppResult<T> = Result<T, AppError>;
