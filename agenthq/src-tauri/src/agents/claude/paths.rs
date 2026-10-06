use std::path::{Path, PathBuf};

use super::super::detect::find_on_path;

/// `~/.claude`
pub fn home_claude(home: &Path) -> PathBuf {
    home.join(".claude")
}

/// `~/.claude.json` (per-project MCP + state). Consumed in Task 3.
#[allow(dead_code)] // Task 3.
pub fn claude_json(home: &Path) -> PathBuf {
    home.join(".claude.json")
}

/// Known Claude executable locations: PATH first, then the user install dir.
pub fn known_exes(home: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![];
    if let Some(p) = find_on_path("claude") {
        out.push(PathBuf::from(p));
    }
    for name in ["claude.exe", "claude.cmd"] {
        let p = home.join(".local").join("bin").join(name);
        if p.is_file() && !out.contains(&p) {
            out.push(p);
        }
    }
    out
}
