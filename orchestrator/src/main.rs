use clap::{Parser, Subcommand};
use cocompute_orchestrator::{
    AppState, auth, build_router, db, email, host_acceptor::HostAcceptor, host_manager::HostManager,
    signup::{self, SignupError, SignupInput},
};
use common::protocols;
use iroh::Endpoint;
use sea_orm::{ActiveModelTrait, Database, Set};
use sea_orm_migration::MigratorTrait;

#[derive(Parser, Debug)]
#[command(name = "cocompute-orchestrator", version)]
struct Args {
    /// Port to listen on
    #[arg(long, default_value = "4000", env = "COCOMPUTE_PORT")]
    port: u16,

    /// SQLite database path
    #[arg(long, default_value = "./cocompute.db", env = "COCOMPUTE_DB_PATH")]
    db_path: String,

    /// Static files directory
    #[arg(
        long,
        default_value = "orchestrator/static",
        env = "COCOMPUTE_STATIC_DIR"
    )]
    static_dir: String,

    /// Session signing secret (64+ bytes). Generated randomly if not set (sessions won't survive restarts).
    #[arg(long, env = "COCOMPUTE_SESSION_SECRET")]
    session_secret: Option<String>,

    /// Public base URL (for email links)
    #[arg(
        long,
        env = "COCOMPUTE_BASE_URL",
        default_value = "http://localhost:4000"
    )]
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

    /// Cloudflare Turnstile site key (public; rendered in HTML). If unset, captcha is disabled (dev mode).
    #[arg(long, env = "TURNSTILE_SITE_KEY")]
    turnstile_site_key: Option<String>,

    /// Cloudflare Turnstile secret key (server-side; verifies the response token). If unset, captcha is disabled (dev mode).
    #[arg(long, env = "TURNSTILE_SECRET_KEY")]
    turnstile_secret_key: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a new API key and store it in the database
    GenerateKey,
    /// Start the orchestrator server (default)
    Serve,
    /// Create a user account from an existing signup record (the beta_invites table)
    InviteUser {
        #[arg(long)]
        email: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env files BEFORE parsing args so clap sees them as if they were
    // exported in the shell. Order: .env.{COCOMPUTE_ENV} (default "development")
    // wins, then .env fills any gaps. Real shell env vars beat both files.
    // Both files are optional; missing files are not an error.
    let env_name = std::env::var("COCOMPUTE_ENV").unwrap_or_else(|_| "development".into());
    let env_specific = format!(".env.{env_name}");
    let env_specific_loaded = dotenvy::from_filename(&env_specific).is_ok();
    let env_default_loaded = dotenvy::dotenv().is_ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    tracing::info!("cocompute-orchestrator v{}", env!("CARGO_PKG_VERSION"));
    if env_specific_loaded {
        tracing::info!("loaded env file: {env_specific}");
    }
    if env_default_loaded {
        tracing::info!("loaded env file: .env");
    }
    if !env_specific_loaded && !env_default_loaded {
        tracing::debug!("no .env files loaded (none present, or all values already in shell env)");
    }

    let db_url = format!("sqlite://{}?mode=rwc", args.db_path);
    let db = Database::connect(&db_url).await?;
    db::migration::Migrator::up(&db, None).await?;

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
            tracing::warn!("SMTP_HOST not set, email sending disabled");
            None
        }
    };

    let session_key = match &args.session_secret {
        Some(secret) => axum_extra::extract::cookie::Key::from(secret.as_bytes()),
        None => {
            tracing::warn!(
                "COCOMPUTE_SESSION_SECRET not set, using ephemeral key (sessions won't survive restarts)"
            );
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

            println!("API key generated (save this, it won't be shown again):");
            println!("{key}");
            Ok(())
        }
        Command::InviteUser { email } => {
            // Operator override: manually create a user from a signup record that's
            // already in the DB (e.g., legacy waitlist data, or a friend you want
            // in fast). The web POST /signup path handles all normal signups now;
            // this CLI subcommand is the escape hatch.
            use db::entities::beta_invites;
            use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

            let invite = beta_invites::Entity::find()
                .filter(beta_invites::Column::Email.eq(&email))
                .one(&db)
                .await?
                .ok_or_else(|| anyhow::anyhow!("no signup record found for {email}"))?;

            let result = signup::create_user_and_invite(
                &db,
                SignupInput {
                    name: invite.name.clone(),
                    email: email.clone(),
                    role: invite.role.clone(),
                    gpu: invite.gpu_info.clone(),
                },
            )
            .await;

            let signup_result = match result {
                Ok(r) => r,
                Err(SignupError::UserAlreadyExists) => {
                    anyhow::bail!("user with email {email} already exists");
                }
                Err(SignupError::Db(e)) => return Err(e.into()),
                Err(SignupError::Hash(e)) => return Err(e),
            };

            let verify_url = format!(
                "{}/verify?token={}",
                args.base_url, signup_result.verification_token
            );
            println!(
                "User created for {} ({email})",
                signup_result.user.name
            );
            println!("Verification URL: {verify_url}");

            if let Some(ref mailer) = mailer {
                let parts = email::templates::invite_email(
                    &signup_result.user.name,
                    &signup_result.verification_token,
                    &args.base_url,
                );
                mailer
                    .send(&email, &parts.subject, &parts.html, &parts.text)
                    .await?;
                println!("Invite email sent.");
            } else {
                println!("Mailer not configured, share the verification URL manually.");
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

            let acceptor = HostAcceptor::new(hosts.clone(), db.clone());
            let _router = iroh::protocol::Router::builder(endpoint.clone())
                .accept(protocols::ALPN, acceptor)
                .spawn();
            tracing::info!("accepting host connections on ALPN cocompute/0");

            if args.turnstile_site_key.is_some() && args.turnstile_secret_key.is_some() {
                tracing::info!("Turnstile captcha enabled for /signup");
            } else {
                tracing::warn!(
                    "TURNSTILE_SITE_KEY/TURNSTILE_SECRET_KEY not set, captcha disabled (dev mode)"
                );
            }

            let state = AppState {
                endpoint,
                endpoint_id,
                db: db.clone(),
                hosts,
                mailer,
                session_key,
                base_url: args.base_url,
                turnstile_site_key: args.turnstile_site_key,
                turnstile_secret_key: args.turnstile_secret_key,
                http: reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .expect("failed to build reqwest client"),
                total_compute_cache: cocompute_orchestrator::web::TotalComputeCache::new(),
            };

            let app = build_router(state, &args.static_dir, true);

            let addr = format!("0.0.0.0:{}", args.port);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            tracing::info!("listening on {addr}");

            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
            )
            .await?;

            Ok(())
        }
    }
}
