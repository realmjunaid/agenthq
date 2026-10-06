//! Models + connections inventory. Names only from adapters; the
//! provider of a `provider/name` model is the part before the first `/`.
//! Connection entries come from Claude settings env key names (values
//! never read); provider is always `env`.

use serde::Serialize;

use crate::agents::traits::{ConnectionInfo, ModelInfo};

#[derive(Debug, Clone, Serialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider: Option<String>,
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub configured: bool,
    pub agent_id: String,
}

/// Id `{agent}:{name}`; provider split on first `/`, else None.
pub fn normalize_models(agent_id: &str, infos: Vec<ModelInfo>) -> Vec<Model> {
    infos
        .into_iter()
        .map(|info| {
            let provider = info
                .name
                .split_once('/')
                .map(|(p, _)| p.to_string())
                .filter(|p| !p.is_empty());
            Model {
                id: format!("{agent_id}:{}", info.name),
                name: info.name,
                provider,
                agent_id: agent_id.to_string(),
            }
        })
        .collect()
}

/// Id `{agent}:{name}`; provider always `env`, configured true (evidence
/// is the key existing in settings).
pub fn normalize_connections(agent_id: &str, infos: Vec<ConnectionInfo>) -> Vec<Connection> {
    infos
        .into_iter()
        .map(|info| Connection {
            id: format!("{agent_id}:{}", info.name),
            name: info.name,
            provider: "env".to_string(),
            configured: true,
            agent_id: agent_id.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{normalize_connections, normalize_models};
    use crate::agents::traits::{ConnectionInfo, ModelInfo};

    fn model(name: &str) -> ModelInfo {
        ModelInfo {
            name: name.to_string(),
        }
    }

    #[test]
    fn test_provider_split() {
        let out = normalize_models("opencode", vec![model("opencode/big-pickle")]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "opencode:opencode/big-pickle");
        assert_eq!(out[0].provider.as_deref(), Some("opencode"));
        assert_eq!(out[0].name, "opencode/big-pickle");
    }

    #[test]
    fn test_bare_name_provider_none() {
        let out = normalize_models("claude", vec![model("auto")]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].provider, None);
    }

    #[test]
    fn test_connections_provider_env() {
        let out = normalize_connections(
            "claude",
            vec![ConnectionInfo {
                name: "FOO".to_string(),
            }],
        );
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "claude:FOO");
        assert_eq!(out[0].provider, "env");
        assert!(out[0].configured);
    }
}
