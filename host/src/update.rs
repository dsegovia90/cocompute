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

/// Embedded minisign public key. MUST match the value in
/// `orchestrator/static/install.sh::MINISIGN_PUBKEY`. The host self-update
/// path verifies new binaries against this key before atomic-replace, so a
/// compromised orchestrator cannot push a backdoored binary to running hosts.
///
/// To rotate: regenerate the keypair, update both this constant AND
/// install.sh, push out a new release. Hosts on the old version will refuse
/// the update (signature mismatch) and need to be reinstalled fresh.
const MINISIGN_PUBKEY: &str = "RWTk079UqKo+d0iStb/kLX57UVEZtKTrcGxY1Ap2yF001IQVijA3hbxF";

/// Download the new binary AND its minisign signature from the orchestrator,
/// verify the signature against the embedded public key, then replace the
/// current executable. Refuses to install if the signature is missing,
/// malformed, or doesn't verify — the host will keep running its current
/// (already-trusted) binary.
pub(crate) async fn perform_update(orchestrator_url: &str) -> anyhow::Result<()> {
    let platform = current_platform();
    if platform == "unknown" {
        anyhow::bail!("cannot update: unknown platform");
    }

    let base = orchestrator_url.trim_end_matches('/');
    let binary_url = format!("{base}/v1/update/{platform}");
    let sig_url = format!("{base}/v1/update-sig/{platform}.minisig");

    tracing::info!("downloading update for {platform}...");
    let bin_response = reqwest::get(&binary_url).await?;
    if !bin_response.status().is_success() {
        let status = bin_response.status();
        let body = bin_response.text().await.unwrap_or_default();
        anyhow::bail!("update download failed: {status} {body}");
    }
    let bytes = bin_response.bytes().await?;
    tracing::info!("downloaded {} bytes", bytes.len());

    // Fetch the matching minisign signature. If the .minisig is missing
    // (e.g., release was cut without signing), refuse the update — running
    // an unverified binary in self-update is the exact attack vector signed
    // releases exist to prevent.
    tracing::info!("fetching signature for verification...");
    let sig_response = reqwest::get(&sig_url).await?;
    if !sig_response.status().is_success() {
        anyhow::bail!(
            "update aborted: signature unavailable ({}). refusing to install unsigned binary.",
            sig_response.status()
        );
    }
    let sig_text = sig_response.text().await?;

    // Verify the binary against the embedded public key. Failures here mean
    // either (a) the orchestrator served a tampered binary, (b) the orchestrator
    // is on a newer signing key than this host knows about, or (c) bit-rot
    // during transfer. All three should refuse the update; case (b) requires
    // a fresh reinstall via install.sh.
    let pubkey = minisign_verify::PublicKey::decode(MINISIGN_PUBKEY)
        .map_err(|e| anyhow::anyhow!("embedded MINISIGN_PUBKEY is invalid: {e}"))?;
    let signature = minisign_verify::Signature::decode(&sig_text)
        .map_err(|e| anyhow::anyhow!("malformed signature from orchestrator: {e}"))?;
    pubkey
        .verify(&bytes, &signature, false)
        .map_err(|e| anyhow::anyhow!("signature verification FAILED: {e}. refusing to install."))?;
    tracing::info!("signature verified.");

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
