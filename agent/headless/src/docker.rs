//! Per-container CPU/memory usage, read straight from the Docker Engine API
//! over its Unix socket — no client library, matching this binary's "no
//! async runtime, nothing fancy" philosophy (see `docs/adr/0014-docker-container-stats.md`).
//!
//! Requires `/var/run/docker.sock` mounted into this container. Anyone who
//! can reach that socket has root-equivalent control over the whole Docker
//! host, not just read access to stats — see the ADR before enabling this.

use std::env;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const SOCKET_ENV_VAR: &str = "ESPIA_DOCKER_SOCKET";
const DEFAULT_SOCKET: &str = "/var/run/docker.sock";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize)]
pub struct ContainerStat {
    pub id: String,
    pub name: String,
    pub image: String,
    pub cpu_pct: f64,
    pub memory_bytes: u64,
    pub memory_limit_bytes: u64,
}

#[derive(Deserialize)]
struct ContainerSummary {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Names")]
    names: Vec<String>,
    #[serde(rename = "Image")]
    image: String,
}

#[derive(Deserialize)]
struct StatsResponse {
    cpu_stats: CpuStats,
    precpu_stats: CpuStats,
    memory_stats: MemoryStats,
}

#[derive(Deserialize, Default)]
struct CpuStats {
    cpu_usage: CpuUsage,
    #[serde(default)]
    system_cpu_usage: u64,
    #[serde(default)]
    online_cpus: u64,
}

#[derive(Deserialize, Default)]
struct CpuUsage {
    #[serde(default)]
    total_usage: u64,
    #[serde(default)]
    percpu_usage: Vec<u64>,
}

#[derive(Deserialize, Default)]
struct MemoryStats {
    #[serde(default)]
    usage: u64,
    #[serde(default)]
    limit: u64,
    #[serde(default)]
    stats: MemoryDetail,
}

#[derive(Deserialize, Default)]
struct MemoryDetail {
    /// cgroup v1 field name for page cache — subtracted from `usage` so this
    /// reads like `docker stats`' MEM USAGE, not the raw cgroup accounting
    /// (which double-counts reclaimable page cache as "used").
    #[serde(default)]
    cache: u64,
    /// cgroup v2's equivalent of `cache`, when present.
    #[serde(default)]
    inactive_file: u64,
}

/// Returns every running container's CPU % and memory usage, sorted by CPU
/// descending — the same shape as `top_processes`. `Err` covers the whole
/// request failing (no socket, permission denied, daemon unreachable); one
/// bad container's stats call is skipped rather than failing the batch.
pub fn container_stats() -> Result<Vec<ContainerStat>, String> {
    let socket = env::var(SOCKET_ENV_VAR).unwrap_or_else(|_| DEFAULT_SOCKET.to_string());
    let containers: Vec<ContainerSummary> = serde_json::from_str(&docker_get(&socket, "/containers/json")?)
        .map_err(|e| format!("could not parse container list: {e}"))?;

    let mut stats: Vec<ContainerStat> = containers
        .into_iter()
        .filter_map(|container| {
            let body = docker_get(&socket, &format!("/containers/{}/stats?stream=false", container.id)).ok()?;
            let response: StatsResponse = serde_json::from_str(&body).ok()?;
            Some(ContainerStat {
                id: container.id.chars().take(12).collect(),
                name: container.names.into_iter().next().unwrap_or_default().trim_start_matches('/').to_string(),
                image: container.image,
                cpu_pct: cpu_percent(&response.cpu_stats, &response.precpu_stats),
                memory_bytes: response.memory_stats.usage.saturating_sub(
                    response.memory_stats.stats.cache.max(response.memory_stats.stats.inactive_file),
                ),
                memory_limit_bytes: response.memory_stats.limit,
            })
        })
        .collect();

    stats.sort_by(|a, b| b.cpu_pct.total_cmp(&a.cpu_pct));
    Ok(stats)
}

