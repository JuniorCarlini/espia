//! Orchestrates the desktop agent's device-facing server: the WebSocket
//! endpoint devices connect to (`protocol/README.md` §2), and — from a
//! later build step — mDNS/UDP discovery and real pairing. Spawned once
//! from `lib.rs`'s Tauri `.setup()` hook via `tauri::async_runtime::spawn`.
//!
//! See the plan this was built from for the staged rollout: this file is
//! build-step 1 (bare WebSocket server, `hello` accepted unconditionally).

mod discovery;
mod protocol;
mod ws;

use std::sync::Arc;
use std::time::Duration;

use espia_core::collectors::system::{SystemCollector, SystemMetrics};
use espia_core::{identity, settings};
use mdns_sd::ServiceDaemon;
use tokio::sync::watch;

const WS_PORT: u16 = 47801;
// The settings UI polls at ~2s (see `main.js`'s `POLL_INTERVAL_MS`); the
// device stream is independent and follows the protocol's own ~1/sec pace.
const METRICS_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

pub struct SharedState {
    pub agent_id: String,
    pub ws_port: u16,
    pub metrics: watch::Sender<SystemMetrics>,
    /// Kept alive for the process's whole lifetime — dropping it
    /// unregisters the mDNS advertisement. `None` if advertising failed at
    /// startup (the server still runs; devices fall back to UDP discovery).
    _mdns: Option<ServiceDaemon>,
}

impl SharedState {
    /// The name to show for this agent — see `settings::agent_name`. Read
    /// fresh on every use (not cached) so a rename takes effect on a
    /// device's *next* connection without restarting the server. (The mDNS
    /// TXT record's `name` is a snapshot from startup instead — updating it
    /// live is Stage 2 work.)
    pub fn agent_name(&self) -> String {
        settings::agent_name()
    }
}

/// Starts the device-facing server. Runs until the app exits.
pub async fn run() {
    let agent_id = identity::load_or_create();
    let agent_name = settings::agent_name();
    eprintln!("espia: agent id {agent_id}, name {agent_name:?}");

    let mdns = discovery::advertise_mdns(&agent_id, &agent_name, WS_PORT)
        .map_err(|error| eprintln!("espia: mDNS advertisement failed, devices can still use UDP discovery: {error}"))
        .ok();

    let mut collector = SystemCollector::new();
    let (metrics_tx, _metrics_rx) = watch::channel(collector.refresh());

    let state = Arc::new(SharedState {
        agent_id,
        ws_port: WS_PORT,
        metrics: metrics_tx,
        _mdns: mdns,
    });

    let collector_state = state.clone();
    let collector_task = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(METRICS_REFRESH_INTERVAL);
        loop {
            ticker.tick().await;
            let snapshot = collector.refresh();
            // No receivers yet is not an error — it just means no device is
            // connected right now.
            let _ = collector_state.metrics.send(snapshot);
        }
    });

    let ws_bind_addr = format!("0.0.0.0:{WS_PORT}");

    tokio::select! {
        _ = collector_task => {}
        _ = ws::serve(&ws_bind_addr, state.clone()) => {}
        _ = discovery::serve_udp(state.clone()) => {}
    }
}
