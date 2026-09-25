# 0010. Agent settings: display language and a weather location override

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

Two related requests: let people use the agent in their own language
(English, Portuguese, and Spanish to start), and let them override the
weather card's IP-based location — useful when the IP-based guess is wrong,
or a machine is on a VPN, or the ambient temperature that matters is
somewhere other than where the computer physically sits.

Both are genuinely settings — persistent, user-chosen state, not something
re-derived from the OS or the network — so this is also the agent's first
general-purpose settings store, not just a place to bolt these two things
on.

## Decision

- **Backend:** `agent/src-tauri/src/settings.rs` owns a small
  `Settings { language, location_override }` struct, persisted as
  `settings.json` in the agent's data directory (same directory and pattern
  as `providers::claude`'s backup file). Three commands: `get_settings`,
  `set_language`, `set_location_override`.
- **Location override** reuses [Open-Meteo's geocoding API](https://open-meteo.com/en/docs/geocoding-api)
  (same provider as the weather itself) to turn a typed place name into
  coordinates — `providers::weather::geocode`. `WeatherCollector` gets a
  `set_override` method; when set, it skips the IP lookup entirely and the
  cached reading is invalidated so the change is reflected immediately
  instead of waiting out the usual refresh interval.
- **Frontend i18n** (`agent/src/i18n.js`) is intentionally simple: three flat
  string dictionaries (`en`, `pt`, `es`) with a `t(key, vars)` lookup and an
  `applyTranslations()` that walks `[data-i18n]` (textContent),
  `[data-i18n-html]` (innerHTML — only the couple of strings with an
  embedded link), `[data-i18n-title]`, and `[data-i18n-placeholder]`
  attributes in `index.html`. No framework, no build step, no `.po` files —
  consistent with [ADR 0004](0004-desktop-agent-stack.md)'s "no framework"
  call, and there's no reason to expect this project to need more locales
  than a person can hand-maintain as flat objects.
- Dynamically generated strings (status words, relative-time and countdown
  phrasing) call `t()` directly inside the functions that build them, rather
  than being pre-rendered once — they already re-render on every poll tick,
  so this stays in sync with the current language for free.
- A settings icon in the header opens a modal with the language picker and
  the location override form; changing the language re-applies translations
  and immediately re-polls every card so the switch is visible at once,
  rather than waiting for each card's own poll timer.

## Consequences

- Every future user-facing string has to be added to all three
  dictionaries — checked by the fact that they're flat objects a "keys
  match across languages" test could enforce later, though nothing enforces
  it automatically yet.
- The settings file uses the same app-data directory and read/write pattern
  established for Claude's pairing backup, so there's exactly one way
  small persistent state gets stored in this codebase, not two.
- Weather's location, previously always IP-derived, now has a second,
  higher-priority source. Both paths share one `Location` type internally
  (`providers::weather`), so the rest of the weather logic doesn't need to
  know which one is active.

## Alternatives considered

- **A full i18n library** (e.g. an ICU-based one) — overkill for three flat
  dictionaries and would be the first non-Rust-ecosystem, non-Tailwind
  frontend dependency in the project.
- **Manual lat/long entry** instead of geocoding a place name — more
  precise, but a worse experience for the common case of "I know the city,
  not its coordinates," and geocoding still allows both (Open-Meteo also
  resolves postal codes and "lat,long"-shaped queries reasonably).
