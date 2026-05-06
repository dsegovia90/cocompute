use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::{db::entities::beta_invites, email, AppState};

#[derive(Deserialize)]
pub struct BetaForm {
    name: String,
    email: String,
    role: String,
    gpu: Option<String>,
    /// Cloudflare Turnstile response token. The widget injects this as a hidden
    /// input named `cf-turnstile-response`. Optional in dev (when Turnstile keys
    /// aren't configured) and absent if a bot bypasses the widget entirely.
    #[serde(rename = "cf-turnstile-response")]
    cf_turnstile_response: Option<String>,
}

/// Verify a Turnstile response token against Cloudflare's siteverify endpoint.
/// Returns Ok(()) if the token is valid, Err with a user-facing reason otherwise.
async fn verify_turnstile(
    http: &reqwest::Client,
    secret: &str,
    token: &str,
) -> Result<(), &'static str> {
    #[derive(serde::Deserialize)]
    struct SiteVerifyResponse {
        success: bool,
        #[serde(default, rename = "error-codes")]
        error_codes: Vec<String>,
    }

    // URL-encode the form body manually (avoids needing reqwest's `form` feature).
    let body = format!(
        "secret={}&response={}",
        urlencoding::encode(secret),
        urlencoding::encode(token),
    );

    let response = http
        .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
        .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("turnstile siteverify request failed: {e}");
            "captcha verification failed"
        })?;

    let body: SiteVerifyResponse = response.json().await.map_err(|e| {
        tracing::error!("turnstile siteverify response parse failed: {e}");
        "captcha verification failed"
    })?;

    if !body.success {
        tracing::warn!("turnstile rejected token: {:?}", body.error_codes);
        return Err("captcha failed");
    }

    Ok(())
}

/// Save a beta invite to the waitlist and send a confirmation email.
pub async fn post_beta(
    State(state): State<AppState>,
    Form(form): Form<BetaForm>,
) -> Response {
    // Captcha gate: only enforce when both site_key and secret_key are set
    // (lets local dev work without Turnstile credentials).
    if let Some(secret) = state.turnstile_secret_key.as_deref() {
        let token = match form.cf_turnstile_response.as_deref() {
            Some(t) if !t.is_empty() => t,
            _ => {
                return Redirect::to("/beta?error=Please+complete+the+captcha")
                    .into_response();
            }
        };
        if let Err(reason) = verify_turnstile(&state.http, secret, token).await {
            let url = format!("/beta?error={}", reason.replace(' ', "+"));
            return Redirect::to(&url).into_response();
        }
    }

    let existing = beta_invites::Entity::find()
        .filter(beta_invites::Column::Email.eq(&form.email))
        .one(&state.db)
        .await;

    if let Ok(Some(_)) = existing {
        return Redirect::to("/beta?error=That+email+is+already+on+the+list").into_response();
    }

    let invite = beta_invites::ActiveModel {
        name: Set(form.name.clone()),
        email: Set(form.email.clone()),
        role: Set(form.role.clone()),
        gpu_info: Set(form.gpu.filter(|g| !g.is_empty())),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    if let Err(e) = invite.insert(&state.db).await {
        tracing::error!("failed to save beta invite: {e}");
        return Redirect::to("/beta?error=Something+went+wrong").into_response();
    }

    if let Some(ref mailer) = state.mailer {
        let parts = email::templates::waitlist_email(&form.email);
        if let Err(e) = mailer.send(&form.email, &parts.subject, &parts.html, &parts.text).await {
            tracing::warn!("failed to send waitlist email: {e}");
        }
    }

    Redirect::to("/beta?success=true").into_response()
}
