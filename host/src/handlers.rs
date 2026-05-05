// SPDX-License-Identifier: AGPL-3.0-only

use common::{
    helpers::{read_p2p, write_p2p},
    protocols::{
        Metering, Request, Response,
        chat::{ChatMessage, ChatRequest, ChatResponse},
        embeddings::{EmbeddingsRequest, EmbeddingsResponse},
        registry::RegistryRequest,
    },
};
use ollama_rs::{
    Ollama,
    generation::{
        chat::{MessageRole, request::ChatMessageRequest},
        embeddings::request::GenerateEmbeddingsRequest,
        parameters::ThinkType,
    },
};

use crate::ollama::{convert_messages, convert_tool_calls, convert_tools, ollama_model_name};

/// Dispatch a single inference stream received from the orchestrator.
pub(crate) async fn handle_inference_stream(
    ollama: Ollama,
    mut send: iroh::endpoint::SendStream,
    recv: iroh::endpoint::RecvStream,
) -> anyhow::Result<()> {
    let request: Request = read_p2p(recv).await?;

    match request {
        Request::Embeddings(req) => {
            tracing::info!("embeddings request for model: {}", req.model);
            let response = handle_embeddings(&ollama, req).await?;
            write_p2p(send, response).await?;
        }
        Request::Chat(req) if req.stream => {
            tracing::info!("streaming chat request for model: {}", req.model);
            handle_chat_stream(&ollama, req, &mut send).await?;
            send.finish()?;
        }
        Request::Chat(req) => {
            tracing::info!("chat request for model: {}", req.model);
            let response = handle_chat(&ollama, req).await?;
            write_p2p(send, response).await?;
        }
        Request::Registry(req) => {
            tracing::debug!("registry request on inference stream");
            let response = handle_registry(req);
            write_p2p(send, response).await?;
        }
    };

    Ok(())
}

pub(crate) async fn handle_embeddings(
    ollama: &Ollama,
    req: EmbeddingsRequest,
) -> anyhow::Result<Response> {
    let request = GenerateEmbeddingsRequest::new(ollama_model_name(&req.model), req.text.into());

    let start = std::time::Instant::now();
    let res = ollama
        .generate_embeddings(request)
        .await
        .map_err(|e| anyhow::anyhow!("ollama embeddings error: {e}"))?;
    let compute_ms = start.elapsed().as_millis() as u64;

    Ok(Response::Embeddings {
        result: EmbeddingsResponse::new(res.embeddings[0].clone()),
        metering: Metering {
            prompt_tokens: 0,
            completion_tokens: 0,
            compute_ms,
        },
    })
}

pub(crate) async fn handle_chat(ollama: &Ollama, req: ChatRequest) -> anyhow::Result<Response> {
    let messages = convert_messages(req.messages);
    let mut request = ChatMessageRequest::new(ollama_model_name(&req.model), messages);

    let think = req.think.unwrap_or(false);
    request = request.think(if think { ThinkType::True } else { ThinkType::False });

    if !req.tools.is_empty() {
        tracing::info!("forwarding {} tool definitions to Ollama", req.tools.len());
        let tool_infos = convert_tools(&req.tools);
        tracing::info!(
            "{} tools successfully parsed (of {} total)",
            tool_infos.len(),
            req.tools.len()
        );
        request = request.tools(tool_infos);
    }

    let start = std::time::Instant::now();
    let res = ollama
        .send_chat_messages(request)
        .await
        .map_err(|e| anyhow::anyhow!("ollama chat error: {e}"))?;
    let compute_ms = start.elapsed().as_millis() as u64;

    let role = match res.message.role {
        MessageRole::Assistant => "assistant",
        MessageRole::User => "user",
        MessageRole::System => "system",
        _ => "assistant",
    };

    let tool_calls = convert_tool_calls(res.message.tool_calls);

    let response_message = ChatMessage {
        role: role.to_string(),
        content: res.message.content,
        images: vec![],
        tool_calls,
        tool_call_id: None,
    };

    let (prompt_tokens, completion_tokens) = res
        .final_data
        .map(|d| (d.prompt_eval_count as u32, d.eval_count as u32))
        .unwrap_or((0, 0));

    Ok(Response::Chat {
        result: ChatResponse {
            message: response_message,
        },
        metering: Metering {
            prompt_tokens,
            completion_tokens,
            compute_ms,
        },
    })
}

