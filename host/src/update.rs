/// Detect the current platform string for update downloads.
pub(crate) fn current_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-arm64",
        ("macos", "aarch64") => "macos-arm64",
        ("macos", "x86_64") => "macos-x86_64",
        (os, arch) => {
            tracing::warn!("unknown platform: {os}/{arch}");
            "unknown"
        }
    }
}

/// Check the orchestrator for a newer version and return it if available.
/// Only triggers on upgrades (remote > local), never downgrades.
pub(crate) async fn check_for_update(
    orchestrator_url: &str,
) -> anyhow::Result<Option<String>> {
    let url = format!("{}/v1/version", orchestrator_url.trim_end_matches('/'));
    let resp: serde_json::Value = reqwest::get(&url).await?.json().await?;

    let remote_str = resp["version"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing version in response"))?;

    let local_str = env!("CARGO_PKG_VERSION");

    let remote: semver::Version = remote_str
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid remote version '{remote_str}': {e}"))?;
    let local: semver::Version = local_str
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid local version '{local_str}': {e}"))?;

    if remote > local {
        Ok(Some(remote_str.to_string()))
    } else {
        Ok(None)
    }
}

/// Download the new binary from the orchestrator and replace the current executable.
pub(crate) async fn perform_update(orchestrator_url: &str) -> anyhow::Result<()> {
    let platform = current_platform();
    if platform == "unknown" {
        anyhow::bail!("cannot update: unknown platform");
    }

    let url = format!(
        "{}/v1/update/{platform}",
        orchestrator_url.trim_end_matches('/')
    );

    tracing::info!("downloading update for {platform}...");
    let response = reqwest::get(&url).await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("update download failed: {status} {body}");
    }

    let bytes = response.bytes().await?;
    tracing::info!("downloaded {} bytes", bytes.len());

    // Get path to current executable
    let current_exe = std::env::current_exe()?;
    let backup_path = current_exe.with_extension("old");
    let temp_path = current_exe.with_extension("new");

    // Write new binary to temp file
    tokio::fs::write(&temp_path, &bytes).await?;

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&temp_path, std::fs::Permissions::from_mode(0o755)).await?;
    }

    // Atomic swap: current → backup, new → current
    if backup_path.exists() {
        tokio::fs::remove_file(&backup_path).await.ok();
    }
    tokio::fs::rename(&current_exe, &backup_path).await?;

    if let Err(e) = tokio::fs::rename(&temp_path, &current_exe).await {
        // Rollback: restore the backup so we're not left with no binary
        tracing::error!("failed to install new binary: {e}. rolling back...");
        tokio::fs::rename(&backup_path, &current_exe).await.ok();
        anyhow::bail!("update failed, rolled back to previous version: {e}");
    }

    // Clean up backup
    tokio::fs::remove_file(&backup_path).await.ok();

    tracing::info!("update complete. restart to use the new version.");
    Ok(())
}
