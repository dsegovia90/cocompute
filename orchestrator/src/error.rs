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
    NotFound(String),
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
            AppError::NotFound(ref msg) => (
                StatusCode::NOT_FOUND,
                json!({ "error": msg }),
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

/// Error type for web (HTML) routes, redirects or re-renders pages with error messages.
#[derive(Debug)]
pub enum WebError {
    /// 303 redirect (e.g., after login failure, redirect to /login?error=...)
    Redirect(String),
    /// Render an error on the current page
    Form { message: String, status: StatusCode },
    /// Internal error
    Internal(anyhow::Error),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        match self {
            WebError::Redirect(url) => axum::response::Redirect::to(&url).into_response(),
            WebError::Form { message, status } => {
                (status, axum::response::Html(format!(
                    r#"<!DOCTYPE html><html><body><h1>Error</h1><p>{}</p></body></html>"#,
                    message
                ))).into_response()
            }
            WebError::Internal(e) => {
                tracing::error!("web internal error: {e:?}");
                (StatusCode::INTERNAL_SERVER_ERROR, axum::response::Html(
                    "<!DOCTYPE html><html><body><h1>Internal Server Error</h1></body></html>".to_string()
                )).into_response()
            }
        }
    }
}

impl From<anyhow::Error> for WebError {
    fn from(e: anyhow::Error) -> Self {
        WebError::Internal(e)
    }
}
