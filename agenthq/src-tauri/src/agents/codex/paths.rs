use std::path::{Path, PathBuf};

use super::super::detect::find_on_path;

/// `~/.codex`
pub fn codex_dir(home: &Path) -> PathBuf {
    home.join(".codex")
}

/// `~/.codex/config.toml`
pub fn config_file(home: &Path) -> PathBuf {
    codex_dir(home).join("config.toml")
}

/// `~/.codex/session_index.jsonl`
pub fn session_index(home: &Path) -> PathBuf {
    codex_dir(home).join("session_index.jsonl")
}

/// Parse `0.160.0` from a dirname like `0.160.0-x86_64-pc-windows-msvc`.
fn release_version(dir_name: &str) -> Option<Vec<u64>> {
    let ver = dir_name.split('-').next()?;
    let parts: Option<Vec<u64>> = ver.split('.').map(|p| p.parse().ok()).collect();
    let parts = parts?;
    if parts.is_empty() {
        return None;
    }
    Some(parts)
}

/// Best `packages/*/releases/*/bin/codex.exe`, highest version wins.
pub fn releases_exe(home: &Path) -> Option<PathBuf> {
    let packages = codex_dir(home).join("packages");
    let pkgs = std::fs::read_dir(&packages).ok()?;
    let mut best: Option<(Vec<u64>, PathBuf)> = None;
    for pkg in pkgs.flatten() {
        let releases = pkg.path().join("releases");
        let entries = match std::fs::read_dir(&releases) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for rel in entries.flatten() {
            let name = rel.file_name().to_string_lossy().into_owned();
            let ver = match release_version(&name) {
                Some(v) => v,
                None => continue,
            };
            let exe = rel.path().join("bin").join("codex.exe");
            if !exe.is_file() {
                continue;
            }
            let replace = match &best {
                Some((bv, _)) => ver > *bv,
                None => true,
            };
            if replace {
                best = Some((ver, exe));
            }
        }
    }
    best.map(|(_, exe)| exe)
}

/// Known Codex executable locations: PATH first, then the releases glob.
pub fn known_exes(home: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![];
    if let Some(p) = find_on_path("codex") {
        out.push(PathBuf::from(p));
    }
    if let Some(exe) = releases_exe(home) {
        if !out.contains(&exe) {
            out.push(exe);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::releases_exe;

    fn fake_release(home: &std::path::Path, ver: &str) {
        let dir = home
            .join(".codex")
            .join("packages")
            .join("app-server-daemon")
            .join("releases")
            .join(format!("{ver}-x86_64-pc-windows-msvc"))
            .join("bin");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("codex.exe"), b"").unwrap();
    }

    #[test]
    fn test_releases_picks_max_version() {
        let mut home = std::env::temp_dir();
        home.push(format!("agenthq-codex-rel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        fake_release(&home, "0.159.0");
        fake_release(&home, "0.160.0");
        let picked = releases_exe(&home).unwrap();
        assert!(picked.to_string_lossy().contains("0.160.0"));
    }
}
