use std::{collections::HashMap, sync::Arc};

use common::protocols::registry::Capabilities;
use tokio::sync::RwLock;

/// A connected host with its capabilities.
#[derive(Debug, Clone)]
pub struct ConnectedHost {
    pub endpoint_id: String,
    pub capabilities: Capabilities,
    pub connected_at: chrono::DateTime<chrono::Utc>,
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
}

impl HostManager {
    pub fn new() -> Self {
        Self {
            hosts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a host with its capabilities.
    pub async fn register(&self, endpoint_id: String, capabilities: Capabilities) {
        let host = ConnectedHost {
            endpoint_id: endpoint_id.clone(),
            capabilities,
            connected_at: chrono::Utc::now(),
        };
        self.hosts.write().await.insert(endpoint_id.clone(), host);
        tracing::info!("host registered: {endpoint_id}");
    }

    /// Remove a host (disconnected).
    pub async fn unregister(&self, endpoint_id: &str) {
        self.hosts.write().await.remove(endpoint_id);
        tracing::info!("host unregistered: {endpoint_id}");
    }

    /// Find a host that has the requested model.
    pub async fn find_host_for_model(&self, model_name: &str) -> Option<ConnectedHost> {
        let hosts = self.hosts.read().await;
        hosts.values().find(|h| h.has_model(model_name)).cloned()
    }

    /// List all available model names across all hosts.
    pub async fn available_models(&self) -> Vec<String> {
        let hosts = self.hosts.read().await;
        hosts
            .values()
            .flat_map(|h| h.capabilities.models.iter().map(|m| m.name.clone()))
            .collect()
    }

    /// Check if any hosts are connected.
    pub async fn has_hosts(&self) -> bool {
        !self.hosts.read().await.is_empty()
    }
}
