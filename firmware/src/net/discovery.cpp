#include "net/discovery.h"

#include <ArduinoJson.h>
#include <ESPmDNS.h>
#include <WiFi.h>
#include <WiFiUdp.h>

namespace net {
namespace discovery {

namespace {

constexpr uint16_t kDiscoveryPort = 47800;
constexpr const char *kDefaultPath = "/v1/ws";
// How long to wait after each UDP broadcast for replies before deciding to
// retry — protocol §1.2 says "wait ≥1s, retry up to 3x".
constexpr uint32_t kUdpRoundMs = 1000;
constexpr uint8_t kUdpMaxRetries = 3;

bool hasAgent(const std::vector<AgentInfo> &agents, const String &agentId) {
    for (const auto &agent : agents) {
        if (agent.agentId == agentId) return true;
    }
    return false;
}

void findViaMdns(std::vector<AgentInfo> &out) {
    static bool mdnsStarted = false;
    if (!mdnsStarted) {
        mdnsStarted = MDNS.begin("espia-device");
        if (!mdnsStarted) {
            Serial.println("espia: mDNS could not start, skipping");
            return;
        }
    }

    int count = MDNS.queryService("espia", "tcp");
    for (int i = 0; i < count; i++) {
        String agentId = MDNS.txt(i, "id");
        if (agentId.isEmpty() || hasAgent(out, agentId)) continue;

        AgentInfo agent;
        agent.agentId = agentId;
        agent.name = MDNS.txt(i, "name");
        agent.ip = MDNS.IP(i);
        agent.port = MDNS.port(i);
        agent.path = MDNS.txt(i, "path");
        if (agent.path.isEmpty()) agent.path = kDefaultPath;
        out.push_back(agent);
    }
    if (count > 0) {
        Serial.printf("espia: mDNS found %d agent(s)\n", count);
    }
}

void broadcastDiscover(WiFiUDP &udp, const String &deviceId) {
    JsonDocument doc;
    doc["v"] = 1;
    doc["type"] = "discover";
    doc["device_id"] = deviceId;

    String out;
    serializeJson(doc, out);

    udp.beginPacket(IPAddress(255, 255, 255, 255), kDiscoveryPort);
    udp.write(reinterpret_cast<const uint8_t *>(out.c_str()), out.length());
    udp.endPacket();
}

void collectUdpReplies(WiFiUDP &udp, uint32_t windowMs, std::vector<AgentInfo> &out) {
    uint32_t deadline = millis() + windowMs;
    uint8_t buf[512];
    while (millis() < deadline) {
        int size = udp.parsePacket();
        if (size <= 0) {
            delay(10);
            continue;
        }
        int len = udp.read(buf, sizeof(buf) - 1);
        if (len <= 0) continue;
        buf[len] = '\0';

        JsonDocument doc;
        if (deserializeJson(doc, buf, len) != DeserializationError::Ok) continue;
        if (doc["type"] != "announce") continue;

        String agentId = doc["agent_id"] | "";
        if (agentId.isEmpty() || hasAgent(out, agentId)) continue;

        AgentInfo agent;
        agent.agentId = agentId;
        agent.name = String(doc["name"] | "");
        agent.ip = udp.remoteIP();
        agent.port = doc["port"] | 0;
        agent.path = String((const char *)(doc["path"] | kDefaultPath));
        out.push_back(agent);
    }
}

void findViaUdp(uint32_t timeoutMs, const String &deviceId, std::vector<AgentInfo> &out) {
    WiFiUDP udp;
    if (!udp.begin(0)) {
        Serial.println("espia: could not open UDP socket for discovery");
        return;
    }

    uint32_t perRoundMs = timeoutMs / kUdpMaxRetries;
    if (perRoundMs < kUdpRoundMs) perRoundMs = kUdpRoundMs;

    for (uint8_t attempt = 0; attempt < kUdpMaxRetries; attempt++) {
        broadcastDiscover(udp, deviceId);
        collectUdpReplies(udp, perRoundMs, out);
        if (!out.empty()) break;
    }
    udp.stop();
    Serial.printf("espia: UDP discovery found %d agent(s)\n", (int)out.size());
}

}  // namespace

std::vector<AgentInfo> find(uint32_t timeoutMs, const String &deviceId) {
    std::vector<AgentInfo> agents;
    findViaMdns(agents);
    findViaUdp(timeoutMs, deviceId, agents);
    return agents;
}

}  // namespace discovery
}  // namespace net
