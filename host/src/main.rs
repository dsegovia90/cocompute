mod connection;
mod handlers;
mod ollama;
mod update;

use clap::Parser;
use ollama_rs::Ollama;

use connection::{connect_and_serve, fetch_orchestrator_id};
use update::{check_for_update, perform_update};

#[derive(Parser, Debug)]
#[command(name = "cocompute-host", version)]
struct Args {
    /// Ollama server URL
    #[arg(long, default_value = "http://localhost", env = "OLLAMA_URL")]
    ollama_url: String,

    /// Ollama server port
    #[arg(long, default_value = "11434", env = "OLLAMA_PORT")]
    ollama_port: u16,

    /// Orchestrator HTTP URL for discovery (e.g. http://192.168.1.100:3000)
    #[arg(long, env = "COCOMPUTE_ORCHESTRATOR_URL")]
    orchestrator_url: String,

    /// Orchestrator endpoint ID (optional — fetched from orchestrator if not provided)
    #[arg(long, env = "COCOMPUTE_ORCHESTRATOR_ID")]
    orchestrator_id: Option<String>,

    /// One-time setup token for pool registration
    #[arg(long, env = "COCOMPUTE_SETUP_TOKEN")]
    setup_token: Option<String>,

    /// Automatically check for updates on startup
    #[arg(long, default_value = "false", env = "COCOMPUTE_AUTO_UPDATE")]
    auto_update: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Check for and install updates from the orchestrator
    SelfUpdate,
    /// Start the host (default)
    Run,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    tracing::info!("cocompute-host v{}", env!("CARGO_PKG_VERSION"));

    let orchestrator_url = args.orchestrator_url.clone();

    // Handle self-update subcommand
    if matches!(args.command, Some(Command::SelfUpdate)) {
        tracing::info!("checking for updates from {}", orchestrator_url);
        match check_for_update(&orchestrator_url).await? {
            Some(new_version) => {
                let local = env!("CARGO_PKG_VERSION");
                tracing::info!("update available: {local} → {new_version}");
                perform_update(&orchestrator_url).await?;
            }
            None => {
                tracing::info!("already on latest version ({})", env!("CARGO_PKG_VERSION"));
            }
        }
        return Ok(());
    }

    // Auto-update check on startup
    if args.auto_update {
        tracing::info!("checking for updates...");
        match check_for_update(&orchestrator_url).await {
            Ok(Some(new_version)) => {
                let local = env!("CARGO_PKG_VERSION");
                tracing::info!("update available: {local} → {new_version}. updating...");
                match perform_update(&orchestrator_url).await {
                    Ok(()) => {
                        tracing::info!("update installed. restarting...");
                        // Re-exec ourselves with the same args
                        let exe = std::env::current_exe()?;
                        let args: Vec<String> = std::env::args().collect();
                        let err = exec::execvp(&exe, &args);
                        anyhow::bail!("failed to restart after update: {err}");
                    }
                    Err(e) => {
                        tracing::warn!(
                            "auto-update failed: {e}. continuing with current version."
                        );
                    }
                }
            }
            Ok(None) => {
                tracing::info!("up to date ({})", env!("CARGO_PKG_VERSION"));
            }
            Err(e) => {
                tracing::warn!("update check failed: {e}. continuing with current version.");
            }
        }
    }

    let endpoint = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .bind()
        .await?;

    tracing::info!("host endpoint id: {:?}", endpoint.addr().id);

    // Persistent host identity — survives restarts, unlike the ephemeral iroh endpoint_id
    let host_id = load_or_create_host_id().await?;
    tracing::info!("host_id: {host_id}");

    let ollama = Ollama::new(args.ollama_url.clone(), args.ollama_port);

    // Connect to orchestrator with reconnection loop
    let mut cached_id: Option<String> = args.orchestrator_id.clone();
    let mut setup_token: Option<String> = args.setup_token.clone();

    loop {
        let orchestrator_id = match &cached_id {
            Some(id) => id.clone(),
            None => {
                tracing::info!(
                    "fetching orchestrator endpoint ID from {}",
                    orchestrator_url
                );
                match fetch_orchestrator_id(&orchestrator_url).await {
                    Ok(id) => {
                        tracing::info!("orchestrator endpoint ID: {id}");
                        cached_id = Some(id.clone());
                        id
                    }
                    Err(e) => {
                        tracing::error!("failed to fetch orchestrator ID: {e}");
                        tracing::info!("retrying in 5 seconds...");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        continue;
                    }
                }
            }
        };

        match connect_and_serve(&endpoint, &orchestrator_id, ollama.clone(), setup_token.take(), host_id.clone()).await {
            Ok(()) => break,
            Err(e) => {
                tracing::error!("disconnected from orchestrator: {e}");
                cached_id = None;
                tracing::info!("reconnecting in 5 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}

/// Load or create a persistent host_id at ~/.cocompute/host_id.
async fn load_or_create_host_id() -> anyhow::Result<String> {
    let dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?
        .join(".cocompute");
    tokio::fs::create_dir_all(&dir).await?;

    let path = dir.join("host_id");
    match tokio::fs::read_to_string(&path).await {
        Ok(id) => {
            let id = id.trim().to_string();
            if !id.is_empty() {
                return Ok(id);
            }
        }
        Err(_) => {}
    }

    let id = uuid::Uuid::new_v4().to_string();
    tokio::fs::write(&path, &id).await?;
    tracing::info!("generated new host_id at {}", path.display());
    Ok(id)
}

#[cfg(test)]
mod tests {
    use crate::ollama::ollama_model_name;

    #[test]
    fn ollama_model_name_strips_dev_suffix() {
        assert_eq!(ollama_model_name("llama3:latest@dev"), "llama3:latest");
    }

    #[test]
    fn ollama_model_name_passthrough_without_suffix() {
        assert_eq!(ollama_model_name("llama3:latest"), "llama3:latest");
    }
}
