// SPDX-License-Identifier: AGPL-3.0-only

use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

/// Host capabilities reported during registration.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct Capabilities {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub quantization: String,
    pub vram_mb: u32,
    pub ram_mb: u32,
}

#[derive(Debug, Encode, Decode)]
pub enum RegistryRequest {
    /// Host registers with its capabilities, stable identity, and optional setup token.
    Register {
        capabilities: Capabilities,
        /// Persistent host UUID, stable across restarts (unlike the ephemeral iroh endpoint_id).
        host_id: String,
        setup_token: Option<String>,
    },
    /// Heartbeat — host is still alive.
    Heartbeat,
}

#[derive(Debug, Encode, Decode)]
pub enum RegistryResponse {
    /// Registration acknowledged.
    Ack,
}
