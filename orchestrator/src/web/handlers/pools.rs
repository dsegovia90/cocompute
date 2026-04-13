use axum::{
    extract::State,
    response::{IntoResponse, Redirect, Response},
    Form,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::Deserialize;

use fake::Fake;
use fake::faker::lorem::en::Word;

use crate::{
    auth::CurrentUser,
    db::entities::{pool_members, pools},
    AppState,
};

fn random_pool_name() -> String {
    let w1: String = Word().fake();
    let w2: String = Word().fake();
    format!("{w1}-{w2}")
}

/// Create a new pool with a random name. The current user becomes the owner.
pub async fn create_pool(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Response {
    let pid = uuid::Uuid::new_v4().to_string();
    let name = random_pool_name();

    let pool = pools::ActiveModel {
        pid: Set(pid),
        name: Set(name),
        owner_id: Set(user.id),
        is_global: Set(false),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };

    let pool = match pool.insert(&state.db).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("failed to create pool: {e}");
            return Redirect::to("/dashboard").into_response();
        }
    };

    // Add the creator as owner in pool_members
    let member = pool_members::ActiveModel {
        pool_id: Set(pool.id),
        user_id: Set(user.id),
        role: Set("owner".to_string()),
        invited_at: Set(chrono::Utc::now()),
        accepted_at: Set(Some(chrono::Utc::now())),
        ..Default::default()
    };
    let _ = member.insert(&state.db).await;

    Redirect::to("/dashboard").into_response()
}

#[derive(Deserialize)]
pub struct RenamePoolForm {
    name: String,
}

/// Rename a pool. Requires pool ownership.
pub async fn rename_pool(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(pool_pid): axum::extract::Path<String>,
    Form(form): Form<RenamePoolForm>,
) -> Response {
    let pool = pools::Entity::find()
        .filter(pools::Column::Pid.eq(&pool_pid))
        .one(&state.db)
        .await;

    let pool = match pool {
        Ok(Some(p)) if p.owner_id == user.id => p,
        _ => return Redirect::to("/dashboard").into_response(),
    };

    let name = form.name.trim().to_string();
    if !name.is_empty() && name != pool.name {
        let mut active: pools::ActiveModel = pool.into();
        active.name = Set(name);
        let _ = active.update(&state.db).await;
    }

    Redirect::to("/dashboard?saved=true").into_response()
}

#[derive(Deserialize)]
pub struct InviteForm {
    email: String,
}

/// Invite a user to a pool by email. Requires pool ownership.
pub async fn invite_member(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(pool_pid): axum::extract::Path<String>,
    Form(form): Form<InviteForm>,
) -> Response {
    // Find pool and verify ownership
    let pool = pools::Entity::find()
        .filter(pools::Column::Pid.eq(&pool_pid))
        .one(&state.db)
        .await;

    let pool = match pool {
        Ok(Some(p)) if p.owner_id == user.id => p,
        _ => return Redirect::to("/dashboard").into_response(),
    };

    // Find the invitee by email
    use crate::db::entities::users;
    let invitee = users::Entity::find()
        .filter(users::Column::Email.eq(&form.email))
        .one(&state.db)
        .await;

    let invitee = match invitee {
        Ok(Some(u)) => u,
        _ => return Redirect::to("/dashboard").into_response(),
    };

    // Check not already a member
    let existing = pool_members::Entity::find()
        .filter(pool_members::Column::PoolId.eq(pool.id))
        .filter(pool_members::Column::UserId.eq(invitee.id))
        .one(&state.db)
        .await;

    if let Ok(Some(_)) = existing {
        return Redirect::to("/dashboard").into_response();
    }

    // Create membership (pending acceptance)
    let member = pool_members::ActiveModel {
        pool_id: Set(pool.id),
        user_id: Set(invitee.id),
        role: Set("member".to_string()),
        invited_at: Set(chrono::Utc::now()),
        ..Default::default()
    };
    let _ = member.insert(&state.db).await;

    // Send invite email
    if let Some(ref mailer) = state.mailer {
        let parts = crate::email::templates::pool_invite_email(
            &user.name,
            &pool.name,
            &format!("{}/pools/{}/accept", state.base_url, pool.pid),
        );
        if let Err(e) = mailer.send(&invitee.email, &parts.subject, &parts.html, &parts.text).await {
            tracing::warn!("failed to send pool invite email: {e}");
        }
    }

    Redirect::to("/dashboard").into_response()
}

/// Accept a pool invitation.
pub async fn accept_invite(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(pool_pid): axum::extract::Path<String>,
) -> Response {
    let pool = pools::Entity::find()
        .filter(pools::Column::Pid.eq(&pool_pid))
        .one(&state.db)
        .await;

    let pool = match pool {
        Ok(Some(p)) => p,
        _ => return Redirect::to("/dashboard").into_response(),
    };

    // Find the user's pending membership
    let membership = pool_members::Entity::find()
        .filter(pool_members::Column::PoolId.eq(pool.id))
        .filter(pool_members::Column::UserId.eq(user.id))
        .one(&state.db)
        .await;

    if let Ok(Some(m)) = membership {
        if m.accepted_at.is_none() {
            let mut active: pool_members::ActiveModel = m.into();
            active.accepted_at = Set(Some(chrono::Utc::now()));
            let _ = active.update(&state.db).await;
        }
    }

    Redirect::to("/dashboard").into_response()
}

#[derive(Deserialize)]
pub struct AddHostForm {
    host_id: String,
}

/// Add an existing host to a pool. Requires pool membership and host ownership.
pub async fn add_host_to_pool(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    axum::extract::Path(pool_pid): axum::extract::Path<String>,
    Form(form): Form<AddHostForm>,
) -> Response {
    use crate::db::entities::{host_pool_memberships, hosts};

    // Verify user is pool owner or member
    let pool = pools::Entity::find()
        .filter(pools::Column::Pid.eq(&pool_pid))
        .one(&state.db)
        .await;

    let pool = match pool {
        Ok(Some(p)) => p,
        _ => return Redirect::to("/dashboard").into_response(),
    };

    let is_member = pool_members::Entity::find()
        .filter(pool_members::Column::PoolId.eq(pool.id))
        .filter(pool_members::Column::UserId.eq(user.id))
        .one(&state.db)
        .await
        .ok()
        .flatten()
        .is_some();

    if !is_member {
        return Redirect::to("/dashboard").into_response();
    }

    // Verify host ownership
    let host = hosts::Entity::find()
        .filter(hosts::Column::EndpointId.eq(&form.host_id))
        .one(&state.db)
        .await;

    match host {
        Ok(Some(h)) if h.user_id == Some(user.id) => {}
        _ => return Redirect::to("/dashboard").into_response(),
    }

    // Create pool membership
    let membership = host_pool_memberships::ActiveModel {
        host_endpoint_id: Set(form.host_id.clone()),
        pool_id: Set(pool.id),
        created_at: Set(chrono::Utc::now()),
        ..Default::default()
    };
    if let Err(e) = membership.insert(&state.db).await {
        tracing::debug!("host pool membership insert (may be duplicate): {e}");
    }

    // Update in-memory HostManager
    let all_memberships = host_pool_memberships::Entity::find()
        .filter(host_pool_memberships::Column::HostEndpointId.eq(&form.host_id))
        .all(&state.db)
        .await
        .unwrap_or_default();
    let pool_ids: Vec<i32> = all_memberships.iter().map(|m| m.pool_id).collect();
    state.hosts.update_pool_ids(&form.host_id, pool_ids).await;

    Redirect::to("/dashboard").into_response()
}
