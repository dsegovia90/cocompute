use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    HostUnavailable,
    ModelNotFound { available: Vec<String> },
    Unauthorized,
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            AppError::HostUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                json!({ "error": "host unavailable", "retry_after": 5 }),
            ),
            AppError::ModelNotFound { ref available } => (
                StatusCode::NOT_FOUND,
                json!({ "error": "model not found", "available_models": available }),
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                json!({ "error": "unauthorized" }),
            ),
            AppError::Internal(ref e) => {
                tracing::error!("internal error: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": "internal server error" }),
                )
            }
        };

        let mut response = (status, Json(body)).into_response();
        if matches!(&self, AppError::HostUnavailable) {
            response
                .headers_mut()
                .insert("Retry-After", "5".parse().unwrap());
        }
        response
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        AppError::Internal(e)
    }
}
