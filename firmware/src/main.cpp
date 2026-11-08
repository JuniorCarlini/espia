// espia firmware entry point.

#include <Arduino.h>
#include <WiFi.h>

#include "boards/board.h"
#include "device_id.h"
#include "net/agent_manager.h"
#include "net/provisioning.h"
#include "ui/ui.h"
#include "version.h"

namespace {

enum class State {
    ProvisioningAp,
    Running,
};

State state = State::ProvisioningAp;
// Tracks the edge, not the level: `showConnectionStatus(false)` should
// draw once right when a session that *was* up drops, not on every loop()
// while still reconnecting (that would just flicker the screen) or during
// the very first connect (nothing was "re-" about that one).
bool wasConnected = false;
// How long to wait for stored WiFi credentials to connect before deciding
// they no longer work and falling back to the setup portal.
constexpr uint32_t kStoredWifiTimeoutMs = 10000;
// Holding the board's button this long forgets the stored WiFi network and
// restarts into the setup portal — the only way back in once WiFi is
// configured, since the device doesn't otherwise run an access point.
constexpr uint32_t kForgetWifiHoldMs = 3000;
// How long the setup page stays reachable on the real network after
// connecting — bounded on purpose, see provisioning.h.
constexpr uint32_t kLanConfigWindowMs = 5UL * 60UL * 1000UL;

// Polled every loop() regardless of state — this is the only way back into
// the setup portal once WiFi is configured (ADR 0015).
void checkForgetWifiButton() {
    static uint32_t pressStart = 0;

    bool pressed = digitalRead(ESPIA_PIN_BUTTON) == LOW;
    if (!pressed) {
        pressStart = 0;
        return;
    }
    if (pressStart == 0) {
        pressStart = millis();
        return;
    }
    if (millis() - pressStart >= kForgetWifiHoldMs) {
        Serial.println("espia: button held — forgetting WiFi and restarting into setup");
        net::provisioning::forget();
        ESP.restart();
    }
}

}  // namespace

void setup() {
    Serial.begin(115200);
    Serial.printf("espia %s on %s\n", ESPIA_FIRMWARE_VERSION, ESPIA_BOARD_NAME);
    Serial.printf("espia: device id %s\n", device_id::get().c_str());

    pinMode(ESPIA_PIN_BUTTON, INPUT_PULLUP);

    ui::begin();
    ui::showBoot(ESPIA_FIRMWARE_VERSION, ESPIA_BOARD_NAME);
    delay(1200);

    if (net::provisioning::hasStoredCredentials()) {
        ui::showWifiConnecting();
        if (net::provisioning::connectStored(kStoredWifiTimeoutMs)) {
            Serial.printf("espia: WiFi connected, IP %s\n", WiFi.localIP().toString().c_str());

            net::provisioning::beginLanConfigWindow(kLanConfigWindowMs);
            ui::showConfigAddress(WiFi.localIP().toString().c_str(),
                                   net::provisioning::lanConfigAccessCode().c_str());
            delay(3000);

            net::agent_manager::begin(device_id::get());
            state = State::Running;
            ui::showDiscovering();
            return;
        }
        Serial.println("espia: stored WiFi credentials didn't connect, starting setup portal");
    }

    state = State::ProvisioningAp;
    ui::showWifiSetup("espia-setup");
    net::provisioning::begin();
}

void loop() {
    checkForgetWifiButton();
    // Pumps both the AP portal (ProvisioningAp state) and the bounded
    // post-connect LAN config window (Running state, until it expires on
    // its own) — provisioning.cpp tracks which one, if either, is active.
    net::provisioning::poll();

    if (state == State::Running) {
        net::agent_manager::loop();

        if (net::agent_manager::hasNewPairingCode()) {
            ui::showPairingCode(net::agent_manager::pairingCode().c_str());
        }
        if (net::agent_manager::hasNewPrimaryMetrics()) {
            ui::showMetrics(net::agent_manager::primaryMetrics(), net::agent_manager::primaryName().c_str());
        }

        bool nowConnected = net::agent_manager::isPrimaryConnected();
        if (wasConnected && !nowConnected) {
            ui::showConnectionStatus(false);
        }
        wasConnected = nowConnected;
    }

    delay(10);
}
