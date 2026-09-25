# 0015. Device provisioning: Wi-Fi setup and adding remote agents

- **Status:** Accepted
- **Date:** 2026-09-25

## Context

Two gaps exist between what's designed so far and what the device
actually needs to do the first time it's powered on:

1. **[ADR 0002](0002-device-discovery.md) assumes the device is already
   on a Wi-Fi network** with agents reachable on the same subnet via
   mDNS. Nothing yet says how the device gets onto that network in the
   first place — firmware networking isn't implemented yet, only a boot
   screen exists. The device has no keyboard, so typing an SSID and a
   Wi-Fi password character by character on it isn't acceptable, the same
   reasoning ADR 0002 already used for IP addresses.
2. **Not every agent is on the device's LAN.** [ADR 0013](0013-headless-agent.md)
   added a headless HTTP agent explicitly for VPS and Docker-host
   deployments, and said discovery/pairing support for it was "a later
   decision, not this one." That decision is now needed: the real
   motivating case is a desk display meant to show a couple of local
   machines *and* several remote VPS agents side by side (see
   [ADR 0007](0007-multi-agent-pairing.md) for the multi-pairing model
   this builds on) — and mDNS ([ADR 0002](0002-device-discovery.md))
   fundamentally cannot reach a machine that isn't on the same subnet, no
   matter how it's configured.

Both gaps share the same constraint and the same answer: the device has
no keyboard, but the user always has a phone or laptop with a browser
right next to it during setup.

## Decision

### Wi-Fi setup: a captive portal, first boot only

- A factory-fresh device (or one that's had Wi-Fi forgotten) boots into
  **access-point mode**, broadcasting its own network (`espia-setup` or
  similar).
- Connecting to that network from a phone or laptop opens a small web
  page the device itself serves — a Wi-Fi scan result list, a password
  field, "Connect". No typing happens on the device; all of it happens in
  a browser the user already has open.
- On success, the device saves the credentials to flash, drops the
  access point, reboots onto the real network, and moves on to normal
  operation (discovery, per [ADR 0002](0002-device-discovery.md)).
- A physical action (a long-press on the existing button, per
  [ADR 0006](0006-multi-board-firmware.md)'s per-board `PIN_BUTTON`) forgets
  the saved network and returns to access-point mode — the only way back
  into setup once Wi-Fi is configured, since the device is normally not
  running an access point at all.

This is the same shape as the "click it once and a setup page comes up"
library already used and liked (mentioned in discussion) — WiFiManager
and equivalents on ESP32 all follow this pattern for the same reason.

### Adding agents: local ones need no typing, remote ones need a URL and a token

Once the device is on a real network, it keeps a **small web server
running on its own LAN address** — the same page family as the captive
portal, just reachable at the device's normal IP (or `.local` name)
instead of only over the setup access point. This is where every pairing
after the first one gets added, without ever re-entering AP mode:

- **Local agents** (same subnet, discovered per [ADR 0002](0002-device-discovery.md)/
  [ADR 0003](0003-transport-and-message-format.md)): listed automatically
  on that page as they're found. Adding one is a tap, not typing — the
  device already has everything it needs from discovery.
- **Remote agents** ([ADR 0013](0013-headless-agent.md)'s headless HTTP
  endpoint — a VPS, a Docker host, anything not on the device's LAN):
  cannot be discovered this way, mDNS doesn't cross networks. Added
  through a small form on the same page instead — the agent's URL and
  its `ESPIA_TOKEN` — pasted from whatever generated them, never typed on
  the device. Both live in flash as one more entry in the multi-pairing
  list [ADR 0007](0007-multi-agent-pairing.md) already established.
- **A remote pairing is polled over plain HTTP** (`GET /metrics`,
  `Authorization: Bearer <token>`), not the WebSocket protocol
  [ADR 0003](0003-transport-and-message-format.md) defines for local
  pairings — there's no persistent local-network connection to keep
  open, and the headless agent doesn't speak that protocol (see ADR
  0013's own note that this was deferred). The device's per-pairing state
  machine (ADR 0007) grows a second transport, not a second data model:
  a remote pairing's `metrics` still arrive in the same shape, just
  fetched on a timer instead of pushed over a socket.

## Consequences

- Two distinct ways a pairing's data actually arrives at the device
  (WebSocket for local, polled HTTP for remote) — more firmware surface
  than a single transport, but each one matches what's actually reachable
  in each case rather than forcing one model onto both.
- The device is now, itself, a small web server for as long as it's
  running — not just a client of agents. New attack surface worth being
  as deliberate about as [ADR 0013](0013-headless-agent.md) was for the
  headless agent's own HTTP surface; a follow-up ADR should cover that
  page's own access control before firmware implementation starts.
- A remote agent's token sits in the device's flash and is sent over
  whatever network reaches that agent's URL — the same "token is the only
  protection, plaintext unless something in front of the agent
  terminates TLS" trade-off ADR 0013 already made, now also true on the
  device side of that same connection.
- Setup and reconfiguration both happen through a browser on hardware the
  user already owns — no companion app, no cloud account, consistent
  with this project's local-first stance ([ADR 0002](0002-device-discovery.md)'s
  "no cloud relay" reasoning applies here too).
- This is a design decision, not an implementation — firmware networking
  doesn't exist yet ([ADR 0006](0006-multi-board-firmware.md) only has a
  boot screen). Building it is future work this ADR sets direction for.

## Alternatives considered

- **A companion mobile app for setup** (scan Wi-Fi, push credentials to
  the device over Bluetooth or similar) — the standard alternative to a
  captive portal, but means building and maintaining an app for a problem
  a plain web page already solves with nothing to install.
- **Buttons/rotary input on the device itself** for Wi-Fi credentials —
  rejected for the same reason typed IP addresses were rejected in ADR
  0002: painful, error-prone text entry on hardware that isn't built for
  it, when a phone already is.
- **A cloud pairing/relay service** (type a short code from the device
  into a website, the service brokers the connection) — would solve
  reaching remote agents too, but reintroduces exactly the infrastructure,
  cost, and privacy trade-off ADR 0002 already rejected a cloud relay
  for. A directly-pasted URL and token costs the user one extra step and
  costs the project zero extra infrastructure.
