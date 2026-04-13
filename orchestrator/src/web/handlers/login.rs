use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum_extra::extract::cookie::SignedCookieJar;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::{auth, db::entities::users, AppState};

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

/// Authenticate with email + password, set a signed session cookie.
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

    if user.email_verified_at.is_none() {
        return error_redirect.into_response();
    }

    if !auth::verify_password(form.password, user.password_hash.clone()).await {
        return error_redirect.into_response();
    }

    let jar = jar.add(auth::make_session_cookie(&user.pid));
    (jar, Redirect::to("/dashboard")).into_response()
}
