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
    email: String,
    role: Option<String>,
    gpu: Option<String>,
}

/// Save a beta invite to the waitlist and send a confirmation email.
pub async fn post_beta(
    State(state): State<AppState>,
    Form(form): Form<BetaForm>,
) -> Response {
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

    if let Some(ref mailer) = state.mailer {
        let parts = email::templates::waitlist_email(&form.email);
        if let Err(e) = mailer.send(&form.email, &parts.subject, &parts.html, &parts.text).await {
            tracing::warn!("failed to send waitlist email: {e}");
        }
    }

    Redirect::to("/beta?success=true").into_response()
}
