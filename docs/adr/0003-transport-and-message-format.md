# 0003. WebSocket transport with JSON messages

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

The agent pushes updates to the device about once per second, and the device
occasionally sends messages back (pairing, screen changes). We need a
transport that supports bidirectional messages, detects dead connections,
and is well supported on both desktop platforms and the ESP32.

## Decision

- Use **WebSocket** over TCP, at the path `/v1/ws`.
- Encode messages as **JSON** objects with a `type` field and a protocol
  version `v`.
- Use WebSocket ping/pong frames for liveness.

## Consequences

- JSON is human-readable, which makes debugging and third-party clients easy.
- A browser can act as a device, enabling a device simulator for UI
  development without hardware.
- JSON is larger and slower to parse than a binary format. At ~1 message per
  second with a few hundred bytes each, this is negligible for the ESP32.
- Protocol v1 has no TLS. See the [security policy](../../SECURITY.md).

## Alternatives considered

- **MQTT** — great for many-to-many IoT, but requires a broker.
- **Plain HTTP polling** — simpler, but wastes requests and adds latency.
- **Binary encodings (CBOR, Protocol Buffers)** — more compact; can be added
  later as an optional encoding if payloads grow.
