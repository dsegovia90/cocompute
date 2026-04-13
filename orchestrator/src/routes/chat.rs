use axum::{Extension, Json, extract::State, response::IntoResponse};
use axum::response::sse::{Event, Sse};
use common::{
    helpers::{read_frame, write_p2p},
    protocols::{ChatStreamFrame, Request, Response},
};

use crate::{
    AppState,
    auth::{ApiKeyId, PoolContext},
    error::AppError,
    openai::{
        OpenAIChatChoice, OpenAIChatMessage, OpenAIChatRequest, OpenAIChatResponse,
        OpenAIChatStreamChunk, OpenAIChatStreamChoice, OpenAIChatStreamDelta,
        OpenAIChatMessageRaw, OpenAIUsage,
    },
    proxy::{connection_rtt_ms, log_metering, route_to_host},
};

/// POST /v1/chat/completions — OpenAI-compatible chat endpoint.
/// Supports both streaming (SSE) and non-streaming responses.
pub(crate) async fn create_chat_completion(
    State(state): State<AppState>,
    Extension(api_key_id): Extension<ApiKeyId>,
    Extension(pool_ctx): Extension<PoolContext>,
    Json(payload): Json<OpenAIChatRequest>,
) -> Result<axum::response::Response, AppError> {
    let model = payload.model.clone();
    let stream = payload.stream;

    // Translate OpenAI format → internal protocol
    let internal_request = common::protocols::chat::ChatRequest {
        model: payload.model,
        messages: payload
            .messages
            .into_iter()
            .map(|m| m.into_chat_message())
            .collect(),
        temperature: payload.temperature,
        stream,
        think: payload.think,
        tools: OpenAIChatMessageRaw::convert_tools(payload.tools),
    };

    if stream {
        create_chat_completion_stream(state, model, internal_request, api_key_id.0, pool_ctx.0).await
    } else {
        create_chat_completion_sync(state, model, internal_request, api_key_id.0, pool_ctx.0).await
    }
}

pub(crate) async fn create_chat_completion_sync(
    state: AppState,
    model: String,
    internal_request: common::protocols::chat::ChatRequest,
    api_key_id: i32,
    pool_id: Option<i32>,
) -> Result<axum::response::Response, AppError> {
    let request = Request::Chat(internal_request);
    let start = std::time::Instant::now();
    let (response, host_id, iroh_rtt) = route_to_host(&state, &model, request, pool_id).await?;
    let total_ms = start.elapsed().as_millis() as i64;

    match response {
        Response::Chat { result, ref metering } => {
            log_metering(
                state.db.clone(),
                host_id,
                model.clone(),
                "chat".into(),
                metering,
                Some(api_key_id),
                pool_id,
                Some(total_ms),
                iroh_rtt,
            );
            let created = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            Ok(Json(OpenAIChatResponse {
                id: format!("chatcmpl-{}", hex::encode(&created.to_be_bytes())),
                object: "chat.completion",
                created,
                model,
                choices: vec![OpenAIChatChoice {
                    index: 0,
                    message: {
                        let tool_calls: Option<Vec<serde_json::Value>> =
                            if result.message.tool_calls.is_empty() {
                                None
                            } else {
                                Some(
                                    result.message.tool_calls.iter().map(|tc| {
                                        serde_json::json!({
                                            "id": tc.id,
                                            "type": tc.call_type,
                                            "function": {
                                                "name": tc.function.name,
                                                "arguments": tc.function.arguments,
                                            }
                                        })
                                    }).collect(),
                                )
                            };
                        OpenAIChatMessage {
                            role: result.message.role,
                            content: if result.message.content.is_empty() && tool_calls.is_some() {
                                None
                            } else {
                                Some(result.message.content)
                            },
                            tool_calls,
                            tool_call_id: None,
                        }
                    },
                    finish_reason: if result.message.tool_calls.is_empty() {
                        "stop"
                    } else {
                        "tool_calls"
                    },
                }],
                usage: OpenAIUsage::from_metering(metering),
            })
            .into_response())
        }
        _ => Err(AppError::Internal(anyhow::anyhow!("unexpected response type"))),
    }
}

