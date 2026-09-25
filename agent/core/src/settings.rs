//! User-facing agent settings: display language and a manual location
//! override for the [ambient weather provider](providers::weather).
//!
//! Persisted as a small JSON file in the agent's data directory — see
//! `providers::claude` for the same pattern (and why it's *that* directory,
//! not `~/.claude`, which is a different concern entirely).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const SETTINGS_FILE: &str = "settings.json";
const APP_DATA_DIR_NAME: &str = "com.espia.agent";

/// Languages the settings UI can switch to. The `code` is what's persisted
/// and sent to the frontend; the frontend owns the actual translated
/// strings — this side only validates and stores the choice.
pub const SUPPORTED_LANGUAGES: [&str; 3] = ["en", "pt", "es"];
const DEFAULT_LANGUAGE: &str = "en";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LocationOverride {
    pub city: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_override: Option<LocationOverride>,
    /// The friendly name devices show for this agent (protocol `hello`'s
    /// `name`, and mDNS's TXT `name`). `None` means "use the OS hostname",
    /// computed live by [`agent_name`] rather than stored — only an
    /// explicit [`set_agent_name`] call persists an override, same as every
    /// other field here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: DEFAULT_LANGUAGE.to_string(),
            location_override: None,
            name: None,
        }
    }
}

/// The name to advertise for this agent: the user's override if they set
/// one, otherwise the OS hostname, otherwise a fixed fallback (a machine
/// with neither is rare, but a device still needs something to show).
pub fn agent_name() -> String {
    load().name.unwrap_or_else(|| {
        sysinfo::System::host_name().unwrap_or_else(|| "espia agent".to_string())
    })
}

/// Sets (or, with `None`, clears) the agent's friendly name override.
pub fn set_agent_name(name: Option<String>) -> Result<Settings, String> {
    let mut settings = load();
    settings.name = name.filter(|n| !n.trim().is_empty());
    save(&settings)?;
    Ok(settings)
}

pub(crate) fn app_data_dir() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .ok_or("Could not determine this OS's application data directory")?
        .join(APP_DATA_DIR_NAME);
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    Ok(dir)
}

fn settings_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join(SETTINGS_FILE))
}

/// Reads the persisted settings, or the defaults if none have been saved
/// yet (or the file can't be read — a corrupt settings file shouldn't be
/// able to make the rest of the app unusable).
pub fn load() -> Settings {
    let Ok(path) = settings_path() else {
        return Settings::default();
    };
    let Ok(contents) = fs::read_to_string(path) else {
        return Settings::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

fn save(settings: &Settings) -> Result<(), String> {
    let path = settings_path()?;
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| format!("Could not write {}: {e}", path.display()))
}

pub fn set_language(language: &str) -> Result<Settings, String> {
    if !SUPPORTED_LANGUAGES.contains(&language) {
        return Err(format!(
            "Unsupported language {language:?}; expected one of {SUPPORTED_LANGUAGES:?}"
        ));
    }
    let mut settings = load();
    settings.language = language.to_string();
    save(&settings)?;
    Ok(settings)
}

pub fn set_location_override(location: Option<LocationOverride>) -> Result<Settings, String> {
    let mut settings = load();
    settings.location_override = location;
    save(&settings)?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::HOME_ENV_LOCK;
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
            let dir = std::env::temp_dir().join(format!("espia-settings-test-{}-{n}", std::process::id()));
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
    fn defaults_to_english_with_no_location_override() {
        let _home = TestHome::new();
        let settings = load();
        assert_eq!(settings.language, "en");
        assert!(settings.location_override.is_none());
    }

    #[test]
    fn set_language_persists_and_rejects_unsupported_codes() {
        let _home = TestHome::new();

        let settings = set_language("pt").expect("pt is supported");
        assert_eq!(settings.language, "pt");
        assert_eq!(load().language, "pt", "must survive a fresh load()");

        let error = set_language("fr").expect_err("fr is not supported");
        assert!(error.contains("fr"));
        assert_eq!(load().language, "pt", "a rejected language must not overwrite the saved one");
    }

    #[test]
    fn set_location_override_persists_and_clears() {
        let _home = TestHome::new();
        let sao_paulo = LocationOverride {
            city: "São Paulo, Brazil".to_string(),
            latitude: -23.5505,
            longitude: -46.6333,
        };

        let settings = set_location_override(Some(sao_paulo.clone())).expect("should save");
        assert_eq!(settings.location_override.unwrap().city, sao_paulo.city);
        assert_eq!(load().location_override.unwrap().city, sao_paulo.city);

        let cleared = set_location_override(None).expect("should clear");
        assert!(cleared.location_override.is_none());
        assert!(load().location_override.is_none());
    }

    #[test]
    fn a_corrupt_settings_file_falls_back_to_defaults_instead_of_breaking() {
        let home = TestHome::new();
        let dir = home.dir.join("Library/Application Support").join(APP_DATA_DIR_NAME);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(SETTINGS_FILE), "{not valid json").unwrap();

        let settings = load();
        assert_eq!(settings.language, "en");
    }
}
