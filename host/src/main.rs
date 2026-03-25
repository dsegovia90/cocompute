use std::path::PathBuf;
use std::str::FromStr;

use anyhow::Context;
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
#[command(name = "cocompute-host")]
struct Args {
    /// Ollama server URL
    #[arg(long, default_value = "http://localhost", env = "OLLAMA_URL")]
    ollama_url: String,

    /// Ollama server port
    #[arg(long, default_value = "11434", env = "OLLAMA_PORT")]
    ollama_port: u16,

    /// Orchestrator endpoint ID to connect to
    #[arg(long, env = "COCOMPUTE_ORCHESTRATOR_ID")]
    orchestrator_id: String,

    /// Path to persist the iroh secret key for stable EndpointId
    #[arg(long, default_value = "~/.cocompute/host.key", env = "COCOMPUTE_KEY_PATH")]
    key_path: String,
}

/// Handle a single inference stream from the orchestrator.
async fn handle_inference_stream(
    ollama: Ollama,
    send: iroh::endpoint::SendStream,
    recv: iroh::endpoint::RecvStream,
) -> anyhow::Result<()> {
    let request: Request = read_p2p(recv).await?;

    let response = match request {
        Request::Embeddings(req) => {
            tracing::info!("embeddings request for model: {}", req.model);
            handle_embeddings(&ollama, req).await?
        }
        Request::Chat(req) => {
            tracing::info!("chat request for model: {}", req.model);
            handle_chat(&ollama, req).await?
        }
        Request::Registry(req) => {
            tracing::debug!("registry request on inference stream");
            handle_registry(req)
        }
    };

    write_p2p(send, response).await?;
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

    let messages: Vec<OllamaChatMessage> = req
        .messages
        .into_iter()
        .map(|m| {
            let role = match m.role.as_str() {
                "system" => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::User,
            };
            OllamaChatMessage::new(role, m.content)
        })
        .collect();

    let request = ChatMessageRequest::new(req.model.clone(), messages);

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

    // Step 2: Loop accepting inference streams from the orchestrator
    tracing::info!("serving inference requests...");
    loop {
        let (send, recv) = match conn.accept_bi().await {
            Ok(streams) => streams,
            Err(e) => {
                tracing::warn!("connection to orchestrator lost: {e}");
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

/// Expand ~ to the user's home directory
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Load or generate an iroh secret key for stable EndpointId across restarts
fn load_or_create_secret_key(key_path: &PathBuf) -> anyhow::Result<iroh::SecretKey> {
    if key_path.exists() {
        let bytes = std::fs::read(key_path).context("failed to read key file")?;
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid key file length"))?;
        Ok(iroh::SecretKey::from_bytes(&key_bytes))
    } else {
        let key = iroh::SecretKey::generate(&mut rand::rng());
        if let Some(parent) = key_path.parent() {
            std::fs::create_dir_all(parent).context("failed to create key directory")?;
        }
        std::fs::write(key_path, key.to_bytes()).context("failed to write key file")?;
        tracing::info!("generated new host key at {}", key_path.display());
        Ok(key)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let key_path = expand_tilde(&args.key_path);

    let secret_key = load_or_create_secret_key(&key_path)?;

    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret_key)
        .bind()
        .await?;

    tracing::info!("host endpoint id: {:?}", endpoint.addr().id);

    let ollama = Ollama::new(args.ollama_url.clone(), args.ollama_port);

    // Connect to orchestrator with reconnection loop
    loop {
        match connect_and_serve(&endpoint, &args.orchestrator_id, ollama.clone()).await {
            Ok(()) => break,
            Err(e) => {
                tracing::error!("disconnected from orchestrator: {e}");
                tracing::info!("reconnecting in 5 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}
