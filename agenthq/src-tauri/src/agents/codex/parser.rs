//! Parsing helpers. Implemented in Step 3.

/// Last whitespace token (`codex-cli 0.160.0`).
pub fn parse_version(raw: &str) -> Option<String> {
    let last = raw.split_whitespace().last()?.trim();
    if last.is_empty() || !last.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some(last.to_string())
}

use serde_json::Value;

/// One row of `session_index.jsonl`.
#[derive(Debug, Clone, PartialEq)]
pub struct IndexSession {
    pub id: String,
}

/// Parse `session_index.jsonl` text (one JSON object per line).
/// Malformed lines and rows without a string `id` are skipped.
pub fn parse_session_index(text: &str) -> Vec<IndexSession> {
    text.lines()
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).ok()?;
            Some(IndexSession {
                id: v.get("id")?.as_str()?.to_string(),
            })
        })
        .collect()
}

/// `[mcp_servers.<name>]` section headers from config.toml text.
/// Values (incl. `env`) are never parsed — names only.
pub fn mcp_server_names(toml_text: &str) -> Vec<String> {
    let mut out = vec![];
    for line in toml_text.lines() {
        let t = line.trim();
        let rest = match t.strip_prefix("[mcp_servers.") {
            Some(r) => r,
            None => continue,
        };
        let inner = match rest.strip_suffix(']') {
            Some(i) => i.trim(),
            None => continue,
        };
        if inner.is_empty() {
            continue;
        }
        let name = inner.strip_prefix('"').and_then(|s| s.strip_suffix('"'));
        out.push(name.unwrap_or(inner).to_string());
    }
    out.sort();
    out.dedup();
    out
}

/// `models[].slug` strings from models_cache.json text. Anything else → empty.
pub fn model_slugs(json: &str) -> Vec<String> {
    let v: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    match v.get("models").and_then(|m| m.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|m| m.get("slug")?.as_str().map(|s| s.to_string()))
            .collect(),
        None => vec![],
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_version_last_token() {
        assert_eq!(
            super::parse_version("codex-cli 0.160.0"),
            Some("0.160.0".to_string())
        );
        assert_eq!(super::parse_version(""), None);
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
    fn test_index_fixture_parses_two() {
        let sessions = super::parse_session_index(&fixture_text("session_index.jsonl"));
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "01a10be9-a7a0-7c03-814e-ff8b0ce85797");
    }

    #[test]
    fn test_bad_line_skipped() {
        let sessions = super::parse_session_index("not json\n{\"id\": \"abc\"}\n");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "abc");
    }
}
