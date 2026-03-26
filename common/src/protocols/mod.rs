pub mod chat;
pub mod embeddings;
pub mod registry;

use bitcode::{Decode, Encode};

/// Single ALPN for all CoCompute protocols.
/// Message type dispatch happens per-stream via the Request/Response enums.
pub const ALPN: &[u8] = b"cocompute/0";

/// Metering data returned alongside every inference response.
#[derive(Debug, Clone, Encode, Decode)]
pub struct Metering {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub compute_ms: u64,
}

/// Top-level request type for dispatch over a single ALPN connection.
/// Each bi-stream carries one request of a specific type.
#[derive(Debug, Encode, Decode)]
pub enum Request {
    Registry(registry::RegistryRequest),
    Embeddings(embeddings::EmbeddingsRequest),
    Chat(chat::ChatRequest),
}

/// Top-level response type.
#[derive(Debug, Encode, Decode)]
pub enum Response {
    Registry(registry::RegistryResponse),
    Embeddings {
        result: embeddings::EmbeddingsResponse,
        metering: Metering,
    },
    Chat {
        result: chat::ChatResponse,
        metering: Metering,
    },
    /// Signals that the response will be streamed as multiple ChatStreamFrame messages.
    ChatStreamStart,
}

/// A frame in a streaming chat response. Sent as multiple frames on the same stream
/// after a ChatStreamStart response.
#[derive(Debug, Encode, Decode)]
pub enum ChatStreamFrame {
    /// A content chunk.
    Delta(String),
    /// Final frame with metering. No more frames after this.
    Done(Metering),
}
