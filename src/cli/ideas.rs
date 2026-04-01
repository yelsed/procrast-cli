use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;
use tabled::{Table, settings::Style};

use crate::api::client::{ApiClient, ApiError};
use crate::api::models::Idea;
use crate::auth;
use crate::cache::Cache;
use crate::markdown::idea_to_markdown;

fn get_client(api_url: &str) -> Result<ApiClient> {
    let token = auth::get_token()?.ok_or_else(|| {
        anyhow::anyhow!("Not logged in. Run `procrast login` first.")
    })?;
    Ok(ApiClient::new(api_url.to_string(), Some(token)))
}

fn handle_api_error(e: &anyhow::Error) -> bool {
    if let Some(api_err) = e.downcast_ref::<ApiError>() {
        if matches!(api_err, ApiError::Unauthorized) {
            let _ = auth::delete_token();
            eprintln!(
                "{} Session expired. Please run `procrast login` again.",
                "!".yellow().bold()
            );
            return true;
        }
    }
    false
}

fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        let end = max.saturating_sub(3);
        let truncated: String = chars[..end].iter().collect();
        format!("{}...", truncated)
    }
}

fn short_uuid(uuid: &str) -> &str {
    // UUIDs are ASCII hex, so byte slicing is safe here
    &uuid[..8.min(uuid.len())]
}

fn ideas_table(ideas: &[Idea]) -> String {
    let rows: Vec<[String; 5]> = ideas
        .iter()
        .map(|idea| {
            let title = idea
                .summary_title
                .as_deref()
                .unwrap_or_else(|| &idea.content);
            [
                short_uuid(&idea.uuid).to_string(),
                truncate(title, 50),
                idea.priority
                    .as_deref()
                    .unwrap_or("-")
                    .to_string(),
                if idea.completed_at.is_some() {
                    "done".to_string()
                } else {
                    match idea.refinement_status.as_deref() {
                        Some("completed") => "refined".to_string(),
                        Some(s) => s.to_string(),
                        None => "-".to_string(),
                    }
                },
                idea.created_at[..10.min(idea.created_at.len())].to_string(),
            ]
        })
        .collect();

    let mut rows_with_header = vec![
        ["UUID".to_string(), "Title".to_string(), "Priority".to_string(), "Status".to_string(), "Created".to_string()],
    ];
    rows_with_header.extend(rows);

    let mut table = Table::new(rows_with_header);
    table.with(Style::rounded());
    table.to_string()
}

pub async fn list(api_url: &str, limit: usize, json: bool, hide_done: bool) -> Result<()> {
    let client = get_client(api_url)?;
    let cache = Cache::open().ok();

    let (ideas, offline) = match client.list_ideas().await {
        Ok(ideas) => {
            if let Some(ref c) = cache {
                let _ = c.upsert_ideas(&ideas);
            }
            (ideas, false)
        }
        Err(e) => {
            if handle_api_error(&e) {
                return Ok(());
            }
            // Try cache fallback
            if let Some(ref c) = cache {
                let cached = c.get_all_ideas()?;
                if !cached.is_empty() {
                    (cached, true)
                } else {
                    return Err(e);
                }
            } else {
                return Err(e);
            }
        }
    };

    let ideas: Vec<_> = ideas
        .into_iter()
        .filter(|idea| !hide_done || idea.completed_at.is_none())
        .take(limit)
        .collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&ideas)?);
        return Ok(());
    }

    if offline {
        eprintln!(
            "{} {}",
            "!".yellow().bold(),
            "offline — showing cached data".yellow()
        );
    }

    if ideas.is_empty() {
        println!("No ideas found.");
        return Ok(());
    }

    println!("{}", ideas_table(&ideas));
    println!(
        "\n{} ideas. Use `procrast show <uuid>` to view details.",
        ideas.len()
    );
    Ok(())
}

