# 0006. Multi-board firmware with per-display-class user interfaces

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

espia should run on many ESP32 boards, not a single reference device. These
boards differ a lot:

- **Chips:** ESP32, ESP32-S3, and others, with or without PSRAM.
- **Displays:** from 0.96" 128x64 monochrome OLEDs to 320x240 or larger color
  TFTs, some with touch.
- **Input:** a single button, several buttons, or touch.
- **Pins:** every board wires its display and power rails differently.

The first target board is the Heltec WiFi Kit 32 (V3): an ESP32-S3 with a
128x64 monochrome OLED, no PSRAM, and one user button.

## Decision

1. **One PlatformIO environment per board.** Each environment sets an
   `ESPIA_BOARD_*` flag.
2. **One header per board** in `firmware/src/boards/` defines pins and
   capabilities (display size, monochrome or color, touch, button). The rest
   of the firmware only uses these definitions, never raw GPIO numbers.
3. **The user interface is implemented per display class**, behind a small
   common API (`firmware/src/ui/ui.h`):
   - **Monochrome** (small OLEDs): rendered with
     [U8g2](https://github.com/olikraus/u8g2).
   - **Color** (TFTs): rendered with [LVGL](https://lvgl.io/), when the first
     color board is added.
4. Networking, discovery, pairing, and protocol handling are shared by all
   boards.

## Consequences

- Adding a board that fits an existing display class only needs a board
  header and a PlatformIO environment.
- Layouts are designed for each display class instead of scaled, which gives
  better results: a 128x64 screen shows one or two metrics per page, while a
  color TFT can show a full dashboard with charts.
- Two rendering libraries must be maintained. This is accepted because the
  layouts for each class would be different anyway.
- U8g2 is small and fast, which suits boards without PSRAM.
- The `hello` message reports the display's capabilities so the agent can
  adapt what it sends.

## Alternatives considered

- **LVGL for every display** — one rendering library, but LVGL is heavier and
  less natural on 1-bit 128x64 screens, where U8g2 is the de facto standard.
- **A single board** — simpler, but contradicts the project's goal of
  supporting the ESP32 boards people already own.
