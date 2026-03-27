use std::str::FromStr;

use clap::Parser;
use common::{
    helpers::{read_p2p, write_p2p},
    protocols::{
        self, Metering, Request, Response,
        chat::{ChatMessage, ChatRequest, ChatResponse},
        embeddings::{EmbeddingsRequest, EmbeddingsResponse},
        registry::{Capabilities, ModelInfo, RegistryRequest},
    },
};
use ollama_rs::{
    Ollama,
    generation::{
        chat::{ChatMessage as OllamaChatMessage, request::ChatMessageRequest},
        embeddings::request::GenerateEmbeddingsRequest,
    },
};

#[derive(Parser, Debug)]
#[command(name = "cocompute-host", version)]
struct Args {
    /// Ollama server URL
    #[arg(long, default_value = "http://localhost", env = "OLLAMA_URL")]
    ollama_url: String,

    /// Ollama server port
    #[arg(long, default_value = "11434", env = "OLLAMA_PORT")]
    ollama_port: u16,

    /// Orchestrator HTTP URL for discovery (e.g. http://192.168.1.100:3000)
    #[arg(long, env = "COCOMPUTE_ORCHESTRATOR_URL")]
    orchestrator_url: String,

    /// Orchestrator endpoint ID (optional — fetched from orchestrator if not provided)
    #[arg(long, env = "COCOMPUTE_ORCHESTRATOR_ID")]
    orchestrator_id: Option<String>,

    /// Path to persist the iroh secret key for stable EndpointId
    #[arg(long, default_value = "~/.cocompute/host.key", env = "COCOMPUTE_KEY_PATH")]
    key_path: String,
}

