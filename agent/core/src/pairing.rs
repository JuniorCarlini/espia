//! The agent's memory of which devices it has paired with (protocol §4.2,
//! ADR 0007): once a device's 6-digit pairing code is confirmed, it's
//! issued a token here so a later reconnect can skip pairing entirely.
//!
//! `devices` is a list from day one, not a single optional pairing — a
//! device may eventually pair with more than one agent, and this agent may
//! eventually accept more than one device, per ADR 0007. Persisted the same
//! way as [`crate::settings`]: a small JSON file in the agent's data
//! directory.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::security::constant_time_eq;
use crate::settings::app_data_dir;

const PAIRINGS_FILE: &str = "pairings.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DevicePairing {
    pub device_id: String,
    pub token: String,
    pub board: String,
    pub firmware: String,
    pub paired_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PairingStore {
    pub devices: Vec<DevicePairing>,
}

fn pairings_path() -> Result<std::path::PathBuf, String> {
    Ok(app_data_dir()?.join(PAIRINGS_FILE))
}

/// Reads every pairing this agent has issued, or an empty store if none
/// have been saved yet (or the file can't be read — a corrupt pairings
/// file shouldn't be able to make the server unusable, only make every
/// device re-pair).
pub fn load() -> PairingStore {
    let Ok(path) = pairings_path() else {
        return PairingStore::default();
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return PairingStore::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save(store: &PairingStore) -> Result<(), String> {
    let path = pairings_path()?;
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("Could not write {}: {e}", path.display()))
}

/// Looks up a pairing by the token a device presented in `hello`. Compares
/// every stored token in constant time — the list is short (ADR 0007 calls
/// this "a handful," not dozens), so the cost of checking all of them
/// rather than short-circuiting on the first match is negligible next to
/// what it buys: no single comparison's timing depends on how much of a
/// guessed token was right.
pub fn find_by_token(token: &str) -> Option<DevicePairing> {
    load()
        .devices
        .into_iter()
        .find(|pairing| constant_time_eq(pairing.token.as_bytes(), token.as_bytes()))
}

/// Issues a fresh token for `device_id`, replacing any previous pairing for
/// that same device (a device only reaches here because it didn't present
/// a token `find_by_token` recognized — including a device pairing again
/// after losing its old token, for example a factory reset).
pub fn add(device_id: &str, board: &str, firmware: &str) -> Result<DevicePairing, String> {
    let mut store = load();
    store.devices.retain(|p| p.device_id != device_id);

    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let paired_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let pairing = DevicePairing {
        device_id: device_id.to_string(),
        token,
        board: board.to_string(),
        firmware: firmware.to_string(),
        paired_at,
    };
    store.devices.push(pairing.clone());
    save(&store)?;
    Ok(pairing)
}

/// Forgets a device's pairing — it will need to pair again to reconnect.
pub fn remove(device_id: &str) -> Result<PairingStore, String> {
    let mut store = load();
    store.devices.retain(|p| p.device_id != device_id);
    save(&store)?;
    Ok(store)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::HOME_ENV_LOCK;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::MutexGuard;

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    struct TestHome {
        dir: PathBuf,
        _lock: MutexGuard<'static, ()>,
    }

    impl TestHome {
        fn new() -> Self {
            let lock = HOME_ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let dir = std::env::temp_dir().join(format!("espia-pairing-test-{}-{n}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            // SAFETY: serialized by `HOME_ENV_LOCK` above.
            unsafe {
                std::env::set_var("HOME", &dir);
            }
            Self { dir, _lock: lock }
        }
    }

    impl Drop for TestHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn defaults_to_no_pairings() {
        let _home = TestHome::new();
        assert!(load().devices.is_empty());
    }

    #[test]
    fn add_persists_and_is_found_by_its_token() {
        let _home = TestHome::new();
        let pairing = add("espia-abc123", "Heltec WiFi Kit 32 V3", "0.1.0").expect("should pair");

        let found = find_by_token(&pairing.token).expect("token should be recognized");
        assert_eq!(found.device_id, "espia-abc123");
        assert_eq!(load().devices.len(), 1);
    }

    #[test]
    fn an_unknown_token_is_not_found() {
        let _home = TestHome::new();
        add("espia-abc123", "Heltec WiFi Kit 32 V3", "0.1.0").expect("should pair");
        assert!(find_by_token("not-a-real-token").is_none());
    }

    #[test]
    fn re_pairing_the_same_device_replaces_its_old_token() {
        let _home = TestHome::new();
        let first = add("espia-abc123", "board", "0.1.0").expect("should pair");
        let second = add("espia-abc123", "board", "0.1.1").expect("should re-pair");

        assert_ne!(first.token, second.token);
        assert!(find_by_token(&first.token).is_none(), "the old token must stop working");
        assert!(find_by_token(&second.token).is_some());
        assert_eq!(load().devices.len(), 1, "must not accumulate duplicate entries");
    }

    #[test]
    fn remove_forgets_a_pairing() {
        let _home = TestHome::new();
        let pairing = add("espia-abc123", "board", "0.1.0").expect("should pair");

        let store = remove("espia-abc123").expect("should remove");
        assert!(store.devices.is_empty());
        assert!(find_by_token(&pairing.token).is_none());
    }
}
