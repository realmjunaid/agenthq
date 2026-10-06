use serde::Serialize;

use crate::monitoring::process::ProcessInfo;

#[derive(Debug, Clone)]
#[allow(dead_code)] // Phases 6-8 construct these; engines enrich them later.
pub struct Unsupported(pub &'static str);

#[allow(dead_code)] // Used in every adapter signature.
pub type CapabilityResult<T> = Result<T, Unsupported>;

#[derive(Debug, Clone)]
#[allow(dead_code)] // Used in trait signatures; constructed by adapters.
pub struct AgentCapabilities {
    pub processes: bool,
    pub sessions: bool,
    pub subagents: bool,
    pub mcp: bool,
    pub skills: bool,
    pub plugins: bool,
    pub models: bool,
    pub connections: bool,
    pub logs: bool,
    pub lifecycle_control: bool,
}

impl AgentCapabilities {
    #[allow(dead_code)] // Used by test fakes + future adapters.
    pub fn none() -> Self {
        AgentCapabilities {
            processes: false,
            sessions: false,
            subagents: false,
            mcp: false,
            skills: false,
            plugins: false,
            models: false,
            connections: false,
            logs: false,
            lifecycle_control: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub installed: bool,
    pub executable_path: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Read by process grouping UI in later phases.
pub struct AgentProcess {
    pub pid: u32,
    pub name: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Returned by sessions() in Phase 6+ adapters.
pub struct AgentSession {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Enriched by the sessions engine (Phase 12).
pub struct SubagentInfo {
    pub id: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Enriched by the MCP engine (Phase 9).
pub struct McpServerInfo {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum McpTransport {
    Stdio,
    Remote,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct McpServerDetail {
    pub name: String,
    pub transport: McpTransport,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env_count: u32,
    pub project: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Enriched by the skills engine (Phase 10).
pub struct SkillInfo {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillScope {
    Global,
    Project,
}

#[derive(Debug, Clone)]
pub struct SkillDetail {
    pub name: String,
    pub description: Option<String>,
    pub path: Option<String>,
    pub scope: SkillScope,
    pub source: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Enriched by the plugins engine (Phase 11).
pub struct PluginInfo {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct PluginDetail {
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Enriched by later phases.
pub struct ModelInfo {
    pub name: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)] // Enriched by later phases.
pub struct ConnectionInfo {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionConfidence {
    Confirmed,
    Detected,
    Estimated,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SessionDetail {
    pub id: String,
    pub project: Option<String>,
    pub status: String,
    pub model: Option<String>,
    pub started_at: Option<i64>,
    pub last_activity: Option<i64>,
    pub confidence: SessionConfidence,
}

#[derive(Debug, Clone)]
pub struct ProjectDetail {
    pub slug: String,
    pub path: Option<String>,
    pub last_seen: Option<i64>,
}

#[allow(dead_code)] // Capability methods are called by Phase 14 detail commands.
pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn executable_names(&self) -> &[&str] {
        &[]
    }
    fn detect_installation(&self) -> DetectionResult;
    fn capabilities(&self) -> AgentCapabilities;
    fn processes(&self, _snapshot: &[ProcessInfo]) -> Vec<AgentProcess> {
        vec![]
    }
    fn sessions(&self) -> CapabilityResult<Vec<AgentSession>> {
        Err(Unsupported("sessions"))
    }
    fn sessions_details(
        &self,
        _project_dir: Option<&std::path::Path>,
    ) -> CapabilityResult<Vec<SessionDetail>> {
        Err(Unsupported("sessions"))
    }
    fn projects_details(&self) -> CapabilityResult<Vec<ProjectDetail>> {
        Err(Unsupported("projects"))
    }
    fn subagents(&self) -> CapabilityResult<Vec<SubagentInfo>> {
        Err(Unsupported("subagents"))
    }
    fn mcp_servers(&self) -> CapabilityResult<Vec<McpServerInfo>> {
        Err(Unsupported("mcp"))
    }
    fn mcp_details(&self) -> CapabilityResult<Vec<McpServerDetail>> {
        Err(Unsupported("mcp"))
    }
    fn skills(&self) -> CapabilityResult<Vec<SkillInfo>> {
        Err(Unsupported("skills"))
    }
    fn skills_details(&self) -> CapabilityResult<Vec<SkillDetail>> {
        Err(Unsupported("skills"))
    }
    fn plugins(&self) -> CapabilityResult<Vec<PluginInfo>> {
        Err(Unsupported("plugins"))
    }
    fn plugins_details(&self) -> CapabilityResult<Vec<PluginDetail>> {
        Err(Unsupported("plugins"))
    }
    fn models(&self) -> CapabilityResult<Vec<ModelInfo>> {
        Err(Unsupported("models"))
    }
    fn connections(&self) -> CapabilityResult<Vec<ConnectionInfo>> {
        Err(Unsupported("connections"))
    }
}

#[cfg(test)]
mod tests {
    use super::AgentAdapter;
    struct Fake;
    impl super::AgentAdapter for Fake {
        fn id(&self) -> &'static str {
            "fake"
        }
        fn name(&self) -> &'static str {
            "Fake"
        }
        fn detect_installation(&self) -> super::DetectionResult {
            super::DetectionResult {
                installed: false,
                executable_path: None,
                version: None,
            }
        }
        fn capabilities(&self) -> super::AgentCapabilities {
            super::AgentCapabilities::none()
        }
    }

    #[test]
    fn test_unsupported_defaults_err() {
        let f = Fake;
        assert!(f.sessions().is_err());
        assert!(f.subagents().is_err());
        assert!(f.mcp_servers().is_err());
        assert!(f.skills().is_err());
        assert!(f.plugins().is_err());
        assert!(f.models().is_err());
        assert!(f.connections().is_err());
        assert!(f.processes(&[]).is_empty());
    }
}
