//! Per-process CPU and memory usage.
//!
//! Unlike [`super::system`], this is not part of the `metrics` protocol
//! message — a process table doesn't fit a small device screen. It only
//! backs the agent's own settings UI, so results are returned pre-sorted
//! and truncated instead of streamed in full.

use serde::Serialize;
use sysinfo::System;

#[derive(Serialize, Clone, Debug)]
pub struct ProcessUsage {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f32,
    pub memory_bytes: u64,
}

/// Returns the `limit` processes with the highest CPU usage, descending.
///
/// As with [`super::system::SystemCollector`], `sysinfo` computes per-process
/// CPU usage from consecutive samples, so this only becomes accurate once
/// the caller has refreshed processes on a steady interval for a moment.
pub fn top_by_cpu(system: &System, limit: usize) -> Vec<ProcessUsage> {
    let mut processes: Vec<ProcessUsage> = system
        .processes()
        .values()
        .map(|process| ProcessUsage {
            pid: process.pid().as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            cpu_pct: process.cpu_usage(),
            memory_bytes: process.memory(),
        })
        .collect();

    processes.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct));
    processes.truncate(limit);
    processes
}
