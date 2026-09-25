//! Wire types for the espia device protocol (`protocol/README.md`). Kept
//! separate from `ws.rs` so discovery and pairing (added in later build
//! steps) can share them without pulling in socket-handling code.

use espia_core::collectors::system::SystemMetrics;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u8 = 1;

/// The first message a device sends after connecting (protocol §4.1).
#[derive(Deserialize)]
pub struct Hello {
    pub device_id: String,
    #[allow(dead_code)] // read for future pairing/logging steps
    pub firmware: String,
    pub board: String,
    #[allow(dead_code)] // wired up once pairing lands (a later build step)
    pub token: Option<String>,
    #[allow(dead_code)]
    pub display: Display,
}

#[derive(Deserialize)]
pub struct Display {
    #[allow(dead_code)]
    pub width: u32,
    #[allow(dead_code)]
    pub height: u32,
    #[allow(dead_code)]
    pub mono: bool,
    #[allow(dead_code)]
    pub touch: bool,
}

/// Sent once a `hello` is accepted (protocol §4.1) — today, unconditionally;
/// the pairing check (`pair_required` instead, when no valid token) lands
/// in a later build step.
#[derive(Serialize)]
pub struct Welcome {
    pub v: u8,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub agent_id: String,
    pub name: String,
    pub agent_version: String,
}

impl Welcome {
    pub fn new(agent_id: String, name: String) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            kind: "welcome",
            agent_id,
            name,
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Live system metrics (protocol §5.1), sent roughly once a second.
/// `SystemMetrics`'s own field names already match the spec 1:1 (see its
/// doc comment in `espia-core`), so this just wraps it in the envelope.
#[derive(Serialize)]
pub struct MetricsMessage {
    pub v: u8,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub ts: u64,
    #[serde(flatten)]
    pub metrics: SystemMetrics,
}

impl MetricsMessage {
    pub fn new(metrics: SystemMetrics, ts: u64) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            kind: "metrics",
            ts,
            metrics,
        }
    }
}

/// A device's UDP broadcast discovery request (protocol §1.2). Only `type`
/// is actually read today — `device_id` isn't needed to reply, but is kept
/// here (unused) so the shape stays a faithful match of the wire message.
#[derive(Deserialize)]
pub struct Discover {
    #[serde(rename = "type")]
    pub kind: String,
    #[allow(dead_code)]
    pub device_id: String,
}

/// This agent's unicast reply to a `discover` datagram (protocol §1.2).
#[derive(Serialize)]
pub struct Announce {
    pub v: u8,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub agent_id: String,
    pub name: String,
    pub port: u16,
    pub path: &'static str,
}
