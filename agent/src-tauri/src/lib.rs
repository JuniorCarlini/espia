//! espia agent — Tauri backend.
//!
//! Collects local metrics and, eventually, streams them to devices over the
//! network. See `docs/architecture.md` at the repository root for the full
//! design; this crate currently implements only local collection, exposed to
//! the settings UI for preview.

use std::sync::Mutex;

use espia_core::collectors;
use espia_core::collectors::system::SystemCollector;
use espia_core::providers;
use espia_core::providers::claude::ClaudeStatusLineStatus;
use espia_core::providers::weather::{AmbientWeather, WeatherCollector};
use espia_core::settings;
use espia_core::settings::{LocationOverride, Settings};
use tauri::State;

struct AppState {
    system: Mutex<SystemCollector>,
    weather: Mutex<WeatherCollector>,
}

/// Returns the latest system metrics snapshot for the settings UI.
#[tauri::command]
fn get_system_metrics(state: State<AppState>) -> collectors::system::SystemMetrics {
    state.system.lock().expect("system collector lock poisoned").refresh()
}

const TOP_PROCESSES_LIMIT: usize = 10;

/// Returns the processes using the most CPU right now, for the settings
/// UI's process table. Refreshes its own process list on a slower, internal
/// cadence — see `SystemCollector::top_processes` — so this doesn't need
/// `get_system_metrics` called first, unlike before.
#[tauri::command]
fn get_top_processes(state: State<AppState>) -> Vec<collectors::processes::ProcessUsage> {
    state
        .system
        .lock()
        .expect("system collector lock poisoned")
        .top_processes(TOP_PROCESSES_LIMIT)
}

/// Returns whether the agent's Claude Code status line bridge is connected,
/// and the last plan limits it collected, if any.
#[tauri::command]
fn get_claude_statusline_status() -> ClaudeStatusLineStatus {
    providers::claude::status()
}

/// Points `~/.claude/settings.json`'s status line at this agent, backing up
/// whatever was configured before. See `providers::claude` for the design.
#[tauri::command]
fn connect_claude_statusline() -> Result<ClaudeStatusLineStatus, String> {
    providers::claude::connect()
}

/// Restores the status line `connect_claude_statusline` backed up.
#[tauri::command]
fn disconnect_claude_statusline() -> Result<ClaudeStatusLineStatus, String> {
    providers::claude::disconnect()
}

/// Returns the last known ambient (outdoor) weather reading for the city the
/// agent's public IP resolves to, fetching a fresh one if the cached reading
/// is stale. `None` if it's never succeeded (for example, no internet).
#[tauri::command]
fn get_ambient_weather(state: State<AppState>) -> Option<AmbientWeather> {
    state
        .weather
        .lock()
        .expect("weather collector lock poisoned")
        .refresh_if_stale()
}

/// Returns the persisted settings (display language, weather location
/// override).
#[tauri::command]
fn get_settings() -> Settings {
    settings::load()
}

/// Sets the settings UI's display language. `language` must be one of
/// `settings::SUPPORTED_LANGUAGES`.
#[tauri::command]
fn set_language(language: String) -> Result<Settings, String> {
    settings::set_language(&language)
}

/// Clears a manual weather location override, reverting to automatic
/// IP-based geolocation. Setting one instead is [`select_weather_location`]
/// — there's no "set by raw string" command because it isn't reliable; see
/// its doc comment.
#[tauri::command]
fn clear_location_override(state: State<AppState>) -> Result<Settings, String> {
    let updated = settings::set_location_override(None)?;
    state.weather.lock().expect("weather collector lock poisoned").set_override(None);
    Ok(updated)
}

/// A place name is only searched once it's at least this long — shorter
/// queries return too many unrelated matches to be a useful suggestion
/// list, and just add extra round-trips while the user is still typing.
const MIN_LOCATION_SEARCH_LEN: usize = 2;
const LOCATION_SEARCH_LIMIT: u8 = 5;

/// Returns up to a handful of place-name candidates matching `query`, for
/// the settings UI's live location suggestions. Each candidate already
/// carries resolved coordinates — see [`select_weather_location`], which
/// saves one directly rather than re-searching it.
#[tauri::command]
fn search_weather_locations(query: String) -> Vec<LocationOverride> {
    let query = query.trim();
    if query.len() < MIN_LOCATION_SEARCH_LEN {
        return Vec::new();
    }
    providers::weather::search(query, LOCATION_SEARCH_LIMIT)
}

/// Saves a location the user picked from [`search_weather_locations`]'s
/// suggestions directly — no re-geocoding, since its coordinates are
/// already resolved. Re-searching an already formatted "City, Region,
/// Country" string (what `set_location_override` would otherwise have to
/// do) isn't reliable — see `providers::weather::geocode`'s doc comment.
#[tauri::command]
fn select_weather_location(state: State<AppState>, location: LocationOverride) -> Result<Settings, String> {
    let updated = settings::set_location_override(Some(location.clone()))?;
    state
        .weather
        .lock()
        .expect("weather collector lock poisoned")
        .set_override(Some(&location));
    Ok(updated)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut weather = WeatherCollector::new();
    weather.set_override(settings::load().location_override.as_ref());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            system: Mutex::new(SystemCollector::new()),
            weather: Mutex::new(weather),
        })
        .invoke_handler(tauri::generate_handler![
            get_system_metrics,
            get_top_processes,
            get_claude_statusline_status,
            connect_claude_statusline,
            disconnect_claude_statusline,
            get_ambient_weather,
            get_settings,
            set_language,
            clear_location_override,
            search_weather_locations,
            select_weather_location
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
