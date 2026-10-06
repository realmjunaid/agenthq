//! Config file watcher: known agent config dirs only (never full disk).
//! A burst of writes coalesces into one quiet engine sync + one
//! `config.changed` event. Watch errors are logged, never fatal.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use tauri::Manager;

use crate::agents::manager::AgentManager;
use crate::database::repository::Db;

/// Quiet period that coalesces a burst of writes into one sync.
pub const DEBOUNCE_SECS: u64 = 2;

/// Gate for one more sync: first event always fires, then at most one
/// per quiet period.
pub fn should_fire(last: Option<Instant>, now: Instant) -> bool {
    match last {
        None => true,
        Some(t) => now.duration_since(t).as_secs() >= DEBOUNCE_SECS,
    }
}

/// Message carries a count only — never paths or file contents.
pub fn changed_msg(dirs: usize) -> String {
    format!("config changed: {dirs} dirs")
}

/// Existing agent config roots under `home`. Missing dirs are skipped.
pub fn watch_roots_in(home: &Path) -> Vec<PathBuf> {
    [
        home.join(".claude"),
        home.join(".config").join("opencode"),
        home.join(".codex"),
        home.join(".grok"),
    ]
    .into_iter()
    .filter(|p| p.is_dir())
    .collect()
}

fn home_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// Spawn the watcher thread (detached). Never panics the caller: setup
/// failures just end the thread after logging one error event.
pub fn spawn(app: tauri::AppHandle) {
    let _ = std::thread::Builder::new()
        .name("agenthq-watch".to_string())
        .spawn(move || {
            let roots = watch_roots_in(&home_dir());
            if roots.is_empty() {
                return;
            }
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = match RecommendedWatcher::new(tx, notify::Config::default()) {
                Ok(w) => w,
                Err(e) => {
                    log_error(&app, &format!("watcher failed to start: {e}"));
                    return;
                }
            };
            for root in &roots {
                if let Err(e) = watcher.watch(root, RecursiveMode::Recursive) {
                    log_error(&app, &format!("watcher cannot watch: {e}"));
                }
            }
            let mut last: Option<Instant> = None;
            loop {
                let msg = match rx.recv() {
                    Ok(m) => m,
                    Err(_) => break,
                };
                if msg.is_err() {
                    continue;
                }
                let now = Instant::now();
                if !should_fire(last, now) {
                    continue;
                }
                // Burst start: events arriving during the quiet period
                // see a fresh `last` and are skipped.
                last = Some(now);
                // Quiet period: let the burst settle, drain it, sync once.
                std::thread::sleep(Duration::from_secs(DEBOUNCE_SECS));
                while rx.try_recv().is_ok() {}
                let ndirs = roots.len();
                let result = (|| -> Result<(), String> {
                    let guard = app.state::<Mutex<AgentManager>>();
                    let manager = guard.lock().map_err(|e| e.to_string())?;
                    let db = app.state::<Db>();
                    crate::startup_sync(&manager, &db)
                })();
                match result {
                    Ok(()) => crate::events::emit_event(
                        &app,
                        &app.state::<Db>(),
                        "info",
                        None,
                        "config.changed",
                        changed_msg(ndirs),
                    ),
                    Err(e) => log_error(&app, &format!("watcher sync failed: {e}")),
                }
            }
        });
}

fn log_error(app: &tauri::AppHandle, message: &str) {
    if let Some(db) = app.try_state::<Db>() {
        let _ = db.insert_event("error", None, "config.watch_failed", message);
    }
}

#[cfg(test)]
mod tests {
    use super::{changed_msg, should_fire, watch_roots_in};
    use std::time::{Duration, Instant};

    #[test]
    fn test_first_event_always_fires() {
        assert!(should_fire(None, Instant::now()));
    }

    #[test]
    fn test_burst_coalesces_into_one() {
        let first = Instant::now();
        assert!(!should_fire(Some(first), first + Duration::from_secs(1)));
        assert!(should_fire(
            Some(first),
            first + Duration::from_secs(super::DEBOUNCE_SECS + 1)
        ));
    }

    #[test]
    fn test_missing_dirs_are_skipped() {
        let mut home = std::env::temp_dir();
        home.push(format!("agenthq-watch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        assert!(watch_roots_in(&home).is_empty());
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        assert_eq!(watch_roots_in(&home).len(), 1);
        std::fs::create_dir_all(home.join(".grok")).unwrap();
        let roots = watch_roots_in(&home);
        assert_eq!(roots.len(), 2);
        assert!(roots.iter().any(|p| p.ends_with(".grok")));
    }

    #[test]
    fn test_message_carries_no_paths_or_contents() {
        let msg = changed_msg(3);
        assert!(!msg.contains('\\'));
        assert!(!msg.contains('/'));
    }
}
