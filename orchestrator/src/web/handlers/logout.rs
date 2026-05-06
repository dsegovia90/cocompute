use axum::{extract::State, response::{IntoResponse, Redirect, Response}};
use axum_extra::extract::cookie::SignedCookieJar;

use crate::{auth, AppState};

/// Clear the session cookie and redirect to the landing page.
pub async fn post_logout(State(state): State<AppState>, jar: SignedCookieJar) -> Response {
    let secure = auth::is_https_base_url(&state.base_url);
    let jar = jar.add(auth::clear_session_cookie(secure));
    (jar, Redirect::to("/")).into_response()
}
