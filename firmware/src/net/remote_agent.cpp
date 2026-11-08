#include "net/remote_agent.h"

#include <ArduinoJson.h>
#include <HTTPClient.h>
#include <WiFiClientSecure.h>

namespace net {

namespace {
constexpr uint32_t kPollIntervalMs = 2000;
}  // namespace

void RemoteAgent::begin(const String &url, const String &token, const String &name) {
    url_ = url;
    token_ = token;
    name_ = name;
    connected_ = false;
    nextPollAt_ = 0;  // poll right away
}

void RemoteAgent::loop() {
    if (millis() < nextPollAt_) return;
    nextPollAt_ = millis() + kPollIntervalMs;
    poll();
}

void RemoteAgent::poll() {
    if (url_.isEmpty()) return;

    String endpoint = url_;
    if (!endpoint.endsWith("/")) endpoint += "/";
    endpoint += "metrics";

    HTTPClient http;
    WiFiClientSecure secureClient;
    bool began;
    if (endpoint.startsWith("https://")) {
        // Protocol v1 has no TLS on the local WebSocket path either
        // (protocol §3, SECURITY.md) — skipping certificate validation
        // here is the same already-accepted trade-off, not a new one, and
        // avoids needing a CA bundle this board's flash can't spare.
        secureClient.setInsecure();
        began = http.begin(secureClient, endpoint);
    } else {
        began = http.begin(endpoint);
    }
    if (!began) {
        connected_ = false;
        return;
    }

    http.addHeader("Authorization", "Bearer " + token_);
    int code = http.GET();
    if (code != 200) {
        Serial.printf("espia: remote agent \"%s\" returned HTTP %d\n", name_.c_str(), code);
        connected_ = false;
        http.end();
        return;
    }

    JsonDocument doc;
    // agent/headless's actual /metrics shape: `MetricsResponse` flattens
    // its `system: SystemMetrics` field (#[serde(flatten)]), so cpu/memory/
    // disks/network sit at the top level, sibling to `top_processes` — not
    // nested under a "system" key.
    DeserializationError error = deserializeJson(doc, http.getStream());
    http.end();
    if (error != DeserializationError::Ok) {
        Serial.printf("espia: could not parse metrics from remote agent \"%s\"\n", name_.c_str());
        connected_ = false;
        return;
    }

    metrics_.cpuUsagePct = doc["cpu"]["usage_pct"] | 0.0f;
    metrics_.memoryUsedBytes = doc["memory"]["used_bytes"] | 0ULL;
    metrics_.memoryTotalBytes = doc["memory"]["total_bytes"] | 0ULL;
    hasNewMetrics_ = true;
    connected_ = true;
}

bool RemoteAgent::hasNewMetrics() {
    bool had = hasNewMetrics_;
    hasNewMetrics_ = false;
    return had;
}

}  // namespace net
