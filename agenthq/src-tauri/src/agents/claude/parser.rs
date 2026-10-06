//! Version parsing. Extended in Tasks 2-3.

/// First whitespace token when it looks like a version (`2.1.289`).
pub fn parse_version(raw: &str) -> Option<String> {
    let first = raw.split_whitespace().next()?.trim();
    let mut chars = first.chars();
    match chars.next() {
        Some(c) if c.is_ascii_digit() => {}
        _ => return None,
    }
    if !first.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    Some(first.to_string())
}

use serde_json::Value;
use std::path::Path;

use crate::agents::traits::{SkillDetail, SkillScope};

/// Read a JSON file, None when missing/unparseable (fresh installs).
#[allow(dead_code)] // Read by sessions()/mcp/models (Phase 14 commands call them).
pub fn read_json_file(path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// First `description:` value from a skill's SKILL.md frontmatter, if any.
#[allow(dead_code)] // Read by skills() (Phase 14 commands call it).
pub fn skill_description(skill_dir: &Path) -> Option<String> {
    let content = std::fs::read_to_string(skill_dir.join("SKILL.md")).ok()?;
    for line in content.lines().take(20) {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("description:") {
            let v = rest.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

/// Max transcript lines scanned per session file (perf bound).
#[allow(dead_code)] // Read by sessions() (Phase 14 commands call it).
pub const SCAN_LINES: usize = 50;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)] // Read by sessions() (Phase 14 commands call it).
pub struct SessionLite {
    pub id: String,
    pub project: String,
    pub started_at: Option<i64>,
    pub last_activity: Option<i64>,
    pub model: Option<String>,
}

fn ts_as_i64(v: &Value) -> Option<i64> {
    v.get("timestamp")
        .and_then(|t| t.as_i64().or_else(|| t.as_f64().map(|f| f as i64)))
}

/// Parse one `<uuid>.jsonl` transcript (≤51 lines). Malformed lines are
/// skipped. `last_activity` prefers file mtime (uncapped, always current).
#[allow(dead_code)] // Called by sessions() (Phase 14 commands call it).
pub fn parse_session_file(path: &Path) -> Option<SessionLite> {
    let content = std::fs::read_to_string(path).ok()?;
    let id = path.file_stem()?.to_string_lossy().into_owned();
    let project = path.parent()?.file_name()?.to_string_lossy().into_owned();
    let mut started: Option<i64> = None;
    let mut max_ts: Option<i64> = None;
    let mut model: Option<String> = None;
    for line in content.lines().take(SCAN_LINES + 1) {
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(ts) = ts_as_i64(&v) {
            started = Some(started.map_or(ts, |s: i64| s.min(ts)));
            max_ts = Some(max_ts.map_or(ts, |m: i64| m.max(ts)));
        }
        if model.is_none() {
            if let Some(m) = v
                .get("message")
                .and_then(|m| m.get("model"))
                .and_then(|m| m.as_str())
            {
                model = Some(m.to_string());
            }
        }
    }
    let mtime = std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64);
    let last_activity = match (mtime, max_ts) {
        (Some(mt), Some(t)) => Some(mt.max(t)),
        (Some(mt), None) => Some(mt),
        (None, Some(t)) => Some(t),
        (None, None) => None,
    };
    Some(SessionLite {
        id,
        project,
        started_at: started,
        last_activity,
        model,
    })
}

use crate::agents::traits::{McpServerDetail, McpTransport};

pub const MAX_ARGS: usize = 32;
pub const MAX_ARG_CHARS: usize = 256;

fn capped_args(v: &Value) -> Vec<String> {
    match v.get("args").and_then(|a| a.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|a| {
                a.as_str().map(|s| {
                    if s.chars().count() > MAX_ARG_CHARS {
                        s.chars().take(MAX_ARG_CHARS).collect()
                    } else {
                        s.to_string()
                    }
                })
            })
            .take(MAX_ARGS)
            .collect(),
        None => vec![],
    }
}

fn env_count(v: &Value) -> u32 {
    v.get("env")
        .and_then(|e| e.as_object())
        .map(|o| o.len() as u32)
        .unwrap_or(0)
}

