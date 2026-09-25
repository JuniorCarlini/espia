//! System metrics: CPU, memory, disk, and network usage.
//!
//! GPU collectors are not implemented yet — see `firmware/README.md`'s
//! counterpart, `agent/README.md`'s roadmap.

use std::time::{Duration, Instant};

use serde::Serialize;
use sysinfo::{Components, Disks, Networks, ProcessesToUpdate, System};

use super::disk::{self, DiskInfo};
use super::network::{self, NetworkInfo};
use super::processes::{self, ProcessUsage};

/// A point-in-time snapshot of system metrics.
///
/// Field names follow the `metrics` message in the
/// [protocol specification](../../../../protocol/README.md#51-metrics-agent--device).
#[derive(Serialize, Clone, Debug)]
pub struct SystemMetrics {
    pub cpu: Cpu,
    pub memory: Memory,
    pub disks: Vec<DiskInfo>,
    pub network: NetworkInfo,
}

#[derive(Serialize, Clone, Debug)]
pub struct Cpu {
    pub usage_pct: f32,
    pub core_count: usize,
    /// `None` when no plausible CPU sensor could be found. See
    /// [`read_cpu_temp`] for how this is approximated on Apple Silicon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_c: Option<f32>,
    /// `true` when `temp_c` is approximated from indirect sensors (Apple
    /// Silicon), rather than a sensor the OS itself labels as the CPU.
    /// Absent (not just `false`) when `temp_c` is absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_estimated: Option<bool>,
}

#[derive(Serialize, Clone, Debug)]
pub struct Memory {
    pub used_bytes: u64,
    pub total_bytes: u64,
}

fn snapshot(
    system: &System,
    components: &Components,
    disks: &Disks,
    networks: &Networks,
) -> SystemMetrics {
    let temp = read_cpu_temp(components);
    SystemMetrics {
        cpu: Cpu {
            usage_pct: system.global_cpu_usage(),
            core_count: system.cpus().len(),
            temp_c: temp.as_ref().map(|t| t.celsius),
            temp_estimated: temp.as_ref().map(|t| t.estimated),
        },
        memory: Memory {
            used_bytes: system.used_memory(),
            total_bytes: system.total_memory(),
        },
        disks: disk::read(disks),
        network: network::read(networks),
    }
}

/// A sensor reading outside this range is a bogus value (seen in practice
/// from at least one Apple Silicon SMC channel) rather than a real
/// temperature, and is discarded.
const PLAUSIBLE_TEMP_RANGE_C: std::ops::RangeInclusive<f32> = -20.0..=130.0;

struct CpuTemp {
    celsius: f32,
    /// `true` when approximated from indirect sensors rather than one the
    /// OS itself labels as the CPU. See [`read_cpu_temp`].
    estimated: bool,
}

/// Approximates CPU temperature from the sensors `sysinfo` can see.
///
/// Linux and Windows typically expose a clearly labeled CPU sensor (for
/// example `Core 0`, `Package id 0`, or `CPU`), which is used directly when
/// present. Apple Silicon Macs do not expose a single "CPU package" sensor
/// through SMC; sysinfo instead surfaces many raw per-die readings labeled
/// `PMU ...` / `PMU2 ...`, none of which alone represents "the" CPU
/// temperature. In that case, this returns the highest plausible one as an
/// estimate — hotter than any individual core, cooler than a true package
/// peak, but a reasonable stand-in for "how hot is this Mac running".
fn read_cpu_temp(components: &Components) -> Option<CpuTemp> {
    let labeled_cpu_sensor = components.list().iter().find_map(|component| {
        let label = component.label().to_ascii_lowercase();
        let looks_like_cpu =
            (label.contains("cpu") || label.contains("core") || label.contains("package"))
                && !label.contains("gpu");
        looks_like_cpu
            .then(|| component.temperature())
            .flatten()
            .filter(|temp| PLAUSIBLE_TEMP_RANGE_C.contains(temp))
    });
    if let Some(celsius) = labeled_cpu_sensor {
        return Some(CpuTemp {
            celsius,
            estimated: false,
        });
    }

    components
        .list()
        .iter()
        .filter(|component| component.label().starts_with("PMU"))
        .filter_map(|component| component.temperature())
        .filter(|temp| PLAUSIBLE_TEMP_RANGE_C.contains(temp))
        .fold(None, |max: Option<f32>, temp| Some(max.map_or(temp, |m| m.max(temp))))
        .map(|celsius| CpuTemp {
            celsius,
            estimated: true,
        })
}

/// Enumerating and sorting every process on the system is the most
/// expensive thing this collector does, so it's refreshed on its own,
/// slower cadence rather than every [`SystemCollector::refresh`] tick (the
/// UI polls that roughly every 2s — see `POLL_INTERVAL_MS` in `main.js`).
/// A process list moves slowly enough that this stays accurate.
const PROCESS_REFRESH_INTERVAL: Duration = Duration::from_secs(6);

/// Keeps a [`System`], [`Components`], [`Disks`], and [`Networks`] list
/// alive between calls so CPU and network usage can be computed from
/// consecutive samples, as `sysinfo` requires.
pub struct SystemCollector {
    system: System,
    components: Components,
    disks: Disks,
    networks: Networks,
    last_process_refresh: Option<Instant>,
}

impl SystemCollector {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        Self {
            system,
            components: Components::new_with_refreshed_list(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last_process_refresh: None,
        }
    }

    /// Refreshes and returns the latest snapshot. Call this on a fixed
    /// interval (every couple of seconds is plenty) so CPU and network
    /// usage stay accurate. Does not touch the process list — see
    /// [`top_processes`](Self::top_processes).
    pub fn refresh(&mut self) -> SystemMetrics {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.components.refresh(false);
        self.disks.refresh(true);
        self.networks.refresh(true);
        snapshot(&self.system, &self.components, &self.disks, &self.networks)
    }

    /// Returns the `limit` processes using the most CPU, refreshing the
    /// process list first if it's been more than [`PROCESS_REFRESH_INTERVAL`]
    /// since the last refresh. Independent of [`refresh`](Self::refresh) —
    /// callers don't need to call that first.
    pub fn top_processes(&mut self, limit: usize) -> Vec<ProcessUsage> {
        let is_stale = self
            .last_process_refresh
            .map(|t| t.elapsed() >= PROCESS_REFRESH_INTERVAL)
            .unwrap_or(true);
        if is_stale {
            self.system.refresh_processes(ProcessesToUpdate::All, true);
            self.last_process_refresh = Some(Instant::now());
        }
        processes::top_by_cpu(&self.system, limit)
    }
}

impl Default for SystemCollector {
    fn default() -> Self {
        Self::new()
    }
}
