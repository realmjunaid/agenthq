// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod agents;
mod dashboard;
mod database;
mod events;
mod lifecycle;
mod mcp;
mod models;
mod monitoring;
mod notify;
mod plugins;
mod sessions;
mod skills;
mod tray;
mod watch;

use agents::claude::adapter::ClaudeAdapter;
use agents::codex::adapter::CodexAdapter;
use agents::grok::adapter::GrokAdapter;
use agents::manager::{AgentManager, ManagerReport};
use agents::opencode::adapter::OpenCodeAdapter;
use dashboard::{recent_events, DashboardData};
use database::repository::{AgentRow, Db, EventRow};
use events::{diff_ids, emit_event};
use mcp::{normalize as normalize_mcp, McpServer, McpStatus};
use models::{normalize_connections, normalize_models, Connection, Model};
use monitoring::process::{ProcessInfo, ProcessMonitor};
use monitoring::resources::SystemStats;
use plugins::{normalize as normalize_plugins, Plugin};
use sessions::{normalize_projects, normalize_sessions, Project, Session};
use skills::{normalize as normalize_skills, Skill};
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn list_agents(db: State<Db>) -> Result<Vec<AgentRow>, String> {
    db.list_agents()
}

/// Refresh-once snapshot, hottest CPU first, capped for IPC payload size.
#[tauri::command]
fn get_process_snapshot(monitor: State<Mutex<ProcessMonitor>>) -> Result<Vec<ProcessInfo>, String> {
    let mut monitor = monitor.lock().map_err(|e| e.to_string())?;
    monitor.refresh();
    let mut procs = monitor.snapshot();
    procs.sort_by(|a, b| {
        b.cpu
            .partial_cmp(&a.cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    procs.truncate(500);
    Ok(procs)
}

#[tauri::command]
fn get_system_stats(monitor: State<Mutex<ProcessMonitor>>) -> Result<SystemStats, String> {
    let mut monitor = monitor.lock().map_err(|e| e.to_string())?;
    monitor.refresh_system();
    let (used_mem_bytes, total_mem_bytes) = monitor.memory_usage();
    Ok(SystemStats {
        total_cpu: monitor.total_cpu(),
        used_mem_bytes,
        total_mem_bytes,
    })
}

fn mcp_source_for(agent_id: &str) -> &'static str {
    match agent_id {
        "claude" => "claude.json",
        "opencode" => "opencode.json",
        "codex" => "config.toml",
        _ => "unknown",
    }
}

/// Quiet first-sync for startup: persist every engine snapshot without
/// emitting transition events (the DB starts empty, so everything would
/// look "added"). Errors propagate; the caller logs them and never fails
/// startup over them.
pub(crate) fn startup_sync(manager: &AgentManager, db: &Db) -> Result<(), String> {
    for (agent_id, details) in manager.collect_mcp() {
        let default = if agent_id == "codex" {
            McpStatus::Unknown
        } else {
            McpStatus::Configured
        };
        let rows = normalize_mcp(&agent_id, mcp_source_for(&agent_id), default, details);
        db.replace_agent_servers(&agent_id, &rows)?;
    }
    for (agent_id, details) in manager.collect_skills() {
        db.replace_agent_skills(&agent_id, &normalize_skills(&agent_id, details))?;
    }
    for (agent_id, details) in manager.collect_plugins() {
        db.replace_agent_plugins(&agent_id, &normalize_plugins(&agent_id, details))?;
    }
    for (agent_id, details) in manager.collect_models() {
        db.replace_agent_models(&agent_id, &normalize_models(&agent_id, details))?;
    }
    for (agent_id, details) in manager.collect_connections() {
        db.replace_agent_connections(&agent_id, &normalize_connections(&agent_id, details))?;
    }
    let mut path_to_id = std::collections::HashMap::new();
    let mut all_projects = vec![];
    for (agent_id, details) in manager.collect_projects() {
        let projects = normalize_projects(&agent_id, details);
        for p in &projects {
            path_to_id.insert(p.path.clone(), p.id.clone());
        }
        all_projects.extend(projects);
    }
    db.replace_projects(&all_projects)?;
    for (agent_id, details) in manager.collect_sessions(None) {
        db.upsert_sessions(&normalize_sessions(&agent_id, &path_to_id, details))?;
    }
    Ok(())
}

/// Re-collect MCP servers from all adapters and persist the snapshot.
/// Emits mcp.configured/removed transitions live; no change → silent.
#[tauri::command]
fn refresh_mcp(
    app: tauri::AppHandle,
    manager: State<Mutex<AgentManager>>,
    db: State<Db>,
) -> Result<Vec<McpServer>, String> {
    fn source_for(agent_id: &str) -> &'static str {
        mcp_source_for(agent_id)
    }
    let manager = manager.lock().map_err(|e| e.to_string())?;
    let mut all = vec![];
    for (agent_id, details) in manager.collect_mcp() {
        let old: Vec<String> = db
            .list_mcp_servers(Some(&agent_id))?
            .into_iter()
            .map(|s| s.id)
            .collect();
        let default = if agent_id == "codex" {
            McpStatus::Unknown
        } else {
            McpStatus::Configured
        };
        let servers = normalize_mcp(&agent_id, source_for(&agent_id), default, details);
        db.replace_agent_servers(&agent_id, &servers)?;
        let new_ids: Vec<String> = servers.iter().map(|s| s.id.clone()).collect();
        let diff = diff_ids(&old, &new_ids);
        for id in &diff.added {
            emit_event(
                &app,
                &db,
                "info",
                Some(&agent_id),
                "mcp.configured",
                format!("mcp server {id} configured"),
            );
        }
        for id in &diff.removed {
            emit_event(
                &app,
                &db,
                "info",
                Some(&agent_id),
                "mcp.removed",
                format!("mcp server {id} removed"),
            );
        }
        all.extend(servers);
    }
    Ok(all)
}

#[tauri::command]
fn get_mcp_servers(db: State<Db>, agent_id: Option<String>) -> Result<Vec<McpServer>, String> {
    db.list_mcp_servers(agent_id.as_deref())
}

/// Re-collect skills from all adapters and persist the snapshot.
/// Emits one skill.changed per agent whose set differs; no change → silent.
#[tauri::command]
fn refresh_skills(
    app: tauri::AppHandle,
    manager: State<Mutex<AgentManager>>,
    db: State<Db>,
) -> Result<Vec<Skill>, String> {
    let manager = manager.lock().map_err(|e| e.to_string())?;
    let mut all = vec![];
    for (agent_id, details) in manager.collect_skills() {
        let old: Vec<String> = db
            .list_skills(Some(&agent_id))?
            .into_iter()
            .map(|s| s.id)
            .collect();
        let servers = normalize_skills(&agent_id, details);
        db.replace_agent_skills(&agent_id, &servers)?;
        let new_ids: Vec<String> = servers.iter().map(|s| s.id.clone()).collect();
        let diff = diff_ids(&old, &new_ids);
        if !diff.added.is_empty() || !diff.removed.is_empty() {
            emit_event(
                &app,
                &db,
                "info",
                Some(&agent_id),
                "skill.changed",
                crate::events::collection_changed_msg(
                    "skill",
                    diff.added.len(),
                    diff.removed.len(),
                ),
            );
        }
        all.extend(servers);
    }
    Ok(all)
}

#[tauri::command]
fn get_skills(db: State<Db>, agent_id: Option<String>) -> Result<Vec<Skill>, String> {
    db.list_skills(agent_id.as_deref())
}

/// Re-collect plugins from all adapters and persist the snapshot.
/// Emits one plugin.changed per agent whose set differs; no change → silent.
#[tauri::command]
fn refresh_plugins(
    app: tauri::AppHandle,
    manager: State<Mutex<AgentManager>>,
    db: State<Db>,
) -> Result<Vec<Plugin>, String> {
    let manager = manager.lock().map_err(|e| e.to_string())?;
    let mut all = vec![];
    for (agent_id, details) in manager.collect_plugins() {
        let old: Vec<String> = db
            .list_plugins(Some(&agent_id))?
            .into_iter()
            .map(|p| p.id)
            .collect();
        let rows = normalize_plugins(&agent_id, details);
        db.replace_agent_plugins(&agent_id, &rows)?;
        let new_ids: Vec<String> = rows.iter().map(|p| p.id.clone()).collect();
        let diff = diff_ids(&old, &new_ids);
        if !diff.added.is_empty() || !diff.removed.is_empty() {
            emit_event(
                &app,
                &db,
                "info",
                Some(&agent_id),
                "plugin.changed",
                crate::events::collection_changed_msg(
                    "plugin",
                    diff.added.len(),
                    diff.removed.len(),
                ),
            );
        }
        all.extend(rows);
    }
    Ok(all)
}

#[tauri::command]
fn get_plugins(db: State<Db>, agent_id: Option<String>) -> Result<Vec<Plugin>, String> {
    db.list_plugins(agent_id.as_deref())
}

/// Re-collect models from all adapters and persist the snapshot. No change → silent.
#[tauri::command]
fn refresh_models(
    manager: State<Mutex<AgentManager>>,
    db: State<Db>,
) -> Result<Vec<Model>, String> {
    let manager = manager.lock().map_err(|e| e.to_string())?;
    let mut all = vec![];
    for (agent_id, details) in manager.collect_models() {
        let rows = normalize_models(&agent_id, details);
        db.replace_agent_models(&agent_id, &rows)?;
        all.extend(rows);
    }
    Ok(all)
}

#[tauri::command]
fn get_models(db: State<Db>, agent_id: Option<String>) -> Result<Vec<Model>, String> {
    db.list_models(agent_id.as_deref())
}

/// Re-collect connections from all adapters and persist the snapshot.
/// Only key names are stored; values are never collected. No change → silent.
#[tauri::command]
fn refresh_connections(
    manager: State<Mutex<AgentManager>>,
    db: State<Db>,
) -> Result<Vec<Connection>, String> {
    let manager = manager.lock().map_err(|e| e.to_string())?;
    let mut all = vec![];
    for (agent_id, details) in manager.collect_connections() {
        let rows = normalize_connections(&agent_id, details);
        db.replace_agent_connections(&agent_id, &rows)?;
        all.extend(rows);
    }
    Ok(all)
}

#[tauri::command]
fn get_connections(db: State<Db>, agent_id: Option<String>) -> Result<Vec<Connection>, String> {
    db.list_connections(agent_id.as_deref())
}

/// Re-collect sessions (+projects) from all adapters and persist.
/// `project_dir` feeds project-scoped sources (OpenCode); others ignore it.
/// Emits session.started/stopped transitions live; no change → silent.
#[tauri::command]
fn refresh_sessions(
    app: tauri::AppHandle,
    manager: State<Mutex<AgentManager>>,
    db: State<Db>,
    project_dir: Option<String>,
) -> Result<Vec<Session>, String> {
    let dir = project_dir.map(std::path::PathBuf::from);
    let manager = manager.lock().map_err(|e| e.to_string())?;
    // Projects first so sessions can link by path. All producers accumulate
    // into ONE replace call so no adapter wipes another's rows.
    let mut path_to_id = std::collections::HashMap::new();
    let mut all_projects = vec![];
    for (agent_id, details) in manager.collect_projects() {
        let projects = normalize_projects(&agent_id, details);
        for p in &projects {
            path_to_id.insert(p.path.clone(), p.id.clone());
        }
        all_projects.extend(projects);
    }
    db.replace_projects(&all_projects)?;
    let mut all = vec![];
    for (agent_id, details) in manager.collect_sessions(dir.as_deref()) {
        let old: Vec<String> = db.unstopped_session_ids(&agent_id)?;
        let sessions = normalize_sessions(&agent_id, &path_to_id, details);
        db.upsert_sessions(&sessions)?;
        let new_ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
        let diff = diff_ids(&old, &new_ids);
        for id in &diff.added {
            emit_event(
                &app,
                &db,
                "info",
                Some(&agent_id),
                "session.started",
                crate::events::session_started_msg(id),
            );
        }
        // Scoped refreshes can't prove absence: only a full refresh may
        // report stops, and marked rows never re-emit.
        if dir.is_none() {
            if !diff.removed.is_empty() {
                db.mark_sessions_stopped(&diff.removed)?;
            }
            for id in &diff.removed {
                emit_event(
                    &app,
                    &db,
                    "info",
                    Some(&agent_id),
                    "session.stopped",
                    crate::events::session_stopped_msg(id),
                );
            }
        }
        all.extend(sessions);
    }
    Ok(all)
}

#[tauri::command]
fn get_sessions(db: State<Db>, agent_id: Option<String>) -> Result<Vec<Session>, String> {
    db.list_sessions(agent_id.as_deref())
}

#[tauri::command]
fn get_projects(db: State<Db>) -> Result<Vec<Project>, String> {
    db.list_projects()
}

/// One aggregated dashboard payload: agent rows, per-agent counts, resource sums.
#[tauri::command]
fn get_dashboard(
    monitor: State<Mutex<ProcessMonitor>>,
    manager: State<Mutex<AgentManager>>,
    db: State<Db>,
) -> Result<DashboardData, String> {
    dashboard::build_dashboard(&db, &monitor, &manager)
}

/// Recent event rows. Limit clamped to 1..=200 (default 50).
#[tauri::command]
fn get_events(db: State<Db>, limit: Option<i64>) -> Result<Vec<EventRow>, String> {
    recent_events(&db, limit)
}
#[tauri::command]
fn refresh_agents(
    app: tauri::AppHandle,
    monitor: State<Mutex<ProcessMonitor>>,
    manager: State<Mutex<AgentManager>>,
    db: State<Db>,
) -> Result<ManagerReport, String> {
    let mut monitor = monitor.lock().map_err(|e| e.to_string())?;
    monitor.refresh();
    let snapshot = monitor.snapshot();
    drop(monitor);
    let manager = manager.lock().map_err(|e| e.to_string())?;
    let report = manager.detect_all(&snapshot, &db)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // detect_all already inserted the rows; broadcast live without dupes.
    let emit = |event: &str, agent: &str, rowid: i64| {
        let _ = app.emit(
            crate::events::FRONTEND_EVENT,
            crate::events::EmittedEvent {
                id: rowid,
                ts: now,
                level: "info".to_string(),
                agent_id: Some(agent.to_string()),
                event: event.to_string(),
                message: format!("{event} {agent}"),
            },
        );
    };
    for (agent, rowid) in &report.installed {
        emit("agent.installed", agent, *rowid);
    }
    for (agent, rowid) in &report.started {
        emit("agent.started", agent, *rowid);
    }
    for (agent, rowid) in &report.stopped {
        emit("agent.stopped", agent, *rowid);
    }
    Ok(report)
}

fn paused() -> bool {
    tray::PAUSED.load(std::sync::atomic::Ordering::SeqCst)
}

#[derive(serde::Serialize)]
struct HostView {
    cpu: f32,
    used_mem_bytes: u64,
    total_mem_bytes: u64,
    disks: Vec<crate::monitoring::resources::DiskStat>,
    networks: Vec<crate::monitoring::resources::NetStat>,
    paused: bool,
}

/// Disk/network plus system totals. Skips a sysinfo refresh while paused.
#[tauri::command]
fn get_host_resources(
    app: tauri::AppHandle,
    monitor: State<Mutex<ProcessMonitor>>,
    sampler: State<Mutex<crate::monitoring::resources::ResourceSampler>>,
    db: State<Db>,
) -> Result<HostView, String> {
    let paused = paused();
    let (cpu, used_mem_bytes, total_mem_bytes) = {
        let mut monitor = monitor.lock().map_err(|e| e.to_string())?;
        if !paused {
            monitor.refresh_system();
        }
        let (used, total) = monitor.memory_usage();
        (monitor.total_cpu(), used, total)
    };
    let host = {
        let mut sampler = sampler.lock().map_err(|e| e.to_string())?;
        if paused {
            sampler.snapshot()
        } else {
            sampler.refresh()
        }
    };
    if !paused && total_mem_bytes > 0 {
        let ratio = (used_mem_bytes as f32 / total_mem_bytes as f32) * 100.0;
        for body in crate::notify::resource_alerts(&db, cpu, ratio) {
            use tauri_plugin_notification::NotificationExt;
            let _ = app
                .notification()
                .builder()
                .title("AgentHQ")
                .body(body)
                .show();
        }
    }
    Ok(HostView {
        cpu,
        used_mem_bytes,
        total_mem_bytes,
        disks: host.disks,
        networks: host.networks,
        paused,
    })
}

#[tauri::command]
fn get_settings(db: State<Db>) -> Result<Vec<(String, String)>, String> {
    db.list_settings()
}

#[tauri::command]
fn set_setting(
    app: tauri::AppHandle,
    db: State<Db>,
    key: String,
    value: String,
) -> Result<(), String> {
    db.set_setting(&key, &value)?;
    if key == "monitoring.paused" {
        tray::PAUSED.store(value == "1", std::sync::atomic::Ordering::SeqCst);
        let _ = tray::refresh_menu(&app);
    }
    if key == "tray.start_with_windows" {
        tray::set_run_at_login(value == "1")?;
    }
    Ok(())
}

#[tauri::command]
fn stop_agent(
    monitor: State<Mutex<ProcessMonitor>>,
    manager: State<Mutex<AgentManager>>,
    agent_id: String,
) -> Result<Vec<u32>, String> {
    let names = manager
        .lock()
        .map_err(|e| e.to_string())?
        .executable_names_for(&agent_id);
    if names.is_empty() {
        return Err("unknown agent".into());
    }
    let refs: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
    let mut monitor = monitor.lock().map_err(|e| e.to_string())?;
    if !paused() {
        monitor.refresh();
    }
    let pids = lifecycle::owned_pids(&refs, &monitor.snapshot(), std::process::id());
    drop(monitor);
    for pid in &pids {
        let status = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err(format!("could not stop pid {pid}"));
        }
    }
    Ok(pids)
}

