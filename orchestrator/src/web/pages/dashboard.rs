use axum::{extract::{Query, State}, response::Html};
use leptos::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use crate::auth::CurrentUser;
use crate::db::entities::{api_keys, host_pool_memberships, hosts, pool_members, pools, users};
use crate::web::components::*;

#[derive(Deserialize)]
pub struct DashboardQuery {
    pub saved: Option<bool>,
}

#[component]
fn Dashboard(
    user_name: String,
    owned_hosts: Vec<OwnedHostView>,
    user_pools: Vec<PoolView>,
    addable_hosts: Vec<(String, String)>,
    saved: bool,
) -> impl IntoView {
    view! {
        <Base title="cocompute — dashboard">
            <PageShell>
                <div class="max-w-5xl mx-auto px-6 py-10">
                    // Toast notification
                    {saved.then(|| view! {
                        <div
                            class="fixed top-4 right-4 rounded-lg bg-emerald-500/10 border border-emerald-500/20 px-4 py-2.5 text-emerald-400 text-sm z-50"
                            style="animation:fadeout 0.3s ease 2s forwards"
                        >
                            "Saved"
                        </div>
                        <style>"@keyframes fadeout{to{opacity:0;visibility:hidden}}"</style>
                    })}

                    // Header
                    <div class="flex items-center justify-between mb-8">
                        <div>
                            <h1 class="text-white text-2xl font-bold">"Dashboard"</h1>
                            <p class="text-[#71717A] text-sm mt-1">{format!("Welcome, {user_name}")}</p>
                        </div>
                        <div class="flex gap-3">
                            <form method="POST" action="/host-token">
                                <button type="submit" class="rounded-lg bg-emerald-600 px-4 py-2 text-white text-sm font-semibold hover:bg-emerald-700 transition cursor-pointer">
                                    "Add Host"
                                </button>
                            </form>
                            <form method="POST" action="/pools">
                                <button type="submit" class="rounded-lg bg-indigo-500 px-4 py-2 text-white text-sm font-semibold hover:bg-indigo-600 transition cursor-pointer">
                                    "New Pool"
                                </button>
                            </form>
                            <form method="POST" action="/logout">
                                <button type="submit" class="rounded-lg bg-[#27272A] px-4 py-2 text-[#A1A1AA] text-sm font-medium hover:text-white transition cursor-pointer">
                                    "Sign out"
                                </button>
                            </form>
                        </div>
                    </div>

                    // My Hosts
                    {if !owned_hosts.is_empty() {
                        view! {
                            <div class="rounded-xl bg-[#16161E] border border-[#27272A] p-6 mb-6">
                                <h2 class="text-white text-lg font-bold mb-4">"My Hosts"</h2>
                                <div class="flex flex-col gap-2">
                                    {owned_hosts.into_iter().map(|host| {
                                        let status_dot = if host.online { "bg-emerald-400" } else { "bg-[#52525B]" };
                                        let status_text = if host.online { "online" } else { "offline" };
                                        let status_color = if host.online { "text-emerald-400" } else { "text-[#52525B]" };
                                        let pools_text = if host.pool_names.is_empty() {
                                            "no pools".to_string()
                                        } else {
                                            host.pool_names.join(", ")
                                        };
                                        let id_short = format!("{}...{}", &host.id_prefix, &host.id_suffix);
                                        let display_name = match host.name.clone() {
                                            Some(name) => format!("{} ({})", name, id_short),
                                            None => id_short,
                                        };
                                        let host_id = host.host_id.clone();
                                        view! {
                                            <div class="flex items-center justify-between bg-[#111118] rounded-lg px-4 py-3 gap-3">
                                                <div class="flex items-center gap-3 min-w-0">
                                                    <span class={format!("w-2 h-2 rounded-full shrink-0 {status_dot}")}></span>
                                                    <form method="POST" action={format!("/hosts/{}/rename", host_id)} class="inline min-w-0">
                                                        <span
                                                            contenteditable="true"
                                                            data-original={display_name.clone()}
                                                            class="text-[#A1A1AA] text-sm font-mono outline-none cursor-text rounded px-1 -mx-1 hover:bg-[#1e1e2e] focus:bg-[#1e1e2e] focus:ring-1 focus:ring-indigo-500 block truncate max-w-[240px]"
                                                            onblur="var v=this.textContent.trim();if(v!==this.dataset.original){this.parentElement.querySelector('input[name=name]').value=v;this.parentElement.submit()}"
                                                            onkeydown="if(event.key==='Enter'){event.preventDefault();this.blur()}"
                                                        >{display_name.clone()}</span>
                                                        <input type="hidden" name="name" value=""/>
                                                    </form>
                                                    <span class={format!("text-xs shrink-0 {status_color}")}>{status_text}</span>
                                                </div>
                                                <div class="flex items-center gap-3 shrink-0">
                                                    <span class="text-[#52525B] text-xs">{pools_text}</span>
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}

                    // Pools
                    {if user_pools.is_empty() {
                        view! {
                            <div class="rounded-xl bg-[#16161E] border border-[#27272A] p-12 text-center">
                                <p class="text-[#71717A] text-sm">"No pools yet. Create one to get started."</p>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="flex flex-col gap-6">
                                {user_pools.into_iter().map(|pool| view! {
                                    <div id={format!("pool-{}", pool.pid)} class="rounded-xl bg-[#16161E] border border-[#27272A] p-6">
                                        <div class="flex items-center justify-between mb-4">
                                            <div>
                                                <form method="POST" action={format!("/pools/{}/rename", pool.pid)} class="inline">
                                                    <h2
                                                        contenteditable="true"
                                                        data-original={pool.name.clone()}
                                                        class="text-white text-lg font-bold outline-none cursor-text rounded px-1 -mx-1 hover:bg-[#1e1e2e] focus:bg-[#1e1e2e] focus:ring-1 focus:ring-indigo-500"
                                                        onblur="var v=this.textContent.trim();if(v!==this.dataset.original){this.parentElement.querySelector('input[name=name]').value=v;this.parentElement.submit()}"
                                                        onkeydown="if(event.key==='Enter'){event.preventDefault();this.blur()}"
                                                    >{pool.name.clone()}</h2>
                                                    <input type="hidden" name="name" value={pool.name.clone()}/>
                                                </form>
                                                <p class="text-[#52525B] text-xs mt-0.5">{format!("{} hosts · {} keys", pool.hosts.len(), pool.key_count)}</p>
                                            </div>
                                            <div class="flex items-center gap-2">
                                                <form method="POST" action={format!("/pools/{}/api-keys", pool.pid)}>
                                                    <input type="hidden" name="label" value=""/>
                                                    <button type="submit" class="rounded-lg bg-[#27272A] px-3 py-1.5 text-[#A1A1AA] text-xs font-medium hover:text-white transition cursor-pointer">
                                                        "New API Key"
                                                    </button>
                                                </form>
                                                {if pool.is_owner {
                                                    let deact_pid = pool.pid.clone();
                                                    view! {
                                                        <form method="POST" action={format!("/pools/{}/deactivate", deact_pid)}>
                                                            <button type="submit" class="rounded-lg bg-[#27272A] px-3 py-1.5 text-[#52525B] text-xs font-medium hover:text-red-400 transition cursor-pointer" onclick="return confirm('Deactivate this pool?')">
                                                                "Deactivate"
                                                            </button>
                                                        </form>
                                                    }.into_any()
                                                } else {
                                                    view! { <span></span> }.into_any()
                                                }}
                                            </div>
                                        </div>

                                        // Hosts in this pool
                                        {if !pool.hosts.is_empty() {
                                            let hosts_pool_pid = pool.pid.clone();
                                            view! {
                                                <div class="mb-4">
                                                    <h3 class="text-[#A1A1AA] text-xs font-medium mb-2">"Hosts"</h3>
                                                    <div class="flex flex-col gap-1">
                                                        {pool.hosts.into_iter().map(|host| {
                                                            let status_color = if host.online { "text-emerald-400" } else { "text-[#52525B]" };
                                                            let status_dot = if host.online { "bg-emerald-400" } else { "bg-[#52525B]" };
                                                            let remove_action = format!("/pools/{}/remove-host/{}", hosts_pool_pid, host.host_id);
                                                            let id_short = format!("{}...{}", &host.id_prefix, &host.id_suffix);
                                                            let host_display = match &host.name {
                                                                Some(name) => format!("{} ({})", name, id_short),
                                                                None => id_short,
                                                            };
                                                            view! {
                                                                <div class="flex items-center justify-between bg-[#111118] rounded-lg px-3 py-2">
                                                                    <div class="flex items-center gap-2">
                                                                        <span class={format!("w-2 h-2 rounded-full {status_dot}")}></span>
                                                                        <span class="text-[#A1A1AA] text-xs font-mono">{host_display}</span>
                                                                    </div>
                                                                    <div class="flex items-center gap-3">
                                                                        <span class="text-[#52525B] text-xs">{host.owner_name}</span>
                                                                        <span class={format!("text-xs {status_color}")}>{if host.online { "online" } else { "offline" }}</span>
                                                                        <form method="POST" action={remove_action}>
                                                                            <button type="submit" class="text-[#3F3F46] text-xs hover:text-red-400 transition cursor-pointer" onclick="return confirm('Remove host from pool?')">
                                                                                "remove"
                                                                            </button>
                                                                        </form>
                                                                    </div>
                                                                </div>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <p class="text-[#3F3F46] text-xs mb-4">"No hosts yet."</p>
                                            }.into_any()
                                        }}

                                        // Add host to pool
                                        {
                                            let pool_pid = pool.pid.clone();
                                            let hosts_for_select = addable_hosts.clone();
                                            if !hosts_for_select.is_empty() {
                                                view! {
                                                    <form method="POST" class="flex items-center gap-2 mb-4">
                                                        <select name="host_id" class="h-8 rounded-lg bg-[#111118] border border-[#27272A] px-2 text-[#A1A1AA] text-xs outline-none flex-1">
                                                            {hosts_for_select.into_iter().map(|(hid, label)| view! {
                                                                <option value={hid}>{label}</option>
                                                            }).collect::<Vec<_>>()}
                                                        </select>
                                                        <button
                                                            type="submit"
                                                            formaction={format!("/pools/{pool_pid}/add-host")}
                                                            class="rounded-lg bg-[#27272A] px-3 py-1.5 text-[#A1A1AA] text-xs font-medium hover:text-white transition cursor-pointer"
                                                        >
                                                            "Add to pool"
                                                        </button>
                                                    </form>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <p class="text-[#3F3F46] text-xs mb-4 italic">
                                                        "No hosts available to add. Register a host first."
                                                    </p>
                                                }.into_any()
                                            }
                                        }

                                        // Members
                                        {if !pool.members.is_empty() {
                                            view! {
                                                <div class="mb-4">
                                                    <h3 class="text-[#A1A1AA] text-xs font-medium mb-2">"Members"</h3>
                                                    <div class="flex flex-col gap-1">
                                                        {pool.members.into_iter().map(|m| {
                                                            let status = if !m.accepted { " (pending)" } else { "" };
                                                            let color = if m.accepted { "text-[#A1A1AA]" } else { "text-[#52525B]" };
                                                            view! {
                                                                <div class="flex items-center justify-between bg-[#111118] rounded-lg px-3 py-2">
                                                                    <span class={format!("text-xs {color}")}>{format!("{}{}", m.name, status)}</span>
                                                                    <span class="text-[#3F3F46] text-xs">{m.role}</span>
                                                                </div>
                                                            }
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! { <div></div> }.into_any()
                                        }}

                                        // Invite member (owner only)
                                        {if pool.is_owner {
                                            let invite_pid = pool.pid.clone();
                                            view! {
                                                <form method="POST" action={format!("/pools/{invite_pid}/invite")} class="flex items-center gap-2 mb-4">
                                                    <input type="email" name="email" required=true placeholder="user@example.com" class="h-8 rounded-lg bg-[#111118] border border-[#27272A] px-3 text-[#A1A1AA] text-xs outline-none focus:border-indigo-500 transition flex-1 placeholder-[#3F3F46]"/>
                                                    <button type="submit" class="rounded-lg bg-[#27272A] px-3 py-1.5 text-[#A1A1AA] text-xs font-medium hover:text-white transition cursor-pointer">
                                                        "Invite"
                                                    </button>
                                                </form>
                                            }.into_any()
                                        } else {
                                            view! { <div></div> }.into_any()
                                        }}

                                        // API Keys for this pool
                                        {if pool.api_keys.is_empty() {
                                            view! {
                                                <p class="text-[#3F3F46] text-xs">"No API keys yet."</p>
                                            }.into_any()
                                        } else {
                                            view! {
                                                <div>
                                                    <h3 class="text-[#A1A1AA] text-xs font-medium mb-2">"API Keys"</h3>
                                                    <div class="flex flex-col gap-1">
                                                        {pool.api_keys.into_iter().map(|key| {
                                                            let hash_short = format!("{}...{}", &key.hash_prefix, &key.hash_suffix);
                                                            let label_display = key.label.clone().unwrap_or_else(|| "unnamed".to_string());
                                                            let key_display = match &key.label {
                                                                Some(_) => format!(" ({})", hash_short),
                                                                None => format!(" {}", hash_short),
                                                            };
                                                            view! {
                                                            <div class="flex items-center justify-between bg-[#111118] rounded-lg px-3 py-2 gap-3">
                                                                <div class="flex items-center gap-0 min-w-0">
                                                                    <form method="POST" action={format!("/api-keys/{}/rename", key.id)} class="inline min-w-0">
                                                                        <span
                                                                            contenteditable="true"
                                                                            data-original={label_display.clone()}
                                                                            class="text-[#A1A1AA] text-xs outline-none cursor-text rounded px-1 hover:bg-[#1e1e2e] focus:bg-[#1e1e2e] focus:ring-1 focus:ring-indigo-500 block truncate max-w-[200px]"
                                                                            onblur="var v=this.textContent.trim();if(v!==this.dataset.original){this.parentElement.querySelector('input[name=label]').value=v;this.parentElement.submit()}"
                                                                            onkeydown="if(event.key==='Enter'){event.preventDefault();this.blur()}"
                                                                        >{label_display.clone()}</span>
                                                                        <input type="hidden" name="label" value=""/>
                                                                    </form>
                                                                    <span class="text-[#52525B] text-xs font-mono shrink-0">{key_display}</span>
                                                                </div>
                                                                <form method="POST" action={format!("/api-keys/{}/deactivate", key.id)}>
                                                                    <button type="submit" class="text-[#52525B] text-xs hover:text-red-400 transition cursor-pointer" onclick="return confirm('Deactivate this API key?')">
                                                                        "revoke"
                                                                    </button>
                                                                </form>
                                                            </div>
                                                        }}).collect::<Vec<_>>()}
                                                    </div>
                                                </div>
                                            }.into_any()
                                        }}
                                    </div>
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }}
                </div>
            </PageShell>
        </Base>
    }
}

#[derive(Clone)]
struct PoolView {
    pid: String,
    name: String,
    is_owner: bool,
    hosts: Vec<HostView>,
    key_count: usize,
    api_keys: Vec<ApiKeyView>,
    members: Vec<MemberView>,
}

#[derive(Clone)]
struct MemberView {
    name: String,
    role: String,
    accepted: bool,
}

#[derive(Clone)]
struct HostView {
    host_id: String,
    id_prefix: String,
    id_suffix: String,
    name: Option<String>,
    owner_name: String,
    online: bool,
}

#[derive(Clone)]
struct OwnedHostView {
    host_id: String,
    id_prefix: String,
    id_suffix: String,
    name: Option<String>,
    online: bool,
    pool_names: Vec<String>,
}

#[derive(Clone)]
struct ApiKeyView {
    id: i32,
    hash_prefix: String,
    hash_suffix: String,
    label: Option<String>,
}

/// GET /dashboard — authenticated dashboard showing user's pools, hosts, and keys.
pub async fn dashboard(
    State(state): State<crate::AppState>,
    CurrentUser(user): CurrentUser,
    Query(params): Query<DashboardQuery>,
) -> Html<String> {
    // Get set of currently connected hosts for liveness check
    let connected = state.hosts.connected_ids().await;

    // Get all hosts owned by this user
    let owned_host_records = hosts::Entity::find()
        .filter(hosts::Column::UserId.eq(Some(user.id)))
        .all(&state.db)
        .await
        .unwrap_or_default();

    // Pre-load lookup maps to avoid N+1 queries
    let all_pools = pools::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();
    let pool_name_map: std::collections::HashMap<i32, String> = all_pools
        .iter()
        .map(|p| (p.id, p.name.clone()))
        .collect();

    let all_users = users::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();
    let user_name_map: std::collections::HashMap<i32, String> = all_users
        .iter()
        .map(|u| (u.id, u.name.clone()))
        .collect();

    let all_hosts = hosts::Entity::find()
        .all(&state.db)
        .await
        .unwrap_or_default();
    let host_owner_map: std::collections::HashMap<String, Option<i32>> = all_hosts
        .iter()
        .map(|h| (h.endpoint_id.clone(), h.user_id))
        .collect();
    let host_name_map: std::collections::HashMap<String, Option<String>> = all_hosts
        .iter()
        .map(|h| (h.endpoint_id.clone(), h.name.clone()))
        .collect();

    let mut owned_hosts = Vec::new();
    for host in &owned_host_records {
        let hid = &host.endpoint_id; // This stores host_id now

        // Get pool memberships for this host (only active; soft-deleted memberships
        // must not appear in the host's "pool_names" display).
        let memberships = host_pool_memberships::Entity::find()
            .filter(host_pool_memberships::Column::HostEndpointId.eq(hid))
            .filter(host_pool_memberships::Column::IsActive.eq(true))
            .all(&state.db)
            .await
            .unwrap_or_default();

        let pool_names: Vec<String> = memberships
            .iter()
            .filter_map(|m| pool_name_map.get(&m.pool_id).cloned())
            .collect();

        owned_hosts.push(OwnedHostView {
            host_id: hid.clone(),
            id_prefix: hid.chars().take(8).collect(),
            id_suffix: hid.chars().rev().take(4).collect::<String>().chars().rev().collect(),
            name: host.name.clone(),
            online: connected.contains(hid),
            pool_names,
        });
    }

    // Get pools the user owns or is a member of
    let memberships = pool_members::Entity::find()
        .filter(pool_members::Column::UserId.eq(user.id))
        .all(&state.db)
        .await
        .unwrap_or_default();

    let pool_ids: Vec<i32> = memberships.iter().map(|m| m.pool_id).collect();

    let mut user_pools = Vec::new();
    for pool_id in &pool_ids {
        let pool = pools::Entity::find_by_id(*pool_id)
            .one(&state.db)
            .await
            .ok()
            .flatten();

        let pool = match pool {
            Some(p) if p.is_active => p,
            _ => continue,
        };

        // Get hosts in this pool with their owners (active memberships only)
        let pool_host_memberships = host_pool_memberships::Entity::find()
            .filter(host_pool_memberships::Column::PoolId.eq(*pool_id))
            .filter(host_pool_memberships::Column::IsActive.eq(true))
            .all(&state.db)
            .await
            .unwrap_or_default();

        let host_views: Vec<HostView> = pool_host_memberships
            .iter()
            .map(|membership| {
                let host_id = &membership.host_endpoint_id;
                let owner_name = host_owner_map
                    .get(host_id)
                    .and_then(|uid| uid.and_then(|id| user_name_map.get(&id).cloned()))
                    .unwrap_or_else(|| "unowned".to_string());

                HostView {
                    host_id: host_id.clone(),
                    id_prefix: host_id.chars().take(8).collect(),
                    id_suffix: host_id.chars().rev().take(4).collect::<String>().chars().rev().collect(),
                    name: host_name_map.get(host_id).cloned().flatten(),
                    owner_name,
                    online: connected.contains(host_id),
                }
            })
            .collect();

        // Get API keys for this pool (active only)
        let keys = api_keys::Entity::find()
            .filter(api_keys::Column::PoolId.eq(Some(*pool_id)))
            .filter(api_keys::Column::IsActive.eq(true))
            .all(&state.db)
            .await
            .unwrap_or_default();

        let api_key_views: Vec<ApiKeyView> = keys
            .iter()
            .map(|k| {
                let hash = &k.key_hash;
                ApiKeyView {
                    id: k.id,
                    hash_prefix: hash.chars().take(8).collect(),
                    hash_suffix: hash.chars().rev().take(4).collect::<String>().chars().rev().collect(),
                    label: k.label.clone(),
                }
            })
            .collect();

        // Get pool members
        let members = pool_members::Entity::find()
            .filter(pool_members::Column::PoolId.eq(*pool_id))
            .all(&state.db)
            .await
            .unwrap_or_default();

        let member_views: Vec<MemberView> = members
            .iter()
            .map(|m| {
                let name = user_name_map
                    .get(&m.user_id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());
                MemberView {
                    name,
                    role: m.role.clone(),
                    accepted: m.accepted_at.is_some(),
                }
            })
            .collect();

        user_pools.push(PoolView {
            pid: pool.pid.clone(),
            name: pool.name.clone(),
            is_owner: pool.owner_id == user.id,
            hosts: host_views,
            key_count: keys.len(),
            api_keys: api_key_views,
            members: member_views,
        });
    }

    // Build addable hosts list for the "add to pool" dropdown
    let addable_hosts: Vec<(String, String)> = owned_host_records
        .iter()
        .map(|h| {
            let hid = &h.endpoint_id;
            let label = h.name.clone().unwrap_or_else(|| {
                format!("{}...{}", &hid[..8.min(hid.len())], &hid[hid.len().saturating_sub(4)..])
            });
            (hid.clone(), label)
        })
        .collect();

    crate::web::render(Dashboard(DashboardProps {
        user_name: user.name.clone(),
        owned_hosts,
        user_pools,
        addable_hosts,
        saved: params.saved.unwrap_or(false),
    }))
}
