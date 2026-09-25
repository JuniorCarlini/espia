//! Ambient (outdoor) temperature for the city the agent is running in.
//!
//! This is the one thing in the agent that leaves the local machine and the
//! local network: it calls two public, keyless HTTPS APIs —
//! [ipwho.is](https://ipwho.is) to turn the machine's public IP into an
//! approximate city, and [Open-Meteo](https://open-meteo.com) for that
//! city's current weather. Open-Meteo blends several national weather
//! services rather than running its own models, which is why it was picked
//! over a single-source API for "the most reliable public one".
//!
//! Both calls are best-effort: no API key, no account, and a failure (for
//! example, no internet connection) just means the card shows nothing
//! rather than an error — see [`WeatherCollector::refresh_if_stale`].

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::settings::LocationOverride;

const IP_LOCATION_URL: &str = "https://ipwho.is/";
const GEOCODING_URL: &str = "https://geocoding-api.open-meteo.com/v1/search";
const WEATHER_URL: &str = "https://api.open-meteo.com/v1/forecast";
/// Weather doesn't change fast enough to justify calling out to these APIs
/// on every metrics tick; re-fetch at most this often.
const REFRESH_INTERVAL: Duration = Duration::from_secs(15 * 60);

#[derive(Serialize, Clone, Debug)]
pub struct AmbientWeather {
    pub city: String,
    pub temp_c: f64,
    pub feels_like_c: f64,
    pub humidity_pct: f64,
    /// Whether it's currently daytime at `city`'s location — Open-Meteo
    /// computes this itself from that location's own sunrise/sunset, not
    /// from wherever this agent happens to be running, so a display icon
    /// (sun vs. moon) driven by this is right even for a remote location.
    pub is_daytime: bool,
    /// Unix epoch milliseconds when this was fetched.
    pub updated_at: i64,
}

#[derive(Deserialize)]
struct IpLocationResponse {
    success: bool,
    city: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
}

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
}

#[derive(Deserialize)]
struct OpenMeteoCurrent {
    temperature_2m: f64,
    apparent_temperature: f64,
    relative_humidity_2m: f64,
    /// `1` by day, `0` by night — Open-Meteo's own reading of it, already
    /// computed against this location's sunrise/sunset, timezone, and time
    /// of year. No separate sunrise/sunset lookup or local math needed.
    is_day: u8,
}

struct Location {
    city: String,
    latitude: f64,
    longitude: f64,
}

impl From<&LocationOverride> for Location {
    fn from(o: &LocationOverride) -> Self {
        Self {
            city: o.city.clone(),
            latitude: o.latitude,
            longitude: o.longitude,
        }
    }
}

#[derive(Deserialize)]
struct GeocodingResponse {
    #[serde(default)]
    results: Vec<GeocodingResult>,
}

#[derive(Deserialize)]
struct GeocodingResult {
    name: String,
    latitude: f64,
    longitude: f64,
    #[serde(default)]
    admin1: Option<String>,
    #[serde(default)]
    country: Option<String>,
}

fn geocoding_request(query: &str, count: u8) -> Vec<GeocodingResult> {
    let Ok(response) = ureq::get(GEOCODING_URL)
        .query("name", query)
        .query("count", &count.to_string())
        .timeout(Duration::from_secs(5))
        .call()
    else {
        return Vec::new();
    };
    let parsed: Result<GeocodingResponse, _> = response.into_json();
    parsed.map(|r| r.results).unwrap_or_default()
}

fn as_location_override(result: GeocodingResult) -> LocationOverride {
    let city = match (result.admin1, result.country) {
        (Some(admin1), Some(country)) => format!("{}, {}, {}", result.name, admin1, country),
        (Some(region), None) | (None, Some(region)) => format!("{}, {}", result.name, region),
        (None, None) => result.name,
    };
    LocationOverride {
        city,
        latitude: result.latitude,
        longitude: result.longitude,
    }
}