/// Handle a single inference stream from the orchestrator.
async fn handle_inference_stream(
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

async fn handle_embeddings(ollama: &Ollama, req: EmbeddingsRequest) -> anyhow::Result<Response> {
    let request = GenerateEmbeddingsRequest::new(req.model.clone(), req.text.into());

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

async fn handle_chat(ollama: &Ollama, req: ChatRequest) -> anyhow::Result<Response> {
    use ollama_rs::generation::chat::MessageRole;
    use ollama_rs::generation::parameters::ThinkType;

    let messages: Vec<OllamaChatMessage> = req
        .messages
        .into_iter()
        .map(|m| {
            let role = match m.role.as_str() {
                "system" => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::User,
            };
            let mut msg = OllamaChatMessage::new(role, m.content);
            if !m.images.is_empty() {
                let images = m.images.into_iter()
                    .map(ollama_rs::generation::images::Image::from_base64)
                    .collect();
                msg = msg.with_images(images);
            }
            msg
        })
        .collect();

    let mut request = ChatMessageRequest::new(req.model.clone(), messages);
    let think = req.think.unwrap_or(false);
    request = request.think(if think { ThinkType::True } else { ThinkType::False });

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

    let response_message = ChatMessage {
        role: role.to_string(),
        content: res.message.content,
        images: vec![],
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

async fn handle_chat_stream(
    ollama: &Ollama,
    req: ChatRequest,
    send: &mut iroh::endpoint::SendStream,
) -> anyhow::Result<()> {
    use common::helpers::write_frame;
    use common::protocols::ChatStreamFrame;
    use ollama_rs::generation::chat::MessageRole;
    use ollama_rs::generation::parameters::ThinkType;
    use tokio_stream::StreamExt;

    let messages: Vec<OllamaChatMessage> = req
        .messages
        .into_iter()
        .map(|m| {
            let role = match m.role.as_str() {
                "system" => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::User,
            };
            let mut msg = OllamaChatMessage::new(role, m.content);
            if !m.images.is_empty() {
                let images = m.images.into_iter()
                    .map(ollama_rs::generation::images::Image::from_base64)
                    .collect();
                msg = msg.with_images(images);
            }
            msg
        })
        .collect();

    let mut request = ChatMessageRequest::new(req.model.clone(), messages);
    let think = req.think.unwrap_or(false);
    request = request.think(if think { ThinkType::True } else { ThinkType::False });

    // Signal that we're starting a stream
    write_frame(send, Response::ChatStreamStart).await?;

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
                // Forward content (actual response tokens)
                let content = resp.message.content.clone();
                if !content.is_empty() {
                    chunk_count += 1;
                    tracing::debug!("streaming chunk #{chunk_count}: {:?}", &content);
                    write_frame(send, ChatStreamFrame::Delta(content)).await?;
                }

                // Forward thinking content if present (reasoning models)
                if let Some(ref thinking) = resp.message.thinking {
                    if !thinking.is_empty() {
                        chunk_count += 1;
                        tracing::debug!("streaming thinking chunk #{chunk_count}");
                        write_frame(send, ChatStreamFrame::Thinking(thinking.clone())).await?;
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

    // Send final frame with metering
    write_frame(
        send,
        ChatStreamFrame::Done(Metering {
            prompt_tokens,
            completion_tokens,
            compute_ms,
        }),
    )
    .await?;

    Ok(())
}

fn handle_registry(req: common::protocols::registry::RegistryRequest) -> Response {
    match req {
        RegistryRequest::Register(caps) => {
            tracing::info!("re-registration with {} models", caps.models.len());
            Response::Registry(common::protocols::registry::RegistryResponse::Ack)
        }
        RegistryRequest::Heartbeat => {
            tracing::debug!("heartbeat");
            Response::Registry(common::protocols::registry::RegistryResponse::Ack)
        }
    }
}

/// Query Ollama for available models and build capabilities.
async fn discover_capabilities(ollama: &Ollama) -> anyhow::Result<Capabilities> {
    let models = ollama
        .list_local_models()
        .await
        .map_err(|e| anyhow::anyhow!("failed to list ollama models: {e}"))?;

    let model_infos: Vec<ModelInfo> = models
        .into_iter()
        .map(|m| ModelInfo {
            name: m.name,
            quantization: String::new(), // Ollama LocalModel doesn't expose quantization
            vram_mb: 0, // Ollama doesn't expose this per-model
            ram_mb: 0,
        })
        .collect();

    tracing::info!("discovered {} models from Ollama", model_infos.len());
    for m in &model_infos {
        tracing::info!("  - {}", m.name);
    }

    Ok(Capabilities { models: model_infos })
}

/// Fetch the orchestrator's endpoint ID from its HTTP API.
async fn fetch_orchestrator_id(orchestrator_url: &str) -> anyhow::Result<String> {
    let url = format!("{}/v1/node-info", orchestrator_url.trim_end_matches('/'));
    let resp: serde_json::Value = reqwest::get(&url)
        .await
        .map_err(|e| anyhow::anyhow!("failed to reach orchestrator at {url}: {e}"))?
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("invalid response from orchestrator: {e}"))?;

    resp["endpoint_id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("orchestrator response missing endpoint_id"))
}

/// Connect to the orchestrator, register, then serve inference requests.
async fn connect_and_serve(
    endpoint: &iroh::Endpoint,
    orchestrator_id: &str,
    ollama: Ollama,
) -> anyhow::Result<()> {
    let orch_id = iroh::EndpointId::from_str(orchestrator_id)
        .map_err(|e| anyhow::anyhow!("invalid orchestrator id: {e}"))?;
    let orch_addr = iroh::EndpointAddr::from(orch_id);

    tracing::info!("connecting to orchestrator: {orchestrator_id}");
    let conn = endpoint.connect(orch_addr, protocols::ALPN).await?;
    tracing::info!("connected to orchestrator");

    // Step 1: Register with capabilities
    let capabilities = discover_capabilities(&ollama).await?;
    let reg_request = Request::Registry(RegistryRequest::Register(capabilities));

    let (send, recv) = conn.open_bi().await?;
    write_p2p(send, reg_request).await?;

    let ack: Response = read_p2p(recv).await?;
    match ack {
        Response::Registry(_) => tracing::info!("registered with orchestrator"),
        _ => anyhow::bail!("unexpected response to registration"),
    }

    // Step 2: Start heartbeat task alongside inference loop
    let heartbeat_conn = conn.clone();
    let heartbeat_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let req = Request::Registry(RegistryRequest::Heartbeat);
            match heartbeat_conn.open_bi().await {
                Ok((send, recv)) => {
                    if let Err(e) = write_p2p(send, req).await {
                        tracing::warn!("heartbeat send failed: {e}");
                        break;
                    }
                    match read_p2p::<Response>(recv).await {
                        Ok(_) => tracing::debug!("heartbeat ack"),
                        Err(e) => {
                            tracing::warn!("heartbeat recv failed: {e}");
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("heartbeat open_bi failed: {e}");
                    break;
                }
            }
        }
    });

    // Step 3: Loop accepting inference streams from the orchestrator
    tracing::info!("serving inference requests...");
    loop {
        let (send, recv) = match conn.accept_bi().await {
            Ok(streams) => streams,
            Err(e) => {
                tracing::warn!("connection to orchestrator lost: {e}");
                heartbeat_handle.abort();
                return Err(e.into());
            }
        };

        let ollama_clone = ollama.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_inference_stream(ollama_clone, send, recv).await {
                tracing::error!("inference stream error: {e:?}");
            }
        });
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let key_path = common::key::expand_tilde(&args.key_path);

    let secret_key = common::key::load_or_create_secret_key(&key_path)?;

    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret_key)
        .bind()
        .await?;

    tracing::info!("host endpoint id: {:?}", endpoint.addr().id);

    let ollama = Ollama::new(args.ollama_url.clone(), args.ollama_port);

    // Connect to orchestrator with reconnection loop.
    // On each attempt, resolve the orchestrator ID (fetch from HTTP if not provided or stale).
    let mut cached_id: Option<String> = args.orchestrator_id.clone();

    loop {
        // Resolve orchestrator ID
        let orchestrator_id = match &cached_id {
            Some(id) => id.clone(),
            None => {
                tracing::info!("fetching orchestrator endpoint ID from {}", args.orchestrator_url);
                match fetch_orchestrator_id(&args.orchestrator_url).await {
                    Ok(id) => {
                        tracing::info!("orchestrator endpoint ID: {id}");
                        cached_id = Some(id.clone());
                        id
                    }
                    Err(e) => {
                        tracing::error!("failed to fetch orchestrator ID: {e}");
                        tracing::info!("retrying in 5 seconds...");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                }
            }
        };

        match connect_and_serve(&endpoint, &orchestrator_id, ollama.clone()).await {
            Ok(()) => break,
            Err(e) => {
                tracing::error!("disconnected from orchestrator: {e}");
                // Clear cached ID so we re-fetch on next attempt (ID may have changed)
                cached_id = None;
                tracing::info!("reconnecting in 5 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}
