use axum::{Extension, Json, extract::State};

use crate::{AppState, auth::PoolContext, error::AppError};

/// GET /v1/models, OpenAI-compatible model listing, scoped to the API key's pool.
pub(crate) async fn list_models(
    State(state): State<AppState>,
    Extension(pool_ctx): Extension<PoolContext>,
) -> Result<Json<serde_json::Value>, AppError> {
    let models = state.hosts.available_models(pool_ctx.0).await;

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
