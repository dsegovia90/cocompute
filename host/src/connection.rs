use std::str::FromStr;

use common::{
    helpers::{read_p2p, write_p2p},
    protocols::{
        self, Request, Response,
        registry::{Capabilities, ModelInfo, RegistryRequest},
    },
};
use ollama_rs::Ollama;

use crate::handlers::handle_inference_stream;

/// Query Ollama for available models and build capabilities.
pub(crate) async fn discover_capabilities(ollama: &Ollama) -> anyhow::Result<Capabilities> {
    let models = ollama
        .list_local_models()
        .await
        .map_err(|e| anyhow::anyhow!("failed to list ollama models: {e}"))?;

    let model_infos: Vec<ModelInfo> = models
        .into_iter()
        .map(|m| {
            let name = if cfg!(debug_assertions) {
                format!("{}@dev", m.name)
            } else {
                m.name
            };
            ModelInfo {
                name,
                quantization: String::new(), // Ollama LocalModel doesn't expose quantization
                vram_mb: 0,                  // Ollama doesn't expose this per-model
                ram_mb: 0,
            }
        })
        .collect();

    tracing::debug!("discovered {} models from Ollama", model_infos.len());
    for m in &model_infos {
        tracing::debug!("  - {}", m.name);
    }

    Ok(Capabilities {
        models: model_infos,
    })
}

/// Fetch the orchestrator's endpoint ID from its HTTP API.
pub(crate) async fn fetch_orchestrator_id(orchestrator_url: &str) -> anyhow::Result<String> {
    let url = format!("{}/v1/node-info", orchestrator_url.trim_end_matches('/'));
    let resp: serde_json::Value = reqwest::get(&url)
        .await
        .map_err(|e| anyhow::anyhow!("failed to reach orchestrator at {url}: {e}"))?
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("invalid response from orchestrator: {e}"))?;

    resp["endpoint_id"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("orchestrator response missing endpoint_id"))
}

/// Connect to the orchestrator, register, then serve inference requests.
pub(crate) async fn connect_and_serve(
    endpoint: &iroh::Endpoint,
    orchestrator_id: &str,
    ollama: Ollama,
    setup_token: Option<String>,
    host_id: String,
) -> anyhow::Result<()> {
    let orch_id = iroh::EndpointId::from_str(orchestrator_id)
        .map_err(|e| anyhow::anyhow!("invalid orchestrator id: {e}"))?;
    let orch_addr = iroh::EndpointAddr::from(orch_id);

    tracing::info!("connecting to orchestrator: {orchestrator_id}");
    let conn = endpoint.connect(orch_addr, protocols::ALPN).await?;
    tracing::info!("connected to orchestrator");

    // Step 1: Register with capabilities
    let capabilities = discover_capabilities(&ollama).await?;
    let mut initial_models: Vec<String> =
        capabilities.models.iter().map(|m| m.name.clone()).collect();
    initial_models.sort();
    let reg_request = Request::Registry(RegistryRequest::Register {
        capabilities,
        host_id: host_id.clone(),
        setup_token,
    });

    let (send, recv) = conn.open_bi().await?;
    write_p2p(send, reg_request).await?;

    let ack: Response = read_p2p(recv).await?;
    match ack {
        Response::Registry(_) => tracing::info!("registered with orchestrator"),
        _ => anyhow::bail!("unexpected response to registration"),
    }

    // Step 2: Start heartbeat + model-refresh task alongside the inference loop
    let heartbeat_conn = conn.clone();
    let heartbeat_ollama = ollama.clone();
    let mut last_models = initial_models;

    let heartbeat_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
        let mut consecutive_failures: u32 = 0;
        const MAX_FAILURES: u32 = 3;

        tracing::info!("heartbeat task started (15s interval)");

        loop {
            interval.tick().await;

            // Send heartbeat first, then check models. Model discovery calls Ollama
            // which can block if a model is loading or running inference. The heartbeat
            // keeping the QUIC connection alive must never be blocked by Ollama.
            let discovery_start = std::time::Instant::now();
            let req = match tokio::time::timeout(
                std::time::Duration::from_secs(5),
                discover_capabilities(&heartbeat_ollama),
            )
            .await
            {
                Ok(Ok(new_caps)) => {
                    tracing::debug!(
                        "heartbeat: model discovery took {}ms",
                        discovery_start.elapsed().as_millis()
                    );
                    let mut new_models: Vec<String> =
                        new_caps.models.iter().map(|m| m.name.clone()).collect();
                    new_models.sort();
                    if new_models != last_models {
                        tracing::info!("model list changed, re-registering with orchestrator");
                        last_models = new_models;
                        Request::Registry(RegistryRequest::Register {
                            capabilities: new_caps,
                            host_id: host_id.clone(),
                            setup_token: None,
                        })
                    } else {
                        Request::Registry(RegistryRequest::Heartbeat)
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!(
                        "heartbeat: model discovery failed after {}ms: {e}",
                        discovery_start.elapsed().as_millis()
                    );
                    Request::Registry(RegistryRequest::Heartbeat)
                }
                Err(_) => {
                    tracing::warn!("heartbeat: model discovery timed out after 5s (Ollama busy?)");
                    Request::Registry(RegistryRequest::Heartbeat)
                }
            };

            match heartbeat_conn.open_bi().await {
                Ok((send, recv)) => {
                    if let Err(e) = write_p2p(send, req).await {
                        consecutive_failures += 1;
                        tracing::warn!(
                            "heartbeat send failed ({consecutive_failures}/{MAX_FAILURES}): {e}"
                        );
                        if consecutive_failures >= MAX_FAILURES {
                            tracing::error!(
                                "heartbeat failed {MAX_FAILURES} times, closing connection"
                            );
                            heartbeat_conn.close(0u32.into(), b"heartbeat failed");
                            break;
                        }
                        continue;
                    }
                    match read_p2p::<Response>(recv).await {
                        Ok(_) => {
                            consecutive_failures = 0;
                            tracing::debug!(
                                "heartbeat ack ({}ms)",
                                discovery_start.elapsed().as_millis()
                            );
                        }
                        Err(e) => {
                            consecutive_failures += 1;
                            tracing::warn!(
                                "heartbeat recv failed ({consecutive_failures}/{MAX_FAILURES}): {e}"
                            );
                            if consecutive_failures >= MAX_FAILURES {
                                tracing::error!(
                                    "heartbeat failed {MAX_FAILURES} times, closing connection"
                                );
                                heartbeat_conn.close(0u32.into(), b"heartbeat failed");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("heartbeat open_bi failed: {e}, closing connection");
                    heartbeat_conn.close(0u32.into(), b"heartbeat failed");
                    break;
                }
            }
        }
        tracing::info!("heartbeat task exiting");
    });

    // Step 3: Loop accepting inference streams from the orchestrator
    tracing::info!("serving inference requests...");
    loop {
        let (send, recv) = match conn.accept_bi().await {
            Ok(streams) => streams,
            Err(e) => {
                tracing::warn!("connection to orchestrator lost: {e}");
                heartbeat_handle.abort();
                return Err(e.into());
            }
        };

        let ollama_clone = ollama.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_inference_stream(ollama_clone, send, recv).await {
                tracing::error!("inference stream error: {e:?}");
            }
        });
    }
}
