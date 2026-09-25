// Heltec WiFi Kit 32 (V3), model HTIT-WB32 V3.
//
// ESP32-S3 with a 0.96" 128x64 SSD1306 OLED on its own I2C bus.
// Pins follow the board's arduino-esp32 variant (heltec_wifi_kit_32_V3).

#pragma once

#define ESPIA_BOARD_NAME "Heltec WiFi Kit 32 V3"

#define ESPIA_DISPLAY_WIDTH 128
#define ESPIA_DISPLAY_HEIGHT 64
#define ESPIA_DISPLAY_MONO 1
#define ESPIA_DISPLAY_TOUCH 0

#define ESPIA_PIN_OLED_SDA 17
#define ESPIA_PIN_OLED_SCL 18
#define ESPIA_PIN_OLED_RST 21

// Vext switches the 3.3 V rail that powers the OLED. It is active low.
#define ESPIA_PIN_VEXT 36
#define ESPIA_VEXT_ON LOW

// The "PRG" button.
#define ESPIA_PIN_BUTTON 0