/// The same formula `docker stats` itself uses: CPU time this container
/// used, as a share of CPU time the whole system used, over the same
/// interval — scaled up by core count so one busy core on an N-core host
/// doesn't read as "100% / N".
fn cpu_percent(current: &CpuStats, previous: &CpuStats) -> f64 {
    let cpu_delta = current.cpu_usage.total_usage.saturating_sub(previous.cpu_usage.total_usage) as f64;
    let system_delta = current.system_cpu_usage.saturating_sub(previous.system_cpu_usage) as f64;
    if system_delta <= 0.0 || cpu_delta <= 0.0 {
        return 0.0;
    }
    let cores = if current.online_cpus > 0 {
        current.online_cpus as f64
    } else {
        current.cpu_usage.percpu_usage.len().max(1) as f64
    };
    (cpu_delta / system_delta) * cores * 100.0
}

fn docker_get(socket_path: &str, path: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|e| format!("connecting to {socket_path}: {e}"))?;
    stream.set_read_timeout(Some(REQUEST_TIMEOUT)).ok();
    stream.set_write_timeout(Some(REQUEST_TIMEOUT)).ok();

    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).map_err(|e| format!("writing request: {e}"))?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(|e| format!("reading response: {e}"))?;

    let header_end = find(&raw, b"\r\n\r\n").ok_or("malformed HTTP response from the Docker socket")?;
    let header_text = String::from_utf8_lossy(&raw[..header_end]);
    let body = &raw[header_end + 4..];

    let status_line = header_text.lines().next().unwrap_or("");
    if !status_line.contains(" 200 ") {
        return Err(format!("Docker API returned {status_line:?} for {path}"));
    }

    let body = if header_text.to_ascii_lowercase().contains("transfer-encoding: chunked") {
        dechunk(body)
    } else {
        body.to_vec()
    };
    String::from_utf8(body).map_err(|e| format!("non-UTF8 response body: {e}"))
}

/// Decodes an HTTP chunked-transfer body. The Docker daemon (a Go `net/http`
/// server) uses chunked encoding for these endpoints when it doesn't know
/// the body length up front, which is the common case here.
fn dechunk(mut data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(line_end) = find(data, b"\r\n") {
        let size_line = String::from_utf8_lossy(&data[..line_end]);
        let Ok(size) = usize::from_str_radix(size_line.trim(), 16) else { break };
        if size == 0 {
            break;
        }
        let chunk_start = line_end + 2;
        let chunk_end = chunk_start + size;
        if chunk_end > data.len() {
            break;
        }
        out.extend_from_slice(&data[chunk_start..chunk_end]);
        data = data.get(chunk_end + 2..).unwrap_or(&[]);
    }
    out
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dechunk_reassembles_a_chunked_body() {
        let chunked = b"7\r\nMozilla\r\n9\r\nDeveloper\r\n0\r\n\r\n";
        assert_eq!(dechunk(chunked), b"MozillaDeveloper");
    }

    #[test]
    fn dechunk_handles_an_empty_body() {
        assert_eq!(dechunk(b"0\r\n\r\n"), Vec::<u8>::new());
    }

    #[test]
    fn cpu_percent_scales_by_core_count() {
        let previous = CpuStats { cpu_usage: CpuUsage { total_usage: 1000, percpu_usage: vec![] }, system_cpu_usage: 10_000, online_cpus: 4 };
        let current = CpuStats { cpu_usage: CpuUsage { total_usage: 1500, percpu_usage: vec![] }, system_cpu_usage: 12_000, online_cpus: 4 };
        // (500 cpu-ns / 2000 system-ns) * 4 cores * 100 = 100%
        assert_eq!(cpu_percent(&current, &previous), 100.0);
    }

    #[test]
    fn cpu_percent_is_zero_with_no_prior_sample() {
        let previous = CpuStats::default();
        let current = CpuStats { cpu_usage: CpuUsage { total_usage: 500, percpu_usage: vec![] }, system_cpu_usage: 0, online_cpus: 2 };
        assert_eq!(cpu_percent(&current, &previous), 0.0);
    }
}
