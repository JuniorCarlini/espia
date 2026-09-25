# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Repository structure, contribution guidelines, and project documentation.
- Architecture overview and initial Architecture Decision Records.
- Draft of protocol v1 (discovery, pairing, and metrics messages).
- Draft of the `claude` provider with subscription plan limits.
- Firmware skeleton with multi-board support and a boot screen for the
  Heltec WiFi Kit 32 (V2 and V3).
- Agent scaffold (Tauri 2, macOS) with a live CPU, memory, and CPU
  temperature preview, styled with Tailwind CSS and the espia brand mark.
- A "Top Processes" table in the agent, showing the 10 processes using the
  most CPU.
- CPU core count, and disk and network usage, in the agent.
- Header badges for total memory and total disk capacity, matching the
  core-count badge on the CPU card.
- A usage history graph on the Memory card, matching the CPU card.
- Claude plan limits (ADR 0005): a `claude-statusline` bridge subcommand
  the agent can install as the user's Claude Code status line, with a
  Connect/Disconnect flow in the settings UI. The bridge always chains to
  whatever status line command was configured before it, so nothing already
  in place stops working.
- ADR 0007: a device may pair with more than one agent, for monitoring
  several computers from one display.
- Fixed the app icon rendering larger and visually different than other
  macOS icons in the Dock and Cmd+Tab switcher. It now compiles as a real
  macOS 26 Liquid Glass icon (ADR 0008) via a hand-authored Icon Composer
  `.icon` bundle compiled with `actool` — not just a corrected flat PNG.
- An ambient (outdoor) weather card: current temperature for the city the
  agent's IP resolves to, via the keyless Open-Meteo and ipwho.is APIs —
  the only thing in the agent that reaches the internet.
- Removed the "Device" placeholder card; the header's "No device paired"
  pill covers that now, leaving a clean 2×3 card grid above Top Processes.
- Fixed a bug (ADR 0009) where a production build (`tauri build`) opened to
  a completely blank window — absolute asset paths in `index.html` resolve
  incorrectly under Tauri's bundled asset protocol, even though the
  identical code works under `tauri dev`.
- Styled the Ambient Weather card to match the rest of the dashboard: an
  icon and a color-coded status word (Cold/Pleasant/Hot), not plain text.
- A countdown to when each Claude plan-limit window resets ("resets in
  3h 19m"), not just the percentage used.
- A settings screen (ADR 0010): English/Português/Español, applied across
  the whole UI immediately, and a manual weather-location override
  (geocoded via Open-Meteo) for when the automatic IP-based guess is wrong.

### Changed

- Lowered CPU/memory/disk/network/temperature polling from every 1s to
  every 2s, and moved the process list to its own slower refresh (ADR
  0011) — measured ~46% less agent CPU use (5.15% → 2.78% average) for a
  1s staleness trade-off that isn't perceptible for these numbers.

- Moved Claude Connect/Disconnect out of the dashboard card and into
  Settings, which now also uses [Tucano](https://juniorcarlini.github.io/tucano/)
  (ADR 0012) for the dialog, the language picker (a themed in-page dropdown
  instead of a native OS popup), and buttons — vendored locally, no CDN.

- The Claude connection status badge is now green when connected and red
  when not, and disconnecting asks for confirmation first (a Tucano
  confirm dialog) instead of disconnecting on the first click.

- The settings location field now shows live place-name suggestions as you
  type, resolved directly by coordinates when picked. Fixed a real bug
  where searching the exact formatted "City, Region, Country" string a
  previous save had shown (for example by copying it back in) returned no
  matches at all — the geocoder only matches on a place's own name.

- The Ambient Weather card's icon now shows a moon instead of a sun at
  night, per the displayed location's own sunrise/sunset (Open-Meteo's
  `is_day`), not this machine's local clock.

- Split the agent's Rust code into a shared `espia-core` library crate and
  two binaries — the existing Tauri desktop GUI, and a new headless HTTP
  server (`espia-headless`) exposing the same metrics as a token-gated
  `GET /metrics` endpoint, for running the agent on a VPS or Docker host
  with no display. See ADR 0013.

- The headless agent now has a `GET /containers` endpoint showing
  per-container CPU and memory usage (sorted highest first) for the
  Docker host it's running on, matching `docker stats`. Opt-in: it needs
  either the real `/var/run/docker.sock` mounted in (root-equivalent
  control of the whole host) or, recommended, a proxy in front of it —
  see ADR 0014.

- Added `espia-socket-guard`: a new, minimal binary that's the only thing
  allowed to touch the real Docker socket, forwarding exactly the two
  read-only calls `GET /containers` needs and refusing everything else
  (`create`, `stop`, `exec`, any other API path) with a `403` before it
  reaches the socket — an in-repo alternative to a third-party proxy
  image, for anyone who'd rather not trust one for this. See ADR 0014.
  A `docker-compose.yml` deploys it together with the headless agent as
  one stack, so using it is one step instead of two.

- Fixed `GET /containers` scaling linearly with container count (~24s for
  12 containers, one at a time) by fetching every container's stats in
  parallel instead — ~2s for the same 12, on both the headless agent and
  `espia-socket-guard`.

- Hardened the headless agent's token check to a constant-time
  comparison, closing a theoretical timing side channel.

- Renamed the project from ESPHub to **espia**.
