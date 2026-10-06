//! AgentManager: registers adapters, detects installed/running agents,
//! persists state + transition events.

use serde::Serialize;
use std::path::Path;

use super::traits::AgentAdapter;
use crate::database::repository::{AgentRow, Db};
use crate::monitoring::process::ProcessInfo;

#[derive(Debug, Clone, Serialize)]
pub struct ManagerReport {
    pub detected: Vec<String>,
    pub running: Vec<String>,
    pub started: Vec<(String, i64)>,
    pub stopped: Vec<(String, i64)>,
    pub installed: Vec<(String, i64)>,
}

pub struct AgentManager {
    adapters: Vec<Box<dyn AgentAdapter>>,
}

impl AgentManager {
    pub fn new() -> Self {
        AgentManager { adapters: vec![] }
    }

    pub fn register(&mut self, adapter: Box<dyn AgentAdapter>) {
        self.adapters.push(adapter);
    }

    #[allow(dead_code)] // Phase 14 dashboard lists registered adapters.
    pub fn adapter_ids(&self) -> Vec<&'static str> {
        self.adapters.iter().map(|a| a.id()).collect()
    }

    /// MCP detail rows per adapter; adapters returning `Unsupported`
    /// are skipped (no error). Used by the MCP engine refresh.
    pub fn collect_mcp(&self) -> Vec<(String, Vec<super::traits::McpServerDetail>)> {
        self.adapters
            .iter()
            .filter_map(|a| match a.mcp_details() {
                Ok(d) => Some((a.id().to_string(), d)),
                Err(_) => None,
            })
            .collect()
    }

    /// Skill detail rows per adapter; adapters returning `Unsupported`
    /// are skipped (no error). Used by the skills engine refresh.
    pub fn collect_skills(&self) -> Vec<(String, Vec<super::traits::SkillDetail>)> {
        self.adapters
            .iter()
            .filter_map(|a| match a.skills_details() {
                Ok(d) => Some((a.id().to_string(), d)),
                Err(_) => None,
            })
            .collect()
    }

    /// Plugin detail rows per adapter; adapters returning `Unsupported`
    /// are skipped (no error). Used by the plugins engine refresh.
    pub fn collect_plugins(&self) -> Vec<(String, Vec<super::traits::PluginDetail>)> {
        self.adapters
            .iter()
            .filter_map(|a| match a.plugins_details() {
                Ok(d) => Some((a.id().to_string(), d)),
                Err(_) => None,
            })
            .collect()
    }

    /// Session detail rows per adapter (project_dir forwarded for
    /// project-scoped sources); `Unsupported` skipped. Sessions engine.
    pub fn collect_sessions(
        &self,
        project_dir: Option<&std::path::Path>,
    ) -> Vec<(String, Vec<super::traits::SessionDetail>)> {
        self.adapters
            .iter()
            .filter_map(|a| match a.sessions_details(project_dir) {
                Ok(d) => Some((a.id().to_string(), d)),
                Err(_) => None,
            })
            .collect()
    }

    /// Project rows per adapter; `Unsupported` skipped. Sessions engine.
    pub fn collect_projects(&self) -> Vec<(String, Vec<super::traits::ProjectDetail>)> {
        self.adapters
            .iter()
            .filter_map(|a| match a.projects_details() {
                Ok(d) => Some((a.id().to_string(), d)),
                Err(_) => None,
            })
            .collect()
    }

    /// Model rows per adapter; `Unsupported` skipped. Models engine.
    pub fn collect_models(&self) -> Vec<(String, Vec<super::traits::ModelInfo>)> {
        self.adapters
            .iter()
            .filter_map(|a| match a.models() {
                Ok(d) => Some((a.id().to_string(), d)),
                Err(_) => None,
            })
            .collect()
    }

    /// Connection rows per adapter; `Unsupported` skipped. Connections engine.
    pub fn collect_connections(&self) -> Vec<(String, Vec<super::traits::ConnectionInfo>)> {
        self.adapters
            .iter()
            .filter_map(|a| match a.connections() {
                Ok(d) => Some((a.id().to_string(), d)),
                Err(_) => None,
            })
            .collect()
    }

    /// Executable names for an adapter id (owned, lowercase). Empty when unknown.
    pub fn executable_names_for(&self, id: &str) -> Vec<String> {
        self.adapters
            .iter()
            .find(|a| a.id() == id)
            .map(|a| a.executable_names().iter().map(|n| n.to_string()).collect())
            .unwrap_or_default()
    }

    pub fn detect_all(&self, snapshot: &[ProcessInfo], db: &Db) -> Result<ManagerReport, String> {
        let mut detected = vec![];
        let mut running = vec![];
        let mut started = vec![];
        let mut stopped = vec![];
        let mut installed = vec![];
        for adapter in &self.adapters {
            let det = adapter.detect_installation();
            let is_running = snapshot
                .iter()
                .any(|p| exe_matches(p, adapter.executable_names()));
            let prev = db.get_agent(adapter.id())?;
            let prev_running = prev.as_ref().map(|row| row.running).unwrap_or(false);
            let first_seen = prev.is_none();
            db.upsert_agent(&AgentRow {
                id: adapter.id().to_string(),
                name: adapter.name().to_string(),
                agent_type: adapter.id().to_string(),
                version: det.version.clone(),
                executable_path: det.executable_path.clone(),
                installed: det.installed,
                running: is_running,
                last_seen: if is_running || det.installed {
                    Some(now_ts())
                } else {
                    None
                },
            })?;
            if det.installed {
                detected.push(adapter.id().to_string());
            }
            if first_seen && det.installed {
                let rowid = db.insert_event(
                    "info",
                    Some(adapter.id()),
                    "agent.installed",
                    &format!("{} detected", adapter.name()),
                )?;
                installed.push((adapter.id().to_string(), rowid));
            }
            if is_running {
                running.push(adapter.id().to_string());
                if !prev_running {
                    let rowid = db.insert_event(
                        "info",
                        Some(adapter.id()),
                        "agent.started",
                        &format!("{} detected running", adapter.name()),
                    )?;
                    started.push((adapter.id().to_string(), rowid));
                }
            } else if prev_running {
                let rowid = db.insert_event(
                    "info",
                    Some(adapter.id()),
                    "agent.stopped",
                    &format!("{} no longer running", adapter.name()),
                )?;
                stopped.push((adapter.id().to_string(), rowid));
            }
        }
        Ok(ManagerReport {
            detected,
            running,
            started,
            stopped,
            installed,
        })
    }
}

