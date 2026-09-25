// Generic ESP32-C3 board with a 0.42" 72x40 SSD1306 OLED soldered directly
// onto it (sold unbranded under many names; hardware-compatible with the
// Waveshare ESP32-C3-0.42LCD design). No reset pin: the panel is wired
// straight to the ESP32-C3's I2C0.
//
// The panel's visible 72x40 area sits inside the SSD1306 controller's full
// 128x64 RAM — the "_ER" U8g2 constructor variant handles that offset, so
// no manual windowing is needed here.

#pragma once

#define ESPIA_BOARD_NAME "ESP32-C3 0.42in OLED"

#define ESPIA_DISPLAY_WIDTH 72
#define ESPIA_DISPLAY_HEIGHT 40
#define ESPIA_DISPLAY_MONO 1
#define ESPIA_DISPLAY_TOUCH 0
#define ESPIA_DISPLAY_U8G2_CLASS U8G2_SSD1306_72X40_ER_F_HW_I2C

#define ESPIA_PIN_OLED_SDA 5
#define ESPIA_PIN_OLED_SCL 6
#define ESPIA_PIN_OLED_RST U8X8_PIN_NONE

// The BOOT button doubles as the user button, same as GPIO0 on the Heltec
// boards — both are the chip's strapping pin, held low only at power-on.
#define ESPIA_PIN_BUTTON 9
