//! Beta signup integration tests. Captcha is disabled in tests (Turnstile keys
//! are None on the test AppState), so the form submits without a token.

mod common;

use axum::{body::Body, http::Request};
use cocompute_orchestrator::db::entities::beta_invites;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use common::build_test_app;

#[tokio::test]
async fn open_signup_creates_beta_invite_row() {
    let app = build_test_app().await;

    let body = "name=Test+User&email=newcomer@example.com&role=consumer&gpu=RTX+3090";
    let req = Request::post("/beta")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();
    let response = app.call_raw(req).await;

    assert!(
        response.status().is_redirection(),
        "expected redirect, got {}",
        response.status()
    );
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(
        location.contains("success=true"),
        "expected redirect to ?success=true, got: {location}"
    );

    let saved = beta_invites::Entity::find()
        .filter(beta_invites::Column::Email.eq("newcomer@example.com"))
        .one(&app.db)
        .await
        .unwrap();
    assert!(saved.is_some(), "expected beta_invite row for newcomer@example.com");
    let saved = saved.unwrap();
    assert_eq!(saved.name, "Test User");
    assert_eq!(saved.role, "consumer");
    assert_eq!(saved.gpu_info.as_deref(), Some("RTX 3090"));
}
