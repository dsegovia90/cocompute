use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use serde::Deserialize;

use crate::{
    email,
    signup::{self, SignupError, SignupInput},
    AppState,
};

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

/// Open signup. Captcha gate, then create the user immediately + send a
/// verification email. The user clicks the link, sets a real password, and
/// is in. No manual invite-user CLI step required.
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

    let result = signup::create_user_and_invite(
        &state.db,
        SignupInput {
            name: form.name.clone(),
            email: form.email.clone(),
            role: form.role.clone(),
            gpu: form.gpu.clone(),
        },
    )
    .await;

    let signup_result = match result {
        Ok(r) => r,
        Err(SignupError::UserAlreadyExists) => {
            return Redirect::to(
                "/beta?error=That+email+is+already+signed+up.+Try+logging+in.",
            )
            .into_response();
        }
        Err(SignupError::Db(e)) => {
            tracing::error!("signup db error: {e}");
            return Redirect::to("/beta?error=Something+went+wrong").into_response();
        }
        Err(SignupError::Hash(e)) => {
            tracing::error!("signup hash error: {e}");
            return Redirect::to("/beta?error=Something+went+wrong").into_response();
        }
    };

    // Send verification email. If sending fails, the user is created in DB
    // but no email arrives — log loudly so the operator can manually surface
    // the verification link from a CLI/log query if needed.
    if let Some(ref mailer) = state.mailer {
        let parts = email::templates::invite_email(
            &signup_result.user.name,
            &signup_result.verification_token,
            &state.base_url,
        );
        if let Err(e) = mailer
            .send(
                &signup_result.user.email,
                &parts.subject,
                &parts.html,
                &parts.text,
            )
            .await
        {
            tracing::warn!(
                "failed to send verification email to {}: {e}",
                signup_result.user.email
            );
        }
    } else {
        // Mailer not configured (local dev) — print the verify URL so the
        // operator can complete signup manually.
        let verify_url = format!(
            "{}/verify?token={}",
            state.base_url, signup_result.verification_token
        );
        tracing::warn!(
            "SMTP not configured — manual verify URL for {}: {verify_url}",
            signup_result.user.email
        );
    }

    Redirect::to("/beta?success=true").into_response()
}
