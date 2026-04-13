use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use crate::{auth, db::entities::users, email, AppState};

#[derive(Deserialize)]
pub struct ForgotForm {
    email: String,
}

/// Generate a password reset token and email it. Always shows success to avoid leaking whether the email exists.
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
