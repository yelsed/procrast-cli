use anyhow::Result;
use colored::Colorize;

use crate::api::client::ApiClient;
use crate::auth;

pub async fn login(api_url: &str) -> Result<()> {
    let email: String = dialoguer::Input::new()
        .with_prompt("Email")
        .interact_text()?;

    let password = rpassword::prompt_password("Password: ")?;

    let client = ApiClient::new(api_url.to_string(), None);
    let response = client.login(&email, &password).await?;

    auth::store_token(&response.token)?;

    println!(
        "{} Logged in as {}",
        "✓".green().bold(),
        response.user.name.bold()
    );
    Ok(())
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
