use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

use super::schema::MIGRATIONS;
use crate::mcp::{McpServer, McpStatus};
use crate::models::{Connection as AgentConnection, Model};
use crate::plugins::Plugin;
use crate::sessions::{Project, Session, SessionConfidence};
use crate::skills::{Skill, SkillScope};

fn mcp_status_str(s: McpStatus) -> &'static str {
    match s {
        McpStatus::Connected => "connected",
        McpStatus::Configured => "configured",
        McpStatus::Offline => "offline",
        McpStatus::Error => "error",
        McpStatus::Unknown => "unknown",
    }
}

fn parse_mcp_status(s: &str) -> McpStatus {
    match s {
        "connected" => McpStatus::Connected,
        "configured" => McpStatus::Configured,
        "offline" => McpStatus::Offline,
        "error" => McpStatus::Error,
        _ => McpStatus::Unknown,
    }
}

pub struct Db(Mutex<Connection>);

pub fn migrate(conn: &Connection) -> Result<(), String> {
    let current: u32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    for (sql, version) in MIGRATIONS {
        if *version > current {
            conn.execute_batch("BEGIN").map_err(|e| e.to_string())?;
            let applied = (|| -> Result<(), rusqlite::Error> {
                conn.execute_batch(sql)?;
                conn.execute_batch(&format!("PRAGMA user_version = {version}"))?;
                Ok(())
            })();
            match applied {
                Ok(()) => conn.execute_batch("COMMIT").map_err(|e| e.to_string())?,
                Err(e) => {
                    let _ = conn.execute_batch("ROLLBACK");
                    return Err(e.to_string());
                }
            }
        }
    }
    Ok(())
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA foreign_keys = ON")
            .map_err(|e| e.to_string())?;
        migrate(&conn)?;
        Ok(Db(Mutex::new(conn)))
    }

    pub fn upsert_agent(&self, row: &AgentRow) -> Result<(), String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let now = now_ts();
        conn.execute(
            "INSERT INTO agents (id, name, type, version, executable_path, installed, running, last_seen, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, type = excluded.type,
               version = excluded.version, executable_path = excluded.executable_path,
               installed = excluded.installed, running = excluded.running,
               last_seen = excluded.last_seen, updated_at = excluded.updated_at",
            params![
                row.id,
                row.name,
                row.agent_type,
                row.version,
                row.executable_path,
                row.installed,
                row.running,
                row.last_seen,
                now,
                now
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_agent(&self, id: &str) -> Result<Option<AgentRow>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT id, name, type, version, executable_path, installed, running, last_seen FROM agents WHERE id = ?1",
            [id],
            row_to_agent,
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn list_agents(&self) -> Result<Vec<AgentRow>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, name, type, version, executable_path, installed, running, last_seen FROM agents ORDER BY name")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], row_to_agent)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub fn insert_event(
        &self,
        level: &str,
        agent_id: Option<&str>,
        event: &str,
        message: &str,
    ) -> Result<i64, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO events (ts, level, agent_id, event, message) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![now_ts(), level, agent_id, event, message],
        )
        .map_err(|e| e.to_string())?;
        Ok(conn.last_insert_rowid())
    }

    #[allow(dead_code)] // Read by the logs/events UI (Phase 13+).
    pub fn list_recent_events(&self, limit: i64) -> Result<Vec<EventRow>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, ts, level, agent_id, event, message FROM events ORDER BY ts DESC, id DESC LIMIT ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([limit], |row| {
                Ok(EventRow {
                    id: row.get(0)?,
                    ts: row.get(1)?,
                    level: row.get(2)?,
                    agent_id: row.get(3)?,
                    event: row.get(4)?,
                    message: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Replace one agent's snapshot atomically (idempotent refresh).
    pub fn replace_agent_servers(
        &self,
        agent_id: &str,
        servers: &[McpServer],
    ) -> Result<(), String> {
        let mut conn = self.0.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM mcp_servers WHERE agent_id = ?1", [agent_id])
            .map_err(|e| e.to_string())?;
        for s in servers {
            let args_json = serde_json::to_string(&s.args).map_err(|e| e.to_string())?;
            tx.execute(
                "INSERT INTO mcp_servers (id, name, transport, command, args, url, env_count, source, status, agent_id, project_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    s.id,
                    s.name,
                    s.transport,
                    s.command,
                    args_json,
                    s.url,
                    s.env_count,
                    s.source,
                    mcp_status_str(s.status),
                    s.agent_id,
                    s.project_id
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_mcp_servers(&self, agent_id: Option<&str>) -> Result<Vec<McpServer>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let sql = match agent_id {
            Some(_) => "SELECT id, name, transport, command, args, url, env_count, source, status, agent_id, project_id FROM mcp_servers WHERE agent_id = ?1 ORDER BY name",
            None => "SELECT id, name, transport, command, args, url, env_count, source, status, agent_id, project_id FROM mcp_servers ORDER BY agent_id, name",
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows: Vec<McpServer> = match agent_id {
            Some(id) => stmt
                .query_map([id], row_to_mcp)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?,
            None => stmt
                .query_map([], row_to_mcp)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?,
        };
        return Ok(rows);

        fn row_to_mcp(row: &rusqlite::Row) -> rusqlite::Result<McpServer> {
            let args_json: String = row.get(4)?;
            let status: String = row.get(8)?;
            Ok(McpServer {
                id: row.get(0)?,
                name: row.get(1)?,
                transport: row.get(2)?,
                command: row.get(3)?,
                args: serde_json::from_str(&args_json).unwrap_or_default(),
                url: row.get(5)?,
                env_count: row.get(6)?,
                source: row.get(7)?,
                status: parse_mcp_status(&status),
                agent_id: row.get(9)?,
                project_id: row.get(10)?,
            })
        }
    }
    /// Replace one agent's skill snapshot atomically (idempotent refresh).
    pub fn replace_agent_skills(&self, agent_id: &str, skills: &[Skill]) -> Result<(), String> {
        let mut conn = self.0.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM skills WHERE agent_id = ?1", [agent_id])
            .map_err(|e| e.to_string())?;
        for s in skills {
            let scope = match s.scope {
                SkillScope::Global => "global",
                SkillScope::Project => "project",
            };
            tx.execute(
                "INSERT INTO skills (id, name, description, path, scope, source, agent_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    s.id,
                    s.name,
                    s.description,
                    s.path,
                    scope,
                    s.source,
                    s.agent_id
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_skills(&self, agent_id: Option<&str>) -> Result<Vec<Skill>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let sql = match agent_id {
            Some(_) => "SELECT id, name, description, path, scope, source, agent_id FROM skills WHERE agent_id = ?1 ORDER BY name",
            None => "SELECT id, name, description, path, scope, source, agent_id FROM skills ORDER BY agent_id, name",
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let map = |row: &rusqlite::Row| {
            let scope: String = row.get(4)?;
            Ok(Skill {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                path: row.get(3)?,
                scope: match scope.as_str() {
                    "project" => SkillScope::Project,
                    "global" => SkillScope::Global,
                    other => {
                        return Err(rusqlite::Error::InvalidColumnType(
                            4,
                            other.to_string(),
                            rusqlite::types::Type::Text,
                        ))
                    }
                },
                source: row.get(5)?,
                agent_id: row.get(6)?,
            })
        };
        match agent_id {
            Some(id) => stmt
                .query_map([id], map)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string()),
            None => stmt
                .query_map([], map)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string()),
        }
    }
    /// Replace one agent's plugin snapshot atomically (idempotent refresh).
    pub fn replace_agent_plugins(&self, agent_id: &str, plugins: &[Plugin]) -> Result<(), String> {
        let mut conn = self.0.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM plugins WHERE agent_id = ?1", [agent_id])
            .map_err(|e| e.to_string())?;
        for p in plugins {
            tx.execute(
                "INSERT INTO plugins (id, name, version, path, source, enabled, agent_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![p.id, p.name, p.version, p.path, p.source, p.enabled, p.agent_id],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_plugins(&self, agent_id: Option<&str>) -> Result<Vec<Plugin>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let sql = match agent_id {
            Some(_) => "SELECT id, name, version, path, source, enabled, agent_id FROM plugins WHERE agent_id = ?1 ORDER BY name",
            None => "SELECT id, name, version, path, source, enabled, agent_id FROM plugins ORDER BY agent_id, name",
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let map = |row: &rusqlite::Row| {
            Ok(Plugin {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                path: row.get(3)?,
                source: row.get(4)?,
                enabled: row.get(5)?,
                agent_id: row.get(6)?,
            })
        };
        match agent_id {
            Some(id) => stmt
                .query_map([id], map)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string()),
            None => stmt
                .query_map([], map)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string()),
        }
    }
    /// Replace one agent's model snapshot atomically (idempotent refresh).
    pub fn replace_agent_models(&self, agent_id: &str, models: &[Model]) -> Result<(), String> {
        let mut conn = self.0.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM models WHERE agent_id = ?1", [agent_id])
            .map_err(|e| e.to_string())?;
        for m in models {
            tx.execute(
                "INSERT INTO models (id, name, provider, agent_id) VALUES (?1, ?2, ?3, ?4)",
                params![m.id, m.name, m.provider, m.agent_id],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_models(&self, agent_id: Option<&str>) -> Result<Vec<Model>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let sql = match agent_id {
            Some(_) => {
                "SELECT id, name, provider, agent_id FROM models WHERE agent_id = ?1 ORDER BY name"
            }
            None => "SELECT id, name, provider, agent_id FROM models ORDER BY agent_id, name",
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let map = |row: &rusqlite::Row| {
            Ok(Model {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                agent_id: row.get(3)?,
            })
        };
        match agent_id {
            Some(id) => stmt
                .query_map([id], map)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string()),
            None => stmt
                .query_map([], map)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string()),
        }
    }

    /// Replace one agent's connection snapshot atomically (idempotent refresh).
    /// Only key names are stored; values are never collected.
    pub fn replace_agent_connections(
        &self,
        agent_id: &str,
        connections: &[AgentConnection],
    ) -> Result<(), String> {
        let mut conn = self.0.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM connections WHERE agent_id = ?1", [agent_id])
            .map_err(|e| e.to_string())?;
        for c in connections {
            tx.execute(
                "INSERT INTO connections (id, name, provider, configured, agent_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![c.id, c.name, c.provider, c.configured, c.agent_id],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_connections(&self, agent_id: Option<&str>) -> Result<Vec<AgentConnection>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let sql = match agent_id {
            Some(_) => "SELECT id, name, provider, configured, agent_id FROM connections WHERE agent_id = ?1 ORDER BY name",
            None => "SELECT id, name, provider, configured, agent_id FROM connections ORDER BY agent_id, name",
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let map = |row: &rusqlite::Row| {
            Ok(AgentConnection {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                configured: row.get(3)?,
                agent_id: row.get(4)?,
            })
        };
        match agent_id {
            Some(id) => stmt
                .query_map([id], map)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string()),
            None => stmt
                .query_map([], map)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string()),
        }
    }
    /// Upsert sessions by id (history-preserving; refresh is idempotent).
    pub fn upsert_sessions(&self, sessions: &[Session]) -> Result<(), String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        for s in sessions {
            conn.execute(
                "INSERT INTO sessions (id, agent_id, project_id, status, model, started_at, last_activity, confidence)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(id) DO UPDATE SET project_id = excluded.project_id, status = excluded.status,
                   model = excluded.model, started_at = excluded.started_at,
                   last_activity = excluded.last_activity, confidence = excluded.confidence",
                params![
                    s.id,
                    s.agent_id,
                    s.project_id,
                    s.status,
                    s.model,
                    s.started_at,
                    s.last_activity,
                    confidence_str(s.confidence)
                ],
            )
            .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn list_sessions(&self, agent_id: Option<&str>) -> Result<Vec<Session>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let sql = match agent_id {
            Some(_) => "SELECT id, agent_id, project_id, status, model, started_at, last_activity, confidence FROM sessions WHERE agent_id = ?1 ORDER BY last_activity DESC NULLS LAST, id",
            None => "SELECT id, agent_id, project_id, status, model, started_at, last_activity, confidence FROM sessions ORDER BY last_activity DESC NULLS LAST, id",
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let map = |row: &rusqlite::Row| {
            let confidence: String = row.get(7)?;
            Ok(Session {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                project_id: row.get(2)?,
                status: row.get(3)?,
                model: row.get(4)?,
                started_at: row.get(5)?,
                last_activity: row.get(6)?,
                confidence: parse_confidence(&confidence),
            })
        };
        match agent_id {
            Some(id) => {
                let rows = stmt
                    .query_map([id], map)
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                Ok(rows)
            }
            None => {
                let rows = stmt
                    .query_map([], map)
                    .map_err(|e| e.to_string())?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                Ok(rows)
            }
        }
    }

    /// Replace the whole project set atomically.
    pub fn replace_projects(&self, projects: &[Project]) -> Result<(), String> {
        let mut conn = self.0.lock().map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM projects", [])
            .map_err(|e| e.to_string())?;
        for p in projects {
            tx.execute(
                "INSERT INTO projects (id, name, path, last_seen) VALUES (?1, ?2, ?3, ?4)",
                params![p.id, p.name, p.path, p.last_seen],
            )
            .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_projects(&self) -> Result<Vec<Project>, String> {
        let ids: Vec<(String, String, String, Option<i64>)> = {
            let conn = self.0.lock().map_err(|e| e.to_string())?;
            let mut stmt = conn
                .prepare("SELECT id, name, path, last_seen FROM projects ORDER BY name")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                })
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            rows
        };
        let mut out = vec![];
        for (id, name, path, last_seen) in ids {
            out.push(Project {
                agent_ids: self.project_agent_ids(&id)?,
                id,
                name,
                path,
                last_seen,
            });
        }
        return Ok(out);
    }

    /// Agents linked to a project, derived from sessions (no stored column).
    pub fn project_agent_ids(&self, project_id: &str) -> Result<Vec<String>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT DISTINCT agent_id FROM sessions WHERE project_id = ?1 ORDER BY agent_id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([project_id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Mark sessions stopped so a disappearance emits exactly once.
    /// Reappearing transcripts overwrite status via upsert (back to unknown).
    pub fn mark_sessions_stopped(&self, ids: &[String]) -> Result<(), String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        for id in ids {
            conn.execute("UPDATE sessions SET status = 'stopped' WHERE id = ?1", [id])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Ids of sessions not already marked stopped (stopped-emit candidates).
    pub fn unstopped_session_ids(&self, agent_id: &str) -> Result<Vec<String>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id FROM sessions WHERE agent_id = ?1 AND status != 'stopped'")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([agent_id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn list_settings(&self) -> Result<Vec<(String, String)>, String> {
        let conn = self.0.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT key, value FROM settings ORDER BY key")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }
}

fn confidence_str(c: SessionConfidence) -> &'static str {
    match c {
        SessionConfidence::Confirmed => "confirmed",
        SessionConfidence::Detected => "detected",
        SessionConfidence::Estimated => "estimated",
        SessionConfidence::Unknown => "unknown",
    }
}

fn parse_confidence(s: &str) -> SessionConfidence {
    match s {
        "confirmed" => SessionConfidence::Confirmed,
        "detected" => SessionConfidence::Detected,
        "estimated" => SessionConfidence::Estimated,
        _ => SessionConfidence::Unknown,
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn row_to_agent(row: &rusqlite::Row) -> rusqlite::Result<AgentRow> {
    Ok(AgentRow {
        id: row.get(0)?,
        name: row.get(1)?,
        agent_type: row.get(2)?,
        version: row.get(3)?,
        executable_path: row.get(4)?,
        installed: row.get(5)?,
        running: row.get(6)?,
        last_seen: row.get(7)?,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AgentRow {
    pub id: String,
    pub name: String,
    pub agent_type: String,
    pub version: Option<String>,
    pub executable_path: Option<String>,
    pub installed: bool,
    pub running: bool,
    pub last_seen: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[allow(dead_code)] // Read by the logs/events UI (Phase 13+).
pub struct EventRow {
    pub id: i64,
    pub ts: i64,
    pub level: String,
    pub agent_id: Option<String>,
    pub event: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::{AgentRow, Db};
    use std::path::PathBuf;

    fn fresh_path(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-test-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn table_count(path: &PathBuf, table: &str) -> i64 {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .unwrap()
    }

    #[test]
    fn test_open_creates_parent_dirs() {
        let mut path = fresh_path("dirs");
        path.push("a");
        path.push("b");
        path.push("test.db");
        let _db = Db::open(&path).unwrap();
        assert!(path.exists(), "db file should exist with parents created");
    }

    #[test]
    fn test_migrate_is_idempotent() {
        let mut path = fresh_path("idem");
        path.push("test.db");
        let _db = Db::open(&path).unwrap();
        let v1: u32 = rusqlite::Connection::open(&path)
            .unwrap()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        let _db2 = Db::open(&path).unwrap();
        let v2: u32 = rusqlite::Connection::open(&path)
            .unwrap()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert!(v1 > 0, "migrations should bump user_version");
        assert_eq!(v1, v2, "second migrate must change nothing");
    }

    #[test]
    fn test_open_corrupt_file_errors() {
        let mut path = fresh_path("corrupt");
        path.push("test.db");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"this is not a database").unwrap();
        assert!(
            Db::open(&path).is_err(),
            "corrupt file must error, not panic"
        );
    }

    #[test]
    fn test_agents_table_exists() {
        let mut path = fresh_path("agents");
        path.push("test.db");
        let _db = Db::open(&path).unwrap();
        assert_eq!(table_count(&path, "agents"), 1);
    }

    fn all_tables(path: &PathBuf) -> Vec<String> {
        let conn = rusqlite::Connection::open(path).unwrap();
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
            .unwrap();
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn test_all_twelve_tables_exist() {
        let mut path = fresh_path("twelve");
        path.push("test.db");
        let _db = Db::open(&path).unwrap();
        let mut tables = all_tables(&path);
        tables.sort();
        let mut expected = vec![
            "agent_processes",
            "agents",
            "connections",
            "events",
            "mcp_servers",
            "models",
            "plugins",
            "projects",
            "resource_snapshots",
            "sessions",
            "settings",
            "skills",
            "subagents",
        ];
        expected.sort();
        assert_eq!(tables, expected);
    }

    #[test]
    fn test_foreign_keys_enforced() {
        let mut path = fresh_path("fk");
        path.push("test.db");
        let _db = Db::open(&path).unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        let now = 1_700_000_000i64;
        let res = conn.execute(
            "INSERT INTO agent_processes (agent_id, pid, name, cpu, ram, started_at) VALUES ('ghost', 1, 'x', 0.0, 0, ?1)",
            [now],
        );
        assert!(res.is_err(), "child row with missing parent must fail");
    }

    #[test]
    fn test_migrate_from_v1_applies_all() {
        let mut path = fresh_path("v1upgrade");
        path.push("test.db");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE agents (id TEXT PRIMARY KEY, name TEXT NOT NULL, type TEXT NOT NULL); PRAGMA user_version = 1;",
        )
        .unwrap();
        drop(conn);
        let _db = Db::open(&path).unwrap();
        let tables = all_tables(&path);
        assert_eq!(
            tables.len(),
            13,
            "v1 db must gain all tables, got {tables:?}"
        );
        let v: u32 = rusqlite::Connection::open(&path)
            .unwrap()
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(v, 6);
    }

    fn sample_agent(id: &str, name: &str) -> super::AgentRow {
        super::AgentRow {
            id: id.to_string(),
            name: name.to_string(),
            agent_type: "claude".to_string(),
            version: Some("1.0".to_string()),
            executable_path: None,
            installed: true,
            running: false,
            last_seen: Some(1_700_000_000),
        }
    }

    #[test]
    fn test_upsert_then_get_roundtrip() {
        let mut path = fresh_path("roundtrip");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        db.upsert_agent(&sample_agent("claude", "Claude Code"))
            .unwrap();
        let got = db.get_agent("claude").unwrap().unwrap();
        assert_eq!(got.name, "Claude Code");
        assert_eq!(got.version.as_deref(), Some("1.0"));
        assert!(got.installed);
        assert!(!got.running);
        assert!(db.get_agent("missing").unwrap().is_none());
    }

    #[test]
    fn test_upsert_updates_existing() {
        let mut path = fresh_path("upsert2");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        db.upsert_agent(&sample_agent("a", "A")).unwrap();
        let mut updated = sample_agent("a", "A");
        updated.version = Some("2.0".to_string());
        updated.running = true;
        db.upsert_agent(&updated).unwrap();
        let got = db.get_agent("a").unwrap().unwrap();
        assert_eq!(got.version.as_deref(), Some("2.0"));
        assert!(got.running);
        assert_eq!(db.list_agents().unwrap().len(), 1);
    }

    #[test]
    fn test_list_agents_ordered() {
        let mut path = fresh_path("ordered");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        db.upsert_agent(&sample_agent("b", "Beta")).unwrap();
        db.upsert_agent(&sample_agent("a", "Alpha")).unwrap();
        let names: Vec<String> = db
            .list_agents()
            .unwrap()
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert_eq!(names, vec!["Alpha".to_string(), "Beta".to_string()]);
    }

    #[test]
    fn test_db_enforces_fk_via_api() {
        let mut path = fresh_path("fkv2");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        let res = db.insert_event("info", Some("ghost"), "x", "ghost parent");
        assert!(
            res.is_err(),
            "Db connection itself must enforce FKs (deleting the open() pragma must fail this test)"
        );
    }

    #[test]
    fn test_events_insert_and_recent_order() {
        let mut path = fresh_path("events");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        db.insert_event("info", None, "a", "first").unwrap();
        db.insert_event("warn", None, "b", "second").unwrap();
        db.insert_event("error", None, "c", "third").unwrap();
        let recent = db.list_recent_events(2).unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].message, "third");
        assert_eq!(recent[1].message, "second");
    }

    fn sample_server(agent: &str, name: &str) -> crate::mcp::McpServer {
        crate::mcp::McpServer {
            id: format!("{agent}:{name}"),
            name: name.to_string(),
            transport: "stdio".to_string(),
            command: Some("c".to_string()),
            args: vec!["a".to_string()],
            url: None,
            env_count: 1,
            source: "test".to_string(),
            status: crate::mcp::McpStatus::Configured,
            agent_id: agent.to_string(),
            project_id: None,
        }
    }

    #[test]
    fn test_replace_is_idempotent() {
        let mut path = fresh_path("mcpidem");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        let servers = vec![sample_server("a", "x"), sample_server("a", "y")];
        db.replace_agent_servers("a", &servers).unwrap();
        db.replace_agent_servers("a", &servers).unwrap();
        let rows = db.list_mcp_servers(Some("a")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "x");
        assert_eq!(rows[0].args, vec!["a".to_string()]);
        assert_eq!(rows[0].env_count, 1);
        assert!(matches!(rows[0].status, crate::mcp::McpStatus::Configured));
        assert_eq!(db.list_mcp_servers(None).unwrap().len(), 2);
    }

    fn sample_skill(agent: &str, name: &str) -> crate::skills::Skill {
        crate::skills::Skill {
            id: format!("{agent}:global:{name}"),
            name: name.to_string(),
            description: Some("d".to_string()),
            path: Some("p".to_string()),
            scope: crate::skills::SkillScope::Global,
            source: Some("s".to_string()),
            agent_id: agent.to_string(),
        }
    }

    #[test]
    fn test_replace_skills_roundtrip_idempotent() {
        let mut path = fresh_path("skillidem");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        let skills = vec![sample_skill("a", "x"), sample_skill("a", "y")];
        db.replace_agent_skills("a", &skills).unwrap();
        db.replace_agent_skills("a", &skills).unwrap();
        let rows = db.list_skills(Some("a")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "x");
        assert_eq!(rows[0].description.as_deref(), Some("d"));
        assert_eq!(rows[0].path.as_deref(), Some("p"));
        assert_eq!(rows[0].source.as_deref(), Some("s"));
        assert_eq!(db.list_skills(None).unwrap().len(), 2);
    }

    fn sample_plugin(agent: &str, name: &str) -> crate::plugins::Plugin {
        crate::plugins::Plugin {
            id: format!("{agent}:{name}"),
            name: name.to_string(),
            version: Some("1.0".to_string()),
            path: Some("p".to_string()),
            source: Some("s".to_string()),
            enabled: true,
            agent_id: agent.to_string(),
        }
    }

    #[test]
    fn test_replace_plugins_roundtrip_idempotent() {
        let mut path = fresh_path("pluginidem");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        let plugins = vec![sample_plugin("a", "x"), sample_plugin("a", "y")];
        db.replace_agent_plugins("a", &plugins).unwrap();
        db.replace_agent_plugins("a", &plugins).unwrap();
        let rows = db.list_plugins(Some("a")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "x");
        assert_eq!(rows[0].version.as_deref(), Some("1.0"));
        assert_eq!(rows[0].path.as_deref(), Some("p"));
        assert_eq!(rows[0].source.as_deref(), Some("s"));
        assert!(rows[0].enabled);
        assert_eq!(db.list_plugins(None).unwrap().len(), 2);
    }

    fn sample_model(agent: &str, name: &str) -> crate::models::Model {
        crate::models::Model {
            id: format!("{agent}:{name}"),
            name: name.to_string(),
            provider: Some("p".to_string()),
            agent_id: agent.to_string(),
        }
    }

    fn sample_connection(agent: &str, name: &str) -> crate::models::Connection {
        crate::models::Connection {
            id: format!("{agent}:{name}"),
            name: name.to_string(),
            provider: "env".to_string(),
            configured: true,
            agent_id: agent.to_string(),
        }
    }

    #[test]
    fn test_replace_models_roundtrip_idempotent() {
        let mut path = fresh_path("modelidem");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        let models = vec![sample_model("a", "x"), sample_model("a", "y")];
        db.replace_agent_models("a", &models).unwrap();
        db.replace_agent_models("a", &models).unwrap();
        let rows = db.list_models(Some("a")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "x");
        assert_eq!(rows[0].provider.as_deref(), Some("p"));
        assert_eq!(db.list_models(None).unwrap().len(), 2);
    }

    #[test]
    fn test_replace_connections_roundtrip_idempotent() {
        let mut path = fresh_path("connidem");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        let conns = vec![sample_connection("a", "x"), sample_connection("a", "y")];
        db.replace_agent_connections("a", &conns).unwrap();
        db.replace_agent_connections("a", &conns).unwrap();
        let rows = db.list_connections(Some("a")).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "x");
        assert_eq!(rows[0].provider, "env");
        assert!(rows[0].configured);
        assert_eq!(db.list_connections(None).unwrap().len(), 2);
    }

    fn sample_session(agent: &str, id: &str, project: Option<&str>) -> crate::sessions::Session {
        crate::sessions::Session {
            id: format!("{agent}:{id}"),
            agent_id: agent.to_string(),
            project_id: project.map(|s| s.to_string()),
            status: "unknown".to_string(),
            model: Some("m".to_string()),
            started_at: Some(100),
            last_activity: Some(200),
            confidence: crate::sessions::SessionConfidence::Detected,
        }
    }

    fn seed_agent(db: &Db, id: &str) {
        db.upsert_agent(&AgentRow {
            id: id.to_string(),
            name: id.to_string(),
            agent_type: id.to_string(),
            version: None,
            executable_path: None,
            installed: true,
            running: false,
            last_seen: None,
        })
        .unwrap();
    }

    #[test]
    fn test_upsert_sessions_no_duplicates() {
        let mut path = fresh_path("sessidem");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        seed_agent(&db, "a");
        db.upsert_sessions(&[sample_session("a", "s1", None)])
            .unwrap();
        db.upsert_sessions(&[sample_session("a", "s1", None)])
            .unwrap();
        let rows = db.list_sessions(Some("a")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].confidence,
            crate::sessions::SessionConfidence::Detected
        );
    }

    #[test]
    fn test_stopped_marks_emit_once() {
        let mut path = fresh_path("stoppedonce");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        seed_agent(&db, "a");
        db.upsert_sessions(&[sample_session("a", "s1", None)])
            .unwrap();
        assert_eq!(db.unstopped_session_ids("a").unwrap().len(), 1);
        db.mark_sessions_stopped(&["a:s1".to_string()]).unwrap();
        assert!(db.unstopped_session_ids("a").unwrap().is_empty());
        // Transcript reappears → upsert restores unknown → candidate again.
        db.upsert_sessions(&[sample_session("a", "s1", None)])
            .unwrap();
        assert_eq!(db.unstopped_session_ids("a").unwrap().len(), 1);
    }

    #[test]
    fn test_project_agents_derive_from_sessions() {
        let mut path = fresh_path("projagents");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        seed_agent(&db, "a");
        seed_agent(&db, "b");
        db.upsert_sessions(&[
            sample_session("a", "s1", Some("p1")),
            sample_session("b", "s2", Some("p1")),
        ])
        .unwrap();
        assert_eq!(db.project_agent_ids("p1").unwrap(), vec!["a", "b"]);
    }

    #[test]
    fn test_006_keeps_rows_drops_unique() {
        let mut path = fresh_path("m006");
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        let mk = |id: &str| crate::sessions::Project {
            id: id.to_string(),
            name: "n".to_string(),
            path: "E:/same".to_string(),
            last_seen: None,
            agent_ids: vec![],
        };
        db.replace_projects(&[mk("a:p1")]).unwrap();
        // Same path, second agent: allowed only after 006 dropped UNIQUE.
        db.replace_projects(&[mk("a:p1"), mk("b:p2")]).unwrap();
        assert_eq!(db.list_projects().unwrap().len(), 2);
    }
}
