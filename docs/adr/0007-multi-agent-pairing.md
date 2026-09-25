# 0007. A device may pair with more than one agent

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

The motivating use case for espia is one desk display showing more than one
computer — for example, a Mac, a Windows PC, and a Linux server, all
monitored from a single ESP32 device. The protocol and firmware were not yet
built with this in mind: [ADR 0002](0002-device-discovery.md) and the
original pairing text in `protocol/README.md` §1.3 assumed a device pairs
with exactly one agent, connects only to it, and ignores everything else on
the network.

Firmware networking is not implemented yet — only a boot screen exists — so
this is a design decision, not a rework.

## Decision

A device MAY hold **more than one pairing at once**. Each pairing — its
discovery, WebSocket connection, reconnection, and token — is independent of
the others; the device simply runs the machinery described in the protocol
once per paired agent instead of once for "the" agent. No wire message
changes: `hello`, `pairing`, `metrics`, and `provider` all already carry
exactly the fields a single connection needs, so N connections are just N
independent instances of the same flow.

Concretely, for the firmware:

- Discovery does not stop at the first reply; it keeps listening, since more
  than one agent the device cares about may be on the network.
- The device stores a **list** of pairings (`agent_id`, `name`, `token`) in
  flash, not a single one, and opens one WebSocket connection per entry.
- One pairing going offline (its PC sleeps, its agent quits) does not affect
  the others — each connection reconnects and rediscovers independently.
- The screen needs a way to show *which* computer's data is on screen: a
  home/selector screen listing paired computers by name, with per-computer
  screens behind it. Exact UI is a firmware detail, not a protocol concern.
- The pairing flow needs an explicit "pair another computer" entry point
  once the device already holds a pairing, rather than only supporting
  pairing on a factory-fresh device.

For the agent: **no changes**. An agent doesn't know or care whether the
devices connected to it are also connected to other agents — it just serves
its own metrics to whoever is paired with it.

The protocol's `name` field (from discovery and `welcome`) is how the device
tells computers apart on screen, so it needs to actually distinguish them.
The agent's settings UI should let the user set a friendly name, defaulting
to the machine's hostname, rather than something generic.

## Consequences

- Resource use on the device scales with the number of pairings: one
  WebSocket connection and one metrics-history buffer per computer. On an
  ESP32(-S3) this is comfortably affordable for the handful of computers
  this feature is meant for (not for dozens).
- The firmware is more complex than a single-connection client: connection
  state, reconnection, and history all become per-pairing rather than
  global.
- Users get one consolidated display instead of needing one device per
  computer, which is the actual point of the feature.
- Protocol v1's shape needed no changes — see `protocol/README.md` §1.3 and
  §4.2 for the (documentation-only) updates this ADR made there.

## Alternatives considered

- **One device per computer** — simplest to build, and still fully
  supported (a device with exactly one pairing is just the N=1 case of this
  design). Rejected as the *only* option because it doesn't serve the
  motivating use case: comparing several machines at a glance on one screen.
- **A hub/relay service that merges several agents into one stream the
  device sees as a single connection** — would keep the device simpler, but
  adds a component to run and keep alive for no benefit here: the device has
  plenty of headroom to hold a few WebSocket connections itself.
