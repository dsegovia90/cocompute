use axum::{Json, extract::State};

use crate::{AppState, error::AppError};

/// GET /v1/stats — Usage statistics from metering logs.
pub(crate) async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder, QuerySelect};

    let total_requests: u64 = crate::db::entities::metering_logs::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("db error: {e}")))?;

    let recent = crate::db::entities::metering_logs::Entity::find()
        .order_by_desc(crate::db::entities::metering_logs::Column::CreatedAt)
        .limit(10)
        .all(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("db error: {e}")))?;

    let recent_entries: Vec<serde_json::Value> = recent
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "host": r.host_endpoint_id,
                "model": r.model,
                "type": r.request_type,
                "prompt_tokens": r.prompt_tokens,
                "completion_tokens": r.completion_tokens,
                "compute_ms": r.compute_ms,
                "total_ms": r.total_ms,
                "overhead_ms": r.total_ms.map(|t| t - r.compute_ms),
                "created_at": r.created_at.to_string(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "total_requests": total_requests,
        "recent": recent_entries,
    })))
}
