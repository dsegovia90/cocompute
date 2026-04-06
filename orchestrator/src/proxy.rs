use common::{
    helpers::{read_p2p, write_p2p},
    protocols::{Metering, Request, Response},
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

use crate::{db, error::AppError, AppState};

/// Send a request to a host using its stored connection.
/// Opens a new bi-stream on the existing connection.
pub(crate) async fn send_to_host(
    connection: &iroh::endpoint::Connection,
    request: Request,
) -> Result<Response, AppError> {
    let (send, recv) = connection
        .open_bi()
        .await
        .map_err(|_| AppError::HostUnavailable)?;

    write_p2p(send, request)
        .await
        .map_err(AppError::Internal)?;

    let response: Response = read_p2p(recv)
        .await
        .map_err(AppError::Internal)?;

    Ok(response)
}

/// Route a request to the appropriate host based on model name.
pub(crate) async fn route_to_host(
    state: &AppState,
    model: &str,
    request: Request,
) -> Result<(Response, String), AppError> {
    let host = state.hosts.find_host_for_model(model).await;

    match host {
        Some(h) => {
            let eid = h.endpoint_id.clone();
            let resp = send_to_host(&h.connection, request).await?;
            Ok((resp, eid))
        }
        None => {
            let available = state.hosts.available_models().await;
            if available.is_empty() {
                Err(AppError::HostUnavailable)
            } else {
                Err(AppError::ModelNotFound { available })
            }
        }
    }
}

/// Log metering data to the database (fire-and-forget).
pub(crate) fn log_metering(
    db: DatabaseConnection,
    host_endpoint_id: String,
    model: String,
    request_type: String,
    metering: &Metering,
    api_key_id: Option<i32>,
    total_ms: Option<i64>,
) {
    let m = metering.clone();
    tokio::spawn(async move {
        let record = db::entities::metering_logs::ActiveModel {
            host_endpoint_id: Set(host_endpoint_id),
            model: Set(model),
            request_type: Set(request_type),
            prompt_tokens: Set(m.prompt_tokens as i32),
            completion_tokens: Set(m.completion_tokens as i32),
            compute_ms: Set(m.compute_ms as i64),
            total_ms: Set(total_ms),
            created_at: Set(chrono::Utc::now()),
            api_key_id: Set(api_key_id),
            ..Default::default()
        };
        if let Err(e) = record.insert(&db).await {
            tracing::error!("failed to log metering: {e}");
        }
    });
}
