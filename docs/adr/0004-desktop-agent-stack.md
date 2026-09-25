# 0004. Desktop agent technology stack

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

The agent must run on Windows, macOS, and Linux, stay running in the
background (system tray, launch at login), and provide a settings UI. The UI
should be built with web technologies (HTML, CSS, JavaScript). Because the
agent is always running, memory footprint and binary size matter.

## Options

### Tauri (Rust backend + web UI)

- Small binaries (~10 MB) and low memory use, since it uses the operating
  system's native webview.
- Built-in support for tray icons, autostart, installers, and auto-updates.
- The Rust `sysinfo` crate covers CPU, memory, disk, and network on all three
  platforms. Mature crates exist for mDNS and WebSocket servers.
- Requires contributors to know some Rust.
- Rendering can vary slightly between webviews (WebView2, WebKit,
  WebKitGTK).

### Electron (Node.js backend + web UI)

- Everything in JavaScript/TypeScript; lowest barrier to contribution.
- The `systeminformation` npm package is comprehensive.
- Consistent rendering, since Chromium is bundled.
- Large binaries (~100 MB) and higher memory use, which is a poor fit for an
  always-on background app.

## Decision

Build the agent with **Tauri 2**: a Rust backend for collectors, providers,
and networking, and a plain HTML, CSS, and JavaScript frontend for the
settings UI.

Development targets **macOS first**. Windows and Linux follow once the macOS
agent works end to end. Code must stay portable from the start: platform
specific code lives behind small, clearly separated modules.

## Consequences

- The agent stays small enough to run in the background all the time.
- Contributors need some Rust for backend work; frontend work stays in plain
  web technologies.
- Tray icon, launch at login, installers, and auto-updates come from Tauri
  and its official plugins instead of custom code.
- Starting with macOS means platform gaps (for example, GPU metrics or
  Windows firewall prompts) are found later. Keeping platform code isolated
  limits the cost of adding the other platforms.
- The frontend uses no framework at first, which keeps it simple. A framework
  can be introduced later with a new ADR if the UI grows.
- Styling uses [Tailwind CSS](https://tailwindcss.com/) (v4, CLI-compiled,
  no JS framework involved) rather than hand-written CSS, since the settings
  UI is expected to grow more screens. `npm run build:css` compiles
  `src/input.css` to `src/styles.css`; Tauri runs it automatically via
  `beforeDevCommand` / `beforeBuildCommand`.
