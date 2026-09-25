// Wraps a WebSocket connection to one paired (or pairing) agent: the
// `hello`/`welcome` handshake (protocol §4.1), a live `metrics` stream
// (§5.1), and reconnect with backoff (§2).
//
// One `AgentLink` instance per pairing. `main.cpp` holds a fixed-size array
// of these (not a single instance, not a `std::vector`) so multi-agent
// pairing (a later build step, ADR 0007) is additive rather than a
// rearchitecture.
//
// Pairing itself (`pair_required`/`pair_request`/`paired`) is not
// implemented yet — that lands in a later build step alongside the agent
// side of pairing. Today, any `pair_required` reply is logged and treated
// as a disconnect.

#pragma once

#include <Arduino.h>
#include <WebSocketsClient.h>

#include "net/discovery.h"
#include "ui/ui.h"

namespace net {

enum class LinkStatus {
    Disconnected,
    Connecting,
    AwaitingWelcome,
    Connected,
};

class AgentLink {
   public:
    // `storedToken` is empty until pairing (a later build step) issues
    // one — today's agent welcomes unconditionally regardless, since it
    // doesn't enforce pairing yet either.
    void begin(const AgentInfo &agent, const String &deviceId, const String &storedToken);
    void loop();

    LinkStatus status() const { return status_; }
    bool isConnected() const { return status_ == LinkStatus::Connected; }

    // True once, right after a fresh `metrics` frame has been parsed —
    // clears itself, so a caller polling every loop() doesn't redraw the
    // same frame twice.
    bool hasNewMetrics();
    const ui::MetricsView &lastMetrics() const { return metrics_; }
    const String &agentName() const { return agentName_; }

    // True once 3 reconnect attempts to this agent have failed (protocol
    // §2) — `main.cpp` checks this to know when to stop retrying this
    // agent and re-run discovery instead.
    bool needsRediscovery() const { return failedAttempts_ >= kMaxAttemptsBeforeRediscovery; }

   private:
    void handleEvent(WStype_t type, uint8_t *payload, size_t length);
    void handleMessage(uint8_t *payload, size_t length);
    void sendHello();

    static constexpr uint8_t kMaxAttemptsBeforeRediscovery = 3;

    WebSocketsClient ws_;
    AgentInfo agent_;
    String deviceId_;
    String token_;
    String agentName_;
    LinkStatus status_ = LinkStatus::Disconnected;
    ui::MetricsView metrics_;
    bool hasNewMetrics_ = false;
    uint8_t failedAttempts_ = 0;
};

}  // namespace net
