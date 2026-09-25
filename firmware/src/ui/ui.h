// Display-independent user interface API.
//
// Each display class (for example, small monochrome OLEDs or color TFTs)
// provides its own implementation, because layouts for a 128x64 OLED and a
// 320x240 TFT have little in common. See docs/adr/0006.

#pragma once

#include <cstdint>

namespace ui {

// Powers up and initializes the display.
void begin();

// Shows the boot screen with the firmware version and board name.
void showBoot(const char *firmwareVersion, const char *boardName);

// Shown while connecting to WiFi with already-stored credentials.
void showWifiConnecting();

// Shown while searching for agents on the network (protocol §1.1-§1.2).
void showDiscovering();

// `connected` true once a session is fully established (`welcome`
// received); false while reconnecting after a drop.
void showConnectionStatus(bool connected);

// A plain, JSON- and WebSocket-agnostic view of the latest `metrics`
// message — filled in by `net::AgentLink`, rendered here. Grows as more of
// protocol §5.1 is wired up; today covers just enough for the smallest
// supported display's layout.
struct MetricsView {
    float cpuUsagePct = 0;
    uint64_t memoryUsedBytes = 0;
    uint64_t memoryTotalBytes = 0;
};

// Renders the latest metrics from the given agent.
void showMetrics(const MetricsView &metrics, const char *agentName);

// Shown when something unrecoverable-by-retry happens (kept short — this
// is a 72x40 screen, not a log viewer).
void showError(const char *message);

}  // namespace ui
