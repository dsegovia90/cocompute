//! Self-signup flow shared between POST /beta and the InviteUser CLI subcommand.
//!
//! Creates a `users` row with a verification token + an analytics-friendly
//! `beta_invites` row, returning both the new user and the verification token
//! the caller should email out.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};

use crate::auth;
use crate::db::entities::{beta_invites, users};

/// Form input for a self-signup. Mirrors the fields on the /beta page.
pub struct SignupInput {
    pub name: String,
    pub email: String,
    pub role: String,
    pub gpu: Option<String>,
}

/// Successful signup result. Caller is responsible for sending the verification
/// email using `verification_token` (typically via `email::templates::invite_email`).
pub struct SignupResult {
    pub user: users::Model,
    pub verification_token: String,
}

#[derive(Debug)]
pub enum SignupError {
    /// A user with this email already exists. Caller should redirect to /login
    /// or show a "you've already signed up" message.
    UserAlreadyExists,
    /// Database error during signup. Caller should treat as a generic 500.
    Db(DbErr),
    /// Password hashing failed (extremely unlikely in practice).
    Hash(anyhow::Error),
}

impl From<DbErr> for SignupError {
    fn from(e: DbErr) -> Self {
        SignupError::Db(e)
    }
}

/// Create a user account from signup form data. Atomic-ish: checks for existing
/// user first, then upserts the beta_invite record (analytics), then creates the
/// user with a verification token.
///
/// The user's password is set to a hashed throwaway value; they choose a real
/// password by clicking the verification link, which routes to /verify and then
/// /reset to set their password.
pub async fn create_user_and_invite(
    db: &DatabaseConnection,
    input: SignupInput,
) -> Result<SignupResult, SignupError> {
    // 1. Bail early if the user already exists. This is the "already signed up,
    //    please log in" path.
    let existing_user = users::Entity::find()
        .filter(users::Column::Email.eq(&input.email))
        .one(db)
        .await?;
    if existing_user.is_some() {
        return Err(SignupError::UserAlreadyExists);
    }

    // 2. Upsert the beta_invite row. We keep this for signup-attribution
    //    analytics (which role + gpu_info each new user reported). If a row
    //    already exists for this email (e.g., legacy waitlist signup that
    //    never got promoted), update it in place rather than failing on the
    //    unique-email constraint.
    let existing_invite = beta_invites::Entity::find()
        .filter(beta_invites::Column::Email.eq(&input.email))
        .one(db)
        .await?;

    match existing_invite {
        Some(row) => {
            let mut active: beta_invites::ActiveModel = row.into();
            active.name = Set(input.name.clone());
            active.role = Set(input.role.clone());
            active.gpu_info = Set(input.gpu.clone().filter(|g| !g.is_empty()));
            active.update(db).await?;
        }
        None => {
            let invite = beta_invites::ActiveModel {
                name: Set(input.name.clone()),
                email: Set(input.email.clone()),
                role: Set(input.role.clone()),
                gpu_info: Set(input.gpu.clone().filter(|g| !g.is_empty())),
                created_at: Set(chrono::Utc::now()),
                ..Default::default()
            };
            invite.insert(db).await?;
        }
    }

    // 3. Create the user with a throwaway password and a verification token.
    //    The user can't log in until they click the verification link and set
    //    a real password (handled by /verify -> /reset flow).
    let pid = uuid::Uuid::new_v4().to_string();
    let token = auth::generate_api_key();
    let throwaway_password = auth::hash_password(auth::generate_api_key())
        .await
        .map_err(SignupError::Hash)?;

    let user = users::ActiveModel {
        pid: Set(pid),
        email: Set(input.email.clone()),
        password_hash: Set(throwaway_password),
        name: Set(input.name.clone()),
        email_verification_token: Set(Some(token.clone())),
        email_verification_sent_at: Set(Some(chrono::Utc::now())),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
        ..Default::default()
    };
    let user = user.insert(db).await?;

    Ok(SignupResult {
        user,
        verification_token: token,
    })
}
