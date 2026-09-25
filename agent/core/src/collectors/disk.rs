//! Disk space usage.
//!
//! Field names follow the `disks[]` entries in the `metrics` message
//! ([protocol specification](../../../../protocol/README.md#51-metrics-agent--device)).

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use sysinfo::{Disk, Disks};

#[derive(Serialize, Clone, Debug)]
pub struct DiskInfo {
    pub name: String,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

/// Reads space usage per physical disk, largest first.
///
/// macOS's APFS splits one physical volume into several mount points (for
/// example `/` and `/System/Volumes/Data`) that all report the same space
/// figures under the same disk name; only the one mounted at `/` is kept for
/// each name, so the same disk isn't listed twice.
pub fn read(disks: &Disks) -> Vec<DiskInfo> {
    let mut by_name: HashMap<String, &Disk> = HashMap::new();
    for disk in disks.list() {
        let name = disk.name().to_string_lossy().into_owned();
        let is_root = disk.mount_point() == Path::new("/");
        by_name
            .entry(name)
            .and_modify(|kept| {
                if is_root {
                    *kept = disk;
                }
            })
            .or_insert(disk);
    }

    let mut result: Vec<DiskInfo> = by_name
        .into_iter()
        .map(|(name, disk)| {
            let total = disk.total_space();
            let available = disk.available_space();
            DiskInfo {
                name,
                used_bytes: total.saturating_sub(available),
                total_bytes: total,
            }
        })
        // A 0-byte total means the entry couldn't be read (not "a very
        // small but real disk"), so it's dropped rather than shown as full.
        .filter(|disk| disk.total_bytes > 0)
        .collect();

    result.sort_by_key(|disk| std::cmp::Reverse(disk.total_bytes));
    result
}
