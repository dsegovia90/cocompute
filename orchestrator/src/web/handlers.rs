use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::SignedCookieJar;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::{
    auth,
    db::entities::{beta_invites, users},
    email, AppState,
};

// ── POST /beta ──

#[derive(Deserialize)]
pub struct BetaForm {
    email: String,
    role: Option<String>,
    gpu: Option<String>,
}

pub async fn post_beta(
    State(state): State<AppState>,
    Form(form): Form<BetaForm>,
) -> Response {
    // Check if already on the list
    let existing = beta_invites::Entity::find()
        .filter(beta_invites::Column::Email.eq(&form.email))
        .one(&state.db)
        .await;

    if let Ok(Some(_)) = existing {
        return Redirect::to("/beta?error=That+email+is+already+on+the+list").into_response();
    }

    let invite = beta_invites::ActiveModel {
        email: Set(form.email.clone()),
        role: Set(form.role.unwrap_or_else(|| "consumer".to_string())),
        gpu_info: Set(form.gpu.filter(|g| !g.is_empty())),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    if let Err(e) = invite.insert(&state.db).await {
        tracing::error!("failed to save beta invite: {e}");
        return Redirect::to("/beta?error=Something+went+wrong").into_response();
    }

    // Send waitlist email
    if let Some(ref mailer) = state.mailer {
        let parts = email::templates::waitlist_email(&form.email);
        if let Err(e) = mailer.send(&form.email, &parts.subject, &parts.html, &parts.text).await {
            tracing::warn!("failed to send waitlist email: {e}");
        }
    }

    Redirect::to("/beta?success=true").into_response()
}

// ── POST /login ──

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

pub async fn post_login(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<LoginForm>,
) -> Response {
    let error_redirect = Redirect::to("/login?error=Invalid+email+or+password");

    let user = users::Entity::find()
        .filter(users::Column::Email.eq(&form.email))
        .one(&state.db)
        .await;

    let user = match user {
        Ok(Some(u)) => u,
        _ => return error_redirect.into_response(),
    };

    // Must be verified
    if user.email_verified_at.is_none() {
        return error_redirect.into_response();
    }

    if !auth::verify_password(form.password, user.password_hash.clone()).await {
        return error_redirect.into_response();
    }

    let jar = jar.add(auth::make_session_cookie(&user.pid));
    (jar, Redirect::to("/")).into_response()
}

// ── POST /logout ──

pub async fn post_logout(jar: SignedCookieJar) -> Response {
    let jar = jar.add(auth::clear_session_cookie());
    (jar, Redirect::to("/")).into_response()
}

// ── POST /verify ──

#[derive(Deserialize)]
pub struct VerifyForm {
    token: String,
    password: String,
}

pub async fn post_verify(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<VerifyForm>,
) -> Response {
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

    let api_key = auth::generate_api_key();
    let api_key_hash = auth::hash_key(&api_key);

    let mut active: users::ActiveModel = user.into();
    active.password_hash = Set(password_hash);
    active.email_verified_at = Set(Some(chrono::Utc::now()));
    active.email_verification_token = Set(None);
    active.email_verification_sent_at = Set(None);
    active.api_key = Set(Some(api_key_hash));
    active.updated_at = Set(chrono::Utc::now());

    let updated = match active.update(&state.db).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("failed to verify user: {e}");
            return Redirect::to("/login").into_response();
        }
    };

    // Send welcome email with API key
    if let Some(ref mailer) = state.mailer {
        let parts = email::templates::welcome_email(&updated.name, &api_key);
        if let Err(e) = mailer.send(&updated.email, &parts.subject, &parts.html, &parts.text).await {
            tracing::warn!("failed to send welcome email: {e}");
        }
    }

    let jar = jar.add(auth::make_session_cookie(&updated.pid));
    (jar, Redirect::to("/")).into_response()
}

// ── POST /forgot ──

#[derive(Deserialize)]
pub struct ForgotForm {
    email: String,
}

pub async fn post_forgot(
    State(state): State<AppState>,
    Form(form): Form<ForgotForm>,
) -> Response {
    // Always redirect to success (don't leak whether email exists)
    let redirect = Redirect::to("/forgot?sent=true");

    if let Ok(Some(user)) = users::Entity::find()
        .filter(users::Column::Email.eq(&form.email))
        .one(&state.db)
        .await
    {
        let token = auth::generate_api_key();
        let mut active: users::ActiveModel = user.clone().into();
        active.reset_token = Set(Some(token.clone()));
        active.reset_sent_at = Set(Some(chrono::Utc::now()));
        active.updated_at = Set(chrono::Utc::now());

        if let Err(e) = active.update(&state.db).await {
            tracing::error!("failed to set reset token: {e}");
            return redirect.into_response();
        }

        if let Some(ref mailer) = state.mailer {
            let parts = email::templates::reset_email(&user.name, &token, &state.base_url);
            if let Err(e) = mailer.send(&user.email, &parts.subject, &parts.html, &parts.text).await {
                tracing::warn!("failed to send reset email: {e}");
            }
        }
    }

    redirect.into_response()
}

// ── POST /reset ──

#[derive(Deserialize)]
pub struct ResetForm {
    token: String,
    password: String,
}

pub async fn post_reset(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Form(form): Form<ResetForm>,
) -> Response {
    let user = users::Entity::find()
        .filter(users::Column::ResetToken.eq(&form.token))
        .one(&state.db)
        .await;

    let user = match user {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/login").into_response(),
    };

    let expired = user.reset_sent_at
        .map(|sent| chrono::Utc::now() - sent > chrono::Duration::hours(1))
        .unwrap_or(true);

    if expired {
        return Redirect::to(&format!("/reset?token={}&error=Link+has+expired", form.token)).into_response();
    }

    if form.password.len() < 8 {
        return Redirect::to(&format!("/reset?token={}&error=Password+must+be+at+least+8+characters", form.token)).into_response();
    }

    let password_hash = match auth::hash_password(form.password).await {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("failed to hash password: {e}");
            return Redirect::to("/login").into_response();
        }
    };

    let mut active: users::ActiveModel = user.into();
    active.password_hash = Set(password_hash);
    active.reset_token = Set(None);
    active.reset_sent_at = Set(None);
    active.updated_at = Set(chrono::Utc::now());

    let updated = match active.update(&state.db).await {
        Ok(u) => u,
        Err(e) => {
            tracing::error!("failed to reset password: {e}");
            return Redirect::to("/login").into_response();
        }
    };

    let jar = jar.add(auth::make_session_cookie(&updated.pid));
    (jar, Redirect::to("/")).into_response()
}
