//! Shared test harness for orchestrator integration tests.
//!
//! Each test gets a fresh tempfile-backed SQLite DB with all migrations applied,
//! a real (but throwaway) iroh Endpoint, and a router built with rate limiters
//! disabled. Tests use `tower::ServiceExt::oneshot` to hit the router directly
//! without a network listener.

use axum::{Router, body::Body, http::{Request, Response, StatusCode}};
use cocompute_orchestrator::{
    AppState, auth, build_router, db,
    db::entities::users,
    host_manager::HostManager,
};
use http_body_util::BodyExt;
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};
use sea_orm_migration::MigratorTrait;
use tempfile::TempDir;
use tower::ServiceExt;

/// A constructed test app holding the router, DB, and the temp dir backing the DB.
/// Drop order matters: keep `_tempdir` alive until the DB is no longer in use.
pub struct TestApp {
    pub router: Router,
    pub db: DatabaseConnection,
    pub session_key: axum_extra::extract::cookie::Key,
    _tempdir: TempDir,
}

impl TestApp {
    /// Send a request, return the response (status + body bytes).
    pub async fn call(&self, request: Request<Body>) -> (StatusCode, Vec<u8>) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes().to_vec();
        (status, bytes)
    }

    /// Send a request and return the full response (for inspecting headers like Set-Cookie).
    pub async fn call_raw(&self, request: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(request).await.unwrap()
    }
}

/// Build a fresh test app: temp DB, migrations applied, router wired without rate limits.
pub async fn build_test_app() -> TestApp {
    let tempdir = tempfile::tempdir().unwrap();
    let db_path = tempdir.path().join("test.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    let db = Database::connect(&db_url).await.expect("connect test db");
    db::migration::Migrator::up(&db, None).await.expect("apply migrations");

    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .bind()
        .await
        .expect("bind iroh endpoint");
    let endpoint_id = format!("{}", endpoint.addr().id);

    let session_key = axum_extra::extract::cookie::Key::generate();

    let state = AppState {
        endpoint,
        endpoint_id,
        db: db.clone(),
        hosts: HostManager::new(),
        mailer: None,
        session_key: session_key.clone(),
        base_url: "http://localhost:4000".into(),
        // Both None: captcha disabled, signup goes through without a Turnstile token.
        turnstile_site_key: None,
        turnstile_secret_key: None,
        http: reqwest::Client::new(),
    };

    // build_router(_, _, false) skips the governor layers — tests don't need them
    // and adding them would couple test results to wall-clock timing.
    let router = build_router(state, "orchestrator/static", false);

    TestApp { router, db, session_key, _tempdir: tempdir }
}

/// Insert a verified user directly via the DB. Returns the user record.
/// The password hash is real argon2; if the test only needs the user to exist
/// (not to log in), pass any string for `password`.
pub async fn create_verified_user(
    db: &DatabaseConnection,
    email: &str,
    password: &str,
    name: &str,
) -> users::Model {
    let pid = uuid::Uuid::new_v4().to_string();
    let password_hash = auth::hash_password(password.to_string()).await.unwrap();

    let user = users::ActiveModel {
        pid: Set(pid),
        email: Set(email.to_string()),
        password_hash: Set(password_hash),
        name: Set(name.to_string()),
        email_verified_at: Set(Some(chrono::Utc::now())),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    };
    user.insert(db).await.expect("insert user")
}

/// Log a user in via POST /login and return the Set-Cookie value to use in subsequent requests.
pub async fn login(app: &TestApp, email: &str, password: &str) -> String {
    let body = format!(
        "email={}&password={}",
        urlencoding::encode(email),
        urlencoding::encode(password),
    );
    let request = Request::post("/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from(body))
        .unwrap();
    let response = app.call_raw(request).await;
    assert!(
        response.status().is_redirection(),
        "login should redirect on success, got {}",
        response.status()
    );

    response
        .headers()
        .get_all("set-cookie")
        .iter()
        .find_map(|v| {
            let s = v.to_str().ok()?;
            if s.starts_with("__session=") {
                // Strip everything after the first ; (path, samesite, etc.)
                Some(s.split(';').next().unwrap().to_string())
            } else {
                None
            }
        })
        .expect("login response missing __session cookie")
}

/// Build a POST request with a session cookie, urlencoded body.
pub fn authed_post(path: &str, cookie: &str, body: &str) -> Request<Body> {
    Request::post(path)
        .header("content-type", "application/x-www-form-urlencoded")
        .header("cookie", cookie)
        .body(Body::from(body.to_string()))
        .unwrap()
}