/// Resolves a place name (city, "City, Country", postal code, ...) the user
/// typed into the settings UI to coordinates, via Open-Meteo's own geocoder
/// — the same provider as the weather itself, so one fewer service to
/// trust. Returns `None` on no match or any failure.
///
/// Open-Meteo's geocoder matches on a place's own name, not on a fully
/// formatted "City, Region, Country" string — searching that (for example
/// pasting back what [`search`] or a previous save already resolved and
/// displayed) returns nothing. If the first word-for-word attempt comes up
/// empty and the query looks like one of those formatted strings, retry
/// with just the part before the first comma.
pub fn geocode(query: &str) -> Option<LocationOverride> {
    let mut results = geocoding_request(query, 1);
    if results.is_empty() {
        if let Some((city_only, _rest)) = query.split_once(',') {
            results = geocoding_request(city_only.trim(), 1);
        }
    }
    results.into_iter().next().map(as_location_override)
}

/// Returns up to `limit` place-name candidates matching `query`, for live
/// suggestions in the settings UI as the user types. Each candidate carries
/// fully resolved coordinates, so picking one never needs a second lookup —
/// unlike [`geocode`], which only wants a single best-effort match. Empty
/// (never an error) on no match or any failure — the caller just shows no
/// suggestions.
pub fn search(query: &str, limit: u8) -> Vec<LocationOverride> {
    geocoding_request(query, limit).into_iter().map(as_location_override).collect()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Approximates the machine's city from its public IP address. Returns
/// `None` on any failure (offline, API down, unexpected response) rather
/// than erroring — there's no local fallback for "what city is this".
fn locate() -> Option<Location> {
    let response: IpLocationResponse = ureq::get(IP_LOCATION_URL)
        .timeout(Duration::from_secs(5))
        .call()
        .ok()?
        .into_json()
        .ok()?;
    if !response.success {
        return None;
    }
    Some(Location {
        city: response.city?,
        latitude: response.latitude?,
        longitude: response.longitude?,
    })
}

fn fetch_weather(location: &Location) -> Option<AmbientWeather> {
    let response: OpenMeteoResponse = ureq::get(WEATHER_URL)
        .query("latitude", &location.latitude.to_string())
        .query("longitude", &location.longitude.to_string())
        .query(
            "current",
            "temperature_2m,relative_humidity_2m,apparent_temperature,is_day",
        )
        .query("timezone", "auto")
        .timeout(Duration::from_secs(5))
        .call()
        .ok()?
        .into_json()
        .ok()?;

    Some(AmbientWeather {
        city: location.city.clone(),
        temp_c: response.current.temperature_2m,
        feels_like_c: response.current.apparent_temperature,
        humidity_pct: response.current.relative_humidity_2m,
        is_daytime: response.current.is_day != 0,
        updated_at: now_ms(),
    })
}

/// Caches the resolved location for the agent's whole lifetime (a
/// computer's city doesn't change mid-session) and the last successful
/// weather reading, re-fetched at most every [`REFRESH_INTERVAL`].
pub struct WeatherCollector {
    /// `None` until a lookup succeeds. Retried on every stale refresh (see
    /// `refresh_if_stale`) rather than just once, so a machine that's
    /// offline when the agent starts still picks up a location once it
    /// reconnects — that retry is already rate-limited to
    /// `REFRESH_INTERVAL` by the caller not reaching this point otherwise.
    /// Ignored once `override_location` is set.
    location: Option<Location>,
    /// Set from `settings::LocationOverride` when the user picks a city in
    /// the settings UI instead of relying on IP geolocation. Takes priority
    /// over `location` whenever present.
    override_location: Option<Location>,
    last: Option<AmbientWeather>,
    last_attempt: Option<Instant>,
}

impl WeatherCollector {
    pub fn new() -> Self {
        Self {
            location: None,
            override_location: None,
            last: None,
            last_attempt: None,
        }
    }

    /// Applies (or clears, if `None`) a manual location override. Clears
    /// the cached reading too, so the next `refresh_if_stale` call fetches
    /// immediately under the new location rather than waiting out
    /// `REFRESH_INTERVAL` under the old one.
    pub fn set_override(&mut self, location: Option<&LocationOverride>) {
        self.override_location = location.map(Location::from);
        self.last = None;
        self.last_attempt = None;
    }

    /// Returns the last known reading, fetching a new one first if it's
    /// stale (or there isn't one yet). Called from a Tauri command, so a
    /// slow network call here delays that call's response, not the rest of
    /// the UI.
    pub fn refresh_if_stale(&mut self) -> Option<AmbientWeather> {
        let is_stale = self.last_attempt.map(|t| t.elapsed() >= REFRESH_INTERVAL).unwrap_or(true);
        if !is_stale {
            return self.last.clone();
        }
        self.last_attempt = Some(Instant::now());

        let location = if let Some(location) = &self.override_location {
            location
        } else {
            if self.location.is_none() {
                self.location = locate();
            }
            let Some(location) = &self.location else {
                return None;
            };
            location
        };

        if let Some(weather) = fetch_weather(location) {
            self.last = Some(weather);
        }
        self.last.clone()
    }
}

impl Default for WeatherCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hits the real ipwho.is and Open-Meteo APIs — no key needed, but it
    /// Also real network, also `#[ignore]`d.
    #[test]
    #[ignore]
    fn geocode_resolves_a_known_city() {
        let result = geocode("Paris, France").expect("Paris should resolve to something");
        assert!(result.city.to_lowercase().contains("paris"), "got city: {}", result.city);
        // Real-world Paris coordinates, with slack for which "Paris" a
        // same-named-city match might prefer.
        assert!((48.0..=49.0).contains(&result.latitude), "implausible latitude: {}", result.latitude);
        assert!((2.0..=3.0).contains(&result.longitude), "implausible longitude: {}", result.longitude);
    }

    /// Regression test: Open-Meteo's geocoder matches on a place's own
    /// name, not on the fully formatted "City, Region, Country" string
    /// `geocode` itself returns — searching that literally (what happens if
    /// a user copies what's already saved and searches it again) used to
    /// come back empty. `geocode` must fall back to just the part before
    /// the first comma in that case.
    #[test]
    #[ignore]
    fn geocode_falls_back_to_the_city_name_when_the_formatted_string_has_no_matches() {
        let formatted = geocode("Paris, France").expect("Paris should resolve to something");
        let city_only = formatted.city.split_once(',').map(|(city, _)| city).unwrap_or(&formatted.city);
        assert_ne!(formatted.city, city_only, "the fixture should actually be a formatted string");

        let result = geocode(&formatted.city).expect("must still resolve from its own formatted city string");
        assert!(result.city.to_lowercase().contains(&city_only.to_lowercase()), "got city: {}", result.city);
    }

    #[test]
    #[ignore]
    fn search_returns_multiple_candidates_with_resolved_coordinates() {
        let results = search("São Miguel", 5);
        assert!(!results.is_empty(), "expected at least one match for São Miguel");
        for candidate in &results {
            assert!(!candidate.city.is_empty());
        }
    }

    #[test]
    #[ignore]
    fn override_location_is_used_instead_of_ip_geolocation() {
        let mut collector = WeatherCollector::new();
        let paris = geocode("Paris, France").expect("Paris should resolve");
        collector.set_override(Some(&paris));

        let weather = collector.refresh_if_stale().expect("expected a reading for the override");
        assert_eq!(weather.city, paris.city);
    }

    /// needs network, so it's `#[ignore]`d by default: `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn refresh_if_stale_gets_a_real_reading() {
        let mut collector = WeatherCollector::new();
        let weather = collector.refresh_if_stale().expect(
            "expected a reading — this test needs network and both APIs reachable",
        );

        assert!(!weather.city.is_empty());
        // Nowhere on Earth is outside this range; this just catches
        // obviously wrong parsing (e.g. reading the wrong JSON field).
        assert!((-90.0..=60.0).contains(&weather.temp_c), "implausible temp_c: {}", weather.temp_c);
        assert!((0.0..=100.0).contains(&weather.humidity_pct), "implausible humidity_pct: {}", weather.humidity_pct);

        // A second call within REFRESH_INTERVAL must be served from cache,
        // not issue another round of HTTP calls.
        let cached = collector.refresh_if_stale().expect("cached reading");
        assert_eq!(cached.updated_at, weather.updated_at);
    }
}
