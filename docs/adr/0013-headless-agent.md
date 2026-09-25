# 0013. A headless HTTP agent for VPS and Docker hosts

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

The agent only ran as a Tauri desktop GUI, but the same local metrics are
just as useful on a machine with no display — a VPS or a Docker host —
where the point isn't a window, it's exposing the numbers to whatever
polls them next (eventually an espia device, today `curl` or a script).

Three shapes were on the table: bolt this into the existing Tauri binary
behind a flag, implement the full device protocol (WebSocket +
mDNS/UDP discovery — see [0002](0002-device-discovery.md) and
[0003](0003-transport-and-message-format.md)) for a headless target too, or
a minimal, separate HTTP endpoint. The explicit ask was for something
"simple and secure — nothing fancy": easy to run in a container today,
without pulling in the full pairing/discovery protocol before there's a
device on the other end of it yet.

## Decision

- **Split the Rust code into a Cargo workspace**: `core` (a plain library
  crate, `espia-core`, with no Tauri dependency — the collectors,
  providers, and settings module) shared by two binaries — the existing
  Tauri desktop GUI (`src-tauri`) and a new headless server (`headless`).
  Neither binary owns logic the other needs; both depend on `core`.
- **`espia-headless` is a single-threaded, synchronous HTTP server** using
  `tiny_http` — no async runtime, no TLS. It exposes `GET /metrics` (the
  same JSON shape the GUI polls: CPU, memory, disk, network, and the top
  processes) and an unauthenticated `GET /health` for container health
  checks.
- **A required bearer token, via the `ESPIA_TOKEN` env var, gates
  `/metrics`.** If it's unset, the server still starts (so a misconfigured
  container is visible in its logs, not just refusing to boot) but *every*
  request to `/metrics` is rejected — it fails closed, never open. The
  token is compared as a plain byte string; no login flow, no OAuth, no
  per-client accounts, matching "nothing fancy."
- **Binds all interfaces (`0.0.0.0`) by default**, since it must for a
  Docker `-p` port mapping to reach it at all — a container's own network
  namespace makes "just bind localhost" meaningless here. Real network
  protection (a firewall, or a reverse proxy in front of it) is the
  deploying host's job, not this binary's, and the README says so.

## Consequences

- Running the agent on a VPS or inside Docker is now `docker run -e
  ESPIA_TOKEN=... -p 8080:8080 ...`, no display or pairing step required.
- The token is the *only* access control. It is not a substitute for TLS —
  it's sent in plaintext unless something in front of this (a reverse
  proxy) terminates HTTPS. That's an explicit, documented trade-off in
  return for staying "nothing fancy."
- `espia-core` becomes a second thing to keep GUI-agnostic going forward —
  any new collector or provider belongs there, not in `src-tauri`, or it
  won't be usable from `headless` either.
- The headless server doesn't speak the device protocol
  ([0002](0002-device-discovery.md)/[0003](0003-transport-and-message-format.md))
  yet — an ESP32 display can't discover or pair with it today. It's a
  `curl`-able endpoint first; protocol support is a later decision, not
  this one.

## Alternatives considered

- **A `--headless` flag on the existing Tauri binary** — avoids a
  workspace split, but still links the whole Tauri/WebView dependency tree
  into a container image that will never open a window, and running a GUI
  toolkit's binary as a network service is an odd shape to reach for.
- **Implement the full WebSocket + mDNS/UDP device protocol for a headless
  target** — the "correct" long-term shape once a device needs to pair
  with a VPS-hosted agent, but a heavier first step than the ask called
  for, and mDNS in particular assumes a LAN a VPS usually isn't on.
- **A log-only proof of concept** (print metrics to stdout, no server at
  all) — simplest possible, but not "install it and `curl` it," which was
  the actual request.
