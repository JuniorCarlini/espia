//! Network throughput, summed across all non-loopback interfaces.
//!
//! Field names follow the `network` object in the `metrics` message
//! ([protocol specification](../../../../protocol/README.md#51-metrics-agent--device)).

use serde::Serialize;
use sysinfo::Networks;

#[derive(Serialize, Clone, Debug)]
pub struct NetworkInfo {
    pub rx_bps: u64,
    pub tx_bps: u64,
}

/// Reads throughput since the last [`Networks::refresh`] call.
///
/// `sysinfo` reports bytes transferred since that last refresh, not a rate,
/// so this is only accurate as "bytes per second" when refreshed on a
/// steady ~1-second cadence — see [`super::system::SystemCollector`], which
/// does exactly that.
pub fn read(networks: &Networks) -> NetworkInfo {
    let mut rx_bps = 0;
    let mut tx_bps = 0;
    for (interface_name, data) in networks {
        if interface_name == "lo" || interface_name.starts_with("lo0") {
            continue;
        }
        rx_bps += data.received();
        tx_bps += data.transmitted();
    }
    NetworkInfo { rx_bps, tx_bps }
}
