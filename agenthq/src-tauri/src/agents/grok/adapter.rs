//! Grok CLI adapter. Extended in Tasks 2-3.
use std::path::PathBuf;

use super::super::detect::run_version;
use super::super::manager::exe_stem_matches;
use super::super::traits::{
    AgentAdapter, AgentCapabilities, AgentProcess, AgentSession, CapabilityResult, DetectionResult,
};
use super::{parser, paths};
use crate::monitoring::process::ProcessInfo;

pub struct GrokAdapter {
    home: PathBuf,
    exe_override: Option<String>,
}

impl GrokAdapter {
    pub fn new() -> Self {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_default();
        Self::with_home(home)
    }

    pub fn with_home(home: PathBuf) -> Self {
        GrokAdapter {
            home,
            exe_override: None,
        }
    }

    /// Hermetic tests: bypass PATH/known-location probing.
    #[cfg(test)]
    pub fn with_home_and_exe(home: PathBuf, exe: &str) -> Self {
        GrokAdapter {
            home,
            exe_override: Some(exe.to_string()),
        }
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

const EXE_NAMES: &[&str] = &["grok"];

impl GrokAdapter {
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

    /// `mcp list --json` names. CLI missing/fails/unparseable → empty.
    pub(crate) fn mcp_list(&self) -> Vec<super::super::traits::McpServerDetail> {
        let exe = match self.exe() {
            Some(e) => e,
            None => return vec![],
        };
        let lines = match super::super::detect::run_command_lines(
            &exe,
            &["mcp", "list", "--json"],
            None,
            10_000,
        ) {
            Ok(l) => l,
            Err(_) => return vec![],
        };
        parser::parse_mcp_list(&lines.join("\n"))
    }
}

impl AgentAdapter for GrokAdapter {
    fn id(&self) -> &'static str {
        "grok"
    }

    fn name(&self) -> &'static str {
        "Grok CLI"
    }

    fn executable_names(&self) -> &[&str] {
        EXE_NAMES
    }

