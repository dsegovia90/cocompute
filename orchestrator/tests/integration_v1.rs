//! Auth tests for the public /v1/* API surface. The most important coverage
//! before going public: an invalid Bearer token must return 401 with no leakage.

mod common;

use axum::{body::Body, http::Request};

use common::build_test_app;

#[tokio::test]
async fn chat_completions_invalid_bearer_returns_401() {
    let app = build_test_app().await;

    let req = Request::post("/v1/chat/completions")
        .header("authorization", "Bearer this-is-not-a-real-key")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"model":"llama3.2","messages":[{"role":"user","content":"hello"}]}"#,
        ))
        .unwrap();
    let (status, _body) = app.call(req).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn models_invalid_bearer_returns_401() {
    let app = build_test_app().await;

    let req = Request::get("/v1/models")
        .header("authorization", "Bearer this-is-not-a-real-key")
        .body(Body::empty())
        .unwrap();
    let (status, _body) = app.call(req).await;
    assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
}
