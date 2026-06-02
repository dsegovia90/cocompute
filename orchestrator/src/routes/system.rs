use axum::{Json, extract::State};

use crate::{AppState, error::AppError};

/// GET /v1/node-info, Returns the orchestrator's current iroh endpoint ID.
/// Unauthenticated, used by hosts to discover/refresh the orchestrator ID.
pub(crate) async fn get_node_info(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "endpoint_id": state.endpoint_id,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// GET /v1/version, Returns the orchestrator version for update checks.
pub(crate) async fn get_version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Platforms we publish host binaries for. Must match the `name` values in
/// `.github/workflows/release.yml` and the `case "$OS"`/`case "$ARCH"` mapping
/// in `orchestrator/static/install.sh`.
const SUPPORTED_PLATFORMS: &[&str] = &[
    "linux-x86_64",
    "linux-arm64",
    "macos-arm64",
    "macos-x86_64",
];

/// GitHub repo slug for release artifacts. The orchestrator redirects host
/// binary download requests to GitHub Releases, which serves them from a CDN
/// without consuming orchestrator bandwidth.
const RELEASES_REPO: &str = "dsegovia90/cocompute";

/// GET /v1/update/:platform, Redirects to the GitHub Release artifact for the
/// orchestrator's current version. The redirect target is:
///
/// ```text
/// https://github.com/{RELEASES_REPO}/releases/download/v{VERSION}/cocompute-host-{platform}
/// ```
///
/// Clients (install.sh, the host binary's self-update flow) must follow the
/// redirect. install.sh uses `curl -sSfL` and update.rs uses reqwest's default
/// redirect-following behavior.
///
/// Sister artifact: cocompute-host-{platform}.minisig, the minisign signature
/// for the binary. install.sh fetches both and verifies the signature before
/// chmod +x.
///
/// Returns 404 for unknown platforms, 302 with Location header otherwise.
pub(crate) async fn get_update(
    axum::extract::Path(platform): axum::extract::Path<String>,
) -> Result<axum::response::Response, AppError> {
    if !SUPPORTED_PLATFORMS.contains(&platform.as_str()) {
        return Err(AppError::NotFound(format!(
            "unknown platform: {platform}. Supported: {}",
            SUPPORTED_PLATFORMS.join(", ")
        )));
    }

    let version = env!("CARGO_PKG_VERSION");
    let url = format!(
        "https://github.com/{RELEASES_REPO}/releases/download/v{version}/cocompute-host-{platform}"
    );

    Ok(axum::response::Response::builder()
        .status(axum::http::StatusCode::FOUND)
        .header("location", url)
        .header("cache-control", "no-cache")
        .body(axum::body::Body::empty())
        .unwrap())
}

/// GET /v1/update/:platform.minisig, Redirects to the minisign signature
/// artifact for the host binary at the orchestrator's current version. install.sh
/// fetches this immediately after the binary and verifies before chmod +x.
pub(crate) async fn get_update_signature(
    axum::extract::Path(platform): axum::extract::Path<String>,
) -> Result<axum::response::Response, AppError> {
    // Path comes in as "linux-x86_64.minisig"; strip the suffix to validate the platform.
    let bare = platform
        .strip_suffix(".minisig")
        .ok_or_else(|| AppError::NotFound(format!("expected .minisig suffix, got: {platform}")))?;

    if !SUPPORTED_PLATFORMS.contains(&bare) {
        return Err(AppError::NotFound(format!(
            "unknown platform: {bare}. Supported: {}",
            SUPPORTED_PLATFORMS.join(", ")
        )));
    }

    let version = env!("CARGO_PKG_VERSION");
    let url = format!(
        "https://github.com/{RELEASES_REPO}/releases/download/v{version}/cocompute-host-{bare}.minisig"
    );

    Ok(axum::response::Response::builder()
        .status(axum::http::StatusCode::FOUND)
        .header("location", url)
        .header("cache-control", "no-cache")
        .body(axum::body::Body::empty())
        .unwrap())
}
