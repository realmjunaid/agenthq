//! Codex CLI adapter. Implemented in Step 3.
use std::path::PathBuf;

use super::super::detect::run_version;
use super::super::manager::exe_stem_matches;
use super::super::traits::{
    AgentAdapter, AgentCapabilities, AgentProcess, AgentSession, CapabilityResult, DetectionResult,
};
use super::{parser, paths};
use crate::monitoring::process::ProcessInfo;

pub struct CodexAdapter {
    home: PathBuf,
}

impl CodexAdapter {
    pub fn new() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_default();
        Self::with_home(home)
    }

    pub fn with_home(home: PathBuf) -> Self {
        CodexAdapter { home }
    }
}

const EXE_NAMES: &[&str] = &["codex"];

impl CodexAdapter {
    /// Pure verdict: exe present OR config dir present means installed.
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

impl AgentAdapter for CodexAdapter {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn name(&self) -> &'static str {
        "Codex CLI"
    }

    fn executable_names(&self) -> &[&str] {
        EXE_NAMES
    }

    fn detect_installation(&self) -> DetectionResult {
        let exe = paths::known_exes(&self.home).into_iter().next();
        let config_present = paths::codex_dir(&self.home).is_dir();
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
        let text = match std::fs::read_to_string(&paths::session_index(&self.home)) {
            Ok(t) => t,
            Err(_) => return Ok(vec![]),
        };
        Ok(parser::parse_session_index(&text)
            .into_iter()
            .map(|s| AgentSession {
                id: s.id,
                status: "unknown".to_string(),
            })
            .collect())
    }

    fn sessions_details(
        &self,
        _project_dir: Option<&std::path::Path>,
    ) -> CapabilityResult<Vec<super::super::traits::SessionDetail>> {
        use super::super::traits::{SessionConfidence, SessionDetail};
        let text = match std::fs::read_to_string(&paths::session_index(&self.home)) {
            Ok(t) => t,
            Err(_) => return Ok(vec![]),
        };
        Ok(parser::parse_session_index(&text)
            .into_iter()
            .map(|s| SessionDetail {
                id: s.id,
                project: None,
                status: "unknown".to_string(),
                model: None,
                started_at: None,
                last_activity: None,
                confidence: SessionConfidence::Detected,
            })
            .collect())
    }

    fn mcp_servers(&self) -> CapabilityResult<Vec<super::super::traits::McpServerInfo>> {
        use super::super::traits::McpServerInfo;
        let text = match std::fs::read_to_string(&paths::config_file(&self.home)) {
            Ok(t) => t,
            Err(_) => return Ok(vec![]),
        };
        Ok(parser::mcp_server_names(&text)
            .into_iter()
            .map(|name| McpServerInfo { name })
            .collect())
    }

    fn mcp_details(&self) -> CapabilityResult<Vec<super::super::traits::McpServerDetail>> {
        use super::super::traits::{McpServerDetail, McpTransport};
        let text = match std::fs::read_to_string(&paths::config_file(&self.home)) {
            Ok(t) => t,
            Err(_) => return Ok(vec![]),
        };
        // Headers only: everything unknown, no env data exists.
        Ok(parser::mcp_server_names(&text)
            .into_iter()
            .map(|name| McpServerDetail {
                name,
                transport: McpTransport::Unknown,
                command: None,
                args: vec![],
                url: None,
                env_count: 0,
                project: None,
                enabled: None,
            })
            .collect())
    }

    fn skills_details(&self) -> CapabilityResult<Vec<super::super::traits::SkillDetail>> {
        use super::super::traits::SkillScope;
        use crate::agents::claude::parser::enumerate_skill_dirs;
        Ok(enumerate_skill_dirs(
            &paths::codex_dir(&self.home).join("skills"),
            SkillScope::Global,
            "codex-global",
        ))
    }

