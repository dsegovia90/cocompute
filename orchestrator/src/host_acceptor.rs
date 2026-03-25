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

            // Wait for the connection to close, then unregister
            connection.closed().await;
            tracing::info!("host disconnected: {endpoint_id}");
            hosts.unregister(&endpoint_id).await;

            Ok(())
        })
    }
}
