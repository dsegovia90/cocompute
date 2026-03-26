use std::path::PathBuf;

use anyhow::Context;

/// Expand ~ to the user's home directory.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Load or generate an iroh secret key for stable EndpointId across restarts.
pub fn load_or_create_secret_key(key_path: &PathBuf) -> anyhow::Result<iroh::SecretKey> {
    if key_path.exists() {
        let bytes = std::fs::read(key_path).context("failed to read key file")?;
        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid key file length"))?;
        Ok(iroh::SecretKey::from_bytes(&key_bytes))
    } else {
        let key = iroh::SecretKey::generate(&mut rand::rng());
        if let Some(parent) = key_path.parent() {
            std::fs::create_dir_all(parent).context("failed to create key directory")?;
        }
        std::fs::write(key_path, key.to_bytes()).context("failed to write key file")?;
        Ok(key)
    }
}
