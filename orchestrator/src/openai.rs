//! OpenAI-compatible request/response types.
//! These translate between the OpenAI API format (JSON over HTTP)
//! and our internal protocol types (bitcode over iroh).

use serde::{Deserialize, Serialize};

// ── Embeddings ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OpenAIEmbeddingsRequest {
    pub model: String,
    pub input: String,
}

#[derive(Debug, Serialize)]
pub struct OpenAIEmbeddingsResponse {
    pub object: &'static str,
    pub data: Vec<OpenAIEmbeddingData>,
    pub model: String,
    pub usage: OpenAIUsage,
}

#[derive(Debug, Serialize)]
pub struct OpenAIEmbeddingData {
    pub object: &'static str,
    pub embedding: Vec<f32>,
    pub index: usize,
}

// ── Chat Completions ────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIChatMessageRaw>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub stream: bool,
    /// Controls thinking/reasoning mode. true = enable, false = disable.
    pub think: Option<bool>,
    /// Tool definitions for function calling.
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenAIChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Raw deserialization type — handles both string and array content formats.
#[derive(Debug, Deserialize)]
pub struct OpenAIChatMessageRaw {
    pub role: String,
    #[serde(rename = "content", default, deserialize_with = "deserialize_content_option")]
    content_parsed: Option<ParsedContent>,
    #[serde(default)]
    pub tool_calls: Vec<serde_json::Value>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}

#[derive(Debug)]
struct ParsedContent {
    text: String,
    images: Vec<String>,
}

impl OpenAIChatMessageRaw {
    /// Convert to internal ChatMessage with images and tool calls extracted.
    pub fn into_chat_message(self) -> common::protocols::chat::ChatMessage {
        use common::protocols::chat::{ToolCall, ToolCallFunction};

        let parsed = self.content_parsed.unwrap_or(ParsedContent { text: String::new(), images: vec![] });

        let tool_calls: Vec<ToolCall> = self.tool_calls.into_iter().map(|tc| {
            ToolCall {
                id: tc["id"].as_str().unwrap_or("").to_string(),
                call_type: tc["type"].as_str().unwrap_or("function").to_string(),
                function: ToolCallFunction {
                    name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                    arguments: tc["function"]["arguments"].as_str()
                        .unwrap_or(&tc["function"]["arguments"].to_string())
                        .to_string(),
                },
            }
        }).collect();

        common::protocols::chat::ChatMessage {
            role: self.role,
            content: parsed.text,
            images: parsed.images,
            tool_calls,
            tool_call_id: self.tool_call_id,
        }
    }

    /// Convert tool definitions from raw JSON to internal format.
    pub fn convert_tools(tools: Vec<serde_json::Value>) -> Vec<common::protocols::chat::ToolDefinition> {
        tools.into_iter().map(|t| {
            common::protocols::chat::ToolDefinition {
                tool_type: t["type"].as_str().unwrap_or("function").to_string(),
                function: common::protocols::chat::ToolFunctionDef {
                    name: t["function"]["name"].as_str().unwrap_or("").to_string(),
                    description: t["function"]["description"].as_str().unwrap_or("").to_string(),
                    parameters: t["function"]["parameters"].to_string(),
                },
            }
        }).collect()
    }
}

/// OpenAI allows `content` to be null, a string, or an array of content parts.
fn deserialize_content_option<'de, D>(deserializer: D) -> Result<Option<ParsedContent>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ContentInner {
        String(String),
        Parts(Vec<ContentPart>),
    }

    #[derive(Deserialize)]
    #[serde(tag = "type")]
    enum ContentPart {
        #[serde(rename = "text")]
        Text { text: String },
        #[serde(rename = "image_url")]
        ImageUrl { image_url: ImageUrl },
    }

    #[derive(Deserialize)]
    struct ImageUrl {
        url: String,
    }

    let opt: Option<ContentInner> = Option::deserialize(deserializer)?;

    match opt {
        None => Ok(None),
        Some(ContentInner::String(s)) => Ok(Some(ParsedContent { text: s, images: vec![] })),
        Some(ContentInner::Parts(parts)) => {
            let mut text_parts = Vec::new();
            let mut images = Vec::new();
            for part in parts {
                match part {
                    ContentPart::Text { text } => text_parts.push(text),
                    ContentPart::ImageUrl { image_url } => {
                        let data = if let Some(pos) = image_url.url.find(";base64,") {
                            image_url.url[pos + 8..].to_string()
                        } else {
                            image_url.url
                        };
                        images.push(data);
                    }
                }
            }
            Ok(Some(ParsedContent { text: text_parts.join(""), images }))
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: &'static str,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChatChoice>,
    pub usage: OpenAIUsage,
}

#[derive(Debug, Serialize)]
pub struct OpenAIChatChoice {
    pub index: usize,
    pub message: OpenAIChatMessage,
    pub finish_reason: &'static str,
}

// ── Streaming Chat (SSE) ────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OpenAIChatStreamChunk {
    pub id: String,
    pub object: &'static str,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChatStreamChoice>,
}

