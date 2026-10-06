use std::path::{Path, PathBuf};

use super::super::detect::find_on_path;

/// `~/.config/opencode`
pub fn config_dir(home: &Path) -> PathBuf {
    home.join(".config").join("opencode")
}

/// Global `opencode.json`
pub fn global_config(home: &Path) -> PathBuf {
    config_dir(home).join("opencode.json")
}

/// Global skills dir
pub fn global_skills(home: &Path) -> PathBuf {
    config_dir(home).join("skills")
}

/// Known OpenCode executable locations: PATH first, then the npm global dir.
pub fn known_exes(_home: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![];
    if let Some(p) = find_on_path("opencode") {
        out.push(PathBuf::from(p));
    }
    if let Some(appdata) = std::env::var_os("APPDATA") {
        for name in ["opencode.cmd", "opencode.exe"] {
            let p = PathBuf::from(&appdata).join("npm").join(name);
            if p.is_file() && !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}
