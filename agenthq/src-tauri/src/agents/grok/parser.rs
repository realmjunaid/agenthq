//! Parsing helpers. Extended in Tasks 2-3.

/// First whitespace token shaped like a version (`grok 1.0.46 (...)`).
/// The leading `grok` token itself is not a version.
pub fn parse_version(raw: &str) -> Option<String> {
    for token in raw.split_whitespace() {
        let t = token.trim();
        let mut chars = t.chars();
        match chars.next() {
            Some(c) if c.is_ascii_digit() => {}
            _ => continue,
        }
        if t.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Some(t.to_string());
        }
    }
    None
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn num(s: &str, lo: i64, hi: i64) -> Option<i64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let v: i64 = s.parse().ok()?;
    (lo <= v && v <= hi).then_some(v)
}

/// RFC3339 (`2026-10-05T10:40:36.640176600Z`, `...+00:00`) → epoch secs.
/// Sub-second precision is not preserved. Unparseable → None.
pub fn parse_rfc3339(s: &str) -> Option<i64> {
    let (date, rest) = s.split_once('T')?;
    let mut date = date.split('-');
    let y = num(date.next()?, 0, 9999)?;
    let mo = num(date.next()?, 1, 12)?;
    let d = num(date.next()?, 1, 31)?;
    if date.next().is_some() {
        return None;
    }
    // Split time from zone: trailing Z or ±hh:mm.
    let (time, offset_secs) = if let Some(t) = rest.strip_suffix(['Z', 'z']) {
        (t, 0)
    } else {
        let i = rest.rfind(['+', '-'])?;
        let (t, z) = rest.split_at(i);
        let sign: i64 = if z.starts_with('+') { 1 } else { -1 };
        let mut zp = z[1..].split(':');
        let zh = num(zp.next()?, 0, 23)?;
        let zm = num(zp.next()?, 0, 59)?;
        if zp.next().is_some() {
            return None;
        }
        (t, sign * (zh * 3600 + zm * 60))
    };
    let time = time.split('.').next()?;
    let mut tp = time.split(':');
    let hh = num(tp.next()?, 0, 23)?;
    let mm = num(tp.next()?, 0, 59)?;
    let ss = num(tp.next()?, 0, 60)?;
    if tp.next().is_some() {
        return None;
    }
    Some(days_from_civil(y, mo, d) * 86400 + hh * 3600 + mm * 60 + ss - offset_secs)
}

/// `%XX` percent-decoding for url-encoded project dirnames.
pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let (Some(h), Some(l)) = (
                hex_val(bytes.get(i + 1).copied()),
                hex_val(bytes.get(i + 2).copied()),
            ) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: Option<u8>) -> Option<u8> {
    let b = b?;
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

use serde_json::Value;

use crate::agents::traits::{McpServerDetail, McpTransport};

/// usage.json essentials: model + updated timestamp. Missing → Nones.
pub struct UsageInfo {
    pub model: Option<String>,
    pub updated_at: Option<i64>,
}

pub fn parse_usage(text: &str) -> UsageInfo {
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            return UsageInfo {
                model: None,
                updated_at: None,
            }
        }
    };
    let model = v
        .get("session")
        .and_then(|s| s.get("primaryModelId"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string());
    let updated_at = v
        .get("updatedAt")
        .and_then(|t| t.as_str())
        .and_then(parse_rfc3339);
    UsageInfo { model, updated_at }
}

/// Max prompt_history lines scanned per project dir (perf bound).
pub const PROMPT_SCAN_LINES: usize = 200;

/// session_id → earliest prompt timestamp. Malformed lines skipped.
pub fn prompt_starts(text: &str) -> std::collections::HashMap<String, i64> {
    let mut out = std::collections::HashMap::new();
    for line in text.lines().take(PROMPT_SCAN_LINES) {
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let (Some(id), Some(ts)) = (
            v.get("session_id").and_then(|i| i.as_str()),
            v.get("timestamp")
                .and_then(|t| t.as_str())
                .and_then(parse_rfc3339),
        ) else {
            continue;
        };
        out.entry(id.to_string())
            .and_modify(|m: &mut i64| *m = (*m).min(ts))
            .or_insert(ts);
    }
    out
}

