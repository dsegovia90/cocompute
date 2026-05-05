// SPDX-License-Identifier: AGPL-3.0-only

use axum::{Json, extract::State};

use crate::{AppState, error::AppError};

/// GET /v1/node-info — Returns the orchestrator's current iroh endpoint ID.
/// Unauthenticated — used by hosts to discover/refresh the orchestrator ID.
pub(crate) async fn get_node_info(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "endpoint_id": state.endpoint_id,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// GET /v1/version — Returns the orchestrator version for update checks.
pub(crate) async fn get_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// GET /v1/update/:platform — Serves host binary for the given platform.
/// Platforms: linux-x86_64, linux-arm64, macos-arm64, macos-x86_64
pub(crate) async fn get_update(
    axum::extract::Path(platform): axum::extract::Path<String>,
) -> Result<axum::response::Response, AppError> {
    let binary_path = format!("/opt/binaries/cocompute-host-{platform}");
    let path = std::path::Path::new(&binary_path);

    if !path.exists() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "binary not found for platform: {platform}. Available: linux-x86_64, linux-arm64, macos-arm64, macos-x86_64"
        )));
    }

    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to read binary: {e}")))?;

    Ok(axum::response::Response::builder()
        .header("content-type", "application/octet-stream")
        .header(
            "content-disposition",
            format!("attachment; filename=\"cocompute-host-{platform}\""),
        )
        .body(axum::body::Body::from(bytes))
        .unwrap())
}
