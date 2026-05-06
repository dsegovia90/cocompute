use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::cookie::SignedCookieJar;

use crate::auth;

/// Clear the session cookie and redirect to the landing page.
pub async fn post_logout(jar: SignedCookieJar) -> Response {
    let jar = jar.add(auth::clear_session_cookie());
    (jar, Redirect::to("/")).into_response()
}
