//! espia-core — collectors, providers, and settings shared by every way
//! this agent can run: the Tauri desktop GUI (`../src-tauri`) and the
//! headless HTTP server (`../headless`), meant for a VPS or a Docker host.
//! See `docs/adr/0013-headless-agent.md`.
//!
//! This crate has no GUI dependency of any kind — that's the entire point
//! of its existing separately from `src-tauri`.

pub mod collectors;
pub mod providers;
pub mod settings;

/// Test-only support shared across modules. `providers::claude` and
/// `settings` both have tests that point the process-wide `HOME` env var at
/// a scratch directory (to exercise real file I/O without touching a real
/// user's files); each module having its own lock for that isn't enough —
/// `cargo test` runs different modules' tests in parallel by default, so
/// both locks must be the same one, or a test in one module can still race
/// a test in the other.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;
    pub(crate) static HOME_ENV_LOCK: Mutex<()> = Mutex::new(());
}
