use axum::{Json, Router, debug_handler, routing::post};
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new()
        // `POST /users` goes to `create_user`
        .route("/embeddings", post(create_embeddings));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Debug, Deserialize)]
struct EmbeddingsRequest {
    text: String,
}

#[derive(Debug, Serialize)]
struct EmbeddingsResponse {
    embeddings: Vec<f64>,
}

impl EmbeddingsResponse {
    pub fn new(embeddings: Vec<f64>) -> Self {
        Self { embeddings }
    }
}

#[debug_handler]
async fn create_embeddings(Json(payload): Json<EmbeddingsRequest>) -> Json<EmbeddingsResponse> {
    Json(EmbeddingsResponse::new(vec![]))
}
