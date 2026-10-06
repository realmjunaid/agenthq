use serde::Serialize;

use crate::agents::traits::PluginDetail;

#[derive(Debug, Clone, Serialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub path: Option<String>,
    pub source: Option<String>,
    pub enabled: bool,
    pub agent_id: String,
}

/// Normalize adapter detail rows. Ids are `{agent}:{name}`; same-name
/// collisions across marketplace kinds are deduped first-wins in the
/// Claude adapter, so ids stay unique per agent here.
pub fn normalize(agent_id: &str, details: Vec<PluginDetail>) -> Vec<Plugin> {
    details
        .into_iter()
        .map(|d| Plugin {
            id: format!("{agent_id}:{}", d.name),
            name: d.name,
            version: d.version,
            path: d.path,
            source: d.source,
            enabled: d.enabled,
            agent_id: agent_id.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalize;
    use crate::agents::traits::PluginDetail;

    fn detail(name: &str) -> PluginDetail {
        PluginDetail {
            name: name.to_string(),
            version: Some("1.0".to_string()),
            path: Some("p".to_string()),
            source: Some("s".to_string()),
            enabled: true,
        }
    }

    #[test]
    fn test_normalize_ids() {
        let out = normalize("a", vec![detail("x")]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "a:x");
        assert!(out[0].enabled);
    }
}
