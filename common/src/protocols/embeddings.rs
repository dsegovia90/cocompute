use bitcode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Encode, Decode, Deserialize)]
pub struct EmbeddingsRequest {
    pub model: String,
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