    fn detect_installation(&self) -> DetectionResult {
        let exe = match &self.exe_override {
            Some(e) => Some(PathBuf::from(e)),
            None => paths::known_exes(&self.home).into_iter().next(),
        };
        let config_present = paths::grok_dir(&self.home).is_dir();
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

    fn mcp_servers(&self) -> CapabilityResult<Vec<super::super::traits::McpServerInfo>> {
        use super::super::traits::McpServerInfo;
        Ok(self
            .mcp_list()
            .into_iter()
            .map(|d| McpServerInfo { name: d.name })
            .collect())
    }

    fn mcp_details(&self) -> CapabilityResult<Vec<super::super::traits::McpServerDetail>> {
        Ok(self.mcp_list())
    }

    fn skills_details(&self) -> CapabilityResult<Vec<super::super::traits::SkillDetail>> {
        use super::super::traits::SkillScope;
        use crate::agents::claude::parser::enumerate_skill_dirs;
        Ok(enumerate_skill_dirs(
            &paths::skills_dir(&self.home),
            SkillScope::Global,
            "grok-global",
        ))
    }

    fn skills(&self) -> CapabilityResult<Vec<super::super::traits::SkillInfo>> {
        use super::super::traits::SkillInfo;
        Ok(self
            .skills_details()
            .unwrap_or_default()
            .into_iter()
            .map(|d| SkillInfo {
                name: d.name,
                description: d.description,
            })
            .collect())
    }

    fn plugins_details(&self) -> CapabilityResult<Vec<super::super::traits::PluginDetail>> {
        use super::super::traits::PluginDetail;
        let mut out = vec![];
        let entries = match std::fs::read_dir(&paths::plugins_dir(&self.home)) {
            Ok(e) => e,
            Err(_) => return Ok(vec![]),
        };
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name.ends_with(".lock") {
                continue;
            }
            out.push(PluginDetail {
                name,
                version: None,
                path: Some(p.to_string_lossy().into_owned()),
                source: None,
                enabled: true,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn plugins(&self) -> CapabilityResult<Vec<super::super::traits::PluginInfo>> {
        use super::super::traits::PluginInfo;
        Ok(self
            .plugins_details()
            .unwrap_or_default()
            .into_iter()
            .map(|d| PluginInfo { name: d.name })
            .collect())
    }

    fn models(&self) -> CapabilityResult<Vec<super::super::traits::ModelInfo>> {
        use super::super::traits::ModelInfo;
        let text = match std::fs::read_to_string(&paths::models_cache(&self.home)) {
            Ok(t) => t,
            Err(_) => return Ok(vec![]),
        };
        Ok(parser::parse_models_cache(&text)
            .into_iter()
            .map(|name| ModelInfo { name })
            .collect())
    }

    fn sessions(&self) -> CapabilityResult<Vec<AgentSession>> {
        Ok(self
            .all_sessions()
            .into_iter()
            .map(|(id, _)| AgentSession {
                id,
                status: "unknown".to_string(),
            })
            .collect())
    }

    fn sessions_details(
        &self,
        _project_dir: Option<&std::path::Path>,
    ) -> CapabilityResult<Vec<super::super::traits::SessionDetail>> {
        use super::super::traits::{SessionConfidence, SessionDetail};
        Ok(self
            .all_sessions()
            .into_iter()
            .map(|(id, s)| SessionDetail {
                id,
                project: Some(s.project),
                status: "unknown".to_string(),
                model: s.model,
                started_at: s.started,
                last_activity: s.last,
                confidence: SessionConfidence::Confirmed,
            })
            .collect())
    }

    fn projects_details(&self) -> CapabilityResult<Vec<super::super::traits::ProjectDetail>> {
        use super::super::traits::ProjectDetail;
        let mut by_project: std::collections::HashMap<String, (String, Option<i64>)> =
            std::collections::HashMap::new();
        for (_, s) in self.all_sessions() {
            by_project
                .entry(s.slug.clone())
                .and_modify(|(_, last)| {
                    *last = (*last).max(s.last);
                })
                .or_insert((s.project.clone(), s.last));
        }
        let mut out: Vec<ProjectDetail> = by_project
            .into_iter()
            .map(|(slug, (path, last_seen))| ProjectDetail {
                slug,
                path: Some(path),
                last_seen,
            })
            .collect();
        out.sort_by(|a, b| a.slug.cmp(&b.slug));
        Ok(out)
    }
}

struct GrokSession {
    project: String,
    slug: String,
    model: Option<String>,
    started: Option<i64>,
    last: Option<i64>,
}

impl GrokAdapter {
    /// All sessions: hash dirs (usage.json) ∪ prompt_history session ids.
    /// Malformed rows skipped; unparseable timestamps → None.
    fn all_sessions(&self) -> Vec<(String, GrokSession)> {
        let mut map: std::collections::HashMap<String, GrokSession> =
            std::collections::HashMap::new();
        let base = paths::sessions_dir(&self.home);
        let projects = match std::fs::read_dir(&base) {
            Ok(e) => e,
            Err(_) => return vec![],
        };
        for proj in projects.flatten() {
            let dir = proj.path();
            if !dir.is_dir() {
                continue;
            }
            let slug = proj.file_name().to_string_lossy().into_owned();
            let project = parser::percent_decode(&slug);
            let starts = std::fs::read_to_string(dir.join("prompt_history.jsonl"))
                .map(|t| parser::prompt_starts(&t))
                .unwrap_or_default();
            let hashes = match std::fs::read_dir(&dir) {
                Ok(f) => f,
                Err(_) => continue,
            };
            for h in hashes.flatten() {
                if !h.path().is_dir() {
                    continue;
                }
                let id = h.file_name().to_string_lossy().into_owned();
                let usage = std::fs::read_to_string(h.path().join("usage.json"))
                    .map(|t| parser::parse_usage(&t))
                    .unwrap_or(parser::UsageInfo {
                        model: None,
                        updated_at: None,
                    });
                let started = starts.get(&id).copied();
                map.entry(id).or_insert(GrokSession {
                    project: project.clone(),
                    slug: slug.clone(),
                    model: usage.model,
                    started,
                    last: usage.updated_at,
                });
            }
            // Prompt-only sessions (no hash dir yet).
            for (id, started) in starts {
                map.entry(id).or_insert(GrokSession {
                    project: project.clone(),
                    slug: slug.clone(),
                    model: None,
                    started: Some(started),
                    last: None,
                });
            }
        }
        let mut out: Vec<(String, GrokSession)> = map.into_iter().collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::GrokAdapter;
    use crate::agents::traits::AgentAdapter;
    use std::path::PathBuf;

    fn temp_home(tag: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-grok-{}_{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_missing_home_is_not_installed() {
        let det = GrokAdapter::evaluate(None, false, None);
        assert!(!det.installed);
    }

    #[test]
    fn test_capabilities_match_verified_set() {
        let a = GrokAdapter::with_home(temp_home("caps"));
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
        let a = GrokAdapter::with_home(temp_home("procs"));
        let rows = vec![
            ProcessInfo::new(
                1,
                None,
                "g".into(),
                Some("C:\\bin\\grok.exe".into()),
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

    fn fixture_home() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("grok")
            .join("home")
    }

    #[test]
    fn test_session_details_full() {
        let a = GrokAdapter::with_home(fixture_home());
        let details = a.sessions_details(None).unwrap();
        assert_eq!(details.len(), 2);
        let full = details
            .iter()
            .find(|d| d.id == "aaaaaaaa-1111-4111-8111-111111111111")
            .unwrap();
        assert_eq!(full.model.as_deref(), Some("grok-4.7-build"));
        assert_eq!(full.project.as_deref(), Some("E:\\demo"));
        assert_eq!(full.status, "unknown");
        assert_eq!(
            full.confidence,
            crate::agents::traits::SessionConfidence::Confirmed
        );
        assert!(full.started_at.is_some());
        assert!(full.last_activity.unwrap() >= full.started_at.unwrap());
    }

    #[test]
    fn test_session_bare_has_unknowns() {
        let a = GrokAdapter::with_home(fixture_home());
        let details = a.sessions_details(None).unwrap();
        let bare = details
            .iter()
            .find(|d| d.id == "bbbbbbbb-2222-4222-8222-222222222222")
            .unwrap();
        assert_eq!(bare.model, None);
        assert!(bare.started_at.is_some());
        assert_eq!(bare.project.as_deref(), Some("E:\\demo"));
    }

    #[test]
    fn test_projects_resolve_decoded_paths() {
        let a = GrokAdapter::with_home(fixture_home());
        let projects = a.projects_details().unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].slug, "E%3A%5Cdemo");
        assert_eq!(projects[0].path.as_deref(), Some("E:\\demo"));
        assert!(projects[0].last_seen.is_some());
    }

    #[test]
    fn test_missing_sessions_dir_is_empty() {
        let a = GrokAdapter::with_home(temp_home("nosess"));
        assert!(a.sessions().unwrap().is_empty());
        assert!(a.sessions_details(None).unwrap().is_empty());
        assert!(a.projects_details().unwrap().is_empty());
    }

    #[test]
    fn test_mcp_empty_without_exe() {
        let a = GrokAdapter::with_home_and_exe(temp_home("nomcp"), "agenthq-no-such-bin-xyz");
        assert!(a.mcp_servers().unwrap().is_empty());
        assert!(a.mcp_details().unwrap().is_empty());
    }

    #[test]
    fn test_skills_details_global() {
        let a = GrokAdapter::with_home(fixture_home());
        let details = a.skills_details().unwrap();
        assert_eq!(details.len(), 2);
        let demo = details.iter().find(|d| d.name == "demo-skill").unwrap();
        assert_eq!(demo.scope, crate::agents::traits::SkillScope::Global);
        assert_eq!(demo.source.as_deref(), Some("grok-global"));
        assert!(demo.description.as_deref().unwrap().contains("fixture"));
        let plain = details.iter().find(|d| d.name == "plain").unwrap();
        assert_eq!(plain.description, None);
    }

    #[test]
    fn test_plugins_skip_lock_files() {
        let a = GrokAdapter::with_home(fixture_home());
        let details = a.plugins_details().unwrap();
        assert_eq!(details.len(), 1);
        assert_eq!(details[0].name, "plugin-a");
        assert_eq!(details[0].version, None);
        assert!(details[0].enabled);
        assert!(details.iter().all(|d| !d.name.ends_with(".lock")));
    }

    #[test]
    fn test_models_from_cache() {
        let a = GrokAdapter::with_home(fixture_home());
        let models = a.models().unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[0].name, "demo-model");
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
}
