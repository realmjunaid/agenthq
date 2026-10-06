//! Start/stop only processes whose executable stem matches the adapter.
//! A missing exe never matches. This process is never a candidate.

use crate::monitoring::process::ProcessInfo;

/// PIDs safe to stop for one agent. `self_pid` is always excluded.
pub fn owned_pids(names: &[&str], snapshot: &[ProcessInfo], self_pid: u32) -> Vec<u32> {
    let mut pids: Vec<u32> = snapshot
        .iter()
        .filter(|p| p.pid != self_pid)
        .filter(|p| crate::agents::manager::exe_stem_matches(p.exe.as_deref(), names))
        .map(|p| p.pid)
        .collect();
    pids.sort();
    pids.dedup();
    pids
}

#[cfg(test)]
mod tests {
    use super::owned_pids;
    use crate::monitoring::process::ProcessInfo;

    fn row(pid: u32, exe: Option<&str>) -> ProcessInfo {
        ProcessInfo::new(
            pid,
            None,
            format!("p{pid}"),
            exe.map(|s| s.to_string()),
            0.0,
            0,
            0,
            None,
        )
    }

    #[test]
    fn test_none_exe_is_never_a_candidate() {
        let snap = vec![
            row(10, None),
            row(11, Some("C:\\bin\\claude.exe")),
            row(12, Some("C:\\bin\\other.exe")),
            row(4, Some("C:\\bin\\claude.exe")),
        ];
        let pids = owned_pids(&["claude"], &snap, 4);
        assert_eq!(pids, vec![11]);
    }
}
