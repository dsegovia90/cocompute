use axum::{Extension, Json, Router, extract::State, middleware, response::IntoResponse, routing::{get, post}};
use auth::ApiKeyId;
use axum::response::sse::{Event, Sse};
use clap::{Parser, Subcommand};
use common::{
    helpers::{read_frame, read_p2p, write_p2p},
    protocols::{self, ChatStreamFrame, Metering, Request, Response},
};
use error::AppError;
use host_acceptor::HostAcceptor;
use host_manager::HostManager;
use iroh::Endpoint;
use openai::{
    OpenAIChatChoice, OpenAIChatMessage, OpenAIChatRequest, OpenAIChatResponse,
    OpenAIChatStreamChunk, OpenAIChatStreamChoice, OpenAIChatStreamDelta,
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
#[command(name = "cocompute-orchestrator", version)]
struct Args {
    /// Port to listen on
    #[arg(long, default_value = "3000", env = "COCOMPUTE_PORT")]
    port: u16,

    /// SQLite database path
    #[arg(long, default_value = "./cocompute.db", env = "COCOMPUTE_DB_PATH")]
    db_path: String,

    /// Path to persist the iroh secret key for stable EndpointId
    #[arg(long, default_value = "~/.cocompute/orchestrator.key", env = "COCOMPUTE_KEY_PATH")]
    key_path: String,

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
    endpoint_id: String,
    db: DatabaseConnection,
    hosts: HostManager,
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

            let key_path = common::key::expand_tilde(&args.key_path);
            let secret_key = common::key::load_or_create_secret_key(&key_path)?;

            let endpoint = Endpoint::builder(iroh::endpoint::presets::N0)
                .secret_key(secret_key)
                .bind()
                .await?;
            let endpoint_id = format!("{}", endpoint.addr().id);
            tracing::info!("orchestrator endpoint id: {endpoint_id}");

            let hosts = HostManager::new();

            // Start iroh router to accept host connections
            let acceptor = HostAcceptor::new(hosts.clone());
            let _router = iroh::protocol::Router::builder(endpoint.clone())
                .accept(protocols::ALPN, acceptor)
                .spawn();
            tracing::info!("accepting host connections on ALPN cocompute/0");
            let state = AppState {
                endpoint,
                endpoint_id,
                db: db.clone(),
                hosts,
            };

            let app = Router::new()
                // Authenticated routes
                .route("/v1/models", get(list_models))
                .route("/v1/embeddings", post(create_embeddings))
                .route("/v1/chat/completions", post(create_chat_completion))
                .route("/v1/stats", get(get_stats))
                .route_layer(middleware::from_fn_with_state(db, auth::require_api_key))
                // Unauthenticated routes (host discovery + updates)
                .route("/v1/node-info", get(get_node_info))
                .route("/v1/version", get(get_version))
                .route("/v1/update/{platform}", get(get_update))
                .layer(tower_http::trace::TraceLayer::new_for_http())
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
    api_key_id: Option<i32>,
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
            api_key_id: Set(api_key_id),
            ..Default::default()
        };
        if let Err(e) = record.insert(&db).await {
            tracing::error!("failed to log metering: {e}");
        }
    });
}

/// GET /v1/models — OpenAI-compatible model listing.
async fn list_models(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let models = state.hosts.available_models().await;

    let data: Vec<serde_json::Value> = models
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "id": name,
                "object": "model",
                "owned_by": "cocompute",
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "object": "list",
        "data": data,
    })))
}

/// GET /v1/node-info — Returns the orchestrator's current iroh endpoint ID.
/// Unauthenticated — used by hosts to discover/refresh the orchestrator ID.
async fn get_node_info(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "endpoint_id": state.endpoint_id,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// GET /v1/version — Returns the orchestrator version for update checks.
async fn get_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// GET /v1/update/:platform — Serves host binary for the given platform.
/// Platforms: linux-x86_64, linux-arm64, macos-arm64, macos-x86_64
async fn get_update(
    axum::extract::Path(platform): axum::extract::Path<String>,
) -> Result<axum::response::Response, AppError> {
    let binary_path = format!("/opt/binaries/cocompute-host-{platform}");
    let path = std::path::Path::new(&binary_path);

    if !path.exists() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "binary not found for platform: {platform}. Available: linux-x86_64, linux-arm64, macos-arm64, macos-x86_64"
        )));
    }

    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("failed to read binary: {e}")))?;

    Ok(axum::response::Response::builder()
        .header("content-type", "application/octet-stream")
        .header(
            "content-disposition",
            format!("attachment; filename=\"cocompute-host-{platform}\""),
        )
        .body(axum::body::Body::from(bytes))
        .unwrap())
}

/// POST /v1/embeddings — OpenAI-compatible embeddings endpoint.
async fn create_embeddings(
    State(state): State<AppState>,
    Extension(api_key_id): Extension<ApiKeyId>,
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
            log_metering(state.db.clone(), host_id, model.clone(), "embeddings".into(), metering, Some(api_key_id.0));
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

/// POST /v1/chat/completions — OpenAI-compatible chat endpoint.
/// Supports both streaming (SSE) and non-streaming responses.
async fn create_chat_completion(
    State(state): State<AppState>,
    Extension(api_key_id): Extension<ApiKeyId>,
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
    };

    if stream {
        create_chat_completion_stream(state, model, internal_request, api_key_id.0).await
    } else {
        create_chat_completion_sync(state, model, internal_request, api_key_id.0).await
    }
}

async fn create_chat_completion_sync(
    state: AppState,
    model: String,
    internal_request: common::protocols::chat::ChatRequest,
    api_key_id: i32,
) -> Result<axum::response::Response, AppError> {
    let request = Request::Chat(internal_request);
    let (response, host_id) = route_to_host(&state, &model, request).await?;

    match response {
        Response::Chat { result, ref metering } => {
            log_metering(state.db.clone(), host_id, model.clone(), "chat".into(), metering, Some(api_key_id));
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
            })
            .into_response())
        }
        _ => Err(AppError::Internal(anyhow::anyhow!("unexpected response type"))),
    }
}

async fn create_chat_completion_stream(
    state: AppState,
    model: String,
    internal_request: common::protocols::chat::ChatRequest,
    api_key_id: i32,
) -> Result<axum::response::Response, AppError> {
    let request = Request::Chat(internal_request);

    // Find host and open a bi-stream
    let host = state.hosts.find_host_for_model(&model).await;
    let host = match host {
        Some(h) => h,
        None => {
            let available = state.hosts.available_models().await;
            if available.is_empty() {
                return Err(AppError::HostUnavailable);
            } else {
                return Err(AppError::ModelNotFound { available });
            }
        }
    };

    let host_id = host.endpoint_id.clone();
    let conn = &host.connection;

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
                            },
                            finish_reason: None,
                        }],
                    }).unwrap()));
                }
                Ok(Some(ChatStreamFrame::Done(metering))) => {
                    log_metering(db.clone(), host_id.clone(), model_clone.clone(), "chat_stream".into(), &metering, Some(api_key_id));

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
                            },
                            finish_reason: Some("stop"),
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
