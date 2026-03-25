use axum::{Json, Router, extract::State, middleware, routing::post};
use clap::{Parser, Subcommand};
use common::{
    helpers::{read_p2p, write_p2p},
    protocols::{self, Request, Response},
};
use error::AppError;
use host_manager::HostManager;
use iroh::{Endpoint, EndpointAddr, EndpointId};
use openai::{
    OpenAIChatChoice, OpenAIChatMessage, OpenAIChatRequest, OpenAIChatResponse,
    OpenAIEmbeddingData, OpenAIEmbeddingsRequest, OpenAIEmbeddingsResponse, OpenAIUsage,
};
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};
use sea_orm_migration::MigratorTrait;
use std::str::FromStr;

mod auth;
mod db;
mod error;
mod host_manager;
mod openai;

#[derive(Parser, Debug)]
#[command(name = "cocompute-orchestrator")]
struct Args {
    /// Port to listen on
    #[arg(long, default_value = "3000", env = "COCOMPUTE_PORT", global = true)]
    port: u16,

    /// SQLite database path
    #[arg(long, default_value = "./cocompute.db", env = "COCOMPUTE_DB_PATH", global = true)]
    db_path: String,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a new API key and store it in the database
    GenerateKey,
    /// Start the orchestrator server (default)
    Serve,
}

#[derive(Clone)]
struct AppState {
    endpoint: Endpoint,
    db: DatabaseConnection,
    hosts: HostManager,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // Initialize database
    let db_url = format!("sqlite://{}?mode=rwc", args.db_path);
    let db = Database::connect(&db_url).await?;
    db::migration::Migrator::up(&db, None).await?;

    match args.command.unwrap_or(Command::Serve) {
        Command::GenerateKey => {
            let key = auth::generate_api_key();
            let key_hash = auth::hash_key(&key);

            let active_model = db::entities::api_keys::ActiveModel {
                key_hash: Set(key_hash),
                created_at: Set(chrono::Utc::now()),
                ..Default::default()
            };
            active_model.insert(&db).await?;

            println!("API key generated (save this — it won't be shown again):");
            println!("{key}");
            Ok(())
        }
        Command::Serve => {
            tracing::info!("database initialized at {}", args.db_path);

            let endpoint = Endpoint::bind(iroh::endpoint::presets::N0).await?;
            tracing::info!("iroh endpoint: {:?}", endpoint.addr().id);

            let hosts = HostManager::new();

            let state = AppState {
                endpoint,
                db: db.clone(),
                hosts,
            };

            let app = Router::new()
                .route("/v1/embeddings", post(create_embeddings))
                .route("/v1/chat/completions", post(create_chat_completion))
                .route_layer(middleware::from_fn_with_state(db, auth::require_api_key))
                .with_state(state);

            let addr = format!("0.0.0.0:{}", args.port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            tracing::info!("listening on {addr}");
            axum::serve(listener, app).await?;

            Ok(())
        }
    }
}

/// Send a request to a host over iroh and get the response.
async fn send_to_host(
    endpoint: &Endpoint,
    endpoint_id: &str,
    request: Request,
) -> Result<Response, AppError> {
    let id = EndpointId::from_str(endpoint_id)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("invalid endpoint id: {e}")))?;
    let addr = EndpointAddr::from(id);

    let conn = endpoint
        .connect(addr, protocols::ALPN)
        .await
        .map_err(|_| AppError::HostUnavailable)?;

    let (send, recv) = conn
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
async fn route_to_host(
    state: &AppState,
    model: &str,
    request: Request,
) -> Result<Response, AppError> {
    let host = state.hosts.find_host_for_model(model).await;

    match host {
        Some(h) => send_to_host(&state.endpoint, &h.endpoint_id, request).await,
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

/// POST /v1/embeddings — OpenAI-compatible embeddings endpoint.
async fn create_embeddings(
    State(state): State<AppState>,
    Json(payload): Json<OpenAIEmbeddingsRequest>,
) -> Result<Json<OpenAIEmbeddingsResponse>, AppError> {
    let model = payload.model.clone();

    // Translate OpenAI format → internal protocol
    let internal_request = common::protocols::embeddings::EmbeddingsRequest {
        model: payload.model,
        text: payload.input,
    };

    let request = Request::Embeddings(internal_request);
    let response = route_to_host(&state, &model, request).await?;

    match response {
        Response::Embeddings { result, metering } => {
            // Translate internal → OpenAI format
            Ok(Json(OpenAIEmbeddingsResponse {
                object: "list",
                data: vec![OpenAIEmbeddingData {
                    object: "embedding",
                    embedding: result.embeddings,
                    index: 0,
                }],
                model,
                usage: OpenAIUsage::from_metering(&metering),
            }))
        }
        _ => Err(AppError::Internal(anyhow::anyhow!("unexpected response type"))),
    }
}

/// POST /v1/chat/completions — OpenAI-compatible chat endpoint (non-streaming).
async fn create_chat_completion(
    State(state): State<AppState>,
    Json(payload): Json<OpenAIChatRequest>,
) -> Result<Json<OpenAIChatResponse>, AppError> {
    let model = payload.model.clone();

    // Translate OpenAI format → internal protocol
    let internal_request = common::protocols::chat::ChatRequest {
        model: payload.model,
        messages: payload
            .messages
            .into_iter()
            .map(|m| common::protocols::chat::ChatMessage {
                role: m.role,
                content: m.content,
            })
            .collect(),
        temperature: payload.temperature,
    };

    let request = Request::Chat(internal_request);
    let response = route_to_host(&state, &model, request).await?;

    match response {
        Response::Chat { result, metering } => {
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
                    message: OpenAIChatMessage {
                        role: result.message.role,
                        content: result.message.content,
                    },
                    finish_reason: "stop",
                }],
                usage: OpenAIUsage::from_metering(&metering),
            }))
        }
        _ => Err(AppError::Internal(anyhow::anyhow!("unexpected response type"))),
    }
}
