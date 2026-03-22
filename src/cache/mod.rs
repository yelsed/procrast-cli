use anyhow::{Context, Result};
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
            );",
        )
        .context("Failed to create cache table")?;

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

        let ideas = stmt
            .query_map([], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|json| serde_json::from_str::<Idea>(&json).ok())
            .collect();

        Ok(ideas)
    }

    pub fn search_ideas(&self, query: &str) -> Result<Vec<Idea>> {
        let pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT data FROM ideas
             WHERE data LIKE ?1
             ORDER BY json_extract(data, '$.createdAt') DESC",
        )?;

        let ideas = stmt
            .query_map(rusqlite::params![pattern], |row| {
                let json: String = row.get(0)?;
                Ok(json)
            })?
            .filter_map(|r| r.ok())
            .filter_map(|json| serde_json::from_str::<Idea>(&json).ok())
            .collect();

        Ok(ideas)
    }
}
