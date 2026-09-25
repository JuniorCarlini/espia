# Security Policy

## Supported versions

espia has not had a release yet. Once it does, only the latest release will
receive security fixes.

## Reporting a vulnerability

Please **do not** open a public issue for security problems. Instead, report
them privately through
[GitHub Security Advisories](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
on this repository, or by email to **juniorsmgks@gmail.com**.

Include as much detail as you can: affected component (agent, firmware,
protocol), version, steps to reproduce, and potential impact. We aim to
acknowledge reports within 7 days.

## Threat model

espia runs on trusted local networks. Understanding what it does and does
not protect against helps set expectations:

**In scope**

- Preventing a device from accidentally connecting to the wrong computer.
- Preventing other computers on the network from taking over a paired device.
- Never sending API keys or credentials to the device — secrets stay in the
  agent, stored in the operating system's credential store.
- Never exposing the agent outside the local network by default.

**Out of scope (for protocol v1)**

- An active attacker on the same local network. Protocol v1 uses plain
  WebSocket (no TLS), so traffic can be observed or tampered with by anyone
  able to intercept it. Encrypted transport is planned for a future version.
- Physical access to the device (flash contents are not encrypted).
