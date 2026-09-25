# 0002. Discover the agent via mDNS and UDP

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

The device and the computer live on a home or office network where IP
addresses are assigned by DHCP and can change at any time. The device has no
keyboard, so asking the user to type an IP address is not acceptable. The
device must find the agent on its own, and find it again if its address
changes.

## Decision

- The **agent acts as the server** and the **device acts as the client**.
- The agent announces itself via **mDNS / DNS-SD** as `_espia._tcp`.
- As a fallback, the agent listens for **UDP broadcast** discovery requests
  and replies with its address and port.
- The device runs discovery on boot and again whenever reconnection to the
  last known agent fails.

Message details are in the [protocol specification](../../protocol/README.md).

## Consequences

- No IP addresses are stored or typed by the user.
- mDNS is the standard, well-supported mechanism on macOS and Linux and is
  available on the ESP32.
- The UDP fallback covers networks where multicast is filtered and Windows
  setups where mDNS is unreliable.
- Discovery only works within a single subnet. Networks with client
  isolation (common on guest Wi-Fi) are not supported.
- The agent needs firewall permission for incoming connections on Windows.

## Alternatives considered

- **Device as server, agent discovers device** — works, but the agent would
  need to scan for devices and a device could only be driven by one computer
  at a time without extra coordination.
- **Cloud relay** — works across networks, but adds infrastructure, cost, and
  privacy concerns. Not needed for a local-first product.
- **Static IP configuration** — rejected; it's exactly the problem we want to
  avoid.
