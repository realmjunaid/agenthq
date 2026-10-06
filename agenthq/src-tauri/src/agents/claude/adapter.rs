use std::path::{Path, PathBuf};

use super::super::detect::run_version;
use super::super::manager::exe_stem_matches;
use super::super::traits::{
    AgentAdapter, AgentCapabilities, AgentProcess, AgentSession, CapabilityResult, DetectionResult,
    McpServerDetail, SkillDetail, SkillScope,
};
use super::{parser, paths};
use crate::monitoring::process::ProcessInfo;

pub struct ClaudeAdapter {
    home: PathBuf,
}

impl ClaudeAdapter {
    pub fn new() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_default();
        Self::with_home(home)
    }

    pub fn with_home(home: PathBuf) -> Self {
        ClaudeAdapter { home }
    }

    /// slug → real path from `.claude.json` project keys.
    fn slug_map(&self) -> std::collections::HashMap<String, String> {
        match parser::read_json_file(&paths::claude_json(&self.home)) {
            Some(doc) => parser::slug_path_map(&doc),
            None => std::collections::HashMap::new(),
        }
    }
}

const EXE_NAMES: &[&str] = &["claude"];

impl ClaudeAdapter {
    /// Pure verdict: exe present OR config present means installed;
    /// a failed version probe never clears installed.
    fn evaluate(
        exe: Option<PathBuf>,
        config_present: bool,
        version: Option<String>,
    ) -> DetectionResult {
        DetectionResult {
            installed: exe.is_some() || config_present,
            executable_path: exe.map(|p| p.to_string_lossy().into_owned()),
            version,
        }
    }
}

