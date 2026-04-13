use common::{
    helpers::{read_p2p, write_p2p},
    protocols::{Metering, Request, Response},
};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

use crate::{db, error::AppError, AppState};

/// Extract the current QUIC RTT from a connection's selected path.
pub(crate) fn connection_rtt_ms(connection: &iroh::endpoint::Connection) -> Option<f64> {
    let info = connection.to_info();
    let path = info.selected_path()?;
    let rtt = path.rtt()?;
    Some(rtt.as_secs_f64() * 1000.0)
}

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

/// Route a request to the appropriate host based on model name, optionally filtered by pool.
/// Returns (response, host_id, iroh_rtt_ms).
pub(crate) async fn route_to_host(
    state: &AppState,
    model: &str,
    request: Request,
    pool_id: Option<i32>,
) -> Result<(Response, String, Option<f64>), AppError> {
    let host = state.hosts.find_host_for_model(model, pool_id).await;

    match host {
        Some(h) => {
            let hid = h.host_id.clone();
            let resp = send_to_host(&h.connection, request).await?;
            let rtt = connection_rtt_ms(&h.connection);
            Ok((resp, hid, rtt))
        }
        None => {
            let available = state.hosts.available_models(pool_id).await;
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
    pool_id: Option<i32>,
    total_ms: Option<i64>,
    iroh_rtt_ms: Option<f64>,
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
            iroh_rtt_ms: Set(iroh_rtt_ms),
            created_at: Set(chrono::Utc::now()),
            api_key_id: Set(api_key_id),
            pool_id: Set(pool_id),
            ..Default::default()
        };
        if let Err(e) = record.insert(&db).await {
            tracing::error!("failed to log metering: {e}");
        }
    });
}
