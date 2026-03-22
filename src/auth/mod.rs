use anyhow::{Context, Result};
use std::path::PathBuf;

fn token_file_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("procrast-cli");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join(".token"))
}

pub fn store_token(token: &str) -> Result<()> {
    // Try keyring first — only fall back to file if keyring fails
    if let Ok(entry) = keyring::Entry::new("procrast-cli", "api-token") {
        if entry.set_password(token).is_ok() {
            return Ok(());
        }
    }

    // Fall back to file only when keyring is unavailable
    write_token_file(token)
}

pub fn get_token() -> Result<Option<String>> {
    // Try keyring first
    if let Ok(entry) = keyring::Entry::new("procrast-cli", "api-token") {
        if let Ok(token) = entry.get_password() {
            return Ok(Some(token));
        }
    }

    // Fall back to file
    read_token_file()
}

pub fn delete_token() -> Result<()> {
    // Clean up keyring
    if let Ok(entry) = keyring::Entry::new("procrast-cli", "api-token") {
        let _ = entry.delete_credential();
    }

    // Clean up file
    let path = token_file_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

fn write_token_file(token: &str) -> Result<()> {
    let path = token_file_path()?;
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    std::io::Write::write_all(&mut opts.open(&path)?, token.as_bytes())
        .context("Failed to write token file")?;
    Ok(())
}

fn read_token_file() -> Result<Option<String>> {
    let path = token_file_path()?;
    if path.exists() {
        let token = std::fs::read_to_string(&path)
            .context("Failed to read token file")?;
        if token.is_empty() {
            Ok(None)
        } else {
            Ok(Some(token))
        }
    } else {
        Ok(None)
    }
}