#[tauri::command]
fn start_agent(db: State<Db>, agent_id: String) -> Result<(), String> {
    let row = db
        .get_agent(&agent_id)?
        .ok_or_else(|| "agent not installed".to_string())?;
    let exe = row
        .executable_path
        .filter(|p| !p.is_empty())
        .ok_or_else(|| "executable path unknown".to_string())?;
    std::process::Command::new(&exe)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn open_agent_terminal(db: State<Db>, agent_id: String) -> Result<(), String> {
    let projects = db.list_projects()?;
    let dir = projects
        .into_iter()
        .find(|p| p.agent_ids.iter().any(|id| id == &agent_id))
        .map(|p| p.path)
        .ok_or_else(|| "no project directory for this agent".to_string())?;
    if !std::path::Path::new(&dir).is_dir() {
        return Err("project directory is missing".into());
    }
    std::process::Command::new("cmd")
        .args(["/c", "start", "", "/D", &dir, "cmd"])
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if tray::close_to_tray(window.app_handle()) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("app data dir unavailable: {e}"))?;
            std::fs::create_dir_all(&dir)
                .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
            let db_path = dir.join("agenthq.db");
            let db = Db::open(&db_path)
                .map_err(|e| format!("database open failed at {}: {e}", db_path.display()))?;
            app.manage(db);
            app.manage(Mutex::new(ProcessMonitor::new()));
            app.manage(Mutex::new(
                crate::monitoring::resources::ResourceSampler::new(),
            ));
            if app
                .state::<Db>()
                .get_setting("monitoring.paused")
                .ok()
                .flatten()
                .as_deref()
                == Some("1")
            {
                tray::PAUSED.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            if let Err(e) = tray::install(app) {
                let _ = app
                    .state::<Db>()
                    .insert_event("error", None, "tray.failed", &e);
            }
            if app
                .state::<Db>()
                .get_setting("tray.launch_minimized")
                .ok()
                .flatten()
                .as_deref()
                == Some("1")
            {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            let mut agent_manager = AgentManager::new();
            agent_manager.register(Box::new(ClaudeAdapter::new()));
            agent_manager.register(Box::new(OpenCodeAdapter::new()));
            agent_manager.register(Box::new(CodexAdapter::new()));
            agent_manager.register(Box::new(GrokAdapter::new()));
            app.manage(Mutex::new(agent_manager));
            if let Err(e) = (|| -> Result<(), String> {
                let monitor = app.state::<Mutex<ProcessMonitor>>();
                let manager = app.state::<Mutex<AgentManager>>();
                let db = app.state::<Db>();
                let mut monitor = monitor.lock().map_err(|e| e.to_string())?;
                monitor.refresh();
                let snapshot = monitor.snapshot();
                drop(monitor);
                let manager = manager.lock().map_err(|e| e.to_string())?;
                manager.detect_all(&snapshot, &db)?;
                // Best-effort engine sync so detail tabs are populated on
                // first launch without waiting for a manual Refresh.
                startup_sync(&manager, &db)?;
                let _ = tray::refresh_menu(app.handle());
                // Config watcher keeps the DB fresh without polling.
                watch::spawn(app.handle().clone());
                Ok(())
            })() {
                let _ = app
                    .state::<Db>()
                    .insert_event("error", None, "agent.detect_failed", &e);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            list_agents,
            get_process_snapshot,
            get_system_stats,
            refresh_agents,
            refresh_mcp,
            get_mcp_servers,
            refresh_skills,
            get_skills,
            refresh_plugins,
            get_plugins,
            refresh_sessions,
            get_sessions,
            get_projects,
            get_dashboard,
            get_events,
            get_host_resources,
            get_settings,
            set_setting,
            stop_agent,
            start_agent,
            open_agent_terminal,
            refresh_models,
            get_models,
            refresh_connections,
            get_connections
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::startup_sync;
    use crate::agents::manager::AgentManager;
    use crate::agents::traits::{
        AgentAdapter, AgentCapabilities, DetectionResult, McpServerDetail, McpTransport,
    };
    use crate::database::repository::Db;

    struct FakeMcp;
    impl AgentAdapter for FakeMcp {
        fn id(&self) -> &'static str {
            "fake-mcp"
        }
        fn name(&self) -> &'static str {
            "Fake MCP"
        }
        fn detect_installation(&self) -> DetectionResult {
            DetectionResult {
                installed: true,
                executable_path: None,
                version: None,
            }
        }
        fn capabilities(&self) -> AgentCapabilities {
            AgentCapabilities {
                processes: false,
                sessions: false,
                subagents: false,
                mcp: true,
                skills: false,
                plugins: false,
                models: false,
                connections: false,
                logs: false,
                lifecycle_control: false,
            }
        }
        fn mcp_details(&self) -> crate::agents::traits::CapabilityResult<Vec<McpServerDetail>> {
            Ok(vec![McpServerDetail {
                name: "gh".to_string(),
                transport: McpTransport::Stdio,
                command: Some("gh-mcp".to_string()),
                args: vec![],
                url: None,
                env_count: 1,
                project: None,
                enabled: None,
            }])
        }
    }

    fn temp_db(tag: &str) -> Db {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-startup-{}_{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        dir.push("t.db");
        Db::open(&dir).unwrap()
    }

    #[test]
    fn test_startup_sync_persists_and_is_idempotent() {
        let db = temp_db("sync");
        let mut manager = AgentManager::new();
        manager.register(Box::new(FakeMcp));
        startup_sync(&manager, &db).unwrap();
        startup_sync(&manager, &db).unwrap();
        let rows = db.list_mcp_servers(Some("fake-mcp")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "gh");
    }
}