pub async fn show(api_url: &str, uuid: &str, markdown: bool, json: bool) -> Result<()> {
    let client = get_client(api_url)?;
    let cache = Cache::open().ok();

    let (idea, offline) = match client.show_idea(uuid).await {
        Ok(idea) => {
            if let Some(ref c) = cache {
                let _ = c.upsert_ideas(&[idea.clone()]);
            }
            (idea, false)
        }
        Err(e) => {
            if handle_api_error(&e) {
                return Ok(());
            }
            // Try cache with prefix match
            if let Some(ref c) = cache {
                if let Some(cached) = c.get_idea(uuid)? {
                    (cached, true)
                } else {
                    return Err(e);
                }
            } else {
                return Err(e);
            }
        }
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&idea)?);
        return Ok(());
    }

    if markdown {
        print!("{}", idea_to_markdown(&idea));
        return Ok(());
    }

    if offline {
        eprintln!(
            "{} {}",
            "!".yellow().bold(),
            "offline — showing cached data".yellow()
        );
    }

    // Pretty print
    let title = idea
        .summary_title
        .as_deref()
        .unwrap_or("Untitled Idea");
    println!("{}", title.bold().underline());
    println!();
    println!("{}", idea.content);

    if let Some(ref refined) = idea.refined_content {
        if !refined.is_empty() {
            println!("\n{}", "Refined Version".cyan().bold());
            println!("{}", refined);
        }
    }

    if let Some(ref summary) = idea.refinement_summary {
        if !summary.is_empty() {
            println!("\n{}", "Summary".cyan().bold());
            println!("{}", summary);
        }
    }

    if let Some(ref steps) = idea.action_steps {
        if !steps.is_empty() {
            println!("\n{}", "Action Steps".cyan().bold());
            for (i, step) in steps.iter().enumerate() {
                println!("  {}. {}", i + 1, step);
            }
        }
    }

    if let Some(ref angles) = idea.creative_angles {
        if !angles.is_empty() {
            println!("\n{}", "Creative Angles".cyan().bold());
            for (i, angle) in angles.iter().enumerate() {
                println!("  {}. {}", i + 1, angle);
            }
        }
    }

    if let Some(ref questions) = idea.key_questions {
        if !questions.is_empty() {
            println!("\n{}", "Key Questions".cyan().bold());
            for (i, q) in questions.iter().enumerate() {
                println!("  {}. {}", i + 1, q);
            }
        }
    }

    println!("\n{}", "─".repeat(40).dimmed());
    let status = if idea.completed_at.is_some() {
        "done"
    } else {
        match idea.refinement_status.as_deref() {
            Some("completed") => "refined",
            Some(s) => s,
            None => "-",
        }
    };
    println!(
        "UUID: {} | Priority: {} | Status: {} | Created: {}",
        idea.uuid.dimmed(),
        idea.priority.as_deref().unwrap_or("-"),
        status,
        &idea.created_at[..10.min(idea.created_at.len())]
    );

    if let Some(ref completed) = idea.completed_at {
        println!(
            "Completed: {}",
            &completed[..completed.len().min(10)]
        );
    }

    Ok(())
}

pub async fn search(api_url: &str, query: &str, limit: usize, json: bool) -> Result<()> {
    let client = get_client(api_url)?;
    let cache = Cache::open().ok();

    let (ideas, offline) = match client.search_ideas(query).await {
        Ok(ideas) => (ideas, false),
        Err(e) => {
            if handle_api_error(&e) {
                return Ok(());
            }
            // Local search fallback
            if let Some(ref c) = cache {
                let cached = c.search_ideas(query)?;
                if !cached.is_empty() {
                    (cached, true)
                } else {
                    return Err(e);
                }
            } else {
                return Err(e);
            }
        }
    };

    let ideas: Vec<_> = ideas.into_iter().take(limit).collect();

    if json {
        println!("{}", serde_json::to_string_pretty(&ideas)?);
        return Ok(());
    }

    if offline {
        eprintln!(
            "{} {}",
            "!".yellow().bold(),
            "offline — searching cached data".yellow()
        );
    }

    if ideas.is_empty() {
        println!("No ideas found for \"{}\".", query);
        return Ok(());
    }

    println!("{}", ideas_table(&ideas));
    println!("\n{} results for \"{}\".", ideas.len(), query);
    Ok(())
}

pub async fn done(api_url: &str, uuid: &str, json: bool) -> Result<()> {
    let client = get_client(api_url)?;
    let cache = Cache::open().ok();

    // Resolve UUID prefix from cache if needed
    let full_uuid = if let Some(ref c) = cache {
        c.get_idea(uuid)?
            .map(|idea| idea.uuid)
            .unwrap_or_else(|| uuid.to_string())
    } else {
        uuid.to_string()
    };

    let idea = match client.toggle_complete(&full_uuid).await {
        Ok(idea) => idea,
        Err(e) => {
            if handle_api_error(&e) {
                return Ok(());
            }
            return Err(e);
        }
    };

    if let Some(ref c) = cache {
        let _ = c.upsert_ideas(&[idea.clone()]);
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&idea)?);
        return Ok(());
    }

    let title = idea
        .summary_title
        .as_deref()
        .unwrap_or_else(|| &idea.content);

    if idea.completed_at.is_some() {
        println!(
            "{} Marked as done: {}",
            "✓".green().bold(),
            truncate(title, 60)
        );
    } else {
        println!(
            "{} Unmarked: {}",
            "☐".yellow().bold(),
            truncate(title, 60)
        );
    }

    Ok(())
}

pub async fn export(api_url: &str, uuid: &str, output: Option<PathBuf>) -> Result<()> {
    let client = get_client(api_url)?;
    let cache = Cache::open().ok();

    let idea = match client.show_idea(uuid).await {
        Ok(idea) => {
            if let Some(ref c) = cache {
                let _ = c.upsert_ideas(&[idea.clone()]);
            }
            idea
        }
        Err(e) => {
            if handle_api_error(&e) {
                return Ok(());
            }
            if let Some(ref c) = cache {
                c.get_idea(uuid)?
                    .context("Idea not found in cache")?
            } else {
                return Err(e);
            }
        }
    };

    let md = idea_to_markdown(&idea);
    let path = output.unwrap_or_else(|| {
        let slug = idea
            .summary_title
            .as_deref()
            .unwrap_or("idea")
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
            .trim_matches('-')
            .to_string();
        PathBuf::from(format!("{}.md", slug))
    });

    std::fs::write(&path, md)?;
    println!(
        "{} Exported to {}",
        "✓".green().bold(),
        path.display()
    );
    Ok(())
}