/// `mcp list --json` array → details. Name from `name`, fallback `id`;
/// anything else skipped. `env` values never read — keys counted only.
/// Non-array / non-JSON input → empty.
pub fn parse_mcp_list(text: &str) -> Vec<McpServerDetail> {
    let arr: Vec<Value> = match serde_json::from_str(text) {
        Ok(a) => a,
        Err(_) => return vec![],
    };
    let mut out = vec![];
    for v in &arr {
        if let Some(s) = v.as_str() {
            out.push(McpServerDetail {
                name: s.to_string(),
                transport: McpTransport::Unknown,
                command: None,
                args: vec![],
                url: None,
                env_count: 0,
                project: None,
                enabled: None,
            });
            continue;
        }
        let obj = match v.as_object() {
            Some(o) => o,
            None => continue,
        };
        let name = match obj
            .get("name")
            .or_else(|| obj.get("id"))
            .and_then(|n| n.as_str())
        {
            Some(n) => n.to_string(),
            None => continue,
        };
        out.push(McpServerDetail {
            name,
            transport: McpTransport::Unknown,
            command: obj
                .get("command")
                .and_then(|c| c.as_str())
                .map(|s| s.to_string()),
            args: vec![],
            url: obj
                .get("url")
                .and_then(|u| u.as_str())
                .map(|s| s.to_string()),
            env_count: obj
                .get("env")
                .and_then(|e| e.as_object())
                .map(|o| o.len() as u32)
                .unwrap_or(0),
            project: None,
            enabled: None,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// models_cache.json `models` string array. Missing/invalid → empty.
pub fn parse_models_cache(text: &str) -> Vec<String> {
    let v: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    match v.get("models").and_then(|m| m.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|m| m.as_str().map(|s| s.to_string()))
            .collect(),
        None => vec![],
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_parse_version_second_token() {
        assert_eq!(
            super::parse_version("grok 1.0.46 (2765805b9442) [stable]"),
            Some("1.0.46".to_string())
        );
        assert_eq!(super::parse_version("grok"), None);
        assert_eq!(super::parse_version(""), None);
    }

    #[test]
    fn test_rfc3339_epoch_zero() {
        assert_eq!(super::parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
    }

    #[test]
    fn test_rfc3339_fraction_and_offset() {
        let z = super::parse_rfc3339("2026-10-05T10:40:36.640176600Z").unwrap();
        let off = super::parse_rfc3339("2026-10-05T10:40:36.640176600+00:00").unwrap();
        assert_eq!(z, off);
        let plus2 = super::parse_rfc3339("2026-10-05T12:40:36+02:00").unwrap();
        assert_eq!(z, plus2);
    }

    #[test]
    fn test_rfc3339_garbage_is_none() {
        assert_eq!(super::parse_rfc3339("not a time"), None);
        assert_eq!(super::parse_rfc3339("2026-13-99T99:99:99Z"), None);
        assert_eq!(super::parse_rfc3339(""), None);
    }

    #[test]
    fn test_percent_decode_project() {
        assert_eq!(
            super::percent_decode("E%3A%5CTempMail%20Project"),
            "E:\\TempMail Project"
        );
        assert_eq!(super::percent_decode("plain"), "plain");
        assert_eq!(super::percent_decode("bad%ZZ"), "bad%ZZ");
    }

    #[test]
    fn test_mcp_json_empty_array() {
        assert!(super::parse_mcp_list("[]").is_empty());
    }

    #[test]
    fn test_mcp_json_object_names() {
        let rows = super::parse_mcp_list(
            r#"[{"name": "gh", "command": "gh-mcp", "env": {"T": "CANARY-MCP-1"}}]"#,
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "gh");
        assert_eq!(rows[0].command.as_deref(), Some("gh-mcp"));
        assert_eq!(rows[0].env_count, 1);
        let dump = format!("{rows:?}");
        assert!(!dump.contains("CANARY-MCP-1"));
    }

    #[test]
    fn test_mcp_json_id_fallback_and_skip() {
        let rows = super::parse_mcp_list(r#"[{"id": "x"}, 42, "bare"]"#);
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().any(|r| r.name == "x"));
        assert!(rows.iter().any(|r| r.name == "bare"));
    }

    #[test]
    fn test_mcp_json_garbage_is_empty() {
        assert!(super::parse_mcp_list("No MCP servers configured.").is_empty());
        assert!(super::parse_mcp_list("").is_empty());
    }

    #[test]
    fn test_models_cache_names() {
        let names = super::parse_models_cache(r#"{"models": ["a", "b"]}"#);
        assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
        assert!(super::parse_models_cache("{}").is_empty());
        assert!(super::parse_models_cache("garbage").is_empty());
    }
}
