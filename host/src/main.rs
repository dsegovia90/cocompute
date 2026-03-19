use anyhow::Context;
use common::protocols::embeddings::Embeddings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = start_accept_side().await?;

    tokio::signal::ctrl_c().await.context("ctrl+c")?;
    router.shutdown().await.context("shutdown")?;

    Ok(())
}

async fn start_accept_side() -> anyhow::Result<iroh::protocol::Router> {
    let endpoint = iroh::Endpoint::bind(iroh::endpoint::presets::N0).await?;

    println!("{:?}", endpoint.addr().id);
    let router = iroh::protocol::Router::builder(endpoint)
        .accept(Embeddings::ALPN, Embeddings) // This makes the router handle incoming connections with our ALPN via Echo::accept!
        .spawn();

    Ok(router)
}