/// Match by executable file stem, case-insensitive. `None` exe never matches.
/// Adapter-declared names must be lowercase.
pub fn exe_stem_matches(exe: Option<&str>, names: &[&str]) -> bool {
    match exe {
        None => false,
        Some(exe) => {
            let stem = Path::new(exe)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            names.iter().any(|n| *n == stem)
        }
    }
}

/// Match by executable file stem, case-insensitive. `None` exe never matches.
fn exe_matches(p: &ProcessInfo, names: &[&str]) -> bool {
    exe_stem_matches(p.exe.as_deref(), names)
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::AgentManager;
    use crate::agents::traits::{AgentAdapter, AgentCapabilities, DetectionResult};
    use crate::database::repository::Db;
    use crate::monitoring::process::ProcessInfo;
    use std::path::PathBuf;

    const FAKE_NAMES: &[&str] = &["fake-agent"];

    struct FakeOk;
    impl AgentAdapter for FakeOk {
        fn id(&self) -> &'static str {
            "fake-ok"
        }
        fn name(&self) -> &'static str {
            "Fake OK"
        }
        fn executable_names(&self) -> &[&str] {
            FAKE_NAMES
        }
        fn detect_installation(&self) -> DetectionResult {
            DetectionResult {
                installed: true,
                executable_path: crate::agents::detect::find_on_path("cargo"),
                version: None,
            }
        }
        fn capabilities(&self) -> AgentCapabilities {
            AgentCapabilities::none()
        }
    }

    struct FakeVersionFail;
    impl AgentAdapter for FakeVersionFail {
        fn id(&self) -> &'static str {
            "fake-vfail"
        }
        fn name(&self) -> &'static str {
            "Fake VFail"
        }
        fn executable_names(&self) -> &[&str] {
            FAKE_NAMES
        }
        fn detect_installation(&self) -> DetectionResult {
            DetectionResult {
                installed: true,
                executable_path: Some("C:\\tools\\fake-agent.exe".to_string()),
                version: None,
            }
        }
        fn capabilities(&self) -> AgentCapabilities {
            AgentCapabilities::none()
        }
    }

    fn temp_db(name: &str) -> (Db, PathBuf) {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-mgr-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&dir);
        let mut path = dir;
        path.push("test.db");
        let db = Db::open(&path).unwrap();
        let path2 = path.clone();
        (db, path2)
    }

    fn proc_row(exe: Option<&str>, name: &str) -> ProcessInfo {
        ProcessInfo::new(
            4242,
            Some(1),
            name.to_string(),
            exe.map(|s| s.to_string()),
            0.0,
            0,
            0,
            None,
        )
    }

    fn manager_with<A: AgentAdapter + 'static>(a: A) -> AgentManager {
        let mut m = AgentManager::new();
        m.register(Box::new(a));
        m
    }

    #[test]
    fn test_install_detected_exe_present_version_fails() {
        let (db, _p) = temp_db("vfail");
        let m = manager_with(FakeVersionFail);
        let report = m.detect_all(&[], &db).unwrap();
        assert_eq!(report.detected, vec!["fake-vfail".to_string()]);
        let row = db.get_agent("fake-vfail").unwrap().unwrap();
        assert!(row.installed);
        assert_eq!(row.version, None);
    }

    #[test]
    fn test_running_matches_exe_stem_case_insensitive() {
        let (db, _p) = temp_db("run");
        let m = manager_with(FakeOk);
        let snap = vec![proc_row(Some("C:\\X\\FAKE-AGENT.EXE"), "fake-agent")];
        let report = m.detect_all(&snap, &db).unwrap();
        assert_eq!(report.running, vec!["fake-ok".to_string()]);
        assert!(db.get_agent("fake-ok").unwrap().unwrap().running);
    }

    #[test]
    fn test_running_ignores_none_exe() {
        let (db, _p) = temp_db("noneexe");
        let m = manager_with(FakeOk);
        let snap = vec![proc_row(None, "fake-agent")];
        let report = m.detect_all(&snap, &db).unwrap();
        assert!(report.running.is_empty());
        assert!(!db.get_agent("fake-ok").unwrap().unwrap().running);
    }

    #[test]
    fn test_running_flips_false_when_process_exits() {
        let (db, _p) = temp_db("flip");
        let m = manager_with(FakeOk);
        let snap = vec![proc_row(Some("C:\\X\\fake-agent.exe"), "fake-agent")];
        m.detect_all(&snap, &db).unwrap();
        assert!(db.get_agent("fake-ok").unwrap().unwrap().running);
        m.detect_all(&[], &db).unwrap();
        assert!(!db.get_agent("fake-ok").unwrap().unwrap().running);
        let events = db.list_recent_events(10).unwrap();
        assert_eq!(
            events.iter().filter(|e| e.event == "agent.stopped").count(),
            1
        );
    }

    #[test]
    fn test_unsupported_adapter_stores_no_extra_state() {
        let (db, _p) = temp_db("unsup");
        let m = manager_with(FakeVersionFail);
        let report = m.detect_all(&[], &db).unwrap();
        assert_eq!(report.detected, vec!["fake-vfail".to_string()]);
        assert!(db.get_agent("fake-vfail").unwrap().is_some());
    }

    #[test]
    fn test_collect_mcp_skips_unsupported() {
        let m = manager_with(FakeVersionFail);
        assert!(m.collect_mcp().is_empty());
    }

    #[test]
    fn test_collect_skills_skips_unsupported() {
        let m = manager_with(FakeVersionFail);
        assert!(m.collect_skills().is_empty());
    }

    #[test]
    fn test_collect_plugins_skips_unsupported() {
        let m = manager_with(FakeVersionFail);
        assert!(m.collect_plugins().is_empty());
    }

    #[test]
    fn test_collect_models_connections_skip_unsupported() {
        let m = manager_with(FakeVersionFail);
        assert!(m.collect_models().is_empty());
        assert!(m.collect_connections().is_empty());
    }

    #[test]
    fn test_executable_names_for_unknown_is_empty() {
        let m = manager_with(FakeVersionFail);
        assert!(m.executable_names_for("nope").is_empty());
    }

    #[test]
    fn test_installed_fires_once() {
        let (db, _p) = temp_db("installonce");
        let m = manager_with(FakeVersionFail);
        m.detect_all(&[], &db).unwrap();
        let count = || {
            db.list_recent_events(50)
                .unwrap()
                .iter()
                .filter(|e| e.event == "agent.installed")
                .count()
        };
        assert_eq!(count(), 1);
        m.detect_all(&[], &db).unwrap();
        assert_eq!(count(), 1);
    }

    #[test]
    fn test_collect_sessions_skips_unsupported() {
        let m = manager_with(FakeVersionFail);
        assert!(m.collect_sessions(None).is_empty());
        assert!(m.collect_projects().is_empty());
    }
}
