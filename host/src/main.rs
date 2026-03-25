use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use common::{
    helpers::{read_p2p, write_p2p},
    protocols::{
        self, Metering, Request, Response,
        chat::{ChatMessage, ChatResponse},
        embeddings::{EmbeddingsRequest, EmbeddingsResponse},
        registry::{RegistryRequest, RegistryResponse},
    },
};
use iroh::protocol::AcceptError;
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

    /// Path to persist the iroh secret key for stable EndpointId
    #[arg(long, default_value = "~/.cocompute/host.key", env = "COCOMPUTE_KEY_PATH")]
    key_path: String,
}

/// Handles all incoming streams on the single cocompute/0 connection.
#[derive(Debug)]
struct CocomputeHandler {
    ollama: Ollama,
}

impl CocomputeHandler {
    fn new(ollama_host: &str, ollama_port: u16) -> Self {
        Self {
            ollama: Ollama::new(ollama_host.to_string(), ollama_port),
        }
    }

    async fn handle_embeddings(
        &self,
        req: EmbeddingsRequest,
    ) -> anyhow::Result<Response> {
        let request = GenerateEmbeddingsRequest::new(
            req.model.clone(),
            req.text.into(),
        );

        let start = std::time::Instant::now();
        let res = self.ollama.generate_embeddings(request).await
            .map_err(|e| anyhow::anyhow!("ollama embeddings error: {e}"))?;
        let compute_ms = start.elapsed().as_millis() as u64;

        Ok(Response::Embeddings {
            result: EmbeddingsResponse::new(res.embeddings[0].clone()),
            metering: Metering {
                prompt_tokens: 0, // embeddings response doesn't expose token counts
                completion_tokens: 0,
                compute_ms,
            },
        })
    }

    async fn handle_chat(
        &self,
        req: common::protocols::chat::ChatRequest,
    ) -> anyhow::Result<Response> {
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
        let res = self.ollama.send_chat_messages(request).await
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
            result: ChatResponse { message: response_message },
            metering: Metering {
                prompt_tokens,
                completion_tokens,
                compute_ms,
            },
        })
    }

    fn handle_registry(&self, req: RegistryRequest) -> Response {
        match req {
            RegistryRequest::Register(caps) => {
                tracing::info!("host registered with {} models", caps.models.len());
                Response::Registry(RegistryResponse::Ack)
            }
            RegistryRequest::Heartbeat => {
                tracing::debug!("heartbeat received");
                Response::Registry(RegistryResponse::Ack)
            }
        }
    }
}

impl iroh::protocol::ProtocolHandler for CocomputeHandler {
    fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let ollama = self.ollama.clone();
        Box::pin(async move {
            let endpoint_id = connection.remote_id();
            tracing::info!("accepted connection from {endpoint_id}");

            // Handle multiple streams on this connection
            loop {
                let (send, recv) = match connection.accept_bi().await {
                    Ok(streams) => streams,
                    Err(_) => break, // Connection closed
                };

                let handler = CocomputeHandler {
                    ollama: ollama.clone(),
                };

                // Spawn each stream handler as a separate task
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(&handler, send, recv).await {
                        tracing::error!("stream error: {e:?}");
                    }
                });
            }

            Ok(())
        })
    }
}

async fn handle_stream(
    handler: &CocomputeHandler,
    send: iroh::endpoint::SendStream,
    recv: iroh::endpoint::RecvStream,
) -> anyhow::Result<()> {
    let request: Request = read_p2p(recv).await?;

    let response = match request {
        Request::Embeddings(req) => {
            tracing::info!("embeddings request for model: {}", req.model);
            handler.handle_embeddings(req).await?
        }
        Request::Chat(req) => {
            tracing::info!("chat request for model: {}", req.model);
            handler.handle_chat(req).await?
        }
        Request::Registry(req) => handler.handle_registry(req),
    };

    write_p2p(send, response).await?;
    Ok(())
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

    let router = start_accept_side(secret_key, &args.ollama_url, args.ollama_port).await?;

    tracing::info!("host running, press Ctrl+C to stop");
    tokio::signal::ctrl_c().await.context("ctrl+c")?;
    router.shutdown().await.context("shutdown")?;

    Ok(())
}

async fn start_accept_side(
    secret_key: iroh::SecretKey,
    ollama_url: &str,
    ollama_port: u16,
) -> anyhow::Result<iroh::protocol::Router> {
    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(secret_key)
        .bind()
        .await?;

    tracing::info!("endpoint id: {:?}", endpoint.addr().id);

    let handler = CocomputeHandler::new(ollama_url, ollama_port);

    let router = iroh::protocol::Router::builder(endpoint)
        .accept(protocols::ALPN, handler)
        .spawn();

    Ok(router)
}
