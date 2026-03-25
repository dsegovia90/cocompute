use axum::{Json, Router, extract::State, middleware, routing::{get, post}};
use clap::{Parser, Subcommand};
use common::{
    helpers::{read_p2p, write_p2p},
    protocols::{self, Metering, Request, Response},
};
use error::AppError;
use host_acceptor::HostAcceptor;
use host_manager::HostManager;
use iroh::Endpoint;
use openai::{
    OpenAIChatChoice, OpenAIChatMessage, OpenAIChatRequest, OpenAIChatResponse,
    OpenAIEmbeddingData, OpenAIEmbeddingsRequest, OpenAIEmbeddingsResponse, OpenAIUsage,
};
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};
use sea_orm_migration::MigratorTrait;

mod auth;
mod db;
mod error;
mod host_acceptor;
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
            tracing::info!("orchestrator endpoint id: {:?}", endpoint.addr().id);

            let hosts = HostManager::new();

            // Start iroh router to accept host connections
            let acceptor = HostAcceptor::new(hosts.clone());
            let _router = iroh::protocol::Router::builder(endpoint.clone())
                .accept(protocols::ALPN, acceptor)
                .spawn();
            tracing::info!("accepting host connections on ALPN cocompute/0");

            let state = AppState {
                endpoint,
                db: db.clone(),
                hosts,
            };

            let app = Router::new()
                .route("/v1/embeddings", post(create_embeddings))
                .route("/v1/chat/completions", post(create_chat_completion))
                .route("/v1/stats", get(get_stats))
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

/// Send a request to a host using its stored connection.
/// Opens a new bi-stream on the existing connection.
async fn send_to_host(
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

/// Route a request to the appropriate host based on model name.
async fn route_to_host(
    state: &AppState,
    model: &str,
    request: Request,
) -> Result<(Response, String), AppError> {
    let host = state.hosts.find_host_for_model(model).await;

    match host {
        Some(h) => {
            let eid = h.endpoint_id.clone();
            let resp = send_to_host(&h.connection, request).await?;
            Ok((resp, eid))
        }
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

/// Log metering data to the database (fire-and-forget).
fn log_metering(
    db: DatabaseConnection,
    host_endpoint_id: String,
    model: String,
    request_type: String,
    metering: &Metering,
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
            created_at: Set(chrono::Utc::now()),
            ..Default::default()
        };
        if let Err(e) = record.insert(&db).await {
            tracing::error!("failed to log metering: {e}");
        }
    });
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
    let (response, host_id) = route_to_host(&state, &model, request).await?;

    match response {
        Response::Embeddings { result, ref metering } => {
            log_metering(state.db.clone(), host_id, model.clone(), "embeddings".into(), metering);
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
    let (response, host_id) = route_to_host(&state, &model, request).await?;

    match response {
        Response::Chat { result, ref metering } => {
            log_metering(state.db.clone(), host_id, model.clone(), "chat".into(), metering);
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

/// GET /v1/stats — Usage statistics from metering logs.
async fn get_stats(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder, QuerySelect};

    let total_requests: u64 = db::entities::metering_logs::Entity::find()
        .count(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("db error: {e}")))?;

    let recent = db::entities::metering_logs::Entity::find()
        .order_by_desc(db::entities::metering_logs::Column::CreatedAt)
        .limit(10)
        .all(&state.db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("db error: {e}")))?;

    let recent_entries: Vec<serde_json::Value> = recent
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "host": r.host_endpoint_id,
                "model": r.model,
                "type": r.request_type,
                "prompt_tokens": r.prompt_tokens,
                "completion_tokens": r.completion_tokens,
                "compute_ms": r.compute_ms,
                "created_at": r.created_at.to_string(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "total_requests": total_requests,
        "recent": recent_entries,
    })))
}
