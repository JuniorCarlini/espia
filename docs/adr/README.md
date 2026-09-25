# Architecture Decision Records

An Architecture Decision Record (ADR) captures an important technical
decision, its context, and its consequences. ADRs are immutable once
accepted: to change a decision, write a new ADR that supersedes the old one.

To add a new ADR, copy [`0000-template.md`](0000-template.md) and use the next
available number.

| ADR                                        | Title                                  | Status   |
| ------------------------------------------ | -------------------------------------- | -------- |
| [0001](0001-monorepo.md)                   | Use a monorepo                         | Accepted |
| [0002](0002-device-discovery.md)           | Discover the agent via mDNS and UDP    | Accepted |
| [0003](0003-transport-and-message-format.md) | WebSocket transport with JSON messages | Accepted |
| [0004](0004-desktop-agent-stack.md)        | Desktop agent technology stack         | Accepted |
| [0005](0005-claude-plan-limits-source.md)  | Read Claude plan limits through the Claude Code status line | Accepted |
| [0006](0006-multi-board-firmware.md)       | Multi-board firmware with per-display-class user interfaces | Accepted |
| [0007](0007-multi-agent-pairing.md)        | A device may pair with more than one agent | Accepted |
| [0008](0008-macos-liquid-glass-icon.md)    | Build the macOS app icon as a real Liquid Glass icon | Accepted |
| [0009](0009-relative-asset-paths.md)       | Frontend asset references must be relative, not absolute | Accepted |
| [0010](0010-settings-i18n-and-location-override.md) | Agent settings: display language and a weather location override | Accepted |
| [0011](0011-lighter-polling.md)            | Poll the heaviest data every 2 seconds, not every 1 | Accepted |
| [0012](0012-tucano.md)                     | Use Tucano for the settings dialog, select, and buttons | Accepted |
| [0013](0013-headless-agent.md)             | A headless HTTP agent for VPS and Docker hosts | Accepted |
| [0014](0014-docker-container-stats.md)     | Per-container stats via the Docker socket | Accepted |
| [0015](0015-device-provisioning.md)        | Device provisioning: Wi-Fi setup and adding remote agents | Accepted |
