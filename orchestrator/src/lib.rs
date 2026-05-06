//! Library surface for the cocompute orchestrator. The bin entrypoint
//! (`src/main.rs`) is a thin shim over this; integration tests in `tests/`
//! consume the same building blocks.

use std::sync::Arc;

use axum::{Router, middleware, routing::{get, post}};
use iroh::Endpoint;
use sea_orm::DatabaseConnection;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

pub mod auth;
pub mod db;
pub mod email;
pub mod error;
pub mod host_acceptor;
pub mod host_manager;
pub mod openai;
pub mod proxy;
pub mod routes;
pub mod web;

use host_manager::HostManager;

#[derive(Clone)]
pub struct AppState {
    pub endpoint: Endpoint,
    pub endpoint_id: String,
    pub db: DatabaseConnection,
    pub hosts: HostManager,
    pub mailer: Option<std::sync::Arc<email::Mailer>>,
    pub session_key: axum_extra::extract::cookie::Key,
    pub base_url: String,
    /// Cloudflare Turnstile site key (public). None disables captcha (dev).
    pub turnstile_site_key: Option<String>,
    /// Cloudflare Turnstile secret key (server-side). None disables captcha (dev).
    pub turnstile_secret_key: Option<String>,
    /// Shared HTTP client for outbound calls (Turnstile siteverify, etc.).
    pub http: reqwest::Client,
}

impl axum::extract::FromRef<AppState> for axum_extra::extract::cookie::Key {
    fn from_ref(state: &AppState) -> Self {
        state.session_key.clone()
    }
}

/// Build the full Axum router with rate limiters configured.
///
/// Production callers (the bin) pass `apply_rate_limits = true`. Tests pass
/// `false` so they don't have to worry about hitting governor thresholds.
pub fn build_router(state: AppState, static_dir: &str, apply_rate_limits: bool) -> Router {
    let db = state.db.clone();

    if apply_rate_limits {
        // Per-IP rate limit on POST /beta: ~10 requests per minute per source IP.
        // SmartIpKeyExtractor honors X-Forwarded-For / Forwarded headers so this
        // works correctly behind reverse proxies (Coolify, Cloudflare, nginx).
        let beta_governor = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(6)
                .burst_size(10)
                .use_headers()
                .finish()
                .expect("invalid /beta governor config"),
        );

        // Per-IP rate limit on /v1/*: ~60 req/min per IP. Applied AFTER auth
        // so unauthenticated requests get rejected before reaching the limiter.
        let v1_governor = Arc::new(
            GovernorConfigBuilder::default()
                .per_second(1)
                .burst_size(60)
                .use_headers()
                .finish()
                .expect("invalid /v1 governor config"),
        );

        // Periodic cleanup so the in-memory IP map doesn't grow unbounded.
        let beta_limiter = beta_governor.limiter().clone();
        let v1_limiter = v1_governor.limiter().clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                beta_limiter.retain_recent();
                v1_limiter.retain_recent();
            }
        });

        let beta_rate_limited = Router::new()
            .route("/beta", post(web::post_beta))
            .layer(GovernorLayer::new(beta_governor));

        Router::new()
            .route("/v1/models", get(routes::models::list_models))
            .route("/v1/embeddings", post(routes::embeddings::create_embeddings))
            .route("/v1/chat/completions", post(routes::chat::create_chat_completion))
            .route("/v1/stats", get(routes::stats::get_stats))
            .route_layer(GovernorLayer::new(v1_governor))
            .route_layer(middleware::from_fn_with_state(db, auth::require_api_key))
            .merge(web::router(static_dir))
            .merge(beta_rate_limited)
            .route("/v1/node-info", get(routes::system::get_node_info))
            .route("/v1/version", get(routes::system::get_version))
            .route("/v1/update/{platform}", get(routes::system::get_update))
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .with_state(state)
    } else {
        // Test variant: same routes, no rate limiters, no background cleanup task.
        let beta_unlimited = Router::new().route("/beta", post(web::post_beta));

        Router::new()
            .route("/v1/models", get(routes::models::list_models))
            .route("/v1/embeddings", post(routes::embeddings::create_embeddings))
            .route("/v1/chat/completions", post(routes::chat::create_chat_completion))
            .route("/v1/stats", get(routes::stats::get_stats))
            .route_layer(middleware::from_fn_with_state(db, auth::require_api_key))
            .merge(web::router(static_dir))
            .merge(beta_unlimited)
            .route("/v1/node-info", get(routes::system::get_node_info))
            .route("/v1/version", get(routes::system::get_version))
            .route("/v1/update/{platform}", get(routes::system::get_update))
            .with_state(state)
    }
}
