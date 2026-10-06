use serde::Serialize;
use std::collections::{HashMap, HashSet};
use sysinfo::{
    CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind,
};

pub const MAX_CMDLINE_CHARS: usize = 512;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub exe: Option<String>,
    pub cpu: f32,
    pub ram_bytes: u64,
    pub started_at: i64,
    pub cmdline: Option<String>,
}

/// Direct children of `pid`. Unknown pids yield an empty vec.
#[allow(dead_code)] // Consumed by process-tree grouping UI (Phase 14+).
pub fn children_of(pid: u32, parent_of: &HashMap<u32, u32>) -> Vec<u32> {
    parent_of
        .iter()
        .filter_map(|(child, parent)| (*parent == pid).then_some(*child))
        .collect()
}

/// pid → parent map, skipping parentless (root) processes.
#[allow(dead_code)] // Consumed by process-tree grouping UI (Phase 14+).
pub fn parent_map(procs: &[ProcessInfo]) -> HashMap<u32, u32> {
    procs
        .iter()
        .filter_map(|p| p.parent_pid.map(|pp| (p.pid, pp)))
        .collect()
}

/// First 512 chars, char-boundary safe. Command lines may carry secrets.
pub fn truncate_cmdline(cmd: &str) -> String {
    if cmd.chars().count() <= MAX_CMDLINE_CHARS {
        return cmd.to_string();
    }
    cmd.chars().take(MAX_CMDLINE_CHARS).collect()
}

impl ProcessInfo {
    pub fn new(
        pid: u32,
        parent_pid: Option<u32>,
        name: String,
        exe: Option<String>,
        cpu: f32,
        ram_bytes: u64,
        started_at: i64,
        cmdline: Option<String>,
    ) -> Self {
        let exe = exe.filter(|s| !s.is_empty());
        let cmdline = cmdline
            .filter(|s| !s.is_empty())
            .map(|s| truncate_cmdline(&s));
        ProcessInfo {
            pid,
            parent_pid,
            name,
            exe,
            cpu,
            ram_bytes,
            started_at,
            cmdline,
        }
    }
}

struct CachedMeta {
    name: String,
    exe: Option<String>,
}

pub struct ProcessMonitor {
    system: System,
    meta: HashMap<u32, CachedMeta>,
    last_full_refresh: Option<std::time::Instant>,
}

/// Minimum gap between full process rescans (matches DEFAULT_TICK_SECS).
pub const REFRESH_TTL_SECS: u64 = 2;

/// Gate for one more full rescan: first call always refreshes, then at
/// most once per TTL. Queued event-loads must not each pay a full scan.
pub fn should_refresh(last: Option<std::time::Instant>, now: std::time::Instant) -> bool {
    match last {
        None => true,
        Some(t) => now.duration_since(t).as_secs() >= REFRESH_TTL_SECS,
    }
}

impl ProcessMonitor {
    pub fn new() -> Self {
        ProcessMonitor {
            system: System::new(),
            meta: HashMap::new(),
            last_full_refresh: None,
        }
    }

