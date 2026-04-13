use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use common::protocols::registry::Capabilities;
use iroh::endpoint::Connection;
use tokio::sync::RwLock;

/// A connected host with its capabilities and live connection.
#[derive(Debug, Clone)]
pub struct ConnectedHost {
    /// Persistent host identity (UUID, survives restarts).
    pub host_id: String,
    /// Ephemeral iroh transport address (changes on restart).
    pub endpoint_id: String,
    pub capabilities: Capabilities,
    pub connection: Connection,
    pub pool_ids: Vec<i32>,
    pub user_id: Option<i32>,
}

impl ConnectedHost {
    /// Check if this host has a specific model available.
    pub fn has_model(&self, model_name: &str) -> bool {
        self.capabilities.models.iter().any(|m| m.name == model_name)
    }
}

/// Manages the set of connected hosts and their capabilities.
#[derive(Debug, Clone)]
pub struct HostManager {
    hosts: Arc<RwLock<HashMap<String, ConnectedHost>>>,
    /// Round-robin counter for distributing requests across hosts.
    counter: Arc<AtomicUsize>,
}

impl HostManager {
    pub fn new() -> Self {
        Self {
            hosts: Arc::new(RwLock::new(HashMap::new())),
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Register a host with its capabilities, connection, and pool memberships.
    /// Keyed by `host_id` (persistent UUID), not `endpoint_id` (ephemeral iroh address).
    pub async fn register(
        &self,
        host_id: String,
        endpoint_id: String,
        capabilities: Capabilities,
        connection: Connection,
        pool_ids: Vec<i32>,
        user_id: Option<i32>,
    ) {
        let host = ConnectedHost {
            host_id: host_id.clone(),
            endpoint_id,
            capabilities,
            connection,
            pool_ids,
            user_id,
        };
        self.hosts.write().await.insert(host_id.clone(), host);
        tracing::info!("host registered: {host_id}");
    }

    /// Update pool memberships for a connected host.
    pub async fn update_pool_ids(&self, host_id: &str, pool_ids: Vec<i32>) {
        if let Some(host) = self.hosts.write().await.get_mut(host_id) {
            host.pool_ids = pool_ids;
            tracing::info!("updated pool_ids for host {host_id}");
        }
    }

    /// Remove a host (disconnected).
    pub async fn unregister(&self, host_id: &str) {
        self.hosts.write().await.remove(host_id);
        tracing::info!("host unregistered: {host_id}");
    }

    /// Find a host that has the requested model, optionally filtered by pool.
    pub async fn find_host_for_model(&self, model_name: &str, pool_id: Option<i32>) -> Option<ConnectedHost> {
        let hosts = self.hosts.read().await;
        let matching: Vec<&ConnectedHost> = hosts
            .values()
            .filter(|h| h.has_model(model_name))
            .filter(|h| match pool_id {
                Some(pid) => h.pool_ids.contains(&pid),
                None => true,
            })
            .collect();

        if matching.is_empty() {
            return None;
        }

        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % matching.len();
        Some(matching[idx].clone())
    }

    /// List available model names, optionally filtered by pool.
    pub async fn available_models(&self, pool_id: Option<i32>) -> Vec<String> {
        let hosts = self.hosts.read().await;
        hosts
            .values()
            .filter(|h| match pool_id {
                Some(pid) => h.pool_ids.contains(&pid),
                None => true,
            })
            .flat_map(|h| h.capabilities.models.iter().map(|m| m.name.clone()))
            .collect()
    }

    /// Check if any hosts are connected.
    pub async fn has_hosts(&self) -> bool {
        !self.hosts.read().await.is_empty()
    }

    /// Check if a specific host is currently connected (by host_id).
    pub async fn is_connected(&self, host_id: &str) -> bool {
        self.hosts.read().await.contains_key(host_id)
    }

    /// Get the set of currently connected host_ids.
    pub async fn connected_ids(&self) -> std::collections::HashSet<String> {
        self.hosts.read().await.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::protocols::registry::ModelInfo;

    fn test_capabilities(models: Vec<&str>) -> Capabilities {
        Capabilities {
            models: models
                .into_iter()
                .map(|name| ModelInfo {
                    name: name.to_string(),
                    quantization: "q4_0".to_string(),
                    vram_mb: 4096,
                    ram_mb: 8192,
                })
                .collect(),
        }
    }

    #[test]
    fn connected_host_has_model() {
        let caps = test_capabilities(vec!["llama3:latest", "mxbai-embed-large:latest"]);
        assert!(caps.models.iter().any(|m| m.name == "llama3:latest"));
        assert!(caps.models.iter().any(|m| m.name == "mxbai-embed-large:latest"));
        assert!(!caps.models.iter().any(|m| m.name == "gpt-4"));
    }

    /// Create a real iroh connection pair for testing.
    async fn make_test_connection() -> (iroh::protocol::Router, iroh::Endpoint, Connection) {
        use common::protocols;

        // Acceptor side: register our ALPN
        let ep_accept = iroh::Endpoint::bind(iroh::endpoint::presets::N0).await.unwrap();
        let addr = ep_accept.addr();

        // Dummy protocol handler that just accepts connections
        #[derive(Debug, Clone)]
        struct DummyHandler;
        impl iroh::protocol::ProtocolHandler for DummyHandler {
            fn accept(
                &self,
                connection: iroh::endpoint::Connection,
            ) -> impl Future<Output = Result<(), iroh::protocol::AcceptError>> + Send {
                Box::pin(async move {
                    connection.closed().await;
                    Ok(())
                })
            }
        }

        let router = iroh::protocol::Router::builder(ep_accept)
            .accept(protocols::ALPN, DummyHandler)
            .spawn();

        // Connector side
        let ep_connect = iroh::Endpoint::bind(iroh::endpoint::presets::N0).await.unwrap();
        let conn = ep_connect.connect(addr, protocols::ALPN).await.unwrap();

        (router, ep_connect, conn)
    }

    #[tokio::test]
    async fn two_hosts_same_model_one_disconnects() {
        let mgr = HostManager::new();

        let (_ep1a, _ep1b, conn1) = make_test_connection().await;
        let (_ep2a, _ep2b, conn2) = make_test_connection().await;

        // Both hosts have llama3
        mgr.register(
            "host-1".into(),
            "ep-1".into(),
            test_capabilities(vec!["llama3:latest", "mxbai-embed-large:latest"]),
            conn1,
            vec![],
            None,
        ).await;
        mgr.register(
            "host-2".into(),
            "ep-2".into(),
            test_capabilities(vec!["llama3:latest"]),
            conn2,
            vec![],
            None,
        ).await;

        // Both models available
        let mut models = mgr.available_models().await;
        models.sort();
        models.dedup();
        assert_eq!(models, vec!["llama3:latest", "mxbai-embed-large:latest"]);

        // Both hosts can serve llama3
        assert!(mgr.find_host_for_model("llama3:latest", None).await.is_some());

        // Host 2 disconnects
        mgr.unregister("host-2").await;

        // llama3 still available via host-1
        let host = mgr.find_host_for_model("llama3:latest", None).await;
        assert!(host.is_some());
        assert_eq!(host.unwrap().host_id, "host-1");

        // mxbai still available (only host-1 had it)
        assert!(mgr.find_host_for_model("mxbai-embed-large:latest", None).await.is_some());
    }

    #[tokio::test]
    async fn all_hosts_disconnect_model_unavailable() {
        let mgr = HostManager::new();

        let (_ep1a, _ep1b, conn1) = make_test_connection().await;

        mgr.register(
            "host-1".into(),
            "ep-1".into(),
            test_capabilities(vec!["llama3:latest"]),
            conn1,
            vec![],
            None,
        ).await;

        assert!(mgr.find_host_for_model("llama3:latest", None).await.is_some());

        mgr.unregister("host-1").await;

        // Model gone
        assert!(mgr.find_host_for_model("llama3:latest", None).await.is_none());
        assert!(mgr.available_models().await.is_empty());
    }

    #[tokio::test]
    async fn round_robin_across_two_hosts() {
        let mgr = HostManager::new();

        let (_ep1a, _ep1b, conn1) = make_test_connection().await;
        let (_ep2a, _ep2b, conn2) = make_test_connection().await;

        mgr.register("host-1".into(), "ep-1".into(), test_capabilities(vec!["llama3:latest"]), conn1, vec![], None).await;
        mgr.register("host-2".into(), "ep-2".into(), test_capabilities(vec!["llama3:latest"]), conn2, vec![], None).await;

        let h1 = mgr.find_host_for_model("llama3:latest", None).await.unwrap();
        let h2 = mgr.find_host_for_model("llama3:latest", None).await.unwrap();

        // Should get different hosts (round-robin)
        assert_ne!(h1.host_id, h2.host_id);
    }
}
