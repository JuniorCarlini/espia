// User interface for small monochrome displays (128x64 SSD1306 OLEDs),
// rendered with U8g2.

#include "boards/board.h"

#if ESPIA_DISPLAY_MONO

#include <Arduino.h>
#include <U8g2lib.h>

#include "ui.h"

namespace ui {

namespace {

U8G2_SSD1306_128X64_NONAME_F_HW_I2C display(U8G2_R0, ESPIA_PIN_OLED_RST,
                                            ESPIA_PIN_OLED_SCL,
                                            ESPIA_PIN_OLED_SDA);

// Draws a string horizontally centered at the given baseline.
void drawCentered(int baseline, const char *text) {
    int x = (display.getDisplayWidth() - display.getStrWidth(text)) / 2;
    display.drawStr(x, baseline, text);
}

}  // namespace

void begin() {
#ifdef ESPIA_PIN_VEXT
    pinMode(ESPIA_PIN_VEXT, OUTPUT);
    digitalWrite(ESPIA_PIN_VEXT, ESPIA_VEXT_ON);
    // Give the OLED time to power up before talking to it.
    delay(50);
#endif
    display.begin();
}

void showBoot(const char *firmwareVersion, const char *boardName) {
    display.clearBuffer();

    display.setFont(u8g2_font_helvB14_tr);
    drawCentered(26, "espia");

    display.setFont(u8g2_font_5x8_tr);
    char version[24];
    snprintf(version, sizeof(version), "v%s", firmwareVersion);
    drawCentered(40, version);
    drawCentered(60, boardName);

    display.sendBuffer();
}

}  // namespace ui

#endif  // ESPIA_DISPLAY_MONO
