use common::{
    helpers::read_p2p,
    protocols::{Request, Response, registry::RegistryResponse},
};
use common::helpers::write_p2p;
use iroh::protocol::AcceptError;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::db::entities::{host_pool_memberships, host_tokens, hosts};
use crate::host_manager::HostManager;

/// Protocol handler on the orchestrator that accepts incoming host connections.
#[derive(Debug, Clone)]
pub struct HostAcceptor {
    hosts: HostManager,
    db: DatabaseConnection,
}

impl HostAcceptor {
    pub fn new(hosts: HostManager, db: DatabaseConnection) -> Self {
        Self { hosts, db }
    }

    /// Validate a setup token and establish user ownership of the host.
    /// Returns the user_id if valid, None if invalid/expired/used.
    async fn validate_token(
        &self,
        token: &str,
        host_id: &str,
    ) -> Option<i32> {
        let token_hash = crate::auth::hash_key(token);

        let record = host_tokens::Entity::find()
            .filter(host_tokens::Column::TokenHash.eq(&token_hash))
            .one(&self.db)
            .await
            .ok()
            .flatten()?;

        if record.used_at.is_some() {
            tracing::warn!("setup token already used for host {host_id}");
            return None;
        }

        if chrono::Utc::now() > record.expires_at {
            tracing::warn!("setup token expired for host {host_id}");
            return None;
        }

        // Mark token as used
        let mut active: host_tokens::ActiveModel = record.clone().into();
        active.used_at = Set(Some(chrono::Utc::now()));
        active.host_id = Set(Some(host_id.to_string()));
        if let Err(e) = active.update(&self.db).await {
            tracing::error!("failed to mark token as used: {e}");
            return None;
        }

        tracing::info!("host {host_id} claimed by user {}", record.user_id);
        Some(record.user_id)
    }

    /// Restore pool memberships from DB for a reconnecting host.
    /// Only ACTIVE memberships count. Soft-deleted rows (is_active=false) must be
    /// ignored, otherwise removed pools leak back into available_models() responses.
    async fn restore_pools(&self, host_id: &str) -> (Vec<i32>, Option<i32>) {
        let memberships = host_pool_memberships::Entity::find()
            .filter(host_pool_memberships::Column::HostEndpointId.eq(host_id))
            .filter(host_pool_memberships::Column::IsActive.eq(true))
            .all(&self.db)
            .await
            .unwrap_or_default();

        let pool_ids: Vec<i32> = memberships.iter().map(|m| m.pool_id).collect();

        let user_id = hosts::Entity::find()
            .filter(hosts::Column::EndpointId.eq(host_id))
            .one(&self.db)
            .await
            .ok()
            .flatten()
            .and_then(|h| h.user_id);

        if !pool_ids.is_empty() {
            tracing::info!("host {host_id} restored {} pool memberships", pool_ids.len());
        }

        (pool_ids, user_id)
    }

    /// Upsert the host record in the DB. Uses host_id as the stable identity in endpoint_id column.
    async fn upsert_host(&self, host_id: &str, user_id: Option<i32>) {
        let existing = hosts::Entity::find()
            .filter(hosts::Column::EndpointId.eq(host_id))
            .one(&self.db)
            .await
            .ok()
            .flatten();

        match existing {
            Some(host) => {
                let mut active: hosts::ActiveModel = host.into();
                active.status = Set("connected".to_string());
                active.last_seen = Set(Some(chrono::Utc::now()));
                if let Some(uid) = user_id {
                    active.user_id = Set(Some(uid));
                }
                let _ = active.update(&self.db).await;
            }
            None => {
                let host = hosts::ActiveModel {
                    endpoint_id: Set(host_id.to_string()),
                    status: Set("connected".to_string()),
                    last_seen: Set(Some(chrono::Utc::now())),
                    user_id: Set(user_id),
                    ..Default::default()
                };
                let _ = host.insert(&self.db).await;
            }
        }
    }
}