#[derive(Debug, Serialize)]
pub struct OpenAIChatStreamChoice {
    pub index: usize,
    pub delta: OpenAIChatStreamDelta,
    pub finish_reason: Option<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct OpenAIChatStreamDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

// ── Shared ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OpenAIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl OpenAIUsage {
    pub fn from_metering(metering: &common::protocols::Metering) -> Self {
        Self {
            prompt_tokens: metering.prompt_tokens,
            completion_tokens: metering.completion_tokens,
            total_tokens: metering.prompt_tokens + metering.completion_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::protocols::Metering;

    #[test]
    fn usage_from_metering_sums_tokens() {
        let m = Metering { prompt_tokens: 10, completion_tokens: 20, compute_ms: 500 };
        let usage = OpenAIUsage::from_metering(&m);
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[test]
    fn embeddings_response_json_format() {
        let resp = OpenAIEmbeddingsResponse {
            object: "list",
            data: vec![OpenAIEmbeddingData {
                object: "embedding",
                embedding: vec![0.1, 0.2],
                index: 0,
            }],
            model: "test-model".into(),
            usage: OpenAIUsage { prompt_tokens: 5, completion_tokens: 0, total_tokens: 5 },
        };
        let json: serde_json::Value = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["object"], "list");
        assert_eq!(json["data"][0]["object"], "embedding");
        assert_eq!(json["data"][0]["index"], 0);
        assert_eq!(json["model"], "test-model");
        assert_eq!(json["usage"]["total_tokens"], 5);
    }

    #[test]
    fn chat_response_json_format() {
        let resp = OpenAIChatResponse {
            id: "chatcmpl-123".into(),
            object: "chat.completion",
            created: 1234567890,
            model: "llama3:latest".into(),
            choices: vec![OpenAIChatChoice {
                index: 0,
                message: OpenAIChatMessage { role: "assistant".into(), content: Some("Hi!".into()), tool_calls: None, tool_call_id: None },
                finish_reason: "stop",
            }],
            usage: OpenAIUsage { prompt_tokens: 10, completion_tokens: 5, total_tokens: 15 },
        };
        let json: serde_json::Value = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["object"], "chat.completion");
        assert_eq!(json["choices"][0]["message"]["role"], "assistant");
        assert_eq!(json["choices"][0]["message"]["content"], "Hi!");
        assert_eq!(json["choices"][0]["finish_reason"], "stop");
    }

    #[test]
    fn chat_stream_chunk_json_format() {
        let chunk = OpenAIChatStreamChunk {
            id: "chatcmpl-123".into(),
            object: "chat.completion.chunk",
            created: 1234567890,
            model: "llama3:latest".into(),
            choices: vec![OpenAIChatStreamChoice {
                index: 0,
                delta: OpenAIChatStreamDelta {
                    role: None,
                    content: Some("Hello".into()),
                },
                finish_reason: None,
            }],
        };
        let json: serde_json::Value = serde_json::to_value(&chunk).unwrap();
        assert_eq!(json["object"], "chat.completion.chunk");
        assert_eq!(json["choices"][0]["delta"]["content"], "Hello");
        // role should be absent when None (skip_serializing_if)
        assert!(json["choices"][0]["delta"].get("role").is_none());
        assert!(json["choices"][0]["finish_reason"].is_null());
    }

    #[test]
    fn chat_stream_final_chunk_json_format() {
        let chunk = OpenAIChatStreamChunk {
            id: "chatcmpl-123".into(),
            object: "chat.completion.chunk",
            created: 1234567890,
            model: "llama3:latest".into(),
            choices: vec![OpenAIChatStreamChoice {
                index: 0,
                delta: OpenAIChatStreamDelta { role: None, content: None },
                finish_reason: Some("stop"),
            }],
        };
        let json: serde_json::Value = serde_json::to_value(&chunk).unwrap();
        assert_eq!(json["choices"][0]["finish_reason"], "stop");
        // Both role and content should be absent
        assert!(json["choices"][0]["delta"].get("role").is_none());
        assert!(json["choices"][0]["delta"].get("content").is_none());
    }

    #[test]
    fn chat_request_deserializes_with_stream() {
        let json = r#"{"model":"llama3:latest","messages":[{"role":"user","content":"Hi"}],"stream":true}"#;
        let req: OpenAIChatRequest = serde_json::from_str(json).unwrap();
        assert!(req.stream);
    }

    #[test]
    fn chat_request_deserializes_without_stream_defaults_false() {
        let json = r#"{"model":"llama3:latest","messages":[{"role":"user","content":"Hi"}]}"#;
        let req: OpenAIChatRequest = serde_json::from_str(json).unwrap();
        assert!(!req.stream);
    }
}
