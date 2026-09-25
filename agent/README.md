# espia Agent

The desktop agent collects system metrics and AI usage data and streams them
to espia devices on the local network.

- **Platforms:** macOS first; Windows and Linux planned
- **Stack:** [Tauri 2](https://tauri.app/) — Rust backend, plain JS frontend
  styled with [Tailwind CSS](https://tailwindcss.com/), no JS framework
  ([ADR 0004](../docs/adr/0004-desktop-agent-stack.md))
- **Status:** Early development — reads live CPU, memory, and CPU
  temperature and shows them in the settings window. Networking is not
  implemented yet; the "Device", "Claude Usage", and "Disk & Network" cards
  are shown as planned, not functional.

## Getting started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Node.js](https://nodejs.org/) 18+
- Platform build tools for Tauri — see the
  [Tauri prerequisites guide](https://tauri.app/start/prerequisites/)
  (on macOS: Xcode Command Line Tools)

### Run in development

```sh
npm install
npm run tauri dev
```

This runs `npm run prep` (vendors Tucano and compiles the CSS, once) and
opens the settings window. Rust changes are watched and trigger an
automatic rebuild and restart; **frontend files (`src/*.html`, `.js`) are
not hot-reloaded** — there is no dev server in front of them, so reload the
window (Cmd+R) or restart `tauri dev` to see changes. To iterate on styles,
run `npm run dev:css` in a second terminal to recompile `src/styles.css` on
every save, then reload the window.

`npm run tauri build` produces a release binary and installer for the
current platform.

### Running headless (VPS / Docker)

See the [headless agent README](headless/README.md) for `espia-headless`
— the same collectors, exposed as a token-gated `GET /metrics` HTTP
endpoint instead of a desktop window ([ADR 0013](../docs/adr/0013-headless-agent.md)).

## Project layout

| Path                        | Description                                             |
| ---------------------------- | -------------------------------------------------------- |
| `src/`                       | Settings UI: `index.html`, `main.js`, `input.css` (Tailwind source, compiles to the git-ignored `styles.css`) |
| `src/i18n.js`                | Translation strings (English/Português/Español) and the `t()`/`applyTranslations()` helpers |
| `src/vendor/tucano/`          | [Tucano](https://juniorcarlini.github.io/tucano/) (ADR 0012), copied in by `npm run vendor` — gitignored, don't edit |
| `src/assets/`                | Brand assets (wordmark, icons)                          |
| `core/src/collectors/`       | System, process, and GPU metric collectors               |
| `core/src/providers/`        | AI usage and external-data providers (`claude`, `weather`) |
| `core/src/settings.rs`       | Persisted agent settings (language, weather location override) |
| `src-tauri/src/`             | The Tauri desktop GUI binary — Tauri commands wrapping `espia-core`, window setup |
| `src-tauri/src/server/`      | WebSocket server, mDNS advertisement, and UDP discovery — device pairing isn't implemented yet |
| `src-tauri/tauri.conf.json`  | App configuration (window, bundling, identifier)         |
| `headless/src/main.rs`       | The headless HTTP server binary ([ADR 0013](../docs/adr/0013-headless-agent.md)) |
| `socket-guard/src/main.rs`   | Minimal read-only Docker socket proxy for `headless`'s `GET /containers` ([ADR 0014](../docs/adr/0014-docker-container-stats.md)) |

`core`, `src-tauri`, and `headless` are members of one Cargo workspace
(`agent/Cargo.toml`) — `espia-core` has no Tauri dependency, so both
binaries depend on it rather than on each other. `socket-guard` is
deliberately its own standalone project, not a member of that workspace
— see its own README for why.

See the [architecture overview](../docs/architecture.md) for how these pieces
fit together, and the [protocol specification](../protocol/README.md) for
the wire format collectors and providers feed into.

## Adding a collector

A collector reads one category of local data and exposes a
`serde`-serializable struct whose field names match the corresponding
section of the [`metrics`](../protocol/README.md#51-metrics-agent--device)
message, so its output can be forwarded without translation. See
`core/src/collectors/system.rs` for an example. Adding it to `core` (not
`src-tauri`) makes it available to the headless server too.

## Adding a provider

See [`protocol/providers/`](../protocol/providers/) to document the
provider's data shape before implementing it, following
[CONTRIBUTING.md](../CONTRIBUTING.md).

## Roadmap

- [x] Read local CPU (with core count), memory, and CPU temperature usage
- [x] Top-processes table (by CPU)
- [x] Disk and network collectors
- [ ] GPU collector
- [x] Claude provider: plan limits via the Claude Code status line
      ([ADR 0005](../docs/adr/0005-claude-plan-limits-source.md)), with a
      Connect/Disconnect flow in the settings UI
- [x] Ambient (outdoor) weather via Open-Meteo and IP geolocation
- [x] Settings screen: language (English/Português/Español) and a manual
      weather location override ([ADR 0010](../docs/adr/0010-settings-i18n-and-location-override.md)),
      built with [Tucano](https://juniorcarlini.github.io/tucano/) ([ADR 0012](../docs/adr/0012-tucano.md))
- [ ] WebSocket server and mDNS / UDP discovery
- [ ] Device pairing
- [ ] Tray icon and launch at login
