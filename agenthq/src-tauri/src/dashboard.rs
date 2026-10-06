//! Dashboard aggregate. Tests first (RED); implementation in Step 3.
use serde::Serialize;

use crate::agents::manager::AgentManager;
use crate::database::repository::{AgentRow, Db, EventRow};
use crate::monitoring::process::{ProcessInfo, ProcessMonitor};
use crate::monitoring::resources::SystemStats;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardAgent {
    pub agent: AgentRow,
    pub ram_bytes: u64,
    pub cpu: f32,
    pub sessions: u64,
    pub subagents: Option<u64>,
    pub mcp: u64,
    pub skills: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardData {
    pub agents: Vec<DashboardAgent>,
    pub total_ram_bytes: u64,
    pub total_cpu: f32,
    pub running: u64,
    pub system: SystemStats,
}

/// Sum RAM/CPU of snapshot rows whose exe stem matches `names`.
pub fn sum_for(names: &[String], snapshot: &[ProcessInfo]) -> (u64, f32) {
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    snapshot
        .iter()
        .filter(|p| crate::agents::manager::exe_stem_matches(p.exe.as_deref(), &refs))
        .fold((0, 0.0), |(ram, cpu), p| (ram + p.ram_bytes, cpu + p.cpu))
}

pub fn build_dashboard(
    db: &Db,
    monitor: &Mutex<ProcessMonitor>,
    manager: &Mutex<AgentManager>,
) -> Result<DashboardData, String> {
    let (snapshot, system) = {
        let mut m = monitor.lock().map_err(|e| e.to_string())?;
        if !crate::tray::PAUSED.load(std::sync::atomic::Ordering::SeqCst) {
            m.refresh();
        }
        let (used_mem_bytes, total_mem_bytes) = m.memory_usage();
        let system = SystemStats {
            total_cpu: m.total_cpu(),
            used_mem_bytes,
            total_mem_bytes,
        };
        (m.snapshot(), system)
    };
    let rows = db.list_agents()?;
    let mut agents = vec![];
    let mut total_ram = 0u64;
    let mut total_cpu = 0.0f32;
    let mut running = 0u64;
    for agent in rows {
        // Adapters upsert a row even when nothing is installed. Those are
        // not dashboard cards — only installed or running agents count.
        if !agent.installed && !agent.running {
            continue;
        }
        let names = manager
            .lock()
            .map_err(|e| e.to_string())?
            .executable_names_for(&agent.id);
        let (ram, cpu) = sum_for(&names, &snapshot);
        total_ram += ram;
        total_cpu += cpu;
        if agent.running {
            running += 1;
        }
        agents.push(DashboardAgent {
            sessions: db
                .list_sessions(Some(&agent.id))?
                .iter()
                .filter(|s| s.status != "stopped")
                .count() as u64,
            mcp: db.list_mcp_servers(Some(&agent.id))?.len() as u64,
            skills: db.list_skills(Some(&agent.id))?.len() as u64,
            subagents: None,
            ram_bytes: ram,
            cpu,
            agent,
        });
    }
    Ok(DashboardData {
        agents,
        total_ram_bytes: total_ram,
        total_cpu,
        running,
        system,
    })
}

pub fn recent_events(db: &Db, limit: Option<i64>) -> Result<Vec<EventRow>, String> {
    let n = limit.unwrap_or(50).clamp(1, 200);
    db.list_recent_events(n)
}

#[cfg(test)]
mod tests {
    use super::sum_for;
    use crate::monitoring::process::ProcessInfo;

    fn row(pid: u32, exe: Option<&str>, cpu: f32, ram: u64) -> ProcessInfo {
        ProcessInfo::new(
            pid,
            None,
            format!("p{pid}"),
            exe.map(|s| s.to_string()),
            cpu,
            ram,
            0,
            None,
        )
    }

    #[test]
    fn test_empty_snapshot_sums_zero() {
        let (ram, cpu) = sum_for(&["claude".to_string()], &[]);
        assert_eq!((ram, cpu), (0, 0.0));
    }

    #[test]
    fn test_resource_sums_match_stems() {
        let snap = vec![
            row(1, Some("C:\\b\\claude.exe"), 2.5, 100),
            row(2, Some("C:\\b\\CLAUDE.EXE"), 1.5, 50),
            row(3, Some("C:\\b\\other.exe"), 9.0, 999),
            row(4, None, 5.0, 500),
        ];
        let (ram, cpu) = sum_for(&["claude".to_string()], &snap);
        assert_eq!(ram, 150);
        assert!((cpu - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_empty_db_empty_dashboard() {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-dash-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.push("t.db");
        let db = crate::database::repository::Db::open(&dir).unwrap();
        let monitor = std::sync::Mutex::new(crate::monitoring::process::ProcessMonitor::new());
        let manager = std::sync::Mutex::new(crate::agents::manager::AgentManager::new());
        let data = super::build_dashboard(&db, &monitor, &manager).unwrap();
        assert!(data.agents.is_empty());
        assert_eq!(
            (data.total_ram_bytes, data.total_cpu, data.running),
            (0, 0.0, 0)
        );
    }

    fn agent(id: &str, installed: bool) -> crate::database::repository::AgentRow {
        crate::database::repository::AgentRow {
            id: id.to_string(),
            name: id.to_string(),
            agent_type: id.to_string(),
            version: None,
            executable_path: None,
            installed,
            running: false,
            last_seen: None,
        }
    }

    #[test]
    fn test_uninstalled_omitted_stopped_not_counted() {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-dash-filter-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.push("t.db");
        let db = crate::database::repository::Db::open(&dir).unwrap();
        db.upsert_agent(&agent("ghost", false)).unwrap();
        db.upsert_agent(&agent("live", true)).unwrap();
        db.upsert_sessions(&[
            crate::sessions::Session {
                id: "live:stopped".to_string(),
                agent_id: "live".to_string(),
                project_id: None,
                status: "stopped".to_string(),
                model: None,
                started_at: None,
                last_activity: None,
                confidence: crate::sessions::SessionConfidence::Detected,
            },
            crate::sessions::Session {
                id: "live:open".to_string(),
                agent_id: "live".to_string(),
                project_id: None,
                status: "unknown".to_string(),
                model: None,
                started_at: None,
                last_activity: None,
                confidence: crate::sessions::SessionConfidence::Detected,
            },
        ])
        .unwrap();
        let monitor = std::sync::Mutex::new(crate::monitoring::process::ProcessMonitor::new());
        let manager = std::sync::Mutex::new(crate::agents::manager::AgentManager::new());
        let data = super::build_dashboard(&db, &monitor, &manager).unwrap();
        assert_eq!(data.agents.len(), 1);
        assert_eq!(data.agents[0].agent.id, "live");
        assert_eq!(data.agents[0].sessions, 1);
        assert_eq!(data.agents[0].subagents, None);
    }
}