impl iroh::protocol::ProtocolHandler for HostAcceptor {
    fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let hosts = self.hosts.clone();
        let acceptor = self.clone();
        Box::pin(async move {
            let endpoint_id = connection.remote_id().to_string();
            tracing::info!("host connecting: endpoint_id={endpoint_id}");

            // Read the first stream, expect a Registry request
            let (send, recv) = connection.accept_bi().await?;

            let request: Request = read_p2p(recv)
                .await
                .map_err(|e| std::io::Error::other(format!("failed to read registry: {e}")))?;

            let (capabilities, host_id, setup_token) = match request {
                Request::Registry(reg_req) => {
                    match reg_req {
                        common::protocols::registry::RegistryRequest::Register { capabilities, host_id, setup_token } => {
                            tracing::info!(
                                "host {host_id} (endpoint={endpoint_id}) registered with {} models",
                                capabilities.models.len()
                            );
                            (capabilities, host_id, setup_token)
                        }
                        _ => {
                            return Err(std::io::Error::other(
                                "expected Register as first message, got Heartbeat",
                            ).into());
                        }
                    }
                }
                _ => {
                    return Err(std::io::Error::other(
                        "expected Registry request as first message",
                    ).into());
                }
            };

            // Reject deregistered hosts. is_active=false means an owner removed
            // this host via the dashboard; they must reinstall to register fresh.
            let existing_host = hosts::Entity::find()
                .filter(hosts::Column::EndpointId.eq(&host_id))
                .one(&acceptor.db)
                .await
                .ok()
                .flatten();

            if let Some(ref h) = existing_host {
                if !h.is_active {
                    tracing::warn!("rejecting connection from deregistered host {host_id}");
                    return Err(std::io::Error::other(
                        "host is deregistered; reinstall to register a new host",
                    )
                    .into());
                }
            }

            // Determine user ownership and pool memberships
            let user_id = if let Some(token) = setup_token {
                // New host with setup token, establishes user ownership
                match acceptor.validate_token(&token, &host_id).await {
                    Some(uid) => Some(uid),
                    None => {
                        tracing::warn!("invalid setup token for {host_id}, accepting as unowned");
                        None
                    }
                }
            } else {
                // Reconnecting host, reuse the lookup above
                existing_host.and_then(|h| h.user_id)
            };

            // Restore pool memberships from DB (works for both new and reconnecting hosts)
            let (pool_ids, _) = acceptor.restore_pools(&host_id).await;

            // Upsert host record using host_id as stable identity
            acceptor.upsert_host(&host_id, user_id).await;

            // Send ack
            let response = Response::Registry(RegistryResponse::Ack);
            write_p2p(send, response)
                .await
                .map_err(|e| std::io::Error::other(format!("failed to send ack: {e}")))?;

            hosts.register(host_id.clone(), endpoint_id.clone(), capabilities, connection.clone(), pool_ids.clone(), user_id).await;

            // Handle subsequent streams (heartbeats + re-registration)
            loop {
                match connection.accept_bi().await {
                    Ok((send, recv)) => {
                        let request: Request = match read_p2p(recv).await {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::warn!("host {host_id} stream read error: {e}");
                                break;
                            }
                        };

                        match request {
                            Request::Registry(common::protocols::registry::RegistryRequest::Heartbeat) => {
                                tracing::debug!("heartbeat from {host_id}");
                                let resp = Response::Registry(RegistryResponse::Ack);
                                if let Err(e) = write_p2p(send, resp).await {
                                    tracing::warn!("heartbeat ack failed for {host_id}: {e}");
                                    break;
                                }
                            }
                            Request::Registry(common::protocols::registry::RegistryRequest::Register { capabilities, .. }) => {
                                tracing::info!("host {host_id} re-registered with {} models", capabilities.models.len());
                                let (pool_ids, user_id) = acceptor.restore_pools(&host_id).await;
                                hosts.register(host_id.clone(), endpoint_id.clone(), capabilities, connection.clone(), pool_ids, user_id).await;
                                let resp = Response::Registry(RegistryResponse::Ack);
                                if let Err(e) = write_p2p(send, resp).await {
                                    tracing::warn!("re-register ack failed for {host_id}: {e}");
                                    break;
                                }
                            }
                            _ => {
                                tracing::warn!("unexpected request from host {host_id}");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::info!("host {host_id} connection closed: {e}");
                        break;
                    }
                }
            }

            if hosts.unregister_if_current(&host_id, connection.stable_id()).await {
                if let Ok(Some(host)) = hosts::Entity::find()
                    .filter(hosts::Column::EndpointId.eq(&host_id))
                    .one(&acceptor.db)
                    .await
                {
                    let mut active: hosts::ActiveModel = host.into();
                    active.status = Set("disconnected".to_string());
                    active.last_seen = Set(Some(chrono::Utc::now()));
                    let _ = active.update(&acceptor.db).await;
                }
                tracing::info!("host disconnected: {host_id}");
            } else {
                tracing::info!("stale connection for {host_id} closed; newer connection still live");
            }

            Ok(())
        })
    }
}
