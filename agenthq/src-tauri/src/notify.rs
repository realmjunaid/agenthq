//! Local notifications. Each type is a settings key, default on.
//! High CPU/RAM notify only on the rising edge so a hot machine does not spam.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::database::repository::Db;

pub static CPU_HIGH: AtomicBool = AtomicBool::new(false);
pub static RAM_HIGH: AtomicBool = AtomicBool::new(false);

pub fn enabled(db: &Db, key: &str) -> bool {
    match db.get_setting(key) {
        Ok(Some(v)) => v == "1",
        _ => true,
    }
}

pub fn threshold(db: &Db, key: &str, default: f32) -> f32 {
    db.get_setting(key)
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Returns a toast body when this event should notify, else None.
pub fn toast_for(db: &Db, event: &str, message: &str) -> Option<String> {
    let key = match event {
        "agent.started" => "notify.agent_started",
        "agent.stopped" => "notify.agent_stopped",
        "agent.error" | "agent.detect_failed" => "notify.agent_error",
        "mcp.removed" => "notify.mcp_disconnected",
        _ => return None,
    };
    if enabled(db, key) {
        Some(message.to_string())
    } else {
        None
    }
}

/// Rising-edge resource alerts. `cpu` is a percent, `ram_ratio` is 0–100.
pub fn resource_alerts(db: &Db, cpu: f32, ram_ratio: f32) -> Vec<String> {
    let mut out = vec![];
    let cpu_limit = threshold(db, "notify.cpu_threshold", 80.0);
    let ram_limit = threshold(db, "notify.ram_threshold", 85.0);
    let cpu_now = cpu >= cpu_limit;
    if cpu_now && !CPU_HIGH.swap(cpu_now, Ordering::SeqCst) && enabled(db, "notify.high_cpu") {
        out.push(format!("High CPU: {cpu:.0}%"));
    }
    if !cpu_now {
        CPU_HIGH.store(false, Ordering::SeqCst);
    }
    let ram_now = ram_ratio >= ram_limit;
    if ram_now && !RAM_HIGH.swap(ram_now, Ordering::SeqCst) && enabled(db, "notify.high_ram") {
        out.push(format!("High RAM: {ram_ratio:.0}%"));
    }
    if !ram_now {
        RAM_HIGH.store(false, Ordering::SeqCst);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{enabled, toast_for};
    use crate::database::repository::Db;

    fn db(tag: &str) -> Db {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-notify-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.push("t.db");
        Db::open(&dir).unwrap()
    }

    #[test]
    fn test_toast_respects_setting() {
        let db = db("gate");
        assert!(toast_for(&db, "agent.started", "Claude started").is_some());
        db.set_setting("notify.agent_started", "0").unwrap();
        assert!(toast_for(&db, "agent.started", "Claude started").is_none());
        assert!(enabled(&db, "notify.agent_stopped"));
    }

    #[test]
    fn test_unknown_event_never_toasts() {
        let db = db("unknown");
        assert!(toast_for(&db, "skill.changed", "x").is_none());
    }
}
