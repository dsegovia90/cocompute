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
    pub endpoint_id: String,
    pub capabilities: Capabilities,
    pub connection: Connection,
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

    /// Register a host with its capabilities and connection.
    pub async fn register(
        &self,
        endpoint_id: String,
        capabilities: Capabilities,
        connection: Connection,
    ) {
        let host = ConnectedHost {
            endpoint_id: endpoint_id.clone(),
            capabilities,
            connection,
        };
        self.hosts.write().await.insert(endpoint_id.clone(), host);
        tracing::info!("host registered: {endpoint_id}");
    }

    /// Remove a host (disconnected).
    pub async fn unregister(&self, endpoint_id: &str) {
        self.hosts.write().await.remove(endpoint_id);
        tracing::info!("host unregistered: {endpoint_id}");
    }

    /// Find a host that has the requested model, rotating across matching hosts.
    pub async fn find_host_for_model(&self, model_name: &str) -> Option<ConnectedHost> {
        let hosts = self.hosts.read().await;
        let matching: Vec<&ConnectedHost> = hosts
            .values()
            .filter(|h| h.has_model(model_name))
            .collect();

        if matching.is_empty() {
            return None;
        }

        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % matching.len();
        Some(matching[idx].clone())
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
}
