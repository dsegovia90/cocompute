use std::str::FromStr;

use axum::{Json, Router, extract::State, routing::post};
use clap::Parser;
use common::{
    helpers::{read_p2p, write_p2p},
    protocols::{self, Request, Response},
    protocols::embeddings::{EmbeddingsRequest, EmbeddingsResponse},
};
use error::AppError;
use iroh::{Endpoint, EndpointAddr, EndpointId};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

mod db;
mod error;

#[derive(Parser, Debug)]
#[command(name = "cocompute-orchestrator")]
struct Args {
    /// Port to listen on
    #[arg(long, default_value = "3000", env = "COCOMPUTE_PORT")]
    port: u16,

    /// SQLite database path
    #[arg(long, default_value = "./cocompute.db", env = "COCOMPUTE_DB_PATH")]
    db_path: String,
}

#[derive(Clone)]
struct AppState {
    endpoint: Endpoint,
    #[allow(dead_code)]
    db: DatabaseConnection,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // Initialize database
    let db_url = format!("sqlite://{}?mode=rwc", args.db_path);
    let db = Database::connect(&db_url).await?;
    db::migration::Migrator::up(&db, None).await?;
    tracing::info!("database initialized at {}", args.db_path);

    // Initialize iroh endpoint
    let endpoint = Endpoint::bind(iroh::endpoint::presets::N0).await?;
    tracing::info!("iroh endpoint: {:?}", endpoint.addr().id);

    let state = AppState { endpoint, db };

    let app = Router::new()
        .route("/embeddings", post(create_embeddings))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

/// Send a request to a host over iroh and get the response.
async fn send_to_host(
    endpoint: &Endpoint,
    addr: EndpointAddr,
    request: Request,
) -> Result<Response, AppError> {
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
        .map_err(|e| AppError::Internal(e))?;

    let response: Response = read_p2p(recv)
        .await
        .map_err(|e| AppError::Internal(e))?;

    Ok(response)
}

async fn create_embeddings(
    State(state): State<AppState>,
    Json(payload): Json<EmbeddingsRequest>,
) -> Result<Json<EmbeddingsResponse>, AppError> {
    // TODO: Look up host from registry instead of hardcoded endpoint
    let endpoint_id: EndpointId =
        EndpointId::from_str("f2ebd84cfc3db91a0cee90bed7c4bad66450eb7942f5541bd21b9706e8d0d46d")
            .map_err(|e| AppError::Internal(anyhow::anyhow!("invalid endpoint id: {e}")))?;
    let address = EndpointAddr::from(endpoint_id);

    let request = Request::Embeddings(payload);
    let response = send_to_host(&state.endpoint, address, request).await?;

    match response {
        Response::Embeddings { result, .. } => Ok(Json(result)),
        _ => Err(AppError::Internal(anyhow::anyhow!("unexpected response type"))),
    }
}