    /// One collection pass: snapshot metadata for new PIDs, refresh
    /// cpu/ram for all, drop exited PIDs. Callers tick every
    /// DEFAULT_TICK_SECS; CPU needs two passes to become nonzero.
    pub fn refresh(&mut self) {
        self.refresh_system();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::new()
                .with_cpu()
                .with_memory()
                .with_cmd(UpdateKind::Always)
                .with_exe(UpdateKind::Always),
        );
        let live: HashSet<u32> = self
            .system
            .processes()
            .keys()
            .map(|pid| pid.as_u32())
            .collect();
        self.meta.retain(|pid, _| live.contains(pid));
        for (pid, proc_) in self.system.processes() {
            let pid = pid.as_u32();
            self.meta.entry(pid).or_insert_with(|| CachedMeta {
                name: proc_.name().to_string_lossy().into_owned(),
                exe: {
                    let e = proc_.exe().map(|p| p.to_string_lossy().into_owned());
                    e.filter(|s| !s.is_empty())
                },
            });
        }
    }

    /// CPU + memory totals only (cheap; used by stats ticks).
    pub fn refresh_system(&mut self) {
        self.system
            .refresh_cpu_specifics(CpuRefreshKind::everything());
        self.system
            .refresh_memory_specifics(MemoryRefreshKind::everything());
    }

    pub fn total_cpu(&self) -> f32 {
        self.system.global_cpu_usage()
    }

    /// (used_bytes, total_bytes)
    pub fn memory_usage(&self) -> (u64, u64) {
        (self.system.used_memory(), self.system.total_memory())
    }

    /// Snapshot-rate refresh for polled/event-driven loads: a full
    /// rescan at most once per TTL; cheap cpu/mem totals every call.
    /// Explicit actions (Refresh button, detail snapshot) use refresh().
    pub fn refresh_rate_limited(&mut self) {
        let now = std::time::Instant::now();
        if should_refresh(self.last_full_refresh, now) {
            self.refresh();
            self.last_full_refresh = Some(now);
        } else {
            self.refresh_system();
        }
    }

    pub fn snapshot(&self) -> Vec<ProcessInfo> {
        self.system
            .processes()
            .iter()
            .map(|(pid, proc_)| {
                let pid = pid.as_u32();
                let cached = self.meta.get(&pid);
                let cmd = proc_
                    .cmd()
                    .iter()
                    .map(|s| s.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" ");
                ProcessInfo::new(
                    pid,
                    proc_.parent().map(|p| p.as_u32()),
                    cached
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| proc_.name().to_string_lossy().into_owned()),
                    cached
                        .and_then(|m| m.exe.clone())
                        .or_else(|| proc_.exe().map(|e| e.to_string_lossy().into_owned())),
                    proc_.cpu_usage(),
                    proc_.memory(),
                    proc_.start_time() as i64,
                    Some(cmd),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{children_of, parent_map, should_refresh, ProcessInfo, REFRESH_TTL_SECS};
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    fn row(pid: u32, parent: Option<u32>) -> ProcessInfo {
        ProcessInfo {
            pid,
            parent_pid: parent,
            name: format!("p{pid}"),
            exe: None,
            cpu: 0.0,
            ram_bytes: 0,
            started_at: 0,
            cmdline: None,
        }
    }

    #[test]
    fn test_children_of_returns_direct_children_only() {
        let rows = vec![
            row(1, None),
            row(2, Some(1)),
            row(3, Some(1)),
            row(4, Some(2)),
        ];
        let map = parent_map(&rows);
        let mut c1 = children_of(1, &map);
        c1.sort();
        assert_eq!(c1, vec![2, 3]);
        assert_eq!(children_of(2, &map), vec![4]);
    }

    #[test]
    fn test_children_of_unknown_pid_is_empty() {
        let map: HashMap<u32, u32> = HashMap::new();
        assert!(children_of(99999, &map).is_empty());
    }

    #[test]
    fn test_first_scan_always_refreshes() {
        assert!(should_refresh(None, Instant::now()));
    }

    #[test]
    fn test_rescan_throttled_within_ttl() {
        let first = Instant::now();
        assert!(!should_refresh(
            Some(first),
            first + Duration::from_secs(REFRESH_TTL_SECS - 1)
        ));
        assert!(should_refresh(
            Some(first),
            first + Duration::from_secs(REFRESH_TTL_SECS)
        ));
    }

    #[test]
    fn test_parent_map_skips_root_processes() {
        let rows = vec![row(1, None), row(2, Some(1))];
        let map = parent_map(&rows);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(&2), Some(&1));
        assert!(!map.contains_key(&1));
    }

    #[test]
    fn test_truncate_cmdline_caps_at_512() {
        let long = "a".repeat(2000);
        let out = super::truncate_cmdline(&long);
        assert!(out.chars().count() <= 512, "must be capped");
        let multi = "é".repeat(2000);
        let out2 = super::truncate_cmdline(&multi);
        assert!(out2.chars().count() <= 512, "char-boundary safe");
    }

    #[test]
    fn test_empty_exe_and_cmd_become_none() {
        let info = super::ProcessInfo::new(
            1,
            None,
            "x".to_string(),
            Some(String::new()),
            0.0,
            0,
            0,
            Some(String::new()),
        );
        assert_eq!(info.exe, None);
        assert_eq!(info.cmdline, None);
    }

    #[test]
    fn test_snapshot_contains_current_process() {
        let mut mon = super::ProcessMonitor::new();
        mon.refresh();
        mon.refresh();
        let me = sysinfo::get_current_pid().unwrap().as_u32();
        let found = mon.snapshot().into_iter().find(|p| p.pid == me);
        let info = found.expect("own process must be in snapshot");
        assert!(!info.name.is_empty());
        assert!(info.exe.is_some());
    }

    #[test]
    fn test_double_refresh_succeeds() {
        let mut mon = super::ProcessMonitor::new();
        mon.refresh();
        mon.refresh();
        assert!(!mon.snapshot().is_empty());
    }

    #[test]
    fn test_monitor_memory_sane_after_refresh() {
        let mut mon = super::ProcessMonitor::new();
        mon.refresh_system();
        let (used, total) = mon.memory_usage();
        assert!(total > 0);
        assert!(used <= total);
    }
}