impl AgentAdapter for ClaudeAdapter {
    fn id(&self) -> &'static str {
        "claude"
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }

    fn executable_names(&self) -> &[&str] {
        EXE_NAMES
    }

    fn detect_installation(&self) -> DetectionResult {
        let exe = paths::known_exes(&self.home).into_iter().next();
        let config_present = paths::home_claude(&self.home).is_dir();
        let version = exe
            .as_ref()
            .and_then(|e| run_version(&e.to_string_lossy(), &["--version"], 5000).ok())
            .and_then(|raw| parser::parse_version(&raw));
        Self::evaluate(exe, config_present, version)
    }

    fn capabilities(&self) -> AgentCapabilities {
        AgentCapabilities {
            processes: true,
            sessions: true,
            subagents: false,
            mcp: true,
            skills: true,
            plugins: true,
            models: true,
            connections: true,
            logs: false,
            lifecycle_control: true,
        }
    }

    #[allow(dead_code)] // Called by Phase 14 detail commands.
    fn processes(&self, snapshot: &[ProcessInfo]) -> Vec<AgentProcess> {
        snapshot
            .iter()
            .filter(|p| exe_stem_matches(p.exe.as_deref(), EXE_NAMES))
            .map(|p| AgentProcess {
                pid: p.pid,
                name: p.name.clone(),
            })
            .collect()
    }

    #[allow(dead_code)] // Called by Phase 14 detail commands.
    fn sessions(&self) -> CapabilityResult<Vec<AgentSession>> {
        let mut out = vec![];
        let projects = paths::home_claude(&self.home).join("projects");
        let entries = match std::fs::read_dir(&projects) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        for proj in entries.flatten() {
            let dir = proj.path();
            if !dir.is_dir() {
                continue;
            }
            let files = match std::fs::read_dir(&dir) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for f in files.flatten() {
                let fp = f.path();
                if fp.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }
                if let Some(lite) = parser::parse_session_file(&fp) {
                    out.push(AgentSession {
                        id: lite.id.clone(),
                        status: "unknown".to_string(),
                    });
                }
            }
        }
        Ok(out)
    }

    fn sessions_details(
        &self,
        _project_dir: Option<&Path>,
    ) -> CapabilityResult<Vec<super::super::traits::SessionDetail>> {
        use super::super::traits::{SessionConfidence, SessionDetail};
        let slug_map = self.slug_map();
        let mut out = vec![];
        let projects = paths::home_claude(&self.home).join("projects");
        let entries = match std::fs::read_dir(&projects) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        for proj in entries.flatten() {
            let dir = proj.path();
            if !dir.is_dir() {
                continue;
            }
            let files = match std::fs::read_dir(&dir) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for f in files.flatten() {
                let fp = f.path();
                if fp.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }
                if let Some(lite) = parser::parse_session_file(&fp) {
                    out.push(SessionDetail {
                        id: lite.id,
                        project: slug_map.get(&lite.project).cloned(),
                        status: "unknown".to_string(),
                        model: lite.model,
                        started_at: lite.started_at,
                        last_activity: lite.last_activity,
                        confidence: SessionConfidence::Confirmed,
                    });
                }
            }
        }
        Ok(out)
    }

    fn projects_details(&self) -> CapabilityResult<Vec<super::super::traits::ProjectDetail>> {
        use super::super::traits::ProjectDetail;
        let slug_map = self.slug_map();
        let projects = paths::home_claude(&self.home).join("projects");
        let entries = match std::fs::read_dir(&projects) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        let mut activity: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        let mut slugs: Vec<String> = vec![];
        for proj in entries.flatten() {
            let dir = proj.path();
            if !dir.is_dir() {
                continue;
            }
            let slug = proj.file_name().to_string_lossy().into_owned();
            let files = match std::fs::read_dir(&dir) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for f in files.flatten() {
                let fp = f.path();
                if fp.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                    continue;
                }
                if let Some(lite) = parser::parse_session_file(&fp) {
                    if let Some(ts) = lite.last_activity {
                        activity
                            .entry(slug.clone())
                            .and_modify(|m| *m = (*m).max(ts))
                            .or_insert(ts);
                    }
                }
            }
            slugs.push(slug);
        }
        let mut out = vec![];
        for slug in slugs {
            if let Some(path) = slug_map.get(&slug) {
                out.push(ProjectDetail {
                    slug: slug.clone(),
                    path: Some(path.clone()),
                    last_seen: activity.get(&slug).copied(),
                });
            }
        }
        out.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(out)
    }

    #[allow(dead_code)] // Called by Phase 14 detail commands.
    fn mcp_servers(&self) -> CapabilityResult<Vec<super::super::traits::McpServerInfo>> {
        use super::super::traits::McpServerInfo;
        let mut out = vec![];
        let doc = match parser::read_json_file(&paths::claude_json(&self.home)) {
            Some(d) => d,
            None => return Ok(vec![]),
        };
        if let Some(projects) = doc.get("projects").and_then(|p| p.as_object()) {
            for (_proj, cfg) in projects {
                // NOTE: server `env` blocks are never read — names only.
                if let Some(servers) = cfg.get("mcpServers").and_then(|s| s.as_object()) {
                    for name in servers.keys() {
                        out.push(McpServerInfo { name: name.clone() });
                    }
                }
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn mcp_details(&self) -> CapabilityResult<Vec<McpServerDetail>> {
        let doc = match parser::read_json_file(&paths::claude_json(&self.home)) {
            Some(d) => d,
            None => return Ok(vec![]),
        };
        Ok(parser::parse_mcp_servers(&doc))
    }

    fn skills_details(&self) -> CapabilityResult<Vec<SkillDetail>> {
        Ok(parser::enumerate_skill_dirs(
            &paths::home_claude(&self.home).join("skills"),
            SkillScope::Global,
            "claude-global",
        ))
    }

    #[allow(dead_code)] // Called by Phase 14 detail commands.
    fn skills(&self) -> CapabilityResult<Vec<super::super::traits::SkillInfo>> {
        use super::super::traits::SkillInfo;
        let mut out = vec![];
        let dir = paths::home_claude(&self.home).join("skills");
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            out.push(SkillInfo {
                name: e.file_name().to_string_lossy().into_owned(),
                description: parser::skill_description(&p),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    #[allow(dead_code)] // Called by Phase 14 detail commands.
    fn plugins(&self) -> CapabilityResult<Vec<super::super::traits::PluginInfo>> {
        use super::super::traits::PluginInfo;
        let mut out = vec![];
        let markets = paths::home_claude(&self.home)
            .join("plugins")
            .join("marketplaces");
        let entries = match std::fs::read_dir(&markets) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        for market in entries.flatten() {
            if !market.path().is_dir() {
                continue;
            }
            for sub in ["plugins", "external_plugins"] {
                let dir = market.path().join(sub);
                let items = match std::fs::read_dir(&dir) {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                for item in items.flatten() {
                    if !item.path().is_dir() {
                        continue;
                    }
                    out.push(PluginInfo {
                        name: item.file_name().to_string_lossy().into_owned(),
                    });
                }
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn plugins_details(&self) -> CapabilityResult<Vec<super::super::traits::PluginDetail>> {
        use super::super::traits::PluginDetail;
        use std::collections::HashSet;
        let mut out = vec![];
        let mut seen = HashSet::new();
        let markets = paths::home_claude(&self.home)
            .join("plugins")
            .join("marketplaces");
        let entries = match std::fs::read_dir(&markets) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        // `plugins/` before `external_plugins/`: same name → first wins.
        for market in entries.flatten() {
            if !market.path().is_dir() {
                continue;
            }
            let market_name = market.file_name().to_string_lossy().into_owned();
            for kind in ["plugins", "external_plugins"] {
                let dir = market.path().join(kind);
                let items = match std::fs::read_dir(&dir) {
                    Ok(i) => i,
                    Err(_) => continue,
                };
                for item in items.flatten() {
                    if !item.path().is_dir() {
                        continue;
                    }
                    let name = item.file_name().to_string_lossy().into_owned();
                    if !seen.insert(name.clone()) {
                        continue;
                    }
                    out.push(PluginDetail {
                        name,
                        version: None,
                        path: Some(item.path().to_string_lossy().into_owned()),
                        source: Some(format!("{market_name}/{kind}")),
                        enabled: true,
                    });
                }
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    #[allow(dead_code)] // Called by Phase 14 detail commands.
    fn models(&self) -> CapabilityResult<Vec<super::super::traits::ModelInfo>> {
        use super::super::traits::ModelInfo;
        let settings =
            match parser::read_json_file(&paths::home_claude(&self.home).join("settings.json")) {
                Some(s) => s,
                None => return Ok(vec![]),
            };
        match settings.get("model").and_then(|m| m.as_str()) {
            Some(name) => Ok(vec![ModelInfo {
                name: name.to_string(),
            }]),
            None => Ok(vec![]),
        }
    }

    #[allow(dead_code)] // Called by Phase 14 detail commands.
    fn connections(&self) -> CapabilityResult<Vec<super::super::traits::ConnectionInfo>> {
        use super::super::traits::ConnectionInfo;
        let settings =
            match parser::read_json_file(&paths::home_claude(&self.home).join("settings.json")) {
                Some(s) => s,
                None => return Ok(vec![]),
            };
        // NOTE: key names only — values are never read.
        let mut out = vec![];
        if let Some(env) = settings.get("env").and_then(|e| e.as_object()) {
            for key in env.keys() {
                out.push(ConnectionInfo { name: key.clone() });
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::ClaudeAdapter;
    use crate::agents::traits::AgentAdapter;
    use std::path::PathBuf;

    fn adapter_in(home: &str) -> (ClaudeAdapter, PathBuf) {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-claude-{}-{}", std::process::id(), home));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = ClaudeAdapter::with_home(dir.clone());
        (a, dir)
    }

    fn fixture_adapter() -> ClaudeAdapter {
        let home = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("claude")
            .join("home");
        ClaudeAdapter::with_home(home)
    }

    fn debug_all(a: &ClaudeAdapter) -> String {
        format!(
            "{:?}{:?}{:?}{:?}{:?}",
            a.mcp_servers(),
            a.skills(),
            a.plugins(),
            a.models(),
            a.connections()
        )
    }

    #[test]
    fn test_missing_home_is_not_installed() {
        let det = ClaudeAdapter::evaluate(None, false, None);
        assert!(!det.installed);
        assert_eq!(det.executable_path, None);
        assert_eq!(det.version, None);
    }

    #[test]
    fn test_exe_present_version_failed_stays_installed() {
        let det = ClaudeAdapter::evaluate(Some(PathBuf::from("C:\\bin\\claude.exe")), false, None);
        assert!(det.installed);
        assert_eq!(det.version, None);
    }

    #[test]
    fn test_config_only_is_installed() {
        let det = ClaudeAdapter::evaluate(None, true, None);
        assert!(det.installed);
        assert_eq!(det.executable_path, None);
    }

    #[test]
    fn test_capabilities_match_verified_set() {
        let (a, _d) = adapter_in("caps");
        let c = a.capabilities();
        assert!(
            c.processes
                && c.sessions
                && c.mcp
                && c.skills
                && c.plugins
                && c.models
                && c.connections
                && c.lifecycle_control
        );
        assert!(!c.subagents && !c.logs);
    }

    #[test]
    fn test_missing_projects_dir_is_empty() {
        let (a, _d) = adapter_in("noproj");
        let sessions = a.sessions().unwrap();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_processes_filters_by_stem() {
        use crate::monitoring::process::ProcessInfo;
        let (a, _d) = adapter_in("procs");
        let rows = vec![
            ProcessInfo::new(
                1,
                None,
                "c".into(),
                Some("C:\\bin\\claude.exe".into()),
                0.0,
                0,
                0,
                None,
            ),
            ProcessInfo::new(
                2,
                None,
                "o".into(),
                Some("C:\\bin\\other.exe".into()),
                0.0,
                0,
                0,
                None,
            ),
            ProcessInfo::new(3, None, "claude".into(), None, 0.0, 0, 0, None),
        ];
        let procs = a.processes(&rows);
        assert_eq!(procs.len(), 1);
        assert_eq!(procs[0].pid, 1);
    }

    #[test]
    fn test_mcp_names_and_status() {
        let a = fixture_adapter();
        let servers = a.mcp_servers().unwrap();
        assert_eq!(servers.len(), 2);
        let names: Vec<&str> = servers.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"gh"));
        assert!(names.contains(&"local-docs"));
    }

    #[test]
    fn test_mcp_env_count_only() {
        let dump = debug_all(&fixture_adapter());
        assert!(!dump.contains("SECRET2"), "env values must never surface");
    }

    #[test]
    fn test_skills_from_dirs() {
        let a = fixture_adapter();
        let skills = a.skills().unwrap();
        assert_eq!(skills.len(), 2);
        let demo = skills.iter().find(|s| s.name == "demo-skill").unwrap();
        assert!(demo
            .description
            .as_deref()
            .unwrap()
            .contains("fixture skill"));
        let plain = skills.iter().find(|s| s.name == "plain-skill").unwrap();
        assert_eq!(plain.description, None);
    }

    #[test]
    fn test_models_and_connections() {
        let a = fixture_adapter();
        let models = a.models().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "auto");
        let conns = a.connections().unwrap();
        assert_eq!(conns.len(), 1);
        assert_eq!(conns[0].name, "FAKE_PROVIDER_KEY");
    }

    #[test]
    fn test_no_secret_values_leak() {
        let dump = debug_all(&fixture_adapter());
        assert!(!dump.contains("FAKE-SECRET-xyz"));
        assert!(!dump.contains("SECRET2"));
    }

    #[test]
    fn test_plugins_from_marketplaces() {
        let a = fixture_adapter();
        let plugins = a.plugins().unwrap();
        let names: Vec<&str> = plugins.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["plugin-a", "plugin-b"]);
    }

    #[test]
    fn test_plugins_details_evidence() {
        let a = fixture_adapter();
        let details = a.plugins_details().unwrap();
        assert_eq!(details.len(), 2);
        let b = details.iter().find(|d| d.name == "plugin-b").unwrap();
        assert_eq!(b.version, None);
        assert!(b.path.as_deref().unwrap().ends_with("plugin-b"));
        assert!(b.source.as_deref().unwrap().contains("demo-market"));
        assert!(b.enabled);
    }

    #[test]
    fn test_skills_details_global() {
        let a = fixture_adapter();
        let details = a.skills_details().unwrap();
        assert_eq!(details.len(), 2);
        let demo = details.iter().find(|d| d.name == "demo-skill").unwrap();
        assert_eq!(demo.scope, crate::agents::traits::SkillScope::Global);
        assert_eq!(demo.source.as_deref(), Some("claude-global"));
        assert!(demo.path.as_deref().unwrap().ends_with("demo-skill"));
        assert!(demo.description.as_deref().unwrap().contains("fixture"));
    }

    #[test]
    fn test_kind_collision_first_wins() {
        let mut home = std::env::temp_dir();
        home.push(format!("agenthq-plug-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let market = home
            .join(".claude")
            .join("plugins")
            .join("marketplaces")
            .join("m");
        std::fs::create_dir_all(market.join("plugins").join("dup")).unwrap();
        std::fs::create_dir_all(market.join("external_plugins").join("dup")).unwrap();
        let a = ClaudeAdapter::with_home(home);
        let details = a.plugins_details().unwrap();
        assert_eq!(details.len(), 1);
        assert!(details[0].source.as_deref().unwrap().ends_with("plugins"));
    }

    fn claude_fixture_home() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("claude")
            .join("home")
    }

    #[test]
    fn test_session_details_full() {
        let a = ClaudeAdapter::with_home(claude_fixture_home());
        let details = a.sessions_details(None).unwrap();
        assert_eq!(details.len(), 2);
        let full = details
            .iter()
            .find(|d| d.id == "11111111-1111-4111-8111-111111111111")
            .unwrap();
        assert_eq!(full.model.as_deref(), Some("sonnet"));
        assert_eq!(full.started_at, Some(1750000000));
        assert!(full.last_activity.unwrap() >= 1750000060);
        assert_eq!(full.status, "unknown");
        assert_eq!(
            full.confidence,
            crate::agents::traits::SessionConfidence::Confirmed
        );
        assert_eq!(full.project.as_deref(), Some("E:/demo"));
    }

    #[test]
    fn test_session_without_model_is_unknowns() {
        let a = ClaudeAdapter::with_home(claude_fixture_home());
        let details = a.sessions_details(None).unwrap();
        let bare = details
            .iter()
            .find(|d| d.id == "22222222-2222-4222-8222-222222222222")
            .unwrap();
        assert_eq!(bare.model, None);
        assert_eq!(bare.started_at, Some(1750000100));
        assert_eq!(bare.status, "unknown");
    }

    #[test]
    fn test_projects_resolve_paths() {
        let a = ClaudeAdapter::with_home(claude_fixture_home());
        let projects = a.projects_details().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].slug, "E--demo");
        assert_eq!(projects[0].path.as_deref(), Some("E:/demo"));
        assert!(projects[0].last_seen.unwrap() >= 1750000060);
    }

    #[test]
    fn test_unmatched_slug_skipped() {
        let mut home = std::env::temp_dir();
        home.push(format!("agenthq-slug-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let proj = home.join(".claude").join("projects").join("Z--nowhere");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(
            proj.join("aaaaaaa1-1111-4111-8111-111111111111.jsonl"),
            "{\"type\":\"user\",\"timestamp\":1750000200}\n",
        )
        .unwrap();
        std::fs::write(home.join(".claude.json"), r#"{"projects": {}}"#).unwrap();
        let a = ClaudeAdapter::with_home(home);
        assert!(a.projects_details().unwrap().is_empty());
        let sessions = a.sessions_details(None).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].project, None);
    }

    #[test]
    fn test_slugify_verified_examples() {
        assert_eq!(
            crate::agents::claude::parser::slugify("C:/Users/j4u87"),
            "C--Users-j4u87"
        );
        assert_eq!(
            crate::agents::claude::parser::slugify("E:/TempMail Project"),
            "E--TempMail-Project"
        );
    }

    #[test]
    fn test_slug_collision_first_wins() {
        let mut home = std::env::temp_dir();
        home.push(format!("agenthq-collide-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".claude").join("projects").join("a-b")).unwrap();
        std::fs::write(
            home.join(".claude.json"),
            r#"{"projects": {"a/b": {}, "a:b": {}}}"#,
        )
        .unwrap();
        let a = ClaudeAdapter::with_home(home);
        let projects = a.projects_details().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].path.as_deref(), Some("a/b"));
    }
}
