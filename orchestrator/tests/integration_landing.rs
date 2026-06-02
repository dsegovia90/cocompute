//! Render tests for the public landing + quickstart pages after the
//! "safely expose your local inference" reframe.
//!
//! These hit the router with `oneshot` (no socket layer) and assert on the
//! server-rendered HTML. They guard three things:
//!   1. the new hero is present for logged-out visitors,
//!   2. logged-in visitors get the dashboard CTA, not the signup hero,
//!   3. the quickstart commands still render after the CodeBlock refactor.

mod common;

use axum::{body::Body, http::Request};

use common::{build_test_app, create_verified_user, login};

#[tokio::test]
async fn landing_logged_out_shows_expose_hero_and_signup() {
    let app = build_test_app().await;

    let (status, bytes) = app
        .call(Request::get("/").body(Body::empty()).unwrap())
        .await;
    assert!(status.is_success(), "GET / should be 200, got {status}");
    let body = String::from_utf8_lossy(&bytes);

    assert!(
        body.contains("fastest way to safely expose"),
        "hero headline missing"
    );
    assert!(body.contains("install.sh"), "host install command missing");
    assert!(body.contains("Base URL"), "connection card Base URL missing");
    assert!(body.contains("/v1"), "base url path missing");
    assert!(
        body.contains("speaks the OpenAI API spec"),
        "OpenAI-spec caption missing"
    );
    assert!(body.contains("Open WebUI"), "Open WebUI tool pill missing");
    assert!(body.contains("LangChain"), "LangChain tool pill missing");
    assert!(body.contains("href=\"/signup\""), "signup CTA missing");
    assert!(
        !body.contains("Go to dashboard"),
        "logged-out hero should not show the dashboard CTA"
    );
    assert!(
        !body.contains("Open infrastructure for the rest of us"),
        "old hero copy still present"
    );
}

#[tokio::test]
async fn landing_logged_in_shows_dashboard_cta() {
    let app = build_test_app().await;
    create_verified_user(&app.db, "host@example.com", "correct horse", "Daniel").await;
    let cookie = login(&app, "host@example.com", "correct horse").await;

    let (status, bytes) = app
        .call(
            Request::get("/")
                .header("cookie", &cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await;
    assert!(status.is_success(), "GET / should be 200, got {status}");
    let body = String::from_utf8_lossy(&bytes);

    assert!(
        body.contains("Go to dashboard"),
        "logged-in hero should show the dashboard CTA"
    );
}

#[tokio::test]
async fn quickstart_still_renders_commands_after_codeblock_refactor() {
    let app = build_test_app().await;

    let (status, bytes) = app
        .call(Request::get("/quickstart").body(Body::empty()).unwrap())
        .await;
    assert!(status.is_success(), "GET /quickstart should be 200, got {status}");
    let body = String::from_utf8_lossy(&bytes);

    assert!(body.contains("install.sh"), "host install command missing");
    assert!(
        body.contains("/v1/chat/completions"),
        "consumer curl command missing"
    );
    assert!(body.contains("/v1/models"), "list-models command missing");
    assert!(body.contains("href=\"/signup\""), "signup link missing");
}
