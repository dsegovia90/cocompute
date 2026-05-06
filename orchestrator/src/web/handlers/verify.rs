use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::SignedCookieJar;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::{auth, db::entities::users, email, AppState};

#[derive(Deserialize)]
pub struct VerifyForm {
    token: String,
    password: String,
}

/// Validate the invite token, set the user's password, and activate the account.
pub async fn post_verify(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<VerifyForm>,
) -> Response {
    // Token must be hex-only (from generate_api_key)
    if !form.token.chars().all(|c| c.is_ascii_hexdigit()) {
        return Redirect::to("/login").into_response();
    }

    let user = users::Entity::find()
        .filter(users::Column::EmailVerificationToken.eq(&form.token))
        .one(&state.db)
        .await;

    let user = match user {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/login").into_response(),
    };

    // Check token not expired (48h)
    let expired = user.email_verification_sent_at
        .map(|sent| chrono::Utc::now() - sent > chrono::Duration::hours(48))
        .unwrap_or(true);

    if expired {
        return Redirect::to(&format!("/verify?token={}&error=Link+has+expired", form.token)).into_response();
    }

    if form.password.len() < 8 {
        return Redirect::to(&format!("/verify?token={}&error=Password+must+be+at+least+8+characters", form.token)).into_response();
    }

    let password_hash = match auth::hash_password(form.password).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("failed to hash password: {e}");
            return Redirect::to(&format!("/verify?token={}&error=Something+went+wrong", form.token)).into_response();
        }
    };

    let mut active: users::ActiveModel = user.into();
    active.password_hash = Set(password_hash);
    active.email_verified_at = Set(Some(chrono::Utc::now()));
    active.email_verification_token = Set(None);
    active.email_verification_sent_at = Set(None);
    active.updated_at = Set(chrono::Utc::now());

    let updated = match active.update(&state.db).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("failed to verify user: {e}");
            return Redirect::to("/login").into_response();
        }
    };

    // Send welcome email
    if let Some(ref mailer) = state.mailer {
        let parts = email::templates::welcome_email(&updated.name);
        if let Err(e) = mailer.send(&updated.email, &parts.subject, &parts.html, &parts.text).await {
            tracing::warn!("failed to send welcome email: {e}");
        }
    }

    let jar = jar.add(auth::make_session_cookie(&state, &updated.pid));
    (jar, Redirect::to("/dashboard")).into_response()
}
