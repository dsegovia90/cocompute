use axum::{Extension, Json, extract::State};
use common::protocols::Request;

use crate::{
    AppState,
    auth::{ApiKeyId, PoolContext},
    error::AppError,
    openai::{OpenAIEmbeddingData, OpenAIEmbeddingsRequest, OpenAIEmbeddingsResponse, OpenAIUsage},
    proxy::{log_metering, route_to_host},
};

/// POST /v1/embeddings, OpenAI-compatible embeddings endpoint.
pub(crate) async fn create_embeddings(
    State(state): State<AppState>,
    Extension(api_key_id): Extension<ApiKeyId>,
    Extension(pool_ctx): Extension<PoolContext>,
    Json(payload): Json<OpenAIEmbeddingsRequest>,
) -> Result<Json<OpenAIEmbeddingsResponse>, AppError> {
    use common::protocols::Response;

    let model = payload.model.clone();

    // Translate OpenAI format → internal protocol
    let internal_request = common::protocols::embeddings::EmbeddingsRequest {
        model: payload.model,
        text: payload.input,
    };

    let request = Request::Embeddings(internal_request);
    let start = std::time::Instant::now();
    let (response, host_id, iroh_rtt) = route_to_host(&state, &model, request, pool_ctx.0).await?;
    let total_ms = start.elapsed().as_millis() as i64;

    match response {
        Response::Embeddings { result, ref metering } => {
            log_metering(
                state.db.clone(),
                host_id,
                model.clone(),
                "embeddings".into(),
                metering,
                Some(api_key_id.0),
                pool_ctx.0,
                Some(total_ms),
                iroh_rtt,
            );
            Ok(Json(OpenAIEmbeddingsResponse {
                object: "list",
                data: vec![OpenAIEmbeddingData {
                    object: "embedding",
                    embedding: result.embeddings,
                    index: 0,
                }],
                model,
                usage: OpenAIUsage::from_metering(metering),
            }))
        }
        _ => Err(AppError::Internal(anyhow::anyhow!("unexpected response type"))),
    }
}
