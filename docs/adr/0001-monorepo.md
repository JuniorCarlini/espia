# 0001. Use a monorepo

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

espia has three parts that evolve together: the desktop agent, the ESP32
firmware, and the protocol between them. A protocol change almost always
requires matching changes on both sides.

## Decision

Keep the agent, firmware, protocol specification, and documentation in a
single repository, in the `agent/`, `firmware/`, `protocol/`, and `docs/`
directories.

## Consequences

- A protocol change and its implementations land in one pull request, so the
  two sides never drift apart.
- One place for issues, releases, and documentation.
- CI must build each part independently, only when its files change.
- Releases need to tag agent and firmware versions separately.

## Alternatives considered

- **Separate repositories per component** — cleaner boundaries, but
  cross-cutting changes would need coordinated pull requests across repos,
  which is overhead a small project doesn't need.
