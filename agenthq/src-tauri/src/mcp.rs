use serde::Serialize;

use crate::agents::traits::{McpServerDetail, McpTransport};

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum McpStatus {
    Connected,
    Configured,
    Offline,
    Error,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub transport: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env_count: u32,
    pub source: String,
    pub status: McpStatus,
    pub agent_id: String,
    pub project_id: Option<String>,
}

fn transport_name(t: McpTransport) -> &'static str {
    match t {
        McpTransport::Stdio => "stdio",
        McpTransport::Remote => "remote",
        McpTransport::Unknown => "unknown",
    }
}

/// Normalize adapter detail rows. `default_status` applies when the detail
/// carries no enabled signal (Claude/OpenCode → Configured, Codex → Unknown);
/// explicit disabled always wins as Offline. Connected is never produced —
/// no liveness probe exists. Ids scope by project when present so the same
/// server name in two projects stays two rows.
pub fn normalize(
    agent_id: &str,
    source: &str,
    default_status: McpStatus,
    details: Vec<McpServerDetail>,
) -> Vec<McpServer> {
    details
        .into_iter()
        .map(|d| {
            let status = match d.enabled {
                Some(false) => McpStatus::Offline,
                Some(true) => McpStatus::Configured,
                None => default_status,
            };
            let id = match &d.project {
                Some(p) => format!("{agent_id}:{p}:{}", d.name),
                None => format!("{agent_id}:{}", d.name),
            };
            McpServer {
                id,
                name: d.name,
                transport: transport_name(d.transport).to_string(),
                command: d.command,
                args: d.args,
                url: d.url,
                env_count: d.env_count,
                source: source.to_string(),
                status,
                agent_id: agent_id.to_string(),
                project_id: d.project,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{normalize, McpStatus};
    use crate::agents::traits::{McpServerDetail, McpTransport};

    fn detail(name: &str, enabled: Option<bool>) -> McpServerDetail {
        McpServerDetail {
            name: name.to_string(),
            transport: McpTransport::Stdio,
            command: Some("c".to_string()),
            args: vec![],
            url: None,
            env_count: 0,
            project: None,
            enabled,
        }
    }

    #[test]
    fn test_normalize_status_mapping() {
        let out = normalize(
            "a",
            "test",
            McpStatus::Configured,
            vec![
                detail("on", Some(true)),
                detail("off", Some(false)),
                detail("na", None),
            ],
        );
        assert_eq!(out[0].status, McpStatus::Configured);
        assert_eq!(out[1].status, McpStatus::Offline);
        assert_eq!(out[2].status, McpStatus::Configured);
        assert_eq!(out[0].id, "a:on");
        let unk = normalize("a", "test", McpStatus::Unknown, vec![detail("x", None)]);
        assert_eq!(unk[0].status, McpStatus::Unknown);
    }

    #[test]
    fn test_same_name_two_projects_stays_two_rows() {
        let mk = |project: Option<&str>| McpServerDetail {
            name: "gh".to_string(),
            transport: McpTransport::Stdio,
            command: Some("c".to_string()),
            args: vec![],
            url: None,
            env_count: 0,
            project: project.map(|s| s.to_string()),
            enabled: None,
        };
        let out = normalize(
            "claude",
            "test",
            McpStatus::Configured,
            vec![mk(Some("E:/d")), mk(Some("E:/e"))],
        );
        assert_eq!(out.len(), 2);
        assert_ne!(out[0].id, out[1].id);
    }

    #[test]
    fn test_no_connected_without_liveness() {
        let shapes = vec![
            normalize(
                "c",
                "claude",
                McpStatus::Configured,
                vec![detail("s", Some(true))],
            ),
            normalize(
                "o",
                "opencode",
                McpStatus::Configured,
                vec![detail("s", None)],
            ),
            normalize("x", "codex", McpStatus::Unknown, vec![detail("s", None)]),
        ];
        for servers in shapes {
            for s in servers {
                assert_ne!(
                    s.status,
                    McpStatus::Connected,
                    "engine must never claim Connected"
                );
            }
        }
    }
}
