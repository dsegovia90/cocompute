use axum::{Router, middleware, routing::{get, post}};
use clap::{Parser, Subcommand};
use common::protocols;
use host_acceptor::HostAcceptor;
use host_manager::HostManager;
use iroh::Endpoint;
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};
use sea_orm_migration::MigratorTrait;

mod auth;
mod db;
mod error;
mod host_acceptor;
mod host_manager;
mod openai;
mod proxy;
mod routes;
mod web;

#[derive(Parser, Debug)]
#[command(name = "cocompute-orchestrator", version)]
struct Args {
    /// Port to listen on
    #[arg(long, default_value = "3000", env = "COCOMPUTE_PORT")]
    port: u16,

    /// SQLite database path
    #[arg(long, default_value = "./cocompute.db", env = "COCOMPUTE_DB_PATH")]
    db_path: String,

    /// Static files directory
    #[arg(long, default_value = "orchestrator/static", env = "COCOMPUTE_STATIC_DIR")]
    static_dir: String,

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
pub(crate) struct AppState {
    pub(crate) endpoint: Endpoint,
    pub(crate) endpoint_id: String,
    pub(crate) db: DatabaseConnection,
    pub(crate) hosts: HostManager,
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

    tracing::info!("cocompute-orchestrator v{}", env!("CARGO_PKG_VERSION"));

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

            let endpoint = Endpoint::builder(iroh::endpoint::presets::N0)
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
                .route("/v1/models", get(routes::models::list_models))
                .route("/v1/embeddings", post(routes::embeddings::create_embeddings))
                .route("/v1/chat/completions", post(routes::chat::create_chat_completion))
                .route("/v1/stats", get(routes::stats::get_stats))
                .route_layer(middleware::from_fn_with_state(db, auth::require_api_key))
                // Web UI
                .merge(web::router(&args.static_dir))
                // Host discovery + updates
                .route("/v1/node-info", get(routes::system::get_node_info))
                .route("/v1/version", get(routes::system::get_version))
                .route("/v1/update/{platform}", get(routes::system::get_update))
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
