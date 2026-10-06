use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Find `name` on PATH. On Windows, PATHEXT extensions are tried when
/// `name` carries no extension.
pub fn find_on_path(name: &str) -> Option<String> {
    let has_ext = Path::new(name).extension().is_some();
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        if !has_ext && cfg!(windows) {
            if let Some(found) = probe_pathext(&dir, name) {
                return Some(found);
            }
        }
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

fn probe_pathext(dir: &Path, name: &str) -> Option<String> {
    let pathext = std::env::var_os("PATHEXT").unwrap_or_else(|| ".EXE".into());
    let mut buf = PathBuf::from(dir);
    buf.push(name);
    let base = buf.into_os_string();
    for ext in std::env::split_paths(&pathext) {
        let mut candidate = base.clone();
        candidate.push(ext);
        let candidate = PathBuf::from(candidate);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

/// First 500 lines of command output (CR-stripped).
pub fn take_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .take(500)
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
        .collect()
}

/// Run `exe args` in `cwd`, capturing full stdout as lines (capped).
/// Killed on timeout; never blocks past the budget.
pub fn run_command_lines(
    exe: &str,
    args: &[&str],
    cwd: Option<&Path>,
    timeout_ms: u64,
) -> Result<Vec<String>, String> {
    use std::io::Read;
    let mut cmd = Command::new(exe);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::null());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => {
                if !status.success() {
                    return Err(format!("exit status {status}"));
                }
                let mut buf = String::new();
                if let Some(mut s) = child.stdout.take() {
                    let _ = s.read_to_string(&mut buf);
                }
                return Ok(take_lines(&buf));
            }
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timed out after {timeout_ms}ms"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}
pub fn run_version(exe: &str, args: &[&str], timeout_ms: u64) -> Result<String, String> {
    let mut child = Command::new(exe)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => {
                if !status.success() {
                    return Err(format!("exit status {status}"));
                }
                let out = child
                    .stdout
                    .take()
                    .map(|mut s| {
                        use std::io::Read;
                        let mut buf = String::new();
                        let _ = s.read_to_string(&mut buf);
                        buf
                    })
                    .unwrap_or_default();
                let first = out.lines().next().unwrap_or("").trim().to_string();
                return Ok(first);
            }
            None => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("timed out after {timeout_ms}ms"));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{find_on_path, run_version};
    use std::time::Instant;

    #[test]
    fn test_find_on_path_finds_cargo() {
        let found = find_on_path("cargo").expect("cargo must be on PATH in dev/test env");
        assert!(found.to_lowercase().ends_with("cargo.exe") || found.ends_with("cargo"));
    }

    #[test]
    fn test_find_on_path_missing_is_none() {
        assert_eq!(find_on_path("agenthq-no-such-bin-xyz"), None);
    }

    #[test]
    fn test_version_command_cargo() {
        let out = run_version("cargo", &["--version"], 5000).expect("cargo --version must work");
        assert!(out.chars().any(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_version_command_times_out() {
        let start = Instant::now();
        let res = run_version(
            "powershell",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
            300,
        );
        assert!(res.is_err(), "hung command must time out");
        assert!(start.elapsed().as_secs() < 10, "must not hang the suite");
    }

    #[test]
    fn test_take_lines_caps_500() {
        let big = (0..600)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let out = super::take_lines(&big);
        assert_eq!(out.len(), 500);
        assert_eq!(out[0], "line 0");
    }

    #[test]
    fn test_run_command_lines_times_out() {
        let start = Instant::now();
        let res = super::run_command_lines(
            "powershell",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
            None,
            300,
        );
        assert!(res.is_err(), "hung command must time out");
        assert!(start.elapsed().as_secs() < 10, "must not hang the suite");
    }
}