/// Map one MCP server entry. Non-objects and entries with neither
/// url nor command are skipped (None). `env` values are never read.
pub fn map_mcp_entry(
    name: &str,
    v: &Value,
    enabled: Option<bool>,
    project: Option<String>,
) -> Option<McpServerDetail> {
    let obj = v.as_object()?;
    let url = obj
        .get("url")
        .and_then(|u| u.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let command = obj
        .get("command")
        .and_then(|c| c.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let transport = if url.is_some() {
        McpTransport::Remote
    } else if command.is_some() {
        McpTransport::Stdio
    } else {
        McpTransport::Unknown
    };
    if url.is_none() && command.is_none() {
        return None;
    }
    Some(McpServerDetail {
        name: name.to_string(),
        transport,
        command,
        args: capped_args(v),
        url,
        env_count: env_count(v),
        project,
        enabled,
    })
}

/// All `mcpServers` across `.claude.json` projects with enabled mapping.
pub fn parse_mcp_servers(doc: &Value) -> Vec<McpServerDetail> {
    let projects = match doc.get("projects").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return vec![],
    };
    let mut out = vec![];
    for (proj, cfg) in projects {
        let servers = match cfg.get("mcpServers").and_then(|s| s.as_object()) {
            Some(s) => s,
            None => continue,
        };
        let enabled: Vec<String> = cfg
            .get("enabledMcpjsonServers")
            .and_then(|e| e.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let disabled: Vec<String> = cfg
            .get("disabledMcpjsonServers")
            .and_then(|e| e.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        for (name, v) in servers {
            let en = if disabled.iter().any(|d| d == name) {
                Some(false)
            } else if enabled.iter().any(|e| e == name) {
                Some(true)
            } else {
                None
            };
            if let Some(d) = map_mcp_entry(name, v, en, Some(proj.clone())) {
                out.push(d);
            }
        }
    }
    out
}

/// Enumerate skill dirs: subdirectories (dotfiles skipped) with optional
/// SKILL.md descriptions. A missing/unreadable SKILL.md yields None, never Err.
pub fn enumerate_skill_dirs(dir: &Path, scope: SkillScope, source: &str) -> Vec<SkillDetail> {
    let mut out = vec![];
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return out,
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
        out.push(SkillDetail {
            name,
            description: skill_description(&p),
            path: Some(p.to_string_lossy().into_owned()),
            scope,
            source: Some(source.to_string()),
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Reverse of Claude's project slug: every non-alphanumeric char became `-`.
/// Verified: `C:/Users/j4u87` → `C--Users-j4u87`.
pub fn slugify(path: &str) -> String {
    path.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Map project slug → real path from `.claude.json` project keys.
/// First sorted key wins on collision.
pub fn slug_path_map(doc: &serde_json::Value) -> std::collections::HashMap<String, String> {
    let mut keys: Vec<String> = doc
        .get("projects")
        .and_then(|p| p.as_object())
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    keys.sort();
    let mut map = std::collections::HashMap::new();
    for k in keys {
        map.entry(slugify(&k)).or_insert(k);
    }
    map
}

#[cfg(test)]
mod session_tests {
    use super::{map_mcp_entry, parse_session_file, SessionLite};
    use serde_json::json;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("claude")
            .join(name)
    }

    #[test]
    fn test_session_fixture_parses() {
        let lite = parse_session_file(&fixture("session.jsonl")).unwrap();
        assert_eq!(lite.id, "session");
        assert_eq!(lite.project, "claude");
        assert_eq!(lite.started_at, Some(1750000000));
        assert!(lite.last_activity.unwrap() >= 1750000060);
        assert_eq!(lite.model.as_deref(), Some("sonnet"));
    }

    #[test]
    fn test_bad_line_skipped() {
        // Fixture contains a non-JSON line; parse must still succeed.
        assert!(parse_session_file(&fixture("session.jsonl")).is_some());
    }

    #[test]
    fn test_scan_caps_lines() {
        // opus-at-line-70 sits past the 50-line scan cap → must not win.
        let lite = parse_session_file(&fixture("session.jsonl")).unwrap();
        assert_eq!(lite.model.as_deref(), Some("sonnet"));
    }

    #[test]
    fn test_missing_file_is_none() {
        assert_eq!(
            parse_session_file(&PathBuf::from("no-such-file.jsonl")),
            None
        );
        let _ = SessionLite {
            id: String::new(),
            project: String::new(),
            started_at: None,
            last_activity: None,
            model: None,
        };
    }

    fn mcp_doc() -> serde_json::Value {
        let text = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures")
                .join("claude")
                .join("mcp.json"),
        )
        .unwrap();
        serde_json::from_str(&text).unwrap()
    }

    #[test]
    fn test_claude_entry_full() {
        let doc = mcp_doc();
        let servers = super::parse_mcp_servers(&doc);
        let gh = servers.iter().find(|s| s.name == "gh").unwrap();
        assert_eq!(gh.transport, crate::agents::traits::McpTransport::Stdio);
        assert_eq!(gh.command.as_deref(), Some("gh-mcp"));
        assert_eq!(gh.env_count, 1);
        assert_eq!(gh.enabled, Some(true));
        assert_eq!(gh.project.as_deref(), Some("E:/d"));
    }

    #[test]
    fn test_claude_remote_and_disabled() {
        let doc = mcp_doc();
        let servers = super::parse_mcp_servers(&doc);
        let r = servers.iter().find(|s| s.name == "remote").unwrap();
        assert_eq!(r.transport, crate::agents::traits::McpTransport::Remote);
        assert_eq!(r.url.as_deref(), Some("https://mcp.example/sse"));
        assert_eq!(r.enabled, Some(false));
    }

    #[test]
    fn test_bad_entry_skipped() {
        let doc = mcp_doc();
        let servers = super::parse_mcp_servers(&doc);
        assert_eq!(servers.len(), 2);
        assert!(servers.iter().all(|s| s.name != "bad"));
    }

    #[test]
    fn test_args_capped() {
        let v =
            json!({"command": "x", "args": (0..40).map(|i| format!("a{i}")).collect::<Vec<_>>()});
        let d = map_mcp_entry("t", &v, None, None).unwrap();
        assert_eq!(d.args.len(), 32);
        let long = "y".repeat(300);
        let v2 = json!({"command": "x", "args": [long]});
        let d2 = map_mcp_entry("t2", &v2, None, None).unwrap();
        assert_eq!(d2.args[0].chars().count(), 256);
    }

    #[test]
    fn test_claude_env_values_hidden() {
        let doc = mcp_doc();
        let servers = super::parse_mcp_servers(&doc);
        let dump = format!("{servers:?}");
        assert!(!dump.contains("CANARY-1"));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_version_strips_suffix() {
        assert_eq!(
            super::parse_version("2.1.289 (Claude Code)"),
            Some("2.1.289".to_string())
        );
        assert_eq!(super::parse_version("2.1.289"), Some("2.1.289".to_string()));
        assert_eq!(super::parse_version(""), None);
    }
}

#[cfg(test)]
mod skill_enum_tests {
    use super::enumerate_skill_dirs;
    use crate::agents::traits::SkillScope;

    fn setup_dirs(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("agenthq-skills-{}_{}", std::process::id(), tag));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::write(
            dir.join("a").join("SKILL.md"),
            "---\ndescription: Alpha skill.\n---\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("b")).unwrap();
        std::fs::create_dir_all(dir.join(".hidden")).unwrap();
        std::fs::write(dir.join("file.txt"), b"x").unwrap();
        dir
    }

    #[test]
    fn test_enumerate_global_skills() {
        let dir = setup_dirs("enum");
        let rows = enumerate_skill_dirs(&dir, SkillScope::Global, "test-global");
        assert_eq!(rows.len(), 2);
        let a = rows.iter().find(|r| r.name == "a").unwrap();
        assert_eq!(a.description.as_deref(), Some("Alpha skill."));
        assert!(a.path.as_deref().unwrap().ends_with('a'));
        assert_eq!(a.scope, SkillScope::Global);
        let b = rows.iter().find(|r| r.name == "b").unwrap();
        assert_eq!(b.description, None);
    }

    #[test]
    fn test_non_utf8_skill_md_is_none() {
        let dir = setup_dirs("utf8");
        std::fs::create_dir_all(dir.join("c")).unwrap();
        std::fs::write(dir.join("c").join("SKILL.md"), [0xff, 0xfe, 0x00]).unwrap();
        let rows = enumerate_skill_dirs(&dir, SkillScope::Global, "t");
        let c = rows.iter().find(|r| r.name == "c").unwrap();
        assert!(rows.iter().any(|r| r.name == "a"));
        assert_eq!(c.description, None);
    }

    #[test]
    fn test_scope_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&SkillScope::Global).unwrap(),
            "\"global\""
        );
        assert_eq!(
            serde_json::to_string(&SkillScope::Project).unwrap(),
            "\"project\""
        );
    }
}
