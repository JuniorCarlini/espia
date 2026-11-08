// A remote agent (protocol §1.4): a headless HTTP agent (ADR 0013), not on
// this device's LAN, so mDNS/UDP discovery can't find it. Added directly by
// URL and token — both typed by a human who already has them, not read off
// this device's screen, so there's no on-device pairing code for this path.
//
// Polled on a timer instead of a persistent WebSocket connection — the
// headless agent doesn't speak the device protocol's handshake, only plain
// `GET /metrics`.

#pragma once

#include <Arduino.h>

#include "ui/ui.h"

namespace net {

class RemoteAgent {
   public:
    void begin(const String &url, const String &token, const String &name);
    void loop();

    bool isConnected() const { return connected_; }
    bool hasNewMetrics();
    const ui::MetricsView &lastMetrics() const { return metrics_; }
    const String &name() const { return name_; }
    const String &url() const { return url_; }

   private:
    void poll();

    String url_;
    String token_;
    String name_;
    bool connected_ = false;
    ui::MetricsView metrics_;
    bool hasNewMetrics_ = false;
    uint32_t nextPollAt_ = 0;
};

}  // namespace net
