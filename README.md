# espia

> A tiny desk display for your computer's vitals and AI usage — powered by an ESP32.

espia mirrors live information from your computer onto a small ESP32-driven
screen sitting on your desk: CPU, memory, GPU, network, and AI usage
(starting with Claude), rendered as clean, glanceable charts.

> [!WARNING]
> espia is in early design. Nothing is usable yet — the architecture and
> protocol are still being defined. Feedback is welcome!

## Platform support

| Platform | Agent                  |
| -------- | ---------------------- |
| macOS    | First target (planned) |
| Windows  | Planned                |
| Linux    | Planned                |

espia supports several ESP32 boards. See the
[firmware documentation](firmware/README.md#supported-boards) for the list.

## How it works

```
┌──────────────── Computer (Windows / macOS / Linux) ────────────────┐           ┌────── ESP32 + display ──────┐
│  Collectors ──► Aggregator ──► WebSocket server + mDNS announcer   │   Wi-Fi   │  Discovery (mDNS / UDP)     │
│  (CPU, RAM, GPU, network, AI providers)                            │◄─────────►│  WebSocket client           │
│  Settings UI (HTML / CSS / JS)                                     │   JSON    │  User interface             │
└────────────────────────────────────────────────────────────────────┘           └─────────────────────────────┘
```

- **Agent** — a lightweight desktop app that collects metrics and streams them
  over the local network.
- **Firmware** — runs on the ESP32, finds the agent automatically, and draws
  the dashboards.
- **Protocol** — a small, versioned JSON protocol shared by both sides.

### Zero-configuration networking

- **Wi-Fi setup without a keyboard:** the device opens a captive portal
  (`espia-Setup`) you can configure from your phone, or receives credentials
  from the agent over USB ([Improv](https://www.improv-wifi.com/)).
- **No hard-coded IP addresses:** the agent announces itself via mDNS, with a
  UDP broadcast fallback. If the connection drops or an IP changes, the device
  simply discovers the agent again.
- **Pairing:** the first connection is confirmed with a short code shown on
  the device screen, so other computers on the network can't take it over.

## Features (planned)

- [ ] System metrics: CPU, memory, disk, network
- [ ] GPU metrics (NVIDIA first; AMD and Apple Silicon later)
- [ ] Claude subscription limits (5-hour and weekly windows)
- [ ] Claude token usage and cost
- [ ] Pluggable providers for other AI services
- [ ] Multiple screens with touch / button navigation
- [ ] Over-the-air (OTA) firmware updates from the agent
- [ ] In-browser device simulator for UI development without hardware

## Repository layout

| Path                     | Description                                   |
| ------------------------ | --------------------------------------------- |
| [`agent/`](agent/)       | Desktop agent (Windows, macOS, Linux)         |
| [`firmware/`](firmware/) | ESP32 firmware                                |
| [`protocol/`](protocol/) | Agent ↔ device protocol specification         |
| [`docs/`](docs/)         | Architecture notes and decision records (ADR) |

## Documentation

- [Architecture overview](docs/architecture.md)
- [Protocol specification](protocol/README.md)
- [Architecture Decision Records](docs/adr/)

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md)
and our [Code of Conduct](CODE_OF_CONDUCT.md) before getting started.

## License

espia is released under the [MIT License](LICENSE).

espia is an independent project and is not affiliated with or endorsed by
Anthropic, Espressif, or any AI provider it integrates with.
