//! Parsing helpers. Implemented in Step 3.

/// Last whitespace token stripped of a leading `v` (`opencode v2.0.23`).
pub fn parse_version(raw: &str) -> Option<String> {
    let last = raw
        .split_whitespace()
        .last()?
        .trim()
        .trim_start_matches('v');
    if last.is_empty() || !last.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some(last.to_string())
}

use serde_json::Value;

/// One row of `opencode session list --format json`. All fields optional
/// except id — a missing key never fails the whole list.
#[derive(Debug, Clone, PartialEq)]
pub struct CliSession {
    pub id: String,
    pub title: Option<String>,
    pub created_ms: Option<i64>,
    pub updated_ms: Option<i64>,
    pub project: Option<String>,
}

fn i64_field(v: &Value, key: &str) -> Option<i64> {
    v.get(key)
        .and_then(|t| t.as_i64().or_else(|| t.as_f64().map(|f| f as i64)))
}

pub fn parse_session_list(json: &str) -> Vec<CliSession> {
    let arr: Vec<Value> = match serde_json::from_str(json) {
        Ok(a) => a,
        Err(_) => return vec![],
    };
    arr.iter()
        .filter_map(|v| {
            Some(CliSession {
                id: v.get("id")?.as_str()?.to_string(),
                title: v
                    .get("title")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string()),
                created_ms: i64_field(v, "created"),
                updated_ms: i64_field(v, "updated"),
                project: v
                    .get("directory")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string()),
            })
        })
        .take(100)
        .collect()
}

/// First whitespace token of each non-empty, non-decoration line.
/// Skips the `ID …` header, `---` separators, and blank lines.
pub fn parse_plugin_list(text: &str) -> Vec<String> {
    parse_plugin_rows(text)
        .into_iter()
        .map(|r| r.name)
        .collect()
}

/// One row of `opencode plugin list`: name + optional version.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginRow {
    pub name: String,
    pub version: Option<String>,
}

pub fn parse_plugin_rows(text: &str) -> Vec<PluginRow> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| {
            !l.is_empty()
                && !l.starts_with("---")
                && !l.to_uppercase().starts_with("ID ")
                && !l.eq_ignore_ascii_case("id")
        })
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            Some(PluginRow {
                name: parts.next()?.to_string(),
                version: parts.next().map(|s| s.to_string()),
            })
        })
        .collect()
}

/// Non-empty trimmed lines (one `provider/model` per line).
pub fn parse_model_list(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// CLI millisecond epochs → seconds (sub-second precision not preserved).
pub fn ms_to_s(ms: Option<i64>) -> Option<i64> {
    ms.map(|m| m / 1000)
}

use crate::agents::claude::parser::map_mcp_entry;
use crate::agents::traits::McpServerDetail;

/// The global config `mcp` object mapped to details (no enabled info).
pub fn parse_mcp_object(doc: &serde_json::Value) -> Vec<McpServerDetail> {
    let mut out = vec![];
    if let Some(servers) = doc.get("mcp").and_then(|m| m.as_object()) {
        for (name, v) in servers {
            if let Some(d) = map_mcp_entry(name, v, None, None) {
                out.push(d);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_version_strips_v() {
        assert_eq!(
            super::parse_version("opencode v2.0.23"),
            Some("2.0.23".to_string())
        );
        assert_eq!(super::parse_version(""), None);
    }

    #[test]
    fn test_session_fixture_parses() {
        let sessions = super::parse_session_list(&fixture_json("sessions.json"));
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].id, "ses_demo0001");
        assert_eq!(sessions[0].title.as_deref(), Some("Demo session"));
        assert_eq!(sessions[0].project.as_deref(), Some("E:/demo"));
    }

    #[test]
    fn test_session_missing_keys_tolerated() {
        let sessions = super::parse_session_list(r#"[{"id": "only-id"}]"#);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, None);
    }

    #[test]
    fn test_opencode_remote_entry() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{"mcp": {"remote": {"url": "https://x.example/sse"}, "bad": 1}}"#,
        )
        .unwrap();
        let details = super::parse_mcp_object(&v);
        let r = details.iter().find(|d| d.name == "remote").unwrap();
        assert_eq!(r.transport, crate::agents::traits::McpTransport::Remote);
        assert_eq!(r.url.as_deref(), Some("https://x.example/sse"));
        assert!(details.iter().all(|d| d.name != "bad"));
    }

    #[test]
    fn test_opencode_bad_entry_skipped() {
        let v: serde_json::Value =
            serde_json::from_str(r#"{"mcp": {"ok": {"command": "c"}}}"#).unwrap();
        let details = super::parse_mcp_object(&v);
        assert_eq!(details.len(), 1);
    }

    #[test]
    fn test_plugin_lines_parsed() {
        let text =
            "ID           VERSION  SOURCE\nsuperpowers  8ca22db  x\n\n   \nsecond  1.0  y\n---\n";
        let ids = super::parse_plugin_list(text);
        assert_eq!(ids, vec!["superpowers".to_string(), "second".to_string()]);
    }

    #[test]
    fn test_models_skip_blanks() {
        let text = "\n  \nopencode/big-pickle\n\nopencode/fledge\n";
        let names = super::parse_model_list(text);
        assert_eq!(
            names,
            vec![
                "opencode/big-pickle".to_string(),
                "opencode/fledge".to_string()
            ]
        );
    }

    #[test]
    fn test_plugin_rows_with_versions() {
        let text = "ID           VERSION  SOURCE\nsuperpowers  8ca22db  x\nsecond  1.0  y\n";
        let rows = super::parse_plugin_rows(text);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "superpowers");
        assert_eq!(rows[0].version.as_deref(), Some("8ca22db"));
        assert_eq!(rows[1].name, "second");
        assert_eq!(rows[1].version.as_deref(), Some("1.0"));
    }

    #[test]
    fn test_short_row_version_none() {
        let rows = super::parse_plugin_rows("lonely\n");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "lonely");
        assert_eq!(rows[0].version, None);
    }

    #[test]
    fn test_plugin_list_still_names_only() {
        let text = "ID  VERSION\nsecond  1.0\n---\n";
        assert_eq!(super::parse_plugin_list(text), vec!["second".to_string()]);
    }

    fn fixture_json(name: &str) -> String {
        std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join("opencode")
                .join(name),
        )
        .unwrap()
    }
}
