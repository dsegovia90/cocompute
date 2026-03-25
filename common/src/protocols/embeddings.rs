use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub const ALPN: &[u8] = b"cocompute/embeddings/0";

#[derive(Debug, Encode, Decode, Deserialize)]
pub struct EmbeddingsRequest {
    pub text: String,
}

#[derive(Debug, Encode, Decode, Serialize)]
pub struct EmbeddingsResponse {
    pub embeddings: Vec<f32>,
}

impl EmbeddingsResponse {
    pub fn new(embeddings: Vec<f32>) -> Self {
        Self { embeddings }
    }
}
