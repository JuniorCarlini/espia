// User interface for small monochrome displays (128x64 SSD1306 OLEDs),
// rendered with U8g2.

#include "boards/board.h"

#if ESPIA_DISPLAY_MONO

#include <Arduino.h>
#include <U8g2lib.h>

#include "ui.h"

namespace ui {

namespace {

ESPIA_DISPLAY_U8G2_CLASS display(U8G2_R0, ESPIA_PIN_OLED_RST,
                                 ESPIA_PIN_OLED_SCL, ESPIA_PIN_OLED_SDA);

// Draws a string horizontally centered at the given baseline.
void drawCentered(int baseline, const char *text) {
    int x = (display.getDisplayWidth() - display.getStrWidth(text)) / 2;
    display.drawStr(x, baseline, text);
}

bool isSmallPanel() {
    return display.getDisplayHeight() < 64;
}

// A single centered status line/word — used by every "in progress" screen
// (WiFi connecting, discovering, errors), so they all read consistently.
void showStatusScreen(const char *line) {
    display.clearBuffer();
    display.setFont(isSmallPanel() ? u8g2_font_4x6_tr : u8g2_font_6x10_tr);
    drawCentered(isSmallPanel() ? 20 : 34, line);
    display.sendBuffer();
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

    char version[24];
    snprintf(version, sizeof(version), "v%s", firmwareVersion);

    if (display.getDisplayHeight() >= 64) {
        display.setFont(u8g2_font_helvB14_tr);
        drawCentered(26, "espia");

        display.setFont(u8g2_font_5x8_tr);
        drawCentered(40, version);
        drawCentered(60, boardName);
    } else {
        // Small panels (for example the 72x40 ESP32-C3 board) have no room
        // for all three lines — drop the board name and shrink the rest.
        display.setFont(u8g2_font_helvB10_tr);
        drawCentered(18, "espia");

        display.setFont(u8g2_font_4x6_tr);
        drawCentered(34, version);
    }

    display.sendBuffer();
}

void showWifiConnecting() {
    showStatusScreen("Connecting WiFi...");
}

void showDiscovering() {
    showStatusScreen("Finding agent...");
}

void showPairingCode(const char *code) {
    display.clearBuffer();

    if (isSmallPanel()) {
        display.setFont(u8g2_font_4x6_tr);
        drawCentered(9, "Enter on agent:");

        display.setFont(u8g2_font_7x14B_tr);
        drawCentered(28, code);
    } else {
        display.setFont(u8g2_font_6x10_tr);
        drawCentered(18, "Enter this code");
        drawCentered(30, "on the agent:");

        display.setFont(u8g2_font_helvB18_tr);
        drawCentered(56, code);
    }

    display.sendBuffer();
}

void showConnectionStatus(bool connected) {
    // Connected has nothing of its own to show — showMetrics takes over
    // immediately once a session is established. This screen only matters
    // for the "lost the connection, retrying" gap in between.
    if (!connected) {
        showStatusScreen("Reconnecting...");
    }
}

void showError(const char *message) {
    showStatusScreen(message);
}

// Truncates `text` to fit `maxChars`, marking the cut with an ellipsis
// character so a long agent name doesn't silently look complete when it
// isn't. Uses a static buffer — the caller draws immediately, not later.
const char *truncated(const String &text, size_t maxChars) {
    static char buf[24];
    size_t n = min(maxChars, sizeof(buf) - 1);
    if (text.length() <= n) {
        text.toCharArray(buf, sizeof(buf));
        return buf;
    }
    text.toCharArray(buf, n);
    buf[n - 1] = '\x85';  // u8g2's ellipsis glyph in most fonts used here
    buf[n] = '\0';
    return buf;
}

void showMetrics(const MetricsView &metrics, const char *agentName) {
    display.clearBuffer();

    char cpuLine[24];
    snprintf(cpuLine, sizeof(cpuLine), "CPU %.0f%%", metrics.cpuUsagePct);

    float memPct = metrics.memoryTotalBytes > 0
                       ? (100.0f * static_cast<float>(metrics.memoryUsedBytes) / static_cast<float>(metrics.memoryTotalBytes))
                       : 0.0f;
    char memLine[24];
    snprintf(memLine, sizeof(memLine), "RAM %.0f%%", memPct);

    if (isSmallPanel()) {
        display.setFont(u8g2_font_4x6_tr);
        drawCentered(9, truncated(agentName, 14));
        drawCentered(22, cpuLine);
        drawCentered(34, memLine);
    } else {
        display.setFont(u8g2_font_6x10_tr);
        drawCentered(14, truncated(agentName, 20));

        display.setFont(u8g2_font_helvB10_tr);
        drawCentered(38, cpuLine);
        drawCentered(58, memLine);
    }

    display.sendBuffer();
}

}  // namespace ui

#endif  // ESPIA_DISPLAY_MONO
