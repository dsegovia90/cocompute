use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Encode, Decode, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Encode, Decode, Serialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
}

/// A single chunk in a streaming chat response.
#[derive(Debug, Encode, Decode)]
pub struct ChatStreamChunk {
    /// Partial content delta for this chunk.
    pub delta_content: String,
    /// True when this is the final chunk (no more content).
    pub done: bool,
}
