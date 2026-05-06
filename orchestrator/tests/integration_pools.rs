//! Regression tests for the pools.rs handlers that previously swallowed DB errors.
//! Each test exercises the happy path end-to-end. If anyone re-introduces the old
//! `let _ = active.update(...).await` pattern, the redirect target changes from
//! ?saved=true to ?error=update_failed under failure conditions — but more
//! importantly, these tests fail loudly if the basic flow breaks at all.

mod common;

use axum::{body::Body, http::Request};
use cocompute_orchestrator::db::entities::{api_keys, host_pool_memberships, hosts, pools, pool_members};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

use common::{authed_post, build_test_app, create_verified_user, login};

async fn create_pool(db: &sea_orm::DatabaseConnection, owner_id: i32, name: &str) -> pools::Model {
    let pid = uuid::Uuid::new_v4().to_string();
    let pool = pools::ActiveModel {
        pid: Set(pid),
        name: Set(name.to_string()),
        owner_id: Set(owner_id),
        is_global: Set(false),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };
    let pool = pool.insert(db).await.unwrap();
    let member = pool_members::ActiveModel {
        pool_id: Set(pool.id),
        user_id: Set(owner_id),
        role: Set("owner".to_string()),
        invited_at: Set(chrono::Utc::now()),
        accepted_at: Set(Some(chrono::Utc::now())),
        ..Default::default()
    };
    let _ = member.insert(db).await;
    pool
}

#[tokio::test]
async fn deactivate_pool_marks_is_active_false() {
    let app = build_test_app().await;
    let user = create_verified_user(&app.db, "alice@example.com", "hunter2hunter2", "Alice").await;
    let pool = create_pool(&app.db, user.id, "pool-to-deactivate").await;

    let cookie = login(&app, "alice@example.com", "hunter2hunter2").await;
    let req = authed_post(&format!("/pools/{}/deactivate", pool.pid), &cookie, "");
    let response = app.call_raw(req).await;

    assert!(response.status().is_redirection(), "expected redirect, got {}", response.status());
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(
        location.contains("?saved=true") && !location.contains("error="),
        "expected ?saved=true with no error, got: {location}"
    );

    let after = pools::Entity::find_by_id(pool.id).one(&app.db).await.unwrap().unwrap();
    assert!(!after.is_active, "pool should be is_active=false after deactivate");
}

#[tokio::test]
async fn reactivate_pool_marks_is_active_true() {
    let app = build_test_app().await;
    let user = create_verified_user(&app.db, "alice@example.com", "hunter2hunter2", "Alice").await;
    let pool = create_pool(&app.db, user.id, "pool-to-reactivate").await;

    // First, deactivate
    let mut active: pools::ActiveModel = pool.clone().into();
    active.is_active = Set(false);
    active.update(&app.db).await.unwrap();

    let cookie = login(&app, "alice@example.com", "hunter2hunter2").await;
    let req = authed_post(&format!("/pools/{}/reactivate", pool.pid), &cookie, "");
    let response = app.call_raw(req).await;

    assert!(response.status().is_redirection());
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("?saved=true") && !location.contains("error="));

    let after = pools::Entity::find_by_id(pool.id).one(&app.db).await.unwrap().unwrap();
    assert!(after.is_active, "pool should be is_active=true after reactivate");
}

#[tokio::test]
async fn deactivate_api_key_marks_is_active_false() {
    let app = build_test_app().await;
    let user = create_verified_user(&app.db, "alice@example.com", "hunter2hunter2", "Alice").await;
    let pool = create_pool(&app.db, user.id, "test-pool").await;

    // Create an API key directly
    let raw_key = "test-key-value";
    let key_hash = cocompute_orchestrator::auth::hash_key(raw_key);
    let key = api_keys::ActiveModel {
        key_hash: Set(key_hash),
        created_at: Set(chrono::Utc::now()),
        user_id: Set(Some(user.id)),
        pool_id: Set(Some(pool.id)),
        label: Set(Some("test key".to_string())),
        ..Default::default()
    };
    let key = key.insert(&app.db).await.unwrap();

    let cookie = login(&app, "alice@example.com", "hunter2hunter2").await;
    let req = authed_post(&format!("/api-keys/{}/deactivate", key.id), &cookie, "");
    let response = app.call_raw(req).await;

    assert!(response.status().is_redirection());
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.contains("?saved=true") && !location.contains("error="));

    let after = api_keys::Entity::find_by_id(key.id).one(&app.db).await.unwrap().unwrap();
    assert!(!after.is_active);
}

#[tokio::test]
async fn remove_host_from_pool_marks_membership_inactive() {
    let app = build_test_app().await;
    let user = create_verified_user(&app.db, "alice@example.com", "hunter2hunter2", "Alice").await;
    let pool = create_pool(&app.db, user.id, "test-pool").await;

    // Register a host owned by alice
    let host_endpoint_id = "test-host-endpoint-id-abcd1234";
    let host = hosts::ActiveModel {
        endpoint_id: Set(host_endpoint_id.to_string()),
        status: Set("connected".to_string()),
        last_seen: Set(Some(chrono::Utc::now())),
        user_id: Set(Some(user.id)),
        ..Default::default()
    };
    host.insert(&app.db).await.unwrap();

    // Add it to the pool
    let membership = host_pool_memberships::ActiveModel {
        host_endpoint_id: Set(host_endpoint_id.to_string()),
        pool_id: Set(pool.id),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };
    membership.insert(&app.db).await.unwrap();

    let cookie = login(&app, "alice@example.com", "hunter2hunter2").await;
    let req = authed_post(
        &format!("/pools/{}/remove-host/{}", pool.pid, host_endpoint_id),
        &cookie,
        "",
    );
    let response = app.call_raw(req).await;

    assert!(response.status().is_redirection());
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    assert!(!location.contains("error="), "did not expect error redirect, got: {location}");

    let after = host_pool_memberships::Entity::find()
        .filter(host_pool_memberships::Column::PoolId.eq(pool.id))
        .filter(host_pool_memberships::Column::HostEndpointId.eq(host_endpoint_id))
        .one(&app.db)
        .await
        .unwrap()
        .unwrap();
    assert!(!after.is_active, "membership should be is_active=false after remove");
}
