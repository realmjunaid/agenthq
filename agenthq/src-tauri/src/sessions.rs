use serde::Serialize;
use std::collections::HashMap;

pub use crate::agents::traits::{ProjectDetail, SessionConfidence, SessionDetail};

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub id: String,
    pub agent_id: String,
    pub project_id: Option<String>,
    pub status: String,
    pub model: Option<String>,
    pub started_at: Option<i64>,
    pub last_activity: Option<i64>,
    pub confidence: SessionConfidence,
}

#[derive(Debug, Clone, Serialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub last_seen: Option<i64>,
    pub agent_ids: Vec<String>,
}

fn basename_of(path: &str) -> String {
    path.replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

/// Prefix native ids with the agent (`claude:<uuid>`); link `project`
/// (a real path from Claude, a directory from OpenCode) via `known`
/// path→id map, else None. Confidence/status pass through untouched.
pub fn normalize_sessions(
    agent_id: &str,
    known: &HashMap<String, String>,
    details: Vec<SessionDetail>,
) -> Vec<Session> {
    details
        .into_iter()
        .map(|d| {
            let project_id = d.project.as_deref().and_then(|p| known.get(p).cloned());
            Session {
                id: format!("{agent_id}:{}", d.id),
                agent_id: agent_id.to_string(),
                project_id,
                status: d.status,
                model: d.model,
                started_at: d.started_at,
                last_activity: d.last_activity,
                confidence: d.confidence,
            }
        })
        .collect()
}

/// Project rows: id `{agent}:{slug}`, name = path basename.
pub fn normalize_projects(agent_id: &str, details: Vec<ProjectDetail>) -> Vec<Project> {
    details
        .into_iter()
        .filter(|d| d.path.is_some())
        .map(|d| {
            let path = d.path.unwrap_or_default();
            Project {
                id: format!("{agent_id}:{}", d.slug),
                name: basename_of(&path),
                path,
                last_seen: d.last_seen,
                agent_ids: vec![],
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{normalize_projects, normalize_sessions};
    use crate::agents::traits::{ProjectDetail, SessionConfidence, SessionDetail};
    use std::collections::HashMap;

    fn detail(id: &str, project: Option<&str>) -> SessionDetail {
        SessionDetail {
            id: id.to_string(),
            project: project.map(|s| s.to_string()),
            status: "unknown".to_string(),
            model: None,
            started_at: None,
            last_activity: None,
            confidence: SessionConfidence::Detected,
        }
    }

    #[test]
    fn test_session_ids_scoped() {
        let a = normalize_sessions("claude", &HashMap::new(), vec![detail("abc", None)]);
        let b = normalize_sessions("opencode", &HashMap::new(), vec![detail("abc", None)]);
        assert_ne!(a[0].id, b[0].id);
        assert_eq!(a[0].id, "claude:abc");
    }

    #[test]
    fn test_project_link_or_none() {
        let mut known = HashMap::new();
        known.insert("E:/demo".to_string(), "claude:E--demo".to_string());
        let out = normalize_sessions(
            "claude",
            &known,
            vec![
                detail("s1", Some("E:/demo")),
                detail("s2", Some("E:/ghost")),
            ],
        );
        assert_eq!(out[0].project_id.as_deref(), Some("claude:E--demo"));
        assert_eq!(out[1].project_id, None);
    }

    #[test]
    fn test_normalize_projects_basename_and_skip() {
        let rows = normalize_projects(
            "a",
            vec![
                ProjectDetail {
                    slug: "s1".to_string(),
                    path: Some("E:/demo".to_string()),
                    last_seen: None,
                },
                ProjectDetail {
                    slug: "s2".to_string(),
                    path: None,
                    last_seen: None,
                },
            ],
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "a:s1");
        assert_eq!(rows[0].name, "demo");
    }
}
