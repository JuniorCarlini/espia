# Device UI simulator

Two standalone HTML files — no build step, no server, just open them in a
browser — for iterating on the device's UI before real firmware exists.
This is the "in-browser device simulator" on the [roadmap](../README.md#roadmap).

```sh
open firmware/simulator/display.html   # the on-device screen
open firmware/simulator/setup.html     # the Wi-Fi + agent configuration page
```

## `display.html` — the on-device screen

A 128×64 monochrome OLED, matching the actual display on a
[supported board](../README.md#supported-boards). Click the board (or the
button below it) to cycle screens, the same way the physical PRG button
will. Rendered on an HTML canvas at native OLED resolution, scaled up with
`image-rendering: pixelated` so it stays chunky and legible rather than
blurring like a photo would.

Screen content is hand-filled example data shaped like the real
[`metrics`/`provider` messages](../../protocol/README.md#5-data-messages) —
useful for judging whether a layout actually fits 128×64 pixels and reads
at a glance, not for testing real data flow.

## `setup.html` — Wi-Fi and agent configuration

What a phone's browser shows when it talks to the device — either its own
access point on first boot, or its normal address on your network
afterward. See [ADR 0015](../../docs/adr/0015-device-provisioning.md) for
the full provisioning design this implements:

- **Wi-Fi tab** — the networks the device found, tap one, enter its
  password, connect.
- **Agents tab** — local agents (same network, found via
  [mDNS](../../docs/adr/0002-device-discovery.md), no typing) separate from
  remote ones (a VPS running the [headless agent](../../docs/adr/0013-headless-agent.md),
  added by pasting a URL and token — mDNS can't reach those, see
  [ADR 0015](../../docs/adr/0015-device-provisioning.md)).

Every button is a preview-only no-op — nothing here submits anywhere,
saves anything, or talks to a real device. That's still future firmware
work; this is for judging the UI on its own.

## Why plain HTML, not the agent's own frontend stack

The desktop agent uses Tailwind and vendors Tucano
([ADR 0004](../../docs/adr/0004-desktop-agent-stack.md),
[ADR 0012](../../docs/adr/0012-tucano.md)) — neither is worth pulling in
here for two files nobody builds or ships; they open directly in a
browser, on purpose. Colors and spacing are hand-matched to the agent's
own tokens (`agent/src/input.css`'s `--color-*` custom properties) so the
two feel like one product, not copied wholesale.
