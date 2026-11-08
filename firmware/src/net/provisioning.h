// WiFi setup via a captive portal (protocol design: ADR 0015) — the device
// has no keyboard, so credentials come from a form served by the device
// itself, filled in from a phone's browser, never typed on the device.

#pragma once

#include <Arduino.h>

namespace net {
namespace provisioning {

// True if a WiFi network is stored from a previous setup. Doesn't verify
// it still works — connectStored() is what finds that out.
bool hasStoredCredentials();

// Tries to join the stored network, waiting up to timeoutMs. False on
// timeout or if nothing is stored.
bool connectStored(uint32_t timeoutMs);

// Starts the "espia-setup" access point and its captive portal. Call
// poll() every loop() afterward. Submitting the setup form saves the
// network and reboots the device directly — there is no "provisioning
// succeeded" return from poll() to wait for.
void begin();
void poll();

// Serves the same setup page on the device's real LAN address (no AP, no
// captive-portal DNS tricks — the network already has real DNS) for
// `windowMs`, then shuts itself down automatically. Generates a fresh
// one-time numeric access code (see lanConfigAccessCode()) that every
// state-changing request to this window must present — anyone on the same
// WiFi can reach the page, but only whoever is standing in front of the
// device's own screen can read the code (ADR 0016). Call poll() every
// loop() while it's open, same as the AP portal.
void beginLanConfigWindow(uint32_t windowMs);
bool isLanConfigWindowOpen();

// The access code beginLanConfigWindow() just generated — show it next to
// the address so whoever is setting up the device can type it into the
// page. Empty if the window isn't open.
const String &lanConfigAccessCode();

// Forgets the stored network. The next boot has nothing to connect with,
// so it starts the portal again instead of trying connectStored().
void forget();

}  // namespace provisioning
}  // namespace net
