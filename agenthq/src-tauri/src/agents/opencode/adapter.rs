use std::path::PathBuf;

use super::super::detect::{run_command_lines, run_version};
use super::super::manager::exe_stem_matches;
use super::super::traits::{
    AgentAdapter, AgentCapabilities, AgentProcess, AgentSession, CapabilityResult, DetectionResult,
};
use super::{parser, paths};
use crate::monitoring::process::ProcessInfo;

pub struct OpenCodeAdapter {
    home: PathBuf,
    exe_override: Option<String>,
    project_dir: Option<PathBuf>,
}

impl OpenCodeAdapter {
    pub fn new() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_default();
        Self::with_home(home)
    }

    pub fn with_home(home: PathBuf) -> Self {
        OpenCodeAdapter {
            home,
            exe_override: None,
            project_dir: None,
        }
    }

    /// Hermetic tests: bypass PATH/known-location probing.
    #[cfg(test)]
    pub fn with_home_and_exe(home: PathBuf, exe: &str) -> Self {
        OpenCodeAdapter {
            home,
            exe_override: Some(exe.to_string()),
            project_dir: None,
        }
    }

    /// Project context for per-project queries (sessions). Phase 12 wires it.
    #[allow(dead_code)] // Wired in Phase 12.
    pub fn with_project_dir(mut self, dir: PathBuf) -> Self {
        self.project_dir = Some(dir);
        self
    }

    fn exe(&self) -> Option<String> {
        if let Some(e) = &self.exe_override {
            return Some(e.clone());
        }
        paths::known_exes(&self.home)
            .into_iter()
            .next()
            .map(|p| p.to_string_lossy().into_owned())
    }
}

const EXE_NAMES: &[&str] = &["opencode"];

impl OpenCodeAdapter {
    /// Pure verdict: exe present OR config present means installed.
    fn evaluate(
        exe: Option<String>,
        config_present: bool,
        version: Option<String>,
    ) -> DetectionResult {
        DetectionResult {
            installed: exe.is_some() || config_present,
            executable_path: exe,
            version,
        }
    }
}

