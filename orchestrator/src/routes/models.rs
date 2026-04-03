use axum::{Json, extract::State};

use crate::{AppState, error::AppError};

/// GET /v1/models — OpenAI-compatible model listing.
pub(crate) async fn list_models(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let models = state.hosts.available_models().await;

    let data: Vec<serde_json::Value> = models
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "id": name,
                "object": "model",
                "owned_by": "cocompute",
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "object": "list",
        "data": data,
    })))
}
