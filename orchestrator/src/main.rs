use std::str::FromStr;

use axum::{Json, Router, debug_handler, extract::State, routing::post};
use common::{
    helpers::{read_p2p, write_p2p},
    protocols::embeddings::{Embeddings, EmbeddingsRequest, EmbeddingsResponse},
};
use iroh::{Endpoint, EndpointAddr, EndpointId};

#[tokio::main]
async fn main() {
    let endpoint = Endpoint::bind(iroh::endpoint::presets::N0)
        .await
        .expect("failed to bind iroh endpoint");

    // build our application with a route
    let app = Router::new()
        .route("/embeddings", post(create_embeddings))
        .with_state(endpoint);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn connect_side(
    endpoint: &Endpoint,
    addr: EndpointAddr,
    embeddings_request: EmbeddingsRequest,
) -> anyhow::Result<EmbeddingsResponse> {
    let conn = endpoint.connect(addr, Embeddings::ALPN).await?;

    let (send, recv) = conn.open_bi().await?;

    write_p2p(send, embeddings_request).await?;

    println!("receiving!");
    let response = read_p2p(recv).await?;
    println!("received!!");

    Ok(response)
}

#[debug_handler]
async fn create_embeddings(
    State(endpoint): State<Endpoint>,
    Json(payload): Json<EmbeddingsRequest>,
) -> Json<EmbeddingsResponse> {
    let endpoint_id: EndpointId =
        EndpointId::from_str("8f5030e8dd6102e6b224ecbc2d2693cb53999fd3e9c63773ec57e0f3c316dbd3")
            .unwrap();
    let address = EndpointAddr::from(endpoint_id);
    let response = connect_side(&endpoint, address, payload).await.unwrap();
    Json(response)
}
