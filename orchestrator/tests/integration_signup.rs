//! Signup integration tests. Captcha is disabled in tests (Turnstile keys
//! are None on the test AppState), so the form submits without a token.

mod common;

use axum::{body::Body, http::Request};
use cocompute_orchestrator::db::entities::{beta_invites, users};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use common::build_test_app;

#[tokio::test]
async fn open_signup_creates_user_and_beta_invite() {
    let app = build_test_app().await;

    let body = "name=Test+User&email=newcomer@example.com&role=consumer&gpu=RTX+3090";
    let req = Request::post("/signup")
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

    // beta_invite row exists for analytics
    let invite = beta_invites::Entity::find()
        .filter(beta_invites::Column::Email.eq("newcomer@example.com"))
        .one(&app.db)
        .await
        .unwrap();
    assert!(invite.is_some(), "expected beta_invite row");
    let invite = invite.unwrap();
    assert_eq!(invite.name, "Test User");
    assert_eq!(invite.role, "consumer");
    assert_eq!(invite.gpu_info.as_deref(), Some("RTX 3090"));

    // User row created with verification token, NOT yet verified
    let user = users::Entity::find()
        .filter(users::Column::Email.eq("newcomer@example.com"))
        .one(&app.db)
        .await
        .unwrap();
    assert!(user.is_some(), "expected user row to be auto-created on signup");
    let user = user.unwrap();
    assert_eq!(user.name, "Test User");
    assert!(
        user.email_verified_at.is_none(),
        "user should NOT be verified until they click the email link"
    );
    assert!(
        user.email_verification_token.is_some(),
        "user should have a verification token set"
    );
    assert!(
        user.email_verification_sent_at.is_some(),
        "user should have a verification_sent_at timestamp"
    );
}

#[tokio::test]
async fn duplicate_signup_returns_generic_success_no_enumeration() {
    // Email enumeration mitigation: existing-email signup MUST respond identically
    // to new-email signup. If a client can distinguish "your email is taken" from
    // "you signed up successfully," they can scrape which emails have accounts.
    let app = build_test_app().await;

    let body = "name=First&email=dupe@example.com&role=consumer";
    let req = Request::post("/signup")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();
    let first = app.call_raw(req).await;
    assert!(first.status().is_redirection());
    let first_location = first.headers().get("location").unwrap().to_str().unwrap();
    assert!(first_location.contains("success=true"));

    // Second signup with the same email, must look identical to the client.
    let req = Request::post("/signup")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(
            "name=Second&email=dupe@example.com&role=host",
        ))
        .unwrap();
    let second = app.call_raw(req).await;
    assert!(second.status().is_redirection());
    let second_location = second.headers().get("location").unwrap().to_str().unwrap();
    assert_eq!(
        first_location, second_location,
        "duplicate signup must redirect to same URL as fresh signup (enumeration prevention). \
         first={first_location} second={second_location}"
    );

    // The second submit should NOT have created a duplicate user or overwritten
    // the first user's name. Verify the database stayed consistent.
    use cocompute_orchestrator::db::entities::users;
    use sea_orm::EntityTrait;
    let count = users::Entity::find()
        .filter(users::Column::Email.eq("dupe@example.com"))
        .all(&app.db)
        .await
        .unwrap()
        .len();
    assert_eq!(count, 1, "duplicate signup must not create a second user row");
}
