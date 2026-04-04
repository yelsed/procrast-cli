mod api;
mod auth;
mod cache;
mod cli;
mod config;
mod focus_score;
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
        /// Hide completed ideas
        #[arg(long)]
        hide_done: bool,
        /// Sort order: smart, priority, newest, oldest
        #[arg(short, long, default_value_t = focus_score::SortMode::Smart, value_enum)]
        sort: focus_score::SortMode,
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
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Toggle an idea's completion status
    Done {
        /// Full UUID or prefix (first 6+ chars)
        uuid: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Export an idea as a markdown file
    Export {
        /// Full UUID or prefix
        uuid: String,
        /// Output file path (defaults to {title}.md)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Check for updates and update the CLI
    Update {
        /// Only check, don't install
        #[arg(long)]
        check: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
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
            // Login if needed, then launch TUI
            let token = match auth::get_token()? {
                Some(t) => t,
                None => {
                    println!("Not logged in. Please log in first.\n");
                    cli::auth::login(&config.api_url).await?
                }
            };
            let api_url = config.api_url;
            let client =
                api::client::ApiClient::new(api_url.clone(), Some(token));
            let app = tui::app::App::new(client);

            let terminal = ratatui::init();
            let result = app.run(terminal).await;
            ratatui::restore();

            match result {
                Ok(true) => {
                    // User pressed 'l' to re-login
                    println!("Session expired. Logging in again...\n");
                    cli::auth::login(&api_url).await?;
                    println!("\nLogin successful! Run `procrast` to launch the TUI.");
                    Ok(())
                }
                Ok(false) => Ok(()),
                Err(e) => Err(e),
            }
        }
        Some(Commands::Login) => cli::auth::login(&config.api_url).await.map(|_| ()),
        Some(Commands::Logout) => cli::auth::logout(&config.api_url).await,
        Some(Commands::List { limit, json, hide_done, sort }) => {
            cli::ideas::list(&config.api_url, limit, json, hide_done, sort).await
        }
        Some(Commands::Show {
            uuid,
            markdown,
            json,
        }) => cli::ideas::show(&config.api_url, &uuid, markdown, json).await,
        Some(Commands::Search { query, limit, json }) => {
            cli::ideas::search(&config.api_url, &query, limit, json).await
        }
        Some(Commands::Done { uuid, json }) => {
            cli::ideas::done(&config.api_url, &uuid, json).await
        }
        Some(Commands::Export { uuid, output }) => {
            cli::ideas::export(&config.api_url, &uuid, output).await
        }
        Some(Commands::Update { check, json }) => cli::update::update(&config.api_url, check, json).await,
    }
}
