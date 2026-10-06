use std::collections::HashSet;

use serde::Serialize;
use tauri::Emitter;

/// Frontend live-event channel.
pub const FRONTEND_EVENT: &str = "agenthq://event";

#[derive(Debug, Clone, PartialEq)]
pub struct ChangeSet {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

/// Order-insensitive id-set diff with sorted, deduped output.
pub fn diff_ids(old: &[String], new: &[String]) -> ChangeSet {
    let old_set: HashSet<&String> = old.iter().collect();
    let new_set: HashSet<&String> = new.iter().collect();
    let mut added: Vec<String> = new_set.difference(&old_set).map(|s| (*s).clone()).collect();
    let mut removed: Vec<String> = old_set.difference(&new_set).map(|s| (*s).clone()).collect();
    added.sort();
    removed.sort();
    ChangeSet { added, removed }
}

/// Names/ids/counts only — never paths or secrets.
pub fn session_started_msg(id: &str) -> String {
    format!("session {id} started")
}

/// Names/ids/counts only — never paths or secrets.
pub fn session_stopped_msg(id: &str) -> String {
    format!("session {id} stopped")
}

/// Names/ids/counts only — never paths or secrets.
pub fn collection_changed_msg(kind: &str, added: usize, removed: usize) -> String {
    format!("{kind} changed: {added} added, {removed} removed")
}

#[derive(Debug, Clone, Serialize)]
pub struct EmittedEvent {
    pub id: i64,
    pub ts: i64,
    pub level: String,
    pub agent_id: Option<String>,
    pub event: String,
    pub message: String,
}

/// Insert the event row, then broadcast it. Broadcast failure is ignored
/// (log-and-continue at the call site is unnecessary — emit is best-effort).
/// `ts` is taken at broadcast; the row's own ts may differ by <1s.
pub fn emit_event(
    app: &tauri::AppHandle,
    db: &crate::database::repository::Db,
    level: &str,
    agent_id: Option<&str>,
    event: &str,
    message: String,
) {
    let id = db
        .insert_event(level, agent_id, event, &message)
        .unwrap_or(-1);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    if let Some(body) = crate::notify::toast_for(db, event, &message) {
        use tauri_plugin_notification::NotificationExt;
        let _ = app
            .notification()
            .builder()
            .title("AgentHQ")
            .body(body)
            .show();
    }
    let _ = app.emit(
        FRONTEND_EVENT,
        EmittedEvent {
            id,
            ts: now,
            level: level.to_string(),
            agent_id: agent_id.map(|s| s.to_string()),
            event: event.to_string(),
            message,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::{diff_ids, ChangeSet};

    fn set(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_diff_added_removed() {
        let c = diff_ids(&set(&["a", "b"]), &set(&["b", "c"]));
        assert_eq!(c.added, vec!["c".to_string()]);
        assert_eq!(c.removed, vec!["a".to_string()]);
        let _ = ChangeSet {
            added: vec![],
            removed: vec![],
        };
    }

    #[test]
    fn test_diff_ignores_order() {
        let c = diff_ids(&set(&["b", "a"]), &set(&["a", "b"]));
        assert!(c.added.is_empty() && c.removed.is_empty());
    }

    #[test]
    fn test_diff_dedupes() {
        let c = diff_ids(&[], &set(&["a", "a"]));
        assert_eq!(c.added, vec!["a".to_string()]);
    }

    #[test]
    fn test_no_change_no_events() {
        let c = diff_ids(&set(&["x", "y"]), &set(&["y", "x"]));
        assert!(c.added.is_empty() && c.removed.is_empty());
    }

    #[test]
    fn test_messages_have_no_paths() {
        for m in [
            super::session_started_msg("ses_abc"),
            super::session_stopped_msg("ses_abc"),
            super::collection_changed_msg("skill", 2, 1),
            super::collection_changed_msg("mcp", 0, 0),
        ] {
            assert!(!m.contains('\\'), "message must not contain paths: {m}");
        }
    }
}
