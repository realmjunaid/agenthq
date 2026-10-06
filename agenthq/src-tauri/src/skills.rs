use serde::Serialize;

pub use crate::agents::traits::{SkillDetail, SkillScope};

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub path: Option<String>,
    pub scope: SkillScope,
    pub source: Option<String>,
    pub agent_id: String,
}

fn scope_name(s: SkillScope) -> &'static str {
    match s {
        SkillScope::Global => "global",
        SkillScope::Project => "project",
    }
}

/// Normalize adapter detail rows. Ids scope by `{agent}:{scope}:{name}`
/// so a future Project row never collides with a Global row.
pub fn normalize(agent_id: &str, details: Vec<SkillDetail>) -> Vec<Skill> {
    details
        .into_iter()
        .map(|d| {
            let id = format!("{agent_id}:{}:{}", scope_name(d.scope), d.name);
            Skill {
                id,
                name: d.name,
                description: d.description,
                path: d.path,
                scope: d.scope,
                source: d.source,
                agent_id: agent_id.to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{normalize, SkillScope};
    use crate::agents::traits::SkillDetail;

    fn detail(name: &str, scope: SkillScope) -> SkillDetail {
        SkillDetail {
            name: name.to_string(),
            description: None,
            path: None,
            scope,
            source: None,
        }
    }

    #[test]
    fn test_normalize_scoped_ids() {
        let out = normalize(
            "a",
            vec![
                detail("x", SkillScope::Global),
                detail("x", SkillScope::Project),
            ],
        );
        assert_eq!(out.len(), 2);
        assert_ne!(out[0].id, out[1].id);
        assert_eq!(out[0].id, "a:global:x");
        assert_eq!(out[1].id, "a:project:x");
    }
}
