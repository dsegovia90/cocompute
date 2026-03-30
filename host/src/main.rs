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

    /// Automatically check for updates on startup
    #[arg(long, default_value = "false", env = "COCOMPUTE_AUTO_UPDATE")]
    auto_update: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Check for and install updates from the orchestrator
    SelfUpdate,
    /// Start the host (default)
    Run,
}

/// Strip the `@dev` suffix from model names in debug builds.
/// In release builds, this is a no-op (the suffix is never added).
fn ollama_model_name(model: &str) -> String {
    if cfg!(debug_assertions) {
        model.strip_suffix("@dev").unwrap_or(model).to_string()
    } else {
        model.to_string()
    }
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

    let mut request = ChatMessageRequest::new(ollama_model_name(&req.model), messages);
    let think = req.think.unwrap_or(false);
    request = request.think(if think { ThinkType::True } else { ThinkType::False });

    // Forward tool definitions to Ollama
    if !req.tools.is_empty() {
        tracing::info!("forwarding {} tool definitions to Ollama", req.tools.len());
        let tool_infos: Vec<ollama_rs::generation::tools::ToolInfo> = req.tools.iter().filter_map(|t| {
            let params: serde_json::Value = match serde_json::from_str(&t.function.parameters) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("tool '{}' has invalid parameters JSON: {e}", t.function.name);
                    return None;
                }
            };
            let schema: schemars::Schema = match serde_json::from_value(params) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("tool '{}' has invalid JSON Schema: {e}", t.function.name);
                    return None;
                }
            };
            Some(ollama_rs::generation::tools::ToolInfo {
                tool_type: ollama_rs::generation::tools::ToolType::Function,
                function: ollama_rs::generation::tools::ToolFunctionInfo {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: schema,
                },
            })
        }).collect();
        tracing::info!("{} tools successfully parsed (of {} total)", tool_infos.len(), req.tools.len());
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

    // Convert tool calls from Ollama format to our format
    let tool_calls: Vec<common::protocols::chat::ToolCall> = res.message.tool_calls.into_iter().enumerate().map(|(i, tc)| {
        common::protocols::chat::ToolCall {
            id: format!("call_{i}"),
            call_type: "function".to_string(),
            function: common::protocols::chat::ToolCallFunction {
                name: tc.function.name,
                arguments: tc.function.arguments.to_string(),
            },
        }
    }).collect();

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

    let mut request = ChatMessageRequest::new(ollama_model_name(&req.model), messages);
    let think = req.think.unwrap_or(false);
    request = request.think(if think { ThinkType::True } else { ThinkType::False });

    // Forward tool definitions to Ollama
    if !req.tools.is_empty() {
        tracing::info!("forwarding {} tool definitions to Ollama (streaming)", req.tools.len());
        let tool_infos: Vec<ollama_rs::generation::tools::ToolInfo> = req.tools.iter().filter_map(|t| {
            let params: serde_json::Value = match serde_json::from_str(&t.function.parameters) {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!("tool '{}' has invalid parameters JSON: {e}", t.function.name);
                    return None;
                }
            };
            let schema: schemars::Schema = match serde_json::from_value(params) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("tool '{}' has invalid JSON Schema: {e}", t.function.name);
                    return None;
                }
            };
            Some(ollama_rs::generation::tools::ToolInfo {
                tool_type: ollama_rs::generation::tools::ToolType::Function,
                function: ollama_rs::generation::tools::ToolFunctionInfo {
                    name: t.function.name.clone(),
                    description: t.function.description.clone(),
                    parameters: schema,
                },
            })
        }).collect();
        tracing::info!("{} tools successfully parsed (streaming)", tool_infos.len());
        request = request.tools(tool_infos);
    }

    // Signal that we're starting a stream
    write_frame(send, Response::ChatStreamStart).await?;

    let has_tools = !req.tools.is_empty();

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

        // Emit content as a single delta if present
        if !res.message.content.is_empty() {
            write_frame(send, ChatStreamFrame::Delta(res.message.content.clone())).await?;
        }

        // Emit tool calls if present
        if !res.message.tool_calls.is_empty() {
            tracing::info!("response contains {} tool calls", res.message.tool_calls.len());
            let tool_calls: Vec<common::protocols::chat::ToolCall> = res.message.tool_calls.into_iter().enumerate().map(|(i, tc)| {
                common::protocols::chat::ToolCall {
                    id: format!("call_{i}"),
                    call_type: "function".to_string(),
                    function: common::protocols::chat::ToolCallFunction {
                        name: tc.function.name,
                        arguments: tc.function.arguments.to_string(),
                    },
                }
            }).collect();
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
    }

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
        .map(|m| {
            let name = if cfg!(debug_assertions) {
                format!("{}@dev", m.name)
            } else {
                m.name
            };
            ModelInfo {
                name,
                quantization: String::new(), // Ollama LocalModel doesn't expose quantization
                vram_mb: 0, // Ollama doesn't expose this per-model
                ram_mb: 0,
            }
        })
        .collect();

    tracing::debug!("discovered {} models from Ollama", model_infos.len());
    for m in &model_infos {
        tracing::debug!("  - {}", m.name);
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
    let mut initial_models: Vec<String> = capabilities.models.iter().map(|m| m.name.clone()).collect();
    initial_models.sort();
    let reg_request = Request::Registry(RegistryRequest::Register(capabilities));

    let (send, recv) = conn.open_bi().await?;
    write_p2p(send, reg_request).await?;

    let ack: Response = read_p2p(recv).await?;
    match ack {
        Response::Registry(_) => tracing::info!("registered with orchestrator"),
        _ => anyhow::bail!("unexpected response to registration"),
    }

    // Step 2: Start heartbeat + model refresh task alongside inference loop
    let heartbeat_conn = conn.clone();
    let heartbeat_ollama = ollama.clone();
    let mut last_models = initial_models;

    let heartbeat_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        let mut consecutive_failures: u32 = 0;
        const MAX_FAILURES: u32 = 3;

        tracing::info!("heartbeat task started (15s interval)");

        loop {
            interval.tick().await;

            // Check if Ollama models changed
            let new_caps = match discover_capabilities(&heartbeat_ollama).await {
                Ok(caps) => caps,
                Err(e) => {
                    tracing::warn!("heartbeat: model discovery failed: {e}");
                    // Still send a heartbeat even if discovery fails
                    match heartbeat_conn.open_bi().await {
                        Ok((send, recv)) => {
                            let req = Request::Registry(RegistryRequest::Heartbeat);
                            if write_p2p(send, req).await.is_ok() {
                                if read_p2p::<Response>(recv).await.is_ok() {
                                    consecutive_failures = 0;
                                    continue;
                                }
                            }
                            consecutive_failures += 1;
                            tracing::warn!("heartbeat failed ({consecutive_failures}/{MAX_FAILURES})");
                            if consecutive_failures >= MAX_FAILURES {
                                tracing::error!("heartbeat failed {MAX_FAILURES} times, closing connection");
                                heartbeat_conn.close(0u32.into(), b"heartbeat failed");
                                break;
                            }
                            continue;
                        }
                        Err(e) => {
                            tracing::error!("heartbeat open_bi failed: {e}, closing connection");
                            heartbeat_conn.close(0u32.into(), b"heartbeat failed");
                            break;
                        }
                    }
                }
            };

            let mut new_models: Vec<String> = new_caps.models.iter().map(|m| m.name.clone()).collect();
            new_models.sort();

            let req = if new_models != last_models {
                tracing::info!("model list changed, re-registering with orchestrator");
                last_models = new_models;
                Request::Registry(RegistryRequest::Register(new_caps))
            } else {
                Request::Registry(RegistryRequest::Heartbeat)
            };

            match heartbeat_conn.open_bi().await {
                Ok((send, recv)) => {
                    if let Err(e) = write_p2p(send, req).await {
                        consecutive_failures += 1;
                        tracing::warn!("heartbeat send failed ({consecutive_failures}/{MAX_FAILURES}): {e}");
                        if consecutive_failures >= MAX_FAILURES {
                            tracing::error!("heartbeat failed {MAX_FAILURES} times, closing connection");
                            heartbeat_conn.close(0u32.into(), b"heartbeat failed");
                            break;
                        }
                        continue;
                    }
                    match read_p2p::<Response>(recv).await {
                        Ok(_) => {
                            consecutive_failures = 0;
                            tracing::debug!("heartbeat ack");
                        }
                        Err(e) => {
                            consecutive_failures += 1;
                            tracing::warn!("heartbeat recv failed ({consecutive_failures}/{MAX_FAILURES}): {e}");
                            if consecutive_failures >= MAX_FAILURES {
                                tracing::error!("heartbeat failed {MAX_FAILURES} times, closing connection");
                                heartbeat_conn.close(0u32.into(), b"heartbeat failed");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("heartbeat open_bi failed: {e}, closing connection");
                    heartbeat_conn.close(0u32.into(), b"heartbeat failed");
                    break;
                }
            }
        }
        tracing::info!("heartbeat task exiting");
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

/// Detect the current platform string for update downloads.
fn current_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", "aarch64") => "macos-arm64",
        ("macos", "x86_64") => "macos-x86_64",
        (os, arch) => {
            tracing::warn!("unknown platform: {os}/{arch}");
            "unknown"
        }
    }
}

/// Check the orchestrator for a newer version and return it if available.
/// Only triggers on upgrades (remote > local), never downgrades.
async fn check_for_update(orchestrator_url: &str) -> anyhow::Result<Option<String>> {
    let url = format!("{}/v1/version", orchestrator_url.trim_end_matches('/'));
    let resp: serde_json::Value = reqwest::get(&url).await?.json().await?;

    let remote_str = resp["version"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing version in response"))?;

    let local_str = env!("CARGO_PKG_VERSION");

    let remote: semver::Version = remote_str.parse()
        .map_err(|e| anyhow::anyhow!("invalid remote version '{remote_str}': {e}"))?;
    let local: semver::Version = local_str.parse()
        .map_err(|e| anyhow::anyhow!("invalid local version '{local_str}': {e}"))?;

    if remote > local {
        Ok(Some(remote_str.to_string()))
    } else {
        Ok(None)
    }
}

/// Download the new binary from the orchestrator and replace the current executable.
async fn perform_update(orchestrator_url: &str) -> anyhow::Result<()> {
    let platform = current_platform();
    if platform == "unknown" {
        anyhow::bail!("cannot update: unknown platform");
    }

    let url = format!(
        "{}/v1/update/{platform}",
        orchestrator_url.trim_end_matches('/')
    );

    tracing::info!("downloading update for {platform}...");
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("update download failed: {status} {body}");
    }

    let bytes = response.bytes().await?;
    tracing::info!("downloaded {} bytes", bytes.len());

    // Get path to current executable
    let current_exe = std::env::current_exe()?;
    let backup_path = current_exe.with_extension("old");
    let temp_path = current_exe.with_extension("new");

    // Write new binary to temp file
    tokio::fs::write(&temp_path, &bytes).await?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o755)).await?;
    }

    // Atomic swap: current → backup, new → current
    if backup_path.exists() {
        tokio::fs::remove_file(&backup_path).await.ok();
    }
    tokio::fs::rename(&current_exe, &backup_path).await?;

    if let Err(e) = tokio::fs::rename(&temp_path, &current_exe).await {
        // Rollback: restore the backup so we're not left with no binary
        tracing::error!("failed to install new binary: {e}. rolling back...");
        tokio::fs::rename(&backup_path, &current_exe).await.ok();
        anyhow::bail!("update failed, rolled back to previous version: {e}");
    }

    // Clean up backup
    tokio::fs::remove_file(&backup_path).await.ok();

    tracing::info!("update complete. restart to use the new version.");
    Ok(())
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

    tracing::info!("cocompute-host v{}", env!("CARGO_PKG_VERSION"));

    let orchestrator_url = args.orchestrator_url.clone();

    // Handle self-update subcommand
    if matches!(args.command, Some(Command::SelfUpdate)) {
        tracing::info!("checking for updates from {}", orchestrator_url);
        match check_for_update(&orchestrator_url).await? {
            Some(new_version) => {
                let local = env!("CARGO_PKG_VERSION");
                tracing::info!("update available: {local} → {new_version}");
                perform_update(&orchestrator_url).await?;
            }
            None => {
                tracing::info!("already on latest version ({})", env!("CARGO_PKG_VERSION"));
            }
        }
        return Ok(());
    }

    // Auto-update check on startup
    if args.auto_update {
        tracing::info!("checking for updates...");
        match check_for_update(&orchestrator_url).await {
            Ok(Some(new_version)) => {
                let local = env!("CARGO_PKG_VERSION");
                tracing::info!("update available: {local} → {new_version}. updating...");
                match perform_update(&orchestrator_url).await {
                    Ok(()) => {
                        tracing::info!("update installed. restarting...");
                        // Re-exec ourselves with the same args
                        let exe = std::env::current_exe()?;
                        let args: Vec<String> = std::env::args().collect();
                        let err = exec::execvp(&exe, &args);
                        anyhow::bail!("failed to restart after update: {err}");
                    }
                    Err(e) => {
                        tracing::warn!("auto-update failed: {e}. continuing with current version.");
                    }
                }
            }
            Ok(None) => {
                tracing::info!("up to date ({})", env!("CARGO_PKG_VERSION"));
            }
            Err(e) => {
                tracing::warn!("update check failed: {e}. continuing with current version.");
            }
        }
    }

    let key_path = common::key::expand_tilde(&args.key_path);
    let secret_key = common::key::load_or_create_secret_key(&key_path)?;

    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret_key)
        .bind()
        .await?;

    tracing::info!("host endpoint id: {:?}", endpoint.addr().id);

    let ollama = Ollama::new(args.ollama_url.clone(), args.ollama_port);

    // Connect to orchestrator with reconnection loop.
    let mut cached_id: Option<String> = args.orchestrator_id.clone();

    loop {
        let orchestrator_id = match &cached_id {
            Some(id) => id.clone(),
            None => {
                tracing::info!("fetching orchestrator endpoint ID from {}", orchestrator_url);
                match fetch_orchestrator_id(&orchestrator_url).await {
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
                cached_id = None;
                tracing::info!("reconnecting in 5 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ollama_model_name_strips_dev_suffix() {
        assert_eq!(ollama_model_name("llama3:latest@dev"), "llama3:latest");
    }

    #[test]
    fn ollama_model_name_passthrough_without_suffix() {
        assert_eq!(ollama_model_name("llama3:latest"), "llama3:latest");
    }
}
