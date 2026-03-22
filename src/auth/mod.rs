use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE_NAME: &str = "procrast-cli";
const TOKEN_KEY: &str = "api-token";

pub fn store_token(token: &str) -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, TOKEN_KEY)
        .context("Failed to create keyring entry")?;
    entry
        .set_password(token)
        .context("Failed to store token in keyring")?;
    Ok(())
}

pub fn get_token() -> Result<Option<String>> {
    let entry = Entry::new(SERVICE_NAME, TOKEN_KEY)
        .context("Failed to create keyring entry")?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow::anyhow!("Failed to read token from keyring: {}", e)),
    }
}

pub fn delete_token() -> Result<()> {
    let entry = Entry::new(SERVICE_NAME, TOKEN_KEY)
        .context("Failed to create keyring entry")?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // Already gone
        Err(e) => Err(anyhow::anyhow!("Failed to delete token from keyring: {}", e)),
    }
}
