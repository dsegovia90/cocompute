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

    #[tokio::test]
    async fn register_and_find_host() {
        let mgr = HostManager::new();
        mgr.register("host-1".into(), test_capabilities(vec!["llama3:latest"])).await;

        let found = mgr.find_host_for_model("llama3:latest").await;
        assert!(found.is_some());
        assert_eq!(found.unwrap().endpoint_id, "host-1");
    }

    #[tokio::test]
    async fn find_returns_none_for_unknown_model() {
        let mgr = HostManager::new();
        mgr.register("host-1".into(), test_capabilities(vec!["llama3:latest"])).await;

        let found = mgr.find_host_for_model("gpt-4").await;
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn unregister_removes_host() {
        let mgr = HostManager::new();
        mgr.register("host-1".into(), test_capabilities(vec!["llama3:latest"])).await;
        assert!(mgr.has_hosts().await);

        mgr.unregister("host-1").await;
        assert!(!mgr.has_hosts().await);
        assert!(mgr.find_host_for_model("llama3:latest").await.is_none());
    }

    #[tokio::test]
    async fn available_models_across_hosts() {
        let mgr = HostManager::new();
        mgr.register("host-1".into(), test_capabilities(vec!["llama3:latest"])).await;
        mgr.register("host-2".into(), test_capabilities(vec!["mxbai-embed-large:latest"])).await;

        let mut models = mgr.available_models().await;
        models.sort();
        assert_eq!(models, vec!["llama3:latest", "mxbai-embed-large:latest"]);
    }

    #[tokio::test]
    async fn empty_manager_has_no_hosts() {
        let mgr = HostManager::new();
        assert!(!mgr.has_hosts().await);
        assert!(mgr.available_models().await.is_empty());
    }

    #[tokio::test]
    async fn register_overwrites_existing_host() {
        let mgr = HostManager::new();
        mgr.register("host-1".into(), test_capabilities(vec!["llama3:latest"])).await;
        mgr.register("host-1".into(), test_capabilities(vec!["mistral:latest"])).await;

        // Should have the new capabilities, not the old
        assert!(mgr.find_host_for_model("mistral:latest").await.is_some());
        assert!(mgr.find_host_for_model("llama3:latest").await.is_none());
    }

    #[test]
    fn connected_host_has_model() {
        let host = ConnectedHost {
            endpoint_id: "test".into(),
            capabilities: test_capabilities(vec!["llama3:latest", "mxbai-embed-large:latest"]),
            connected_at: chrono::Utc::now(),
        };
        assert!(host.has_model("llama3:latest"));
        assert!(host.has_model("mxbai-embed-large:latest"));
        assert!(!host.has_model("gpt-4"));
    }
}