impl AgentAdapter for OpenCodeAdapter {
    fn id(&self) -> &'static str {
        "opencode"
    }

    fn name(&self) -> &'static str {
        "OpenCode"
    }

    fn executable_names(&self) -> &[&str] {
        EXE_NAMES
    }

    fn detect_installation(&self) -> DetectionResult {
        let exe = self.exe();
        let config_present = paths::global_config(&self.home).is_file();
        let version = exe
            .as_ref()
            .and_then(|e| run_version(e, &["--version"], 5000).ok())
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
            connections: false,
            logs: false,
            lifecycle_control: true,
        }
    }

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

    fn sessions(&self) -> CapabilityResult<Vec<AgentSession>> {
        let dir = match &self.project_dir {
            Some(d) => d,
            None => return Ok(vec![]),
        };
        let exe = match self.exe() {
            Some(e) => e,
            None => return Ok(vec![]),
        };
        let lines = match run_command_lines(
            &exe,
            &["session", "list", "--format", "json", "-n", "100"],
            Some(dir),
            10_000,
        ) {
            Ok(l) => l,
            Err(_) => return Ok(vec![]),
        };
        Ok(parser::parse_session_list(&lines.join("\n"))
            .into_iter()
            .map(|s| AgentSession {
                id: s.id,
                status: "unknown".to_string(),
            })
            .collect())
    }

    fn sessions_details(
        &self,
        project_dir: Option<&std::path::Path>,
    ) -> CapabilityResult<Vec<super::super::traits::SessionDetail>> {
        use super::super::traits::{SessionConfidence, SessionDetail};
        let dir = match project_dir {
            Some(d) => d,
            None => return Ok(vec![]),
        };
        let exe = match self.exe() {
            Some(e) => e,
            None => return Ok(vec![]),
        };
        let lines = match run_command_lines(
            &exe,
            &["session", "list", "--format", "json", "-n", "100"],
            Some(dir),
            10_000,
        ) {
            Ok(l) => l,
            Err(_) => return Ok(vec![]),
        };
        Ok(parser::parse_session_list(&lines.join("\n"))
            .into_iter()
            .map(|s| SessionDetail {
                id: s.id,
                project: s.project,
                status: "unknown".to_string(),
                model: None,
                started_at: parser::ms_to_s(s.created_ms),
                last_activity: parser::ms_to_s(s.updated_ms),
                confidence: SessionConfidence::Detected,
            })
            .collect())
    }

    fn mcp_servers(&self) -> CapabilityResult<Vec<super::super::traits::McpServerInfo>> {
        use super::super::traits::McpServerInfo;
        // NOTE: server `env` blocks are never read — names only.
        let doc = match crate::agents::claude::parser::read_json_file(&paths::global_config(
            &self.home,
        )) {
            Some(d) => d,
            None => return Ok(vec![]),
        };
        let mut out = vec![];
        if let Some(servers) = doc.get("mcp").and_then(|m| m.as_object()) {
            for name in servers.keys() {
                out.push(McpServerInfo { name: name.clone() });
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn mcp_details(&self) -> CapabilityResult<Vec<super::super::traits::McpServerDetail>> {
        let doc = match crate::agents::claude::parser::read_json_file(&paths::global_config(
            &self.home,
        )) {
            Some(d) => d,
            None => return Ok(vec![]),
        };
        Ok(parser::parse_mcp_object(&doc))
    }

    fn skills_details(&self) -> CapabilityResult<Vec<super::super::traits::SkillDetail>> {
        use super::super::traits::SkillScope;
        use crate::agents::claude::parser::enumerate_skill_dirs;
        Ok(enumerate_skill_dirs(
            &paths::global_skills(&self.home),
            SkillScope::Global,
            "opencode-global",
        ))
    }

    fn skills(&self) -> CapabilityResult<Vec<super::super::traits::SkillInfo>> {
        use super::super::traits::SkillInfo;
        use crate::agents::claude::parser::skill_description;
        let mut out = vec![];
        let entries = match std::fs::read_dir(&paths::global_skills(&self.home)) {
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
                description: skill_description(&p),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn plugins(&self) -> CapabilityResult<Vec<super::super::traits::PluginInfo>> {
        use super::super::traits::PluginInfo;
        let exe = match self.exe() {
            Some(e) => e,
            None => return Ok(vec![]),
        };
        let lines = match run_command_lines(&exe, &["plugin", "list"], None, 10_000) {
            Ok(l) => l,
            Err(_) => return Ok(vec![]),
        };
        Ok(parser::parse_plugin_list(&lines.join("\n"))
            .into_iter()
            .map(|name| PluginInfo { name })
            .collect())
    }

    fn plugins_details(&self) -> CapabilityResult<Vec<super::super::traits::PluginDetail>> {
        use super::super::traits::PluginDetail;
        let exe = match self.exe() {
            Some(e) => e,
            None => return Ok(vec![]),
        };
        let lines = match run_command_lines(&exe, &["plugin", "list"], None, 10_000) {
            Ok(l) => l,
            Err(_) => return Ok(vec![]),
        };
        Ok(parser::parse_plugin_rows(&lines.join("\n"))
            .into_iter()
            .map(|r| PluginDetail {
                name: r.name,
                version: r.version,
                path: None,
                source: None,
                enabled: true,
            })
            .collect())
    }

    fn models(&self) -> CapabilityResult<Vec<super::super::traits::ModelInfo>> {
        use super::super::traits::ModelInfo;
        let exe = match self.exe() {
            Some(e) => e,
            None => return Ok(vec![]),
        };
        let lines = match run_command_lines(&exe, &["models"], None, 10_000) {
            Ok(l) => l,
            Err(_) => return Ok(vec![]),
        };
        Ok(parser::parse_model_list(&lines.join("\n"))
            .into_iter()
            .map(|name| ModelInfo { name })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::OpenCodeAdapter;
    use crate::agents::traits::AgentAdapter;
    use std::path::PathBuf;

    #[test]
    fn test_missing_home_is_not_installed() {
        let det = OpenCodeAdapter::evaluate(None, false, None);
        assert!(!det.installed);
    }

    #[test]
    fn test_sessions_without_project_is_empty() {
        let a = OpenCodeAdapter::with_home(PathBuf::from("no-such-dir"));
        assert!(a.sessions().unwrap().is_empty());
    }

    #[test]
    fn test_sessions_cli_error_is_empty() {
        let a = OpenCodeAdapter::with_home_and_exe(
            PathBuf::from("no-such-dir"),
            "agenthq-no-such-bin-xyz",
        )
        .with_project_dir(PathBuf::from("no-such-dir"));
        assert!(a.sessions().unwrap().is_empty());
    }

    #[test]
    fn test_session_details_ms_to_seconds() {
        assert_eq!(
            crate::agents::opencode::parser::ms_to_s(Some(1791202563262)),
            Some(1791202563)
        );
        assert_eq!(crate::agents::opencode::parser::ms_to_s(None), None);
    }

    fn fixture_adapter() -> OpenCodeAdapter {
        let home = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("opencode")
            .join("home");
        OpenCodeAdapter::with_home(home)
    }

    #[test]
    fn test_mcp_names_only() {
        let a = fixture_adapter();
        let servers = a.mcp_servers().unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "gh");
        let dump = format!("{servers:?}");
        assert!(!dump.contains("SECRET3"));
    }

    #[test]
    fn test_adapter_details_match_parser() {
        let a = fixture_adapter();
        let details = a.mcp_details().unwrap();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].name, "gh");
        assert_eq!(details[0].env_count, 1);
        let dump = format!("{details:?}");
        assert!(!dump.contains("SECRET3"));
    }

    #[test]
    fn test_skills_from_dirs() {
        let a = fixture_adapter();
        let skills = a.skills().unwrap();
        assert_eq!(skills.len(), 2);
        let demo = skills.iter().find(|s| s.name == "demo-skill").unwrap();
        assert!(demo.description.as_deref().unwrap().contains("fixture"));
        let plain = skills.iter().find(|s| s.name == "plain").unwrap();
        assert_eq!(plain.description, None);
    }

    #[test]
    fn test_no_secret_values_leak() {
        let a = fixture_adapter();
        let dump = format!("{:?}{:?}{:?}", a.mcp_servers(), a.skills(), a.models());
        assert!(!dump.contains("SECRET3"));
    }

    #[test]
    fn test_capabilities_match_verified_set() {
        let a = OpenCodeAdapter::with_home(PathBuf::from("no-such-dir"));
        let c = a.capabilities();
        assert!(
            c.processes
                && c.sessions
                && c.mcp
                && c.skills
                && c.plugins
                && c.models
                && c.lifecycle_control
        );
        assert!(!c.subagents && !c.connections && !c.logs);
    }

    #[test]
    fn test_processes_filters_by_stem() {
        use crate::monitoring::process::ProcessInfo;
        let a = OpenCodeAdapter::with_home(PathBuf::from("no-such-dir"));
        let rows = vec![
            ProcessInfo::new(
                1,
                None,
                "o".into(),
                Some("C:\\npm\\opencode.cmd".into()),
                0.0,
                0,
                0,
                None,
            ),
            ProcessInfo::new(2, None, "x".into(), None, 0.0, 0, 0, None),
        ];
        let procs = a.processes(&rows);
        assert_eq!(procs.len(), 1);
        assert_eq!(procs[0].pid, 1);
    }

    #[test]
    fn test_skills_details_global() {
        let home = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("opencode")
            .join("home");
        let a = OpenCodeAdapter::with_home(home);
        let details = a.skills_details().unwrap();
        assert_eq!(details.len(), 2);
        let demo = details.iter().find(|d| d.name == "demo-skill").unwrap();
        assert_eq!(demo.scope, crate::agents::traits::SkillScope::Global);
        assert_eq!(demo.source.as_deref(), Some("opencode-global"));
        assert!(demo.description.as_deref().unwrap().contains("fixture"));
    }

    #[test]
    fn test_plugins_details_empty_without_exe() {
        let a = OpenCodeAdapter::with_home_and_exe(
            PathBuf::from("no-such-dir"),
            "agenthq-no-such-bin-xyz",
        );
        assert!(a.plugins_details().unwrap().is_empty());
    }
}
