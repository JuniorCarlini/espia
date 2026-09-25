//! Collectors read metrics from the local machine.
//!
//! Each collector owns one concern (system, GPU, ...) and exposes a
//! `serde`-serializable snapshot whose field names match the
//! [protocol specification](../../../../protocol/README.md), so collector
//! output can be forwarded to devices with no translation layer.

pub mod disk;
pub mod network;
pub mod processes;
pub mod system;
