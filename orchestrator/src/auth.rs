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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_key_deterministic() {
        let hash1 = hash_key("test-key-123");
        let hash2 = hash_key("test-key-123");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn hash_key_different_inputs_different_outputs() {
        let hash1 = hash_key("key-a");
        let hash2 = hash_key("key-b");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn hash_key_is_hex_sha256() {
        let hash = hash_key("hello");
        // SHA-256 of "hello" is well-known
        assert_eq!(hash, "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824");
    }

    #[test]
    fn hash_key_output_length() {
        let hash = hash_key("any-key");
        assert_eq!(hash.len(), 64); // SHA-256 = 32 bytes = 64 hex chars
    }

    #[test]
    fn generate_api_key_length() {
        let key = generate_api_key();
        assert_eq!(key.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn generate_api_key_is_hex() {
        let key = generate_api_key();
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generate_api_key_unique() {
        let key1 = generate_api_key();
        let key2 = generate_api_key();
        assert_ne!(key1, key2);
    }
}
