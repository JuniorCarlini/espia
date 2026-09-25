# espia Firmware

Firmware for the ESP32 display that renders data received from the agent.

- **Status:** Early development — shows a boot screen.
- **Stack:** [PlatformIO](https://platformio.org/), Arduino framework,
  [U8g2](https://github.com/olikraus/u8g2) for monochrome displays
  ([ADR 0006](../docs/adr/0006-multi-board-firmware.md)).

## Supported boards

| Board                                                                 | Environment             | Display              | Input      |
| --------------------------------------------------------------------- | ----------------------- | -------------------- | ---------- |
| [Heltec WiFi Kit 32 (V3)](https://heltec.org/project/wifi-kit32-v3/)  | `heltec_wifi_kit_32_v3` | 0.96" 128x64 OLED    | PRG button |
| Heltec WiFi Kit 32 (V2)                                               | `heltec_wifi_kit_32_v2` | 0.96" 128x64 OLED    | PRG button |

## Building and flashing

Install [PlatformIO Core](https://docs.platformio.org/en/latest/core/installation/index.html),
then run from this directory:

```sh
# Build the default environment (Heltec WiFi Kit 32 V3)
pio run

# Build, flash, and open the serial monitor for a specific board
pio run -e heltec_wifi_kit_32_v3 -t upload -t monitor
```

## Project layout

| Path                | Description                                        |
| ------------------- | -------------------------------------------------- |
| `platformio.ini`    | One environment per supported board                |
| `src/main.cpp`      | Entry point                                        |
| `src/boards/`       | Pin and capability definitions for each board      |
| `src/ui/`           | User interface, one implementation per display class |

## Adding a board

1. Create `src/boards/<board>.h` defining the macros listed in
   [`src/boards/board.h`](src/boards/board.h).
2. Add it to the `#if` chain in `board.h` with a new `ESPIA_BOARD_*` flag.
3. Add a `[env:<board>]` section to `platformio.ini` that sets the flag.
4. If its display fits an existing display class, you're done. Otherwise,
   add a new implementation under `src/ui/`.
5. Add the board to the table above.

## Roadmap

- [x] Boot screen
- [ ] Wi-Fi provisioning (captive portal, Improv over USB)
- [ ] Agent discovery (mDNS, UDP fallback)
- [ ] WebSocket connection and pairing
- [ ] Screens: system metrics, Claude plan limits
- [ ] Screen navigation with the button
