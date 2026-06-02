use axum::{extract::Query, response::Html};
use leptos::prelude::*;
use serde::Deserialize;
use crate::web::components::*;

#[derive(Deserialize)]
pub struct ResetQuery {
    pub token: String,
    pub error: Option<String>,
}

#[component]
fn ResetPage(token: String, error: Option<String>) -> impl IntoView {
    view! {
        <Base title="cocompute · reset password">
            <PageShell>
                <div class="flex items-center justify-center min-h-screen">
                    <form method="POST" action="/reset" class="w-[400px] rounded-xl bg-[#16161E] border border-[#27272A] px-10 pt-12 pb-10 flex flex-col gap-7">
                        <div class="flex flex-col gap-2">
                            <h1 class="text-white text-2xl font-bold">"cocompute"</h1>
                            <p class="text-[#71717A] text-sm">"Choose a new password"</p>
                        </div>

                        {error.map(|msg| view! {
                            <div class="rounded-lg bg-red-500/10 border border-red-500/20 px-4 py-3 text-red-400 text-sm">{msg}</div>
                        })}

                        <input type="hidden" name="token" value={token}/>
                        <TextInput label="New Password" r#type="password" name="password" placeholder="Choose a strong password"/>

                        <button
                            type="submit"
                            class="h-11 rounded-lg bg-indigo-500 text-white text-sm font-semibold hover:bg-indigo-600 transition cursor-pointer"
                        >
                            "Reset Password"
                        </button>
                    </form>
                </div>
            </PageShell>
        </Base>
    }
}

#[component]
fn ResetExpired() -> impl IntoView {
    view! {
        <Base title="cocompute · link expired">
            <PageShell>
                <div class="flex items-center justify-center min-h-screen">
                    <div class="w-[400px] rounded-xl bg-[#16161E] border border-[#27272A] px-10 pt-12 pb-10 flex flex-col gap-5 items-center text-center">
                        <h1 class="text-white text-2xl font-bold">"Link expired"</h1>
                        <p class="text-[#71717A] text-sm">"This reset link has expired or is invalid."</p>
                        <a href="/forgot" class="text-indigo-500 text-sm font-medium hover:underline">"Request a new one"</a>
                    </div>
                </div>
            </PageShell>
        </Base>
    }
}

pub async fn reset_page(
    Query(params): Query<ResetQuery>,
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Html<String> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use crate::db::entities::users;

    let user = users::Entity::find()
        .filter(users::Column::ResetToken.eq(&params.token))
        .one(&state.db)
        .await
        .ok()
        .flatten();

    match user {
        Some(u) => {
            let expired = u.reset_sent_at
                .map(|sent| chrono::Utc::now() - sent > chrono::Duration::hours(1))
                .unwrap_or(true);

            if expired {
                crate::web::render(ResetExpired())
            } else {
                crate::web::render(ResetPage(ResetPageProps { token: params.token, error: params.error }))
            }
        }
        None => crate::web::render(ResetExpired()),
    }
}
