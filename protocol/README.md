# espia Protocol v1

- **Status:** Draft — may change without notice until the first release.

This document specifies how an espia **agent** (desktop app) and **device**
(ESP32 display) find each other and exchange data. The design rationale is in
[ADR 0002](../docs/adr/0002-device-discovery.md) and
[ADR 0003](../docs/adr/0003-transport-and-message-format.md).

The key words "MUST", "SHOULD", and "MAY" are to be interpreted as described
in [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119).

## Contents

1. [Discovery](#1-discovery)
2. [Transport](#2-transport)
3. [Message envelope](#3-message-envelope)
4. [Session and pairing](#4-session-and-pairing)
5. [Data messages](#5-data-messages)
6. [Conventions](#6-conventions)
7. [Versioning](#7-versioning)

## 1. Discovery

The agent MUST support both mechanisms below. The device SHOULD try mDNS
first and fall back to UDP.

### 1.1 mDNS / DNS-SD

The agent advertises the service type `_espia._tcp` on the port of its
WebSocket server, with these TXT records:

| Key    | Example                                | Description                      |
| ------ | -------------------------------------- | -------------------------------- |
| `v`    | `1`                                    | Protocol major version           |
| `id`   | `3f2a9c1e-7b4d-4e8a-9f10-2c6b8d5e4a71` | Stable, unique agent ID (UUID)   |
| `name` | `Office Workstation`                     | Human-readable agent name        |
| `path` | `/v1/ws`                               | WebSocket path                   |

### 1.2 UDP broadcast fallback

- The agent listens on **UDP port 47800**.
- The device broadcasts a `discover` datagram to `255.255.255.255:47800`.
- Every agent that receives it replies **unicast** to the sender's address
  and port with an `announce` datagram.
- The device SHOULD wait at least 1 second to collect replies and SHOULD
  retry up to 3 times.

Datagrams use the same JSON [envelope](#3-message-envelope) as WebSocket
messages. See [`discover.json`](examples/discover.json) and
[`announce.json`](examples/announce.json).

### 1.3 Choosing an agent

A device MAY pair with more than one agent — for example, a desk display
showing a Mac, a Windows PC, and a Linux server side by side. Each pairing is
independent: its own discovery, connection, reconnection, and token, all as
described in the rest of this document. Nothing above changes; the device
simply runs that machinery once per paired agent instead of once.

- Discovery keeps listening rather than stopping at the first reply, since
  more than one agent the device cares about may answer.
- For each **paired** `agent_id` found, the device connects to it.
- For an **unpaired** agent found while the device itself has no pairings at
  all yet, first-run behavior is unchanged: if exactly one agent is found, the
  device MAY connect automatically; if several are found, it SHOULD let the
  user choose. Once the device holds at least one pairing, further unpaired
  agents on the network are ignored unless the user explicitly starts pairing
  another (see [4.2](#42-pairing)).

## 2. Transport

- WebSocket (RFC 6455) at the path advertised by discovery (`/v1/ws`).
- One JSON message per text frame, UTF-8 encoded.
- The agent MUST send a WebSocket ping at least every 10 seconds. The device
  SHOULD treat the connection as dead after 30 seconds without traffic.
- On disconnection, the device SHOULD reconnect with exponential backoff
  (1 s, 2 s, 4 s, … capped at 30 s). After 3 failed attempts it MUST run
  discovery again, since the agent's address may have changed.

## 3. Message envelope

Every message is a JSON object with these fields:

| Field  | Type    | Required | Description                                   |
| ------ | ------- | -------- | --------------------------------------------- |
| `v`    | integer | yes      | Protocol major version (`1`)                  |
| `type` | string  | yes      | Message type                                  |
| `ts`   | integer | no       | Sender timestamp, Unix epoch in milliseconds  |

Additional fields depend on `type`. Receivers MUST ignore unknown fields and
SHOULD ignore messages with an unknown `type`.

## 4. Session and pairing

### 4.1 Handshake

```
Device                                   Agent
  │ ── hello {device_id, token?} ──────────► │
  │                                           │  token valid?
  │ ◄───────────────────────── welcome ───── │  yes → session starts
  │ ◄───────────────────── pair_required ─── │  no  → pairing (4.2)
```

- **`hello`** (device → agent) — first message after connecting. It
  identifies the device and describes its display, so the agent can adapt
  what it sends. See [`hello.json`](examples/hello.json).

  | Field            | Type           | Description                              |
  | ---------------- | -------------- | ---------------------------------------- |
  | `device_id`      | string         | Stable, unique device ID                 |
  | `firmware`       | string         | Firmware version                         |
  | `board`          | string         | Human-readable board name                |
  | `token`          | string \| null | Pairing token, or `null` if not paired   |
  | `display.width`  | integer        | Width in pixels                          |
  | `display.height` | integer        | Height in pixels                         |
  | `display.mono`   | boolean        | `true` for monochrome displays           |
  | `display.touch`  | boolean        | `true` if the display accepts touch      |
- **`welcome`** (agent → device) — the session is established; data messages
  follow. See [`welcome.json`](examples/welcome.json).

### 4.2 Pairing

Pairing proves that the person using the agent can see the device's screen.

1. The agent sends **`pair_required`**.
2. The device generates a random 6-digit code, shows it on screen, and sends
   **`pair_request`** containing the code.
3. The agent asks the user to type the code shown on the device.
4. If the codes match, the agent sends **`paired`** with a secret `token`,
   followed by `welcome`. The device MUST store the token alongside the
   `agent_id`, as one entry in its list of pairings — see
   [1.3](#13-choosing-an-agent).
5. If they don't match or the user cancels, the agent sends **`error`** with
   code `pairing_rejected` and closes the connection.

A pairing code MUST expire after 2 minutes.

> [!NOTE]
> Pairing prevents accidental or casual takeover, not an active attacker on
> the local network. See the [security policy](../SECURITY.md).

### 4.3 Errors

**`error`** (either direction) carries a machine-readable `code` and a
human-readable `message`. See [`error.json`](examples/error.json).

| Code                   | Meaning                                    |
| ---------------------- | ------------------------------------------ |
| `unsupported_version`  | The peer's protocol version is unsupported |
| `invalid_token`        | The token is unknown or was revoked        |
| `pairing_rejected`     | The pairing code was wrong or cancelled    |
| `pairing_expired`      | The pairing code expired                   |
| `bad_message`          | The message could not be parsed            |

## 5. Data messages

### 5.1 `metrics` (agent → device)

System metrics, sent about once per second. Every section is optional: if a
metric is not supported on the host (for example, no GPU), it is omitted.
See [`metrics.json`](examples/metrics.json).

| Path                    | Type      | Description                         |
| ----------------------- | --------- | ----------------------------------- |
| `cpu.usage_pct`         | number    | Total CPU usage                     |
| `cpu.core_count`        | integer   | Number of logical cores             |
| `cpu.cores_pct`         | number[]  | Per-core usage                      |
| `cpu.temp_c`            | number    | CPU temperature. May be approximated on hosts with no single package sensor (see `cpu.temp_estimated`) |
| `cpu.temp_estimated`    | boolean   | `true` if `temp_c` is approximated from indirect sensors rather than a sensor the OS labels as the CPU. Omitted when `temp_c` is omitted |
| `memory.used_bytes`     | integer   | Used physical memory                |
| `memory.total_bytes`    | integer   | Total physical memory               |
| `disks[]`               | object[]  | `name`, `used_bytes`, `total_bytes` |
| `network.rx_bps`        | integer   | Download rate, bytes per second     |
| `network.tx_bps`        | integer   | Upload rate, bytes per second       |
| `gpus[]`                | object[]  | `name`, `usage_pct`, `memory_used_bytes`, `memory_total_bytes`, `temp_c` |

### 5.2 `provider` (agent → device)

Usage data from an AI provider, sent when it changes (typically every
30–60 seconds). `provider` identifies the source, and `data` holds
provider-specific fields documented under [`providers/`](providers/).
See [`provider-claude.json`](examples/provider-claude.json).

## 6. Conventions

- Field names use `snake_case`.
- Units are encoded in the field name suffix: `_pct` (0–100), `_bytes`,
  `_bps` (bytes per second), `_c` (degrees Celsius), `_ms`
  (milliseconds), `_usd` (US dollars).
- Timestamps use the `_at` suffix (or `ts`) and are Unix epoch
  milliseconds.
- **Omitted** means "not supported on this host"; **`null`** means
  "supported but temporarily unavailable".

## 7. Versioning

- The envelope's `v` is the protocol **major** version. It changes only for
  breaking changes, and the WebSocket path changes with it (`/v2/ws`).
- Adding optional fields or new message types is not a breaking change.
- If the agent does not support the device's version, it MUST reply with an
  `error` (`unsupported_version`) and close the connection.
