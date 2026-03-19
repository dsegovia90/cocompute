use bitcode::{Decode, Encode};
use iroh::protocol::AcceptError;
use serde::{Deserialize, Serialize};

use crate::helpers::{read_p2p, write_p2p};

#[derive(Debug)]
pub struct Embeddings;

impl Embeddings {
    pub const ALPN: &[u8] = b"cocompute/embeddings/0";
}

#[derive(Debug, Encode, Decode, Deserialize)]
pub struct EmbeddingsRequest {
    text: String,
}

#[derive(Debug, Encode, Decode, Serialize)]
pub struct EmbeddingsResponse {
    embeddings: Vec<f64>,
}

impl EmbeddingsResponse {
    pub fn new(embeddings: Vec<f64>) -> Self {
        Self { embeddings }
    }
}

impl iroh::protocol::ProtocolHandler for Embeddings {
    /// The `accept` method is called for each incoming connection for our ALPN.
    ///
    /// The returned future runs on a newly spawned tokio task, so it can run as long as
    /// the connection lasts without blocking other connections.
    fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> impl Future<Output = Result<(), AcceptError>> + Send {
        Box::pin(async move {
            // We can get the remote's endpoint id from the connection.
            let endpoint_id = connection.remote_id();
            println!("accepted connection from {endpoint_id}");

            // Our protocol is a simple request-response protocol, so we expect the
            // connecting peer to open a single bi-directional stream.
            let (send, recv) = connection.accept_bi().await?;

            // Echo any bytes received back directly.
            // This will keep copying until the sender signals the end of data on the stream.
            let req: EmbeddingsRequest = read_p2p(recv).await.unwrap();
            println!("embedding text: {}", req.text);

            write_p2p(send, EmbeddingsResponse { embeddings: vec![] })
                .await
                .unwrap();

            // Wait until the remote closes the connection, which it does once it
            // received the response.
            connection.closed().await;

            Ok(())
        })
    }
}
