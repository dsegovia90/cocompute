use axum::{extract::State, response::{IntoResponse, Redirect, Response}};
use axum_extra::extract::cookie::SignedCookieJar;

use crate::{auth, AppState};

/// Clear the session cookie and redirect to the landing page.
pub async fn post_logout(State(state): State<AppState>, jar: SignedCookieJar) -> Response {
    let jar = jar.add(auth::clear_session_cookie(&state));
    (jar, Redirect::to("/")).into_response()
}
