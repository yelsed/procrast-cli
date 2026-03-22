use anyhow::{Context, Result};
use chrono::{NaiveDateTime, Utc};
use rusqlite::Connection;

use crate::api::models::Idea;

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn open() -> Result<Self> {
        let dir = dirs::cache_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
            .join("procrast-cli");
        std::fs::create_dir_all(&dir)?;

        let db_path = dir.join("cache.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open cache database")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ideas (
                uuid TEXT PRIMARY KEY,
                data TEXT NOT NULL,
                fetched_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )
        .context("Failed to create cache tables")?;

        Ok(Self { conn })
    }

    pub fn upsert_ideas(&self, ideas: &[Idea]) -> Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO ideas (uuid, data, fetched_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(uuid) DO UPDATE SET data = ?2, fetched_at = datetime('now')",
        )?;

        for idea in ideas {
            let json = serde_json::to_string(idea)?;
            stmt.execute(rusqlite::params![idea.uuid, json])?;
        }
        Ok(())
    }

    pub fn get_idea(&self, uuid_prefix: &str) -> Result<Option<Idea>> {
        let mut stmt = self.conn.prepare(
            "SELECT data FROM ideas WHERE uuid LIKE ?1 || '%' LIMIT 1",
        )?;

        let result = stmt.query_row(rusqlite::params![uuid_prefix], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let idea: Idea = serde_json::from_str(&json)?;
                Ok(Some(idea))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_all_ideas(&self) -> Result<Vec<Idea>> {
        let mut stmt = self
            .conn
            .prepare("SELECT data FROM ideas ORDER BY json_extract(data, '$.createdAt') DESC")?;

        let mut ideas = Vec::new();
        let mut skipped = 0u32;
        let rows = stmt.query_map([], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        for row in rows {
            match row {
                Ok(json) => match serde_json::from_str::<Idea>(&json) {
                    Ok(idea) => ideas.push(idea),
                    Err(_) => skipped += 1,
                },
                Err(_) => skipped += 1,
            }
        }

        if skipped > 0 {
            eprintln!("warning: skipped {} corrupted cache entries", skipped);
        }

        Ok(ideas)
    }

    pub fn set_last_synced(&self) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES ('last_synced_at', datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = datetime('now')",
            [],
        )?;
        Ok(())
    }

    pub fn get_last_synced(&self) -> Option<String> {
        // Try meta table first, fall back to most recent fetched_at from ideas
        self.conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'last_synced_at'",
                [],
                |row| row.get(0),
            )
            .ok()
            .or_else(|| {
                self.conn
                    .query_row(
                        "SELECT MAX(fetched_at) FROM ideas",
                        [],
                        |row| row.get(0),
                    )
                    .ok()
                    .flatten()
            })
    }

    pub fn last_synced_age_text(&self) -> Option<String> {
        let timestamp_str: String = self.get_last_synced()?;
        let synced_at = NaiveDateTime::parse_from_str(&timestamp_str, "%Y-%m-%d %H:%M:%S").ok()?;
        let now = Utc::now().naive_utc();
        let duration = now.signed_duration_since(synced_at);

        let text = if duration.num_days() > 0 {
            let days = duration.num_days();
            if days == 1 { "1 day ago".to_string() } else { format!("{days} days ago") }
        } else if duration.num_hours() > 0 {
            let hours = duration.num_hours();
            if hours == 1 { "1 hour ago".to_string() } else { format!("{hours} hours ago") }
        } else {
            let mins = duration.num_minutes();
            if mins <= 1 { "just now".to_string() } else { format!("{mins} minutes ago") }
        };
        Some(text)
    }

    pub fn search_ideas(&self, query: &str) -> Result<Vec<Idea>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT data FROM ideas
             WHERE data LIKE ?1
             ORDER BY json_extract(data, '$.createdAt') DESC",
        )?;

        let mut ideas = Vec::new();
        let mut skipped = 0u32;
        let rows = stmt.query_map(rusqlite::params![pattern], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        })?;

        for row in rows {
            match row {
                Ok(json) => match serde_json::from_str::<Idea>(&json) {
                    Ok(idea) => ideas.push(idea),
                    Err(_) => skipped += 1,
                },
                Err(_) => skipped += 1,
            }
        }

        if skipped > 0 {
            eprintln!("warning: skipped {} corrupted cache entries", skipped);
        }

        Ok(ideas)
    }
}
