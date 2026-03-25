use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use common::{
    helpers::{read_p2p, write_p2p},
    protocols::embeddings::{self, EmbeddingsRequest, EmbeddingsResponse},
};
use iroh::protocol::AcceptError;
use ollama_rs::{Ollama, generation::embeddings::request::GenerateEmbeddingsRequest};

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

#[derive(Debug)]
struct EmbeddingsHandler {
    ollama: Ollama,
}

impl EmbeddingsHandler {
    fn new(ollama_host: &str, ollama_port: u16) -> Self {
        Self {
            ollama: Ollama::new(ollama_host.to_string(), ollama_port),
        }
    }
}

impl iroh::protocol::ProtocolHandler for EmbeddingsHandler {
    fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let ollama = self.ollama.clone();
        Box::pin(async move {
            let endpoint_id = connection.remote_id();
            tracing::info!("accepted connection from {endpoint_id}");

            let (send, recv) = connection.accept_bi().await?;

            let req: EmbeddingsRequest = read_p2p(recv)
                .await
                .map_err(|e| std::io::Error::other(e.context("failed to read request")))?;

            tracing::info!("embedding text: {}", req.text);

            let request = GenerateEmbeddingsRequest::new(
                "mxbai-embed-large:latest".to_string(),
                req.text.into(),
            );

            let res = ollama
                .generate_embeddings(request)
                .await
                .map_err(|e| std::io::Error::other(format!("ollama error: {e}")))?;

            write_p2p(send, EmbeddingsResponse::new(res.embeddings[0].clone()))
                .await
                .map_err(|e| std::io::Error::other(e.context("failed to write response")))?;

            connection.closed().await;

            Ok(())
        })
    }
}

/// Expand ~ to the user's home directory
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs_home() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
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

    let handler = EmbeddingsHandler::new(ollama_url, ollama_port);

    let router = iroh::protocol::Router::builder(endpoint)
        .accept(embeddings::ALPN, handler)
        .spawn();

    Ok(router)
}
