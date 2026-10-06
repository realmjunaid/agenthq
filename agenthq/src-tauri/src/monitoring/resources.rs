//! Host CPU/RAM live on `ProcessMonitor` (two-tick CPU). Disk and network
//! live here so the first network sample can stay 0 instead of a fake rate.
use serde::Serialize;
use sysinfo::{Disks, Networks};

/// Default resource update interval, seconds (§14).
#[allow(dead_code)] // Pinned by test; consumed by Phase 14 polling.
pub const DEFAULT_TICK_SECS: u64 = 2;

#[derive(Debug, Clone, Serialize)]
pub struct SystemStats {
    pub total_cpu: f32,
    pub used_mem_bytes: u64,
    pub total_mem_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiskStat {
    pub name: String,
    pub mount: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetStat {
    pub name: String,
    /// Bytes since the previous refresh. Zero on the first sample.
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HostResources {
    pub disks: Vec<DiskStat>,
    pub networks: Vec<NetStat>,
}

pub struct ResourceSampler {
    disks: Disks,
    networks: Networks,
}

impl ResourceSampler {
    pub fn new() -> Self {
        ResourceSampler {
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
        }
    }

    /// One tick. Network counters are deltas since the last call.
    pub fn refresh(&mut self) -> HostResources {
        self.disks.refresh();
        self.networks.refresh();
        self.snapshot()
    }

    /// Last sample, no new sysinfo refresh.
    pub fn snapshot(&self) -> HostResources {
        let mut disks: Vec<DiskStat> = self
            .disks
            .list()
            .iter()
            .map(|d| DiskStat {
                name: d.name().to_string_lossy().into_owned(),
                mount: d.mount_point().to_string_lossy().into_owned(),
                total_bytes: d.total_space(),
                available_bytes: d.available_space(),
            })
            .collect();
        disks.sort_by(|a, b| a.mount.cmp(&b.mount));
        let mut networks: Vec<NetStat> = self
            .networks
            .list()
            .iter()
            .map(|(name, data)| NetStat {
                name: name.clone(),
                received_bytes: data.received(),
                transmitted_bytes: data.transmitted(),
            })
            .collect();
        networks.sort_by(|a, b| a.name.cmp(&b.name));
        HostResources { disks, networks }
    }
}

#[cfg(test)]
mod tests {
    use super::ResourceSampler;

    #[test]
    fn test_tick_interval_default_is_two_seconds() {
        assert_eq!(super::DEFAULT_TICK_SECS, 2);
    }

    #[test]
    fn test_disk_totals_are_sane() {
        let mut sampler = ResourceSampler::new();
        let snap = sampler.refresh();
        assert!(
            snap.disks.iter().any(|d| d.total_bytes > 0),
            "this machine should report at least one disk"
        );
        for d in &snap.disks {
            assert!(d.available_bytes <= d.total_bytes);
        }
    }

    #[test]
    fn test_first_network_sample_does_not_panic() {
        let mut sampler = ResourceSampler::new();
        let snap = sampler.refresh();
        for n in &snap.networks {
            assert!(!n.name.is_empty());
        }
    }
}
