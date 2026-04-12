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
mod email;
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

    /// Session signing secret (64+ bytes). Generated randomly if not set (sessions won't survive restarts).
    #[arg(long, env = "COCOMPUTE_SESSION_SECRET")]
    session_secret: Option<String>,

    /// Public base URL (for email links)
    #[arg(long, env = "COCOMPUTE_BASE_URL", default_value = "http://localhost:3000")]
    base_url: String,

    /// SMTP host (defaults to localhost for Mailpit)
    #[arg(long, env = "SMTP_HOST", default_value = "localhost")]
    smtp_host: Option<String>,

    /// SMTP port (defaults to 1025 for Mailpit)
    #[arg(long, env = "SMTP_PORT", default_value = "1025")]
    smtp_port: u16,

    /// SMTP username
    #[arg(long, env = "SMTP_USER")]
    smtp_user: Option<String>,

    /// SMTP password
    #[arg(long, env = "SMTP_PASSWORD")]
    smtp_password: Option<String>,

    /// SMTP from address
    #[arg(long, env = "SMTP_FROM")]
    smtp_from: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a new API key and store it in the database
    GenerateKey,
    /// Start the orchestrator server (default)
    Serve,
    /// Invite a beta user — looks up their beta invite and creates a user record
    InviteUser {
        #[arg(long)]
        email: String,
    },
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) endpoint: Endpoint,
    pub(crate) endpoint_id: String,
    pub(crate) db: DatabaseConnection,
    pub(crate) hosts: HostManager,
    pub(crate) mailer: Option<std::sync::Arc<email::Mailer>>,
    pub(crate) session_key: axum_extra::extract::cookie::Key,
    pub(crate) base_url: String,
}

impl axum::extract::FromRef<AppState> for axum_extra::extract::cookie::Key {
    fn from_ref(state: &AppState) -> Self {
        state.session_key.clone()
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

    tracing::info!("cocompute-orchestrator v{}", env!("CARGO_PKG_VERSION"));

    // Initialize database
    let db_url = format!("sqlite://{}?mode=rwc", args.db_path);
    let db = Database::connect(&db_url).await?;
    db::migration::Migrator::up(&db, None).await?;

    // Build mailer (None if SMTP not configured)
    let mailer = match &args.smtp_host {
        Some(host) => {
            let from = args.smtp_from.as_deref().unwrap_or("noreply@cocompute.ai");
            match email::Mailer::new(
                host,
                args.smtp_port,
                args.smtp_user.as_deref(),
                args.smtp_password.as_deref(),
                from,
            ) {
                Ok(m) => {
                    tracing::info!("mailer configured via {host}:{}", args.smtp_port);
                    Some(std::sync::Arc::new(m))
                }
                Err(e) => {
                    tracing::warn!("failed to configure mailer: {e}");
                    None
                }
            }
        }
        None => {
            tracing::warn!("SMTP_HOST not set — email sending disabled");
            None
        }
    };

    let session_key = match &args.session_secret {
        Some(secret) => axum_extra::extract::cookie::Key::from(secret.as_bytes()),
        None => {
            tracing::warn!("COCOMPUTE_SESSION_SECRET not set — using ephemeral key (sessions won't survive restarts)");
            axum_extra::extract::cookie::Key::generate()
        }
    };

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
        Command::InviteUser { email } => {
            use db::entities::{beta_invites, users};
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

            // Must have a beta invite
            let invite = beta_invites::Entity::find()
                .filter(beta_invites::Column::Email.eq(&email))
                .one(&db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("no beta invite found for {email}"))?;

            // Check if user already exists
            let existing = users::Entity::find()
                .filter(users::Column::Email.eq(&email))
                .one(&db)
                .await?;
            if existing.is_some() {
                anyhow::bail!("user with email {email} already exists");
            }

            let pid = uuid::Uuid::new_v4().to_string();
            let token = auth::generate_api_key();
            let throwaway_password = auth::hash_password(auth::generate_api_key()).await?;

            let user = users::ActiveModel {
                pid: Set(pid),
                email: Set(email.clone()),
                password_hash: Set(throwaway_password),
                name: Set(invite.name.clone()),
                email_verification_token: Set(Some(token.clone())),
                email_verification_sent_at: Set(Some(chrono::Utc::now())),
                created_at: Set(chrono::Utc::now()),
                updated_at: Set(chrono::Utc::now()),
                ..Default::default()
            };
            user.insert(&db).await?;

            let verify_url = format!("{}/verify?token={}", args.base_url, token);
            println!("User created for {} ({email})", invite.name);
            println!("Verification URL: {verify_url}");

            if let Some(ref mailer) = mailer {
                let parts = email::templates::invite_email(&invite.name, &token, &args.base_url);
                mailer.send(&email, &parts.subject, &parts.html, &parts.text).await?;
                println!("Invite email sent.");
            } else {
                println!("Mailer not configured — share the verification URL manually.");
            }

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
                mailer,
                session_key,
                base_url: args.base_url,
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