pub(crate) async fn create_chat_completion_stream(
    state: AppState,
    model: String,
    internal_request: common::protocols::chat::ChatRequest,
    api_key_id: i32,
    pool_id: Option<i32>,
) -> Result<axum::response::Response, AppError> {
    let request = Request::Chat(internal_request);
    let start = std::time::Instant::now();

    // Find host and open a bi-stream
    let host = state.hosts.find_host_for_model(&model, pool_id).await;
    let host = match host {
        Some(h) => h,
        None => {
            let available = state.hosts.available_models(pool_id).await;
            if available.is_empty() {
                return Err(AppError::HostUnavailable);
            } else {
                return Err(AppError::ModelNotFound { available });
            }
        }
    };

    let host_id = host.host_id.clone();
    let conn = host.connection.clone();

    let (send, mut recv) = conn
        .open_bi()
        .await
        .map_err(|_| AppError::HostUnavailable)?;

    write_p2p(send, request)
        .await
        .map_err(AppError::Internal)?;

    // Read the ChatStreamStart response
    let first: Option<Response> = read_frame(&mut recv)
        .await
        .map_err(AppError::Internal)?;

    match first {
        Some(Response::ChatStreamStart) => {}
        _ => return Err(AppError::Internal(anyhow::anyhow!("expected ChatStreamStart"))),
    }

    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let chat_id = format!("chatcmpl-{}", hex::encode(&created.to_be_bytes()));

    let db = state.db.clone();
    let model_clone = model.clone();

    // Build SSE stream that reads frames from iroh
    let stream = async_stream::stream! {
        let mut has_tool_calls = false;

        // First chunk: role
        yield Ok::<_, std::convert::Infallible>(Event::default().data(serde_json::to_string(&OpenAIChatStreamChunk {
            id: chat_id.clone(),
            object: "chat.completion.chunk",
            created,
            model: model.clone(),
            choices: vec![OpenAIChatStreamChoice {
                index: 0,
                delta: OpenAIChatStreamDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: None,
                },
                finish_reason: None,
            }],
        }).unwrap()));

        // Read chunks from iroh
        loop {
            match read_frame::<ChatStreamFrame>(&mut recv).await {
                Ok(Some(ChatStreamFrame::Delta(content))) => {
                    yield Ok(Event::default().data(serde_json::to_string(&OpenAIChatStreamChunk {
                        id: chat_id.clone(),
                        object: "chat.completion.chunk",
                        created,
                        model: model.clone(),
                        choices: vec![OpenAIChatStreamChoice {
                            index: 0,
                            delta: OpenAIChatStreamDelta {
                                role: None,
                                content: Some(content),
                                tool_calls: None,
                            },
                            finish_reason: None,
                        }],
                    }).unwrap()));
                }
                Ok(Some(ChatStreamFrame::Thinking(thinking))) => {
                    // Forward thinking as content — most clients display it inline
                    yield Ok(Event::default().data(serde_json::to_string(&OpenAIChatStreamChunk {
                        id: chat_id.clone(),
                        object: "chat.completion.chunk",
                        created,
                        model: model.clone(),
                        choices: vec![OpenAIChatStreamChoice {
                            index: 0,
                            delta: OpenAIChatStreamDelta {
                                role: None,
                                content: Some(thinking),
                                tool_calls: None,
                            },
                            finish_reason: None,
                        }],
                    }).unwrap()));
                }
                Ok(Some(ChatStreamFrame::ToolCalls(tool_calls))) => {
                    let tc_json: Vec<serde_json::Value> = tool_calls.iter().enumerate().map(|(i, tc)| {
                        serde_json::json!({
                            "index": i,
                            "id": tc.id,
                            "type": tc.call_type,
                            "function": {
                                "name": tc.function.name,
                                "arguments": tc.function.arguments,
                            }
                        })
                    }).collect();

                    yield Ok(Event::default().data(serde_json::to_string(&OpenAIChatStreamChunk {
                        id: chat_id.clone(),
                        object: "chat.completion.chunk",
                        created,
                        model: model.clone(),
                        choices: vec![OpenAIChatStreamChoice {
                            index: 0,
                            delta: OpenAIChatStreamDelta {
                                role: None,
                                content: None,
                                tool_calls: Some(tc_json),
                            },
                            finish_reason: None,
                        }],
                    }).unwrap()));

                    has_tool_calls = true;
                }
                Ok(Some(ChatStreamFrame::Done(metering))) => {
                    let total_ms = start.elapsed().as_millis() as i64;
                    let iroh_rtt = connection_rtt_ms(&conn);
                    log_metering(db.clone(), host_id.clone(), model_clone.clone(), "chat_stream".into(), &metering, Some(api_key_id), pool_id, Some(total_ms), iroh_rtt);

                    // Final chunk with finish_reason
                    yield Ok(Event::default().data(serde_json::to_string(&OpenAIChatStreamChunk {
                        id: chat_id.clone(),
                        object: "chat.completion.chunk",
                        created,
                        model: model.clone(),
                        choices: vec![OpenAIChatStreamChoice {
                            index: 0,
                            delta: OpenAIChatStreamDelta {
                                role: None,
                                content: None,
                                tool_calls: None,
                            },
                            finish_reason: Some(if has_tool_calls { "tool_calls" } else { "stop" }),
                        }],
                    }).unwrap()));

                    yield Ok(Event::default().data("[DONE]".to_string()));
                    break;
                }
                Ok(None) => {
                    yield Ok(Event::default().data("[DONE]".to_string()));
                    break;
                }
                Err(e) => {
                    tracing::error!("stream frame error: {e}");
                    break;
                }
            }
        }
    };

    Ok(Sse::new(stream).into_response())
}
