mod api;
mod auth;
mod cache;
mod cli;
mod config;
mod markdown;
mod tui;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "procrast",
    about = "CLI and TUI for Procrast idea capture",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with the Procrast API
    Login,
    /// Log out and invalidate token
    Logout,
    /// List your ideas
    List {
        /// Maximum number of ideas to show
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show a single idea by UUID (or prefix)
    Show {
        /// Full UUID or prefix (first 6+ chars)
        uuid: String,
        /// Output as agent-friendly markdown
        #[arg(long)]
        markdown: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Search ideas
    Search {
        /// Search query
        query: String,
        /// Maximum number of results
        #[arg(short, long, default_value_t = 20)]
        limit: usize,
    },
    /// Export an idea as a markdown file
    Export {
        /// Full UUID or prefix
        uuid: String,
        /// Output file path (defaults to {title}.md)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Launch the TUI
    Tui,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::Config::load()?;

    match cli.command {
        None | Some(Commands::Tui) => {
            // Launch TUI
            let token = auth::get_token()?.ok_or_else(|| {
                anyhow::anyhow!("Not logged in. Run `procrast login` first.")
            })?;
            let client =
                api::client::ApiClient::new(config.api_url, Some(token));
            let app = tui::app::App::new(client);

            let terminal = ratatui::init();
            let result = app.run(terminal).await;
            ratatui::restore();
            result
        }
        Some(Commands::Login) => cli::auth::login(&config.api_url).await,
        Some(Commands::Logout) => cli::auth::logout(&config.api_url).await,
        Some(Commands::List { limit, json }) => {
            cli::ideas::list(&config.api_url, limit, json).await
        }
        Some(Commands::Show {
            uuid,
            markdown,
            json,
        }) => cli::ideas::show(&config.api_url, &uuid, markdown, json).await,
        Some(Commands::Search { query, limit }) => {
            cli::ideas::search(&config.api_url, &query, limit).await
        }
        Some(Commands::Export { uuid, output }) => {
            cli::ideas::export(&config.api_url, &uuid, output).await
        }
    }
}