pub(crate) async fn handle_chat_stream(
    ollama: &Ollama,
    req: ChatRequest,
    send: &mut iroh::endpoint::SendStream,
) -> anyhow::Result<()> {
    use common::helpers::write_frame;
    use common::protocols::ChatStreamFrame;
    use tokio_stream::StreamExt;

    let messages = convert_messages(req.messages);
    let mut request = ChatMessageRequest::new(ollama_model_name(&req.model), messages);

    let think = req.think.unwrap_or(false);
    request = request.think(if think { ThinkType::True } else { ThinkType::False });

    let has_tools = !req.tools.is_empty();

    if has_tools {
        tracing::info!(
            "forwarding {} tool definitions to Ollama (streaming)",
            req.tools.len()
        );
        let tool_infos = convert_tools(&req.tools);
        tracing::info!("{} tools successfully parsed (streaming)", tool_infos.len());
        request = request.tools(tool_infos);
    }

    // Signal that we're starting a stream
    write_frame(send, Response::ChatStreamStart).await?;

    // Ollama does not support tool calls in streaming mode — tools are silently ignored.
    // When tools are present, use the non-streaming API and emit the result as stream frames.
    if has_tools {
        tracing::info!("tools present, using non-streaming Ollama API for tool call support");
        let start = std::time::Instant::now();
        let res = ollama
            .send_chat_messages(request)
            .await
            .map_err(|e| anyhow::anyhow!("ollama chat error: {e}"))?;
        let compute_ms = start.elapsed().as_millis() as u64;

        if !res.message.content.is_empty() {
            write_frame(send, ChatStreamFrame::Delta(res.message.content.clone())).await?;
        }

        if !res.message.tool_calls.is_empty() {
            tracing::info!(
                "response contains {} tool calls",
                res.message.tool_calls.len()
            );
            let tool_calls = convert_tool_calls(res.message.tool_calls);
            write_frame(send, ChatStreamFrame::ToolCalls(tool_calls)).await?;
        }

        let (prompt_tokens, completion_tokens) = res
            .final_data
            .map(|d| (d.prompt_eval_count as u32, d.eval_count as u32))
            .unwrap_or((0, 0));

        write_frame(
            send,
            ChatStreamFrame::Done(Metering {
                prompt_tokens,
                completion_tokens,
                compute_ms,
            }),
        )
        .await?;
    } else {
        let start = std::time::Instant::now();
        let mut stream = ollama
            .send_chat_messages_stream(request)
            .await
            .map_err(|e| anyhow::anyhow!("ollama stream error: {e}"))?;

        let mut prompt_tokens: u32 = 0;
        let mut completion_tokens: u32 = 0;
        let mut chunk_count = 0u32;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(resp) => {
                    let content = resp.message.content.clone();
                    if !content.is_empty() {
                        chunk_count += 1;
                        tracing::debug!("streaming chunk #{chunk_count}: {:?}", &content);
                        write_frame(send, ChatStreamFrame::Delta(content)).await?;
                    }

                    if let Some(ref thinking) = resp.message.thinking {
                        if !thinking.is_empty() {
                            chunk_count += 1;
                            tracing::debug!("streaming thinking chunk #{chunk_count}");
                            write_frame(send, ChatStreamFrame::Thinking(thinking.clone()))
                                .await?;
                        }
                    }

                    if resp.done {
                        tracing::debug!("ollama stream done after {chunk_count} chunks");
                        if let Some(final_data) = resp.final_data {
                            prompt_tokens = final_data.prompt_eval_count as u32;
                            completion_tokens = final_data.eval_count as u32;
                        }
                    }
                }
                Err(_) => {
                    tracing::warn!("ollama stream chunk error");
                    break;
                }
            }
        }
        tracing::debug!("ollama stream ended, sent {chunk_count} chunks");

        let compute_ms = start.elapsed().as_millis() as u64;

        write_frame(
            send,
            ChatStreamFrame::Done(Metering {
                prompt_tokens,
                completion_tokens,
                compute_ms,
            }),
        )
        .await?;
    }

    Ok(())
}

pub(crate) fn handle_registry(req: RegistryRequest) -> Response {
    match req {
        RegistryRequest::Register { capabilities, .. } => {
            tracing::info!("re-registration with {} models", capabilities.models.len());
            Response::Registry(common::protocols::registry::RegistryResponse::Ack)
        }
        RegistryRequest::Heartbeat => {
            tracing::debug!("heartbeat");
            Response::Registry(common::protocols::registry::RegistryResponse::Ack)
        }
    }
}
