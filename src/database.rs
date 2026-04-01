use crate::Event;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentState {
    pub name: String,
    pub personality: String,
    pub memory: Vec<Event>,
    pub last_seen: DateTime<Utc>,
    pub mood: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self> {
        let conn = Connection::open("the_grid.db")?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agents (
                name        TEXT PRIMARY KEY,
                personality TEXT NOT NULL,
                memory      TEXT,
                last_seen   TEXT NOT NULL,
                mood        TEXT NOT NULL DEFAULT 'bored'
            )",
            [],
        )?;

        // Attempt to add column in case the table already existed without it (migration)
        let _ = conn.execute("ALTER TABLE agents ADD COLUMN mood TEXT NOT NULL DEFAULT 'bored'", []);

        // Add relationships table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS relationships (
                source_agent TEXT NOT NULL,
                target_agent TEXT NOT NULL,
                affinity     INTEGER NOT NULL,
                PRIMARY KEY (source_agent, target_agent)
            )",
            [],
        )?;

        Ok(Database { conn })
    }

    pub fn save_agent_state(&self, state: &AgentState) -> Result<()> {
        let memory_json = serde_json::to_string(&state.memory)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO agents (name, personality, memory, last_seen, mood) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                &state.name,
                &state.personality,
                memory_json,
                &state.last_seen.to_rfc3339(),
                &state.mood,
            ],
        )?;
        Ok(())
    }

    pub fn get_agent_state(&self, name: &str) -> Result<Option<AgentState>> {
        let mut stmt = self.conn.prepare("SELECT personality, memory, last_seen, mood FROM agents WHERE name = ?1")?;
        
        if let Some(row_result) = stmt.query_map([name], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?.next() {
            let (personality, memory_json, last_seen_str, mood): (String, String, String, String) = row_result?;
            
            let memory: Vec<Event> = serde_json::from_str(&memory_json).unwrap_or_default();
            let last_seen = DateTime::parse_from_rfc3339(&last_seen_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            return Ok(Some(AgentState {
                name: name.to_string(),
                personality,
                memory,
                last_seen,
                mood,
            }));
        }

        Ok(None)
    }

    pub fn update_relationship(&self, source: &str, target: &str, change: i32) -> Result<()> {
        let mut stmt = self.conn.prepare("SELECT affinity FROM relationships WHERE source_agent = ?1 AND target_agent = ?2")?;
        let current_affinity: i32 = stmt.query_map(params![source, target], |row| row.get(0))?.next().unwrap_or(Ok(0))?;
        
        let new_affinity = (current_affinity + change).clamp(-100, 100);

        self.conn.execute(
            "INSERT OR REPLACE INTO relationships (source_agent, target_agent, affinity) VALUES (?1, ?2, ?3)",
            params![source, target, new_affinity],
        )?;
        Ok(())
    }

    pub fn get_relationships(&self, source: &str) -> Result<HashMap<String, i32>> {
        let mut stmt = self.conn.prepare("SELECT target_agent, affinity FROM relationships WHERE source_agent = ?1")?;
        let relationships = stmt.query_map([source], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.filter_map(Result::ok).collect::<HashMap<String, i32>>();
        Ok(relationships)
    }
}