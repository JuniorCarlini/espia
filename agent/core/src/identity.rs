//! The agent's own stable identity: a UUID used in mDNS advertisements and
//! the `welcome` handshake message (protocol §4) so a device can tell this
//! agent apart from any other it discovers.
//!
//! Generated once and persisted immediately on first use — unlike
//! [`crate::settings`], which only writes when a user explicitly changes
//! something, this has no user action to wait for: the server needs a
//! stable id from its very first mDNS announcement.

use std::fs;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::settings::app_data_dir;

const IDENTITY_FILE: &str = "identity.json";

#[derive(Serialize, Deserialize)]
struct Identity {
    agent_id: String,
}

/// Returns the agent's stable UUID, generating and persisting one on first
/// call if none exists yet. Falls back to a fresh, unpersisted UUID if the
/// app data directory can't be read or written — the server should still
/// start, just without a stable id across restarts.
pub fn load_or_create() -> String {
    let Ok(dir) = app_data_dir() else {
        return Uuid::new_v4().to_string();
    };
    let path = dir.join(IDENTITY_FILE);

    if let Ok(contents) = fs::read_to_string(&path) {
        if let Ok(identity) = serde_json::from_str::<Identity>(&contents) {
            return identity.agent_id;
        }
    }

    let agent_id = Uuid::new_v4().to_string();
    let identity = Identity {
        agent_id: agent_id.clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&identity) {
        let _ = fs::write(&path, json);
    }
    agent_id
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
            let dir = std::env::temp_dir().join(format!("espia-identity-test-{}-{n}", std::process::id()));
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
    fn generates_and_persists_an_id_on_first_call() {
        let _home = TestHome::new();
        let first = load_or_create();
        assert!(Uuid::parse_str(&first).is_ok());
        let second = load_or_create();
        assert_eq!(first, second, "a second call must reuse the persisted id");
    }
}
