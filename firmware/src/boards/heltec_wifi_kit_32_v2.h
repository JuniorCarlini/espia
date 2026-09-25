// Heltec WiFi Kit 32 (V2), model HTIT-WB32.
//
// ESP32 with a 0.96" 128x64 SSD1306 OLED.
// Pins follow the board's arduino-esp32 variant (heltec_wifi_kit_32_v2).

#pragma once

#define ESPIA_BOARD_NAME "Heltec WiFi Kit 32 V2"

#define ESPIA_DISPLAY_WIDTH 128
#define ESPIA_DISPLAY_HEIGHT 64
#define ESPIA_DISPLAY_MONO 1
#define ESPIA_DISPLAY_TOUCH 0
#define ESPIA_DISPLAY_U8G2_CLASS U8G2_SSD1306_128X64_NONAME_F_HW_I2C

#define ESPIA_PIN_OLED_SDA 4
#define ESPIA_PIN_OLED_SCL 15
#define ESPIA_PIN_OLED_RST 16

// Vext switches the external 3.3 V rail. It is active low.
#define ESPIA_PIN_VEXT 21
#define ESPIA_VEXT_ON LOW

// The "PRG" button.
#define ESPIA_PIN_BUTTON 0
