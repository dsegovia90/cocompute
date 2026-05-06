use axum::{
    Form,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::SignedCookieJar;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::{AppState, auth, db::entities::users};

#[derive(Deserialize)]
pub struct ResetForm {
    token: String,
    password: String,
}

/// Validate the reset token and set a new password.
pub async fn post_reset(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<ResetForm>,
) -> Response {
    if !form.token.chars().all(|c| c.is_ascii_hexdigit()) {
        return Redirect::to("/login").into_response();
    }

    let user = users::Entity::find()
        .filter(users::Column::ResetToken.eq(&form.token))
        .one(&state.db)
        .await;

    let user = match user {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/login").into_response(),
    };

    let expired = user
        .reset_sent_at
        .map(|sent| chrono::Utc::now() - sent > chrono::Duration::hours(1))
        .unwrap_or(true);

    if expired {
        return Redirect::to(&format!(
            "/reset?token={}&error=Link+has+expired",
            form.token
        ))
        .into_response();
    }

    if form.password.len() < 8 {
        return Redirect::to(&format!(
            "/reset?token={}&error=Password+must+be+at+least+8+characters",
            form.token
        ))
        .into_response();
    }

    let password_hash = match auth::hash_password(form.password).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("failed to hash password: {e}");
            return Redirect::to("/login").into_response();
        }
    };

    let was_unverified = user.email_verified_at.is_none();
    let mut active: users::ActiveModel = user.into();
    active.password_hash = Set(password_hash);
    active.reset_token = Set(None);
    active.reset_sent_at = Set(None);
    if was_unverified {
        active.email_verified_at = Set(Some(chrono::Utc::now()));
        active.email_verification_token = Set(None);
        active.email_verification_sent_at = Set(None);
    }
    active.updated_at = Set(chrono::Utc::now());

    let updated = match active.update(&state.db).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("failed to reset password: {e}");
            return Redirect::to("/login").into_response();
        }
    };

    let jar = jar.add(auth::make_session_cookie(&state, &updated.pid));
    (jar, Redirect::to("/dashboard")).into_response()
}
