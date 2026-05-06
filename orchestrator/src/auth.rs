use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header::AUTHORIZATION, request::Parts},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::{Cookie, SignedCookieJar};

/// The authenticated API key's database ID, inserted into request extensions by the auth middleware.
#[derive(Clone, Copy, Debug)]
pub struct ApiKeyId(pub i32);

/// The authenticated web user, extracted from a signed session cookie.
#[derive(Clone, Debug)]
pub struct CurrentUser(pub crate::db::entities::users::Model);

/// Pool context from the API key, inserted into request extensions.
#[derive(Clone, Copy, Debug)]
pub struct PoolContext(pub Option<i32>);

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use sha2::{Digest, Sha256};

use crate::db::entities::{api_keys, users};
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
        .filter(api_keys::Column::IsActive.eq(true))
        .one(&db)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("db error: {e}")))?;

    let api_key = match exists {
        Some(key) => key,
        None => return Err(AppError::Unauthorized),
    };

    let mut request = request;
    request.extensions_mut().insert(ApiKeyId(api_key.id));
    request.extensions_mut().insert(PoolContext(api_key.pool_id));

    Ok(next.run(request).await)
}

pub const SESSION_COOKIE: &str = "__session";
pub const SESSION_MAX_AGE_DAYS: i64 = 30;

/// Create a signed session cookie for a user pid.
///
/// `secure` controls the cookie's Secure flag. Pass `true` whenever the site is
/// served over HTTPS — without it, the session cookie travels over plain HTTP
/// and is sniffable. Callers typically derive this from `base_url.starts_with("https://")`.
pub fn make_session_cookie(pid: &str, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE.to_string(), pid.to_string()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(time::Duration::days(SESSION_MAX_AGE_DAYS))
        .build()
}

/// Convenience: returns true if `base_url` is an HTTPS URL.
/// Used to decide whether the session cookie should set Secure.
pub fn is_https_base_url(base_url: &str) -> bool {
    base_url.starts_with("https://")
}

/// Create a cookie that clears the session.
pub fn clear_session_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE.to_string(), String::new()))
        .path("/")
        .http_only(true)
        .secure(secure)
        .max_age(time::Duration::ZERO)
        .build()
}

impl FromRequestParts<crate::AppState> for CurrentUser {
    type Rejection = axum::response::Redirect;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &crate::AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = SignedCookieJar::from_headers(&parts.headers, state.session_key.clone());

        let pid = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or_else(|| axum::response::Redirect::to("/login"))?;

        let user = users::Entity::find()
            .filter(users::Column::Pid.eq(&pid))
            .filter(users::Column::EmailVerifiedAt.is_not_null())
            .one(&state.db)
            .await
            .map_err(|_| axum::response::Redirect::to("/login"))?
            .ok_or_else(|| axum::response::Redirect::to("/login"))?;

        Ok(CurrentUser(user))
    }
}

/// Hash a password using argon2id. Runs on a blocking thread.
pub async fn hash_password(password: String) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("hash error: {e}"))?;
        Ok(hash.to_string())
    })
    .await?
}

/// Verify a password against an argon2id hash. Runs on a blocking thread.
pub async fn verify_password(password: String, hash: String) -> bool {
    tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash)
            .ok()
            .map_or(false, |parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            })
    })
    .await
    .unwrap_or(false)
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
