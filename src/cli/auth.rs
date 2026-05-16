use anyhow::{anyhow, Result};
use colored::Colorize;

use crate::api::client::{ApiClient, ApiError};
use crate::auth;

const MAX_LOGIN_ATTEMPTS: u8 = 3;

/// Returns the token on success
pub async fn login(api_url: &str) -> Result<String> {
    let client = ApiClient::new(api_url.to_string(), None);

    for attempt in 1..=MAX_LOGIN_ATTEMPTS {
        let email: String = dialoguer::Input::new()
            .with_prompt("Email")
            .interact_text()?;

        let password = rpassword::prompt_password("Password: ")?;

        match client.login(&email, &password).await {
            Ok(response) => {
                let token = response.token.clone();
                auth::store_token(&response.token)?;
                println!(
                    "{} Logged in as {}",
                    "✓".green().bold(),
                    response.user.name.bold()
                );
                return Ok(token);
            }
            Err(e) => match e.downcast_ref::<ApiError>() {
                Some(ApiError::ValidationError(msg)) => {
                    eprintln!("{} {}", "✗".red().bold(), msg);
                    if attempt < MAX_LOGIN_ATTEMPTS {
                        eprintln!(
                            "  {} attempt(s) remaining.\n",
                            MAX_LOGIN_ATTEMPTS - attempt
                        );
                        continue;
                    }
                    return Err(anyhow!("Login failed after {} attempts", MAX_LOGIN_ATTEMPTS));
                }
                Some(ApiError::RateLimited) => {
                    return Err(anyhow!(
                        "Too many login attempts. Wait a few minutes and try again."
                    ));
                }
                _ => return Err(e),
            },
        }
    }

    Err(anyhow!("Login failed"))
}

pub async fn logout(api_url: &str) -> Result<()> {
    let token = auth::get_token()?;
    if let Some(token) = token {
        let client = ApiClient::new(api_url.to_string(), Some(token));
        let _ = client.logout().await; // Best-effort server invalidation
    }
    auth::delete_token()?;
    println!("{} Logged out", "✓".green().bold());
    Ok(())
}
