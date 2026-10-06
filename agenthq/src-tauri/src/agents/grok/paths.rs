use std::path::{Path, PathBuf};

use super::super::detect::find_on_path;

/// `~/.grok`
pub fn grok_dir(home: &Path) -> PathBuf {
    home.join(".grok")
}

/// `~/.grok/config.toml` (safe sections only; auth/settings-cache never read)
///
/// Reserved for a future config-surface probe; today only dir existence
/// feeds installation detection.
#[allow(dead_code)]
pub fn config_file(home: &Path) -> PathBuf {
    grok_dir(home).join("config.toml")
}

/// `~/.grok/skills`
pub fn skills_dir(home: &Path) -> PathBuf {
    grok_dir(home).join("skills")
}

/// `~/.grok/installed-plugins`
pub fn plugins_dir(home: &Path) -> PathBuf {
    grok_dir(home).join("installed-plugins")
}

/// `~/.grok/models_cache.json`
pub fn models_cache(home: &Path) -> PathBuf {
    grok_dir(home).join("models_cache.json")
}

/// `~/.grok/sessions`
pub fn sessions_dir(home: &Path) -> PathBuf {
    grok_dir(home).join("sessions")
}

/// Known Grok executable locations: PATH first, then the user install dir.
pub fn known_exes(home: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = vec![];
    if let Some(p) = find_on_path("grok") {
        out.push(PathBuf::from(p));
    }
    for name in ["grok.exe", "grok.cmd"] {
        let p = home.join(".grok").join("bin").join(name);
        if p.is_file() && !out.contains(&p) {
            out.push(p);
        }
    }
    out
}
