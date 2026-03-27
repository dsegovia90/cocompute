use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    /// Base64-encoded images for multimodal models.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    /// Tool calls returned by the model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    /// Tool call ID (for tool response messages where role = "tool").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// A tool call from the model.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    /// JSON-encoded arguments string.
    pub arguments: String,
}

/// A tool definition passed in the request.
#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunctionDef,
}

#[derive(Debug, Clone, Encode, Decode, Serialize, Deserialize)]
pub struct ToolFunctionDef {
    pub name: String,
    pub description: String,
    /// JSON Schema for parameters, stored as a JSON string for bitcode compatibility.
    pub parameters: String,
}

#[derive(Debug, Encode, Decode, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    #[serde(default)]
    pub stream: bool,
    /// Controls thinking/reasoning. None = model default, Some(true) = enable, Some(false) = disable.
    pub think: Option<bool>,
    /// Tool definitions for function calling.
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
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
