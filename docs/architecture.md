# Architecture Overview

This document describes how espia's components fit together. Decisions
behind the design are recorded in [`adr/`](adr/).

## Components

```
┌──────────────────────────── Agent (desktop) ─────────────────────────────┐
│                                                                          │
│  ┌──────────────┐    ┌────────────┐    ┌──────────────────────────────┐  │
│  │  Collectors  │───►│ Aggregator │───►│ WebSocket server (/v1/ws)    │  │
│  │  system, gpu │    │  snapshot  │    │ mDNS announcer (_espia._tcp)│  │
│  └──────────────┘    │  + history │    │ UDP discovery responder      │  │
│  ┌──────────────┐    │            │    └──────────────┬───────────────┘  │
│  │  Providers   │───►│            │                   │                  │
│  │  claude, ... │    └────────────┘                   │                  │
│  └──────────────┘                                     │                  │
│  ┌──────────────────────────────────────────┐         │                  │
│  │ Settings UI: pairing, providers, layout  │         │                  │
│  └──────────────────────────────────────────┘         │                  │
└───────────────────────────────────────────────────────┼──────────────────┘
                                                        │ local network
┌───────────────────────── Device (ESP32) ──────────────┼──────────────────┐
│  Wi-Fi provisioning ──► Discovery ──► WebSocket client ──► State store   │
│  (captive portal / Improv)                                  │            │
│                                                   Screens ◄─┘            │
└──────────────────────────────────────────────────────────────────────────┘
```

### Agent

A background desktop application for Windows, macOS, and Linux.

- **Collectors** read local system metrics (CPU, memory, disk, network, GPU).
- **Providers** read usage data from AI services. Each provider is an
  independent module with a common interface, so new services can be added
  without touching the rest of the agent. Claude is the first provider.
- **Aggregator** merges collector and provider output into snapshots and keeps
  a short history for charts.
- **Server** exposes a WebSocket endpoint and makes the agent discoverable on
  the local network.
- **Settings UI** handles pairing, provider credentials, and screen layout.

Secrets (such as API keys) never leave the agent. They are stored in the
operating system's credential store and are never sent to the device.

### Device

ESP32 firmware that renders the data.

- **Wi-Fi provisioning** without a keyboard: a captive portal or credentials
  sent over USB by the agent.
- **Discovery** finds the agent via mDNS, falling back to UDP broadcast.
- **WebSocket client** keeps a persistent connection and reconnects (and
  rediscovers) automatically.
- **State store** holds the latest snapshot and a ring buffer for charts.
- **Screens** are designed per display class: U8g2 for small monochrome
  OLEDs, LVGL for color TFTs. Board pins and capabilities are isolated in
  board definitions, so many ESP32 boards can be supported
  ([ADR 0006](adr/0006-multi-board-firmware.md)).

### Protocol

A versioned JSON protocol over WebSocket. See
[`protocol/README.md`](../protocol/README.md).

## Connection lifecycle

1. The device boots and joins Wi-Fi. If no credentials are stored, it starts
   provisioning.
2. The device discovers agents via mDNS (`_espia._tcp`) or UDP broadcast.
3. The device connects to the agent's WebSocket and sends `hello`.
4. If the device is not yet paired, the pairing flow runs (a code shown on
   the device is typed into the agent).
5. The agent streams `metrics` and `provider` messages.
6. If the connection drops, the device retries with backoff and then returns
   to step 2, so IP address changes on either side are handled transparently.

## Design principles

- **Zero configuration** — no IP addresses to type, ever.
- **Lightweight** — the agent runs all the time and must stay small.
- **Graceful degradation** — a missing metric (for example, no GPU) is simply
  omitted, never an error.
- **Extensible** — providers and screens are modular.
