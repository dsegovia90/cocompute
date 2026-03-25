use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use sha2::{Digest, Sha256};

use crate::db::entities::api_keys;
use crate::error::AppError;

/// Hash an API key using SHA-256.
pub fn hash_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

/// Generate a random API key (32 bytes, hex-encoded = 64 chars).
pub fn generate_api_key() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    let bytes: [u8; 32] = rng.random();
    hex::encode(bytes)
}

/// Axum middleware that checks the Authorization: Bearer <key> header.
pub async fn require_api_key(
    State(db): State<DatabaseConnection>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let key = match auth_header {
        Some(header) if header.starts_with("Bearer ") => &header[7..],
        _ => return Err(AppError::Unauthorized),
    };

    let key_hash = hash_key(key);

    let exists = api_keys::Entity::find()
        .filter(api_keys::Column::KeyHash.eq(&key_hash))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("db error: {e}")))?;

    if exists.is_none() {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(request).await)
}
