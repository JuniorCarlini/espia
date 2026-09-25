#include "net/agent_link.h"

#include <ArduinoJson.h>

#include "boards/board.h"
#include "version.h"

namespace net {

namespace {
// 1s, 2s, 4s, ... capped at 30s (protocol §2). `failedAttempts_` starts at
// 1 for the first retry, so this is `2^(n-1)` seconds, clamped.
uint32_t backoffMsFor(uint8_t failedAttempts) {
    uint32_t seconds = 1UL << (failedAttempts - 1);
    if (seconds > 30) seconds = 30;
    return seconds * 1000UL;
}
}  // namespace

void AgentLink::begin(const AgentInfo &agent, const String &deviceId, const String &storedToken) {
    agent_ = agent;
    deviceId_ = deviceId;
    token_ = storedToken;
    agentName_ = agent.name;
    failedAttempts_ = 0;
    hasNewMetrics_ = false;
    status_ = LinkStatus::Connecting;

    ws_.onEvent([this](WStype_t type, uint8_t *payload, size_t length) { handleEvent(type, payload, length); });
    ws_.begin(agent_.ip, agent_.port, agent_.path.c_str());

    Serial.printf("espia: connecting to agent %s at %s:%u%s\n", agent_.name.c_str(), agent_.ip.toString().c_str(),
                  agent_.port, agent_.path.c_str());
}

void AgentLink::loop() {
    if (needsRediscovery()) {
        // Stop retrying this agent — `main.cpp` re-runs discovery and calls
        // begin() again (possibly with a different agent), which resets
        // failedAttempts_ and starts a fresh WebSocketsClient target.
        return;
    }
    ws_.loop();
}

bool AgentLink::hasNewMetrics() {
    bool had = hasNewMetrics_;
    hasNewMetrics_ = false;
    return had;
}

void AgentLink::handleEvent(WStype_t type, uint8_t *payload, size_t length) {
    switch (type) {
        case WStype_CONNECTED:
            Serial.println("espia: WebSocket connected, sending hello");
            status_ = LinkStatus::AwaitingWelcome;
            sendHello();
            break;

        case WStype_DISCONNECTED:
            Serial.println("espia: WebSocket disconnected");
            status_ = LinkStatus::Disconnected;
            failedAttempts_++;
            ws_.setReconnectInterval(backoffMsFor(failedAttempts_));
            break;

        case WStype_TEXT:
            handleMessage(payload, length);
            break;

        default:
            break;  // pings/pongs/binary: nothing to do yet
    }
}

void AgentLink::handleMessage(uint8_t *payload, size_t length) {
    JsonDocument doc;
    if (deserializeJson(doc, payload, length) != DeserializationError::Ok) {
        Serial.println("espia: could not parse a message from the agent");
        return;
    }

    const char *type = doc["type"] | "";

    if (strcmp(type, "welcome") == 0) {
        agentName_ = String((const char *)(doc["name"] | agent_.name.c_str()));
        status_ = LinkStatus::Connected;
        failedAttempts_ = 0;
        Serial.printf("espia: paired session established with %s\n", agentName_.c_str());
        return;
    }

    if (strcmp(type, "pair_required") == 0) {
        // Pairing isn't implemented yet (a later build step) — today's
        // agent doesn't send this either, but log it clearly in case that
        // changes before the firmware side catches up.
        Serial.println("espia: agent requires pairing, which isn't implemented yet");
        return;
    }

    if (strcmp(type, "metrics") == 0) {
        metrics_.cpuUsagePct = doc["cpu"]["usage_pct"] | 0.0f;
        metrics_.memoryUsedBytes = doc["memory"]["used_bytes"] | 0ULL;
        metrics_.memoryTotalBytes = doc["memory"]["total_bytes"] | 0ULL;
        hasNewMetrics_ = true;
        return;
    }

    if (strcmp(type, "error") == 0) {
        const char *code = doc["code"] | "unknown";
        Serial.printf("espia: agent sent error: %s\n", code);
        return;
    }

    // Unknown message types are ignored, not an error (protocol §3).
}

void AgentLink::sendHello() {
    JsonDocument doc;
    doc["v"] = 1;
    doc["type"] = "hello";
    doc["device_id"] = deviceId_;
    doc["firmware"] = ESPIA_FIRMWARE_VERSION;
    doc["board"] = ESPIA_BOARD_NAME;
    if (token_.isEmpty()) {
        doc["token"] = nullptr;
    } else {
        doc["token"] = token_;
    }

    JsonObject display = doc["display"].to<JsonObject>();
    display["width"] = ESPIA_DISPLAY_WIDTH;
    display["height"] = ESPIA_DISPLAY_HEIGHT;
    display["mono"] = static_cast<bool>(ESPIA_DISPLAY_MONO);
    display["touch"] = static_cast<bool>(ESPIA_DISPLAY_TOUCH);

    String out;
    serializeJson(doc, out);
    ws_.sendTXT(out);
}

}  // namespace net
