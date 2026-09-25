// espia firmware entry point.

#include <Arduino.h>
#include <WiFi.h>

#include "boards/board.h"
#include "device_id.h"
#include "net/agent_link.h"
#include "net/discovery.h"
#include "net/pairing_store.h"
#include "ui/ui.h"
#include "version.h"
// Build-step-3 placeholder: hardcoded WiFi, no captive portal yet (that's
// a later build step, ADR 0015). Copy wifi_credentials.h.example to
// wifi_credentials.h (gitignored) and fill in a real network before
// flashing.
#include "wifi_credentials.h"

namespace {

enum class State {
    ConnectingWifi,
    Discovering,
    LinkRunning,
};

State state = State::ConnectingWifi;
net::AgentLink agentLink;
uint32_t nextDiscoveryAttempt = 0;
// Tracks the edge, not the level: `showConnectionStatus(false)` should
// draw once right when a session that *was* up drops, not on every loop()
// while still reconnecting (that would just flicker the screen) or during
// the very first connect (nothing was "re-" about that one).
bool wasConnected = false;
// How long to spend on one discovery attempt (mDNS + up to 3 UDP rounds)
// before giving up and trying again.
constexpr uint32_t kDiscoveryTimeoutMs = 4000;
constexpr uint32_t kDiscoveryRetryDelayMs = 3000;

void beginDiscovery() {
    ui::showDiscovering();
    Serial.println("espia: discovering agents...");

    auto agents = net::discovery::find(kDiscoveryTimeoutMs, device_id::get());
    if (agents.empty()) {
        Serial.println("espia: no agents found, will retry");
        nextDiscoveryAttempt = millis() + kDiscoveryRetryDelayMs;
        return;
    }

    const net::AgentInfo &chosen = agents[0];
    Serial.printf("espia: found %d agent(s), connecting to %s\n", (int)agents.size(), chosen.name.c_str());
    String storedToken = net::pairing_store::tokenFor(chosen.agentId);
    agentLink.begin(chosen, device_id::get(), storedToken);
    state = State::LinkRunning;
}

}  // namespace

void setup() {
    Serial.begin(115200);
    Serial.printf("espia %s on %s\n", ESPIA_FIRMWARE_VERSION, ESPIA_BOARD_NAME);
    Serial.printf("espia: device id %s\n", device_id::get().c_str());

    ui::begin();
    ui::showBoot(ESPIA_FIRMWARE_VERSION, ESPIA_BOARD_NAME);
    delay(1200);

    ui::showWifiConnecting();
    WiFi.mode(WIFI_STA);
    WiFi.begin(ESPIA_WIFI_SSID, ESPIA_WIFI_PASSWORD);
    Serial.printf("espia: connecting to WiFi \"%s\"...\n", ESPIA_WIFI_SSID);
}

void loop() {
    switch (state) {
        case State::ConnectingWifi:
            if (WiFi.status() == WL_CONNECTED) {
                Serial.printf("espia: WiFi connected, IP %s\n", WiFi.localIP().toString().c_str());
                state = State::Discovering;
                beginDiscovery();
            }
            break;

        case State::Discovering:
            if (millis() >= nextDiscoveryAttempt) {
                beginDiscovery();
            }
            break;

        case State::LinkRunning: {
            agentLink.loop();
            if (agentLink.hasNewPairingCode()) {
                ui::showPairingCode(agentLink.pairingCode().c_str());
            }
            if (agentLink.hasNewMetrics()) {
                ui::showMetrics(agentLink.lastMetrics(), agentLink.agentName().c_str());
            }
            bool nowConnected = agentLink.isConnected();
            if (wasConnected && !nowConnected) {
                ui::showConnectionStatus(false);
            }
            wasConnected = nowConnected;
            if (agentLink.needsRediscovery()) {
                Serial.println("espia: giving up on this agent, re-discovering");
                state = State::Discovering;
                nextDiscoveryAttempt = millis();
                wasConnected = false;
            }
            break;
        }
    }

    delay(10);
}
