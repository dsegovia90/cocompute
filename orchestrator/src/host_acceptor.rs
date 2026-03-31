use common::{
    helpers::read_p2p,
    protocols::{Request, Response, registry::RegistryResponse},
};
use common::helpers::write_p2p;
use iroh::protocol::AcceptError;

use crate::host_manager::HostManager;

/// Protocol handler on the orchestrator that accepts incoming host connections.
/// When a host connects:
/// 1. Read the first stream — expect a Registry request with capabilities
/// 2. Store the connection + capabilities in the HostManager
/// 3. Monitor the connection — when it closes, unregister the host
#[derive(Debug, Clone)]
pub struct HostAcceptor {
    hosts: HostManager,
}

impl HostAcceptor {
    pub fn new(hosts: HostManager) -> Self {
        Self { hosts }
    }
}

impl iroh::protocol::ProtocolHandler for HostAcceptor {
    fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send {
        let hosts = self.hosts.clone();
        Box::pin(async move {
            let endpoint_id = connection.remote_id().to_string();
            tracing::info!("host connecting: {endpoint_id}");

            // Read the first stream — expect a Registry request
            let (send, recv) = connection.accept_bi().await?;

            let request: Request = read_p2p(recv)
                .await
                .map_err(|e| std::io::Error::other(format!("failed to read registry: {e}")))?;

            let capabilities = match request {
                Request::Registry(reg_req) => {
                    match reg_req {
                        common::protocols::registry::RegistryRequest::Register(caps) => {
                            tracing::info!(
                                "host {endpoint_id} registered with {} models",
                                caps.models.len()
                            );
                            caps
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

            // Send ack
            let response = Response::Registry(RegistryResponse::Ack);
            write_p2p(send, response)
                .await
                .map_err(|e| std::io::Error::other(format!("failed to send ack: {e}")))?;

            // Store the connection in the HostManager
            hosts.register(endpoint_id.clone(), capabilities, connection.clone()).await;

            // Handle subsequent streams from the host (heartbeats)
            loop {
                match connection.accept_bi().await {
                    Ok((send, recv)) => {
                        let request: Request = match read_p2p(recv).await {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::warn!("host {endpoint_id} stream read error: {e}");
                                break;
                            }
                        };

                        match request {
                            Request::Registry(common::protocols::registry::RegistryRequest::Heartbeat) => {
                                tracing::debug!("heartbeat from {endpoint_id}");
                                let resp = Response::Registry(RegistryResponse::Ack);
                                if let Err(e) = write_p2p(send, resp).await {
                                    tracing::warn!("heartbeat ack failed for {endpoint_id}: {e}");
                                    break;
                                }
                            }
                            Request::Registry(common::protocols::registry::RegistryRequest::Register(caps)) => {
                                tracing::info!("host {endpoint_id} re-registered with {} models", caps.models.len());
                                hosts.register(endpoint_id.clone(), caps, connection.clone()).await;
                                let resp = Response::Registry(RegistryResponse::Ack);
                                if let Err(e) = write_p2p(send, resp).await {
                                    tracing::warn!("re-register ack failed for {endpoint_id}: {e}");
                                    break;
                                }
                            }
                            _ => {
                                tracing::warn!("unexpected request from host {endpoint_id} on host-initiated stream");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::info!("host {endpoint_id} connection closed: {e}");
                        break;
                    }
                }
            }

            tracing::info!("host disconnected: {endpoint_id}");
            hosts.unregister(&endpoint_id).await;

            Ok(())
        })
    }
}
