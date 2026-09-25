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
    pub firmware: String,
    pub board: String,
    pub token: Option<String>,
    // Not read yet — reserved for adapting what's sent by display class
    // (ADR 0006), once more than one class of device exists.
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

/// Sent instead of `welcome` when `hello`'s token is missing or not
/// recognized (protocol §4.2).
#[derive(Serialize)]
pub struct PairRequired {
    pub v: u8,
    #[serde(rename = "type")]
    pub kind: &'static str,
}

impl PairRequired {
    pub fn new() -> Self {
        Self {
            v: PROTOCOL_VERSION,
            kind: "pair_required",
        }
    }
}

/// The device's reply to `pair_required`, carrying the code it generated
/// and is showing on its own screen.
#[derive(Deserialize)]
pub struct PairRequest {
    #[serde(rename = "type")]
    pub kind: String,
    pub code: String,
}

/// Sent once a human confirms the code a device showed (protocol §4.2),
/// immediately followed by `welcome`.
#[derive(Serialize)]
pub struct Paired {
    pub v: u8,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub agent_id: String,
    pub token: String,
}

impl Paired {
    pub fn new(agent_id: String, token: String) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            kind: "paired",
            agent_id,
            token,
        }
    }
}

/// A protocol-level error (§4.3), sent right before closing the
/// connection.
#[derive(Serialize)]
pub struct ErrorMessage {
    pub v: u8,
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub code: &'static str,
    pub message: String,
}

impl ErrorMessage {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            v: PROTOCOL_VERSION,
            kind: "error",
            code,
            message: message.into(),
        }
    }
}