    fn skills(&self) -> CapabilityResult<Vec<super::super::traits::SkillInfo>> {
        use super::super::traits::SkillInfo;
        use crate::agents::claude::parser::skill_description;
        let mut out = vec![];
        let dir = paths::codex_dir(&self.home).join("skills");
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            out.push(SkillInfo {
                name,
                description: skill_description(&p),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn plugins(&self) -> CapabilityResult<Vec<super::super::traits::PluginInfo>> {
        use super::super::traits::PluginInfo;
        let mut out = vec![];
        let dir = paths::codex_dir(&self.home).join("plugins");
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            out.push(PluginInfo {
                name: e.file_name().to_string_lossy().into_owned(),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn plugins_details(&self) -> CapabilityResult<Vec<super::super::traits::PluginDetail>> {
        use super::super::traits::PluginDetail;
        let mut out = vec![];
        let dir = paths::codex_dir(&self.home).join("plugins");
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            out.push(PluginDetail {
                name: e.file_name().to_string_lossy().into_owned(),
                version: None,
                path: Some(p.to_string_lossy().into_owned()),
                source: None,
                enabled: true,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn models(&self) -> CapabilityResult<Vec<super::super::traits::ModelInfo>> {
        use super::super::traits::ModelInfo;
        let text =
            match std::fs::read_to_string(paths::codex_dir(&self.home).join("models_cache.json")) {
                Ok(t) => t,
                Err(_) => return Ok(vec![]),
            };
        Ok(parser::model_slugs(&text)
            .into_iter()
            .map(|name| ModelInfo { name })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::CodexAdapter;
    use crate::agents::traits::AgentAdapter;
    use std::path::PathBuf;

    fn temp_home(tag: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-codex-{}-{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_missing_home_is_not_installed() {
        let det = CodexAdapter::evaluate(None, false, None);
        assert!(!det.installed);
    }

    #[test]
    fn test_capabilities_match_verified_set() {
        let a = CodexAdapter::with_home(temp_home("caps"));
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
    fn test_missing_index_is_empty() {
        let a = CodexAdapter::with_home(temp_home("noidx"));
        assert!(a.sessions().unwrap().is_empty());
        assert!(a.sessions_details(None).unwrap().is_empty());
    }

    #[test]
    fn test_codex_index_details_detected() {
        let home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("codex")
            .join("home");
        let a = CodexAdapter::with_home(home);
        let details = a.sessions_details(None).unwrap();
        assert_eq!(details.len(), 2);
        assert_eq!(details[0].model, None);
        assert_eq!(
            details[0].confidence,
            crate::agents::traits::SessionConfidence::Detected
        );
        assert_eq!(details[0].status, "unknown");
    }

    fn fixture_adapter() -> CodexAdapter {
        let home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("codex")
            .join("home");
        CodexAdapter::with_home(home)
    }

    #[test]
    fn test_mcp_names_headers_only() {
        let names = super::super::parser::mcp_server_names(&fixture_text("config.toml"));
        assert_eq!(names, vec!["gh".to_string(), "quoted name".to_string()]);
    }

    #[test]
    fn test_config_canary_absent() {
        let a = fixture_adapter();
        let dump = format!("{:?}", a.mcp_servers());
        assert!(!dump.contains("CANARY-SECRET-1"));
    }

    #[test]
    fn test_skills_from_dirs() {
        let a = fixture_adapter();
        let skills = a.skills().unwrap();
        assert_eq!(skills.len(), 2);
        let demo = skills.iter().find(|s| s.name == "demo-skill").unwrap();
        assert!(demo.description.as_deref().unwrap().contains("fixture"));
    }

    #[test]
    fn test_codex_headers_become_unknown() {
        let a = fixture_adapter();
        let details = a.mcp_details().unwrap();
        assert_eq!(details.len(), 2);
        for d in &details {
            assert_eq!(d.transport, crate::agents::traits::McpTransport::Unknown);
            assert_eq!(d.env_count, 0);
            assert_eq!(d.enabled, None);
        }
    }

    #[test]
    fn test_plugins_and_models() {
        let a = fixture_adapter();
        let plugins = a.plugins().unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "plugin-a");
        let models = a.models().unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "demo-model");
        let empty = super::super::parser::model_slugs("{}");
        assert!(empty.is_empty());
    }

    fn fixture_text(name: &str) -> String {
        std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join("codex")
                .join("home")
                .join(".codex")
                .join(name),
        )
        .unwrap()
    }

    #[test]
    fn test_processes_filters_by_stem() {
        use crate::monitoring::process::ProcessInfo;
        let a = CodexAdapter::with_home(temp_home("procs"));
        let rows = vec![
            ProcessInfo::new(
                1,
                None,
                "c".into(),
                Some("C:\\bin\\codex.exe".into()),
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
    fn test_no_auth_reader_exists() {
        // Needle built dynamically so this test file itself doesn't match.
        let needle = ["auth", "json"].join(".");
        assert!(
            !include_str!("adapter.rs").contains(&needle)
                && !include_str!("parser.rs").contains(&needle)
                && !include_str!("paths.rs").contains(&needle)
        );
    }

    #[test]
    fn test_skills_details_global() {
        let home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("codex")
            .join("home");
        let a = CodexAdapter::with_home(home);
        let details = a.skills_details().unwrap();
        assert_eq!(details.len(), 2);
        let demo = details.iter().find(|d| d.name == "demo-skill").unwrap();
        assert_eq!(demo.scope, crate::agents::traits::SkillScope::Global);
        assert_eq!(demo.source.as_deref(), Some("codex-global"));
        assert!(demo.description.as_deref().unwrap().contains("fixture"));
    }

    #[test]
    fn test_plugins_details_evidence() {
        let home = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("codex")
            .join("home");
        let a = CodexAdapter::with_home(home);
        let details = a.plugins_details().unwrap();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].name, "plugin-a");
        assert_eq!(details[0].version, None);
        assert!(details[0].enabled);
    }
}
