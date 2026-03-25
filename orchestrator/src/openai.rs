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
    pub messages: Vec<OpenAIChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatMessage {
    pub role: String,
    pub content: String,
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
