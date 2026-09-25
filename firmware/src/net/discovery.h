// Agent discovery (protocol/README.md §1.1-§1.2): mDNS, with a UDP
// broadcast fallback for networks mDNS can't reach.

#pragma once

#include <Arduino.h>
#include <IPAddress.h>

#include <vector>

namespace net {

struct AgentInfo {
    String agentId;
    String name;
    IPAddress ip;
    uint16_t port = 0;
    String path;
};

namespace discovery {

// Tries mDNS first, then always also runs the UDP broadcast fallback
// regardless of the mDNS result — known arduino-esp32 mDNS flakiness means
// "try both", not "fall back only on failure" (protocol §1.1-§1.2). Spends
// up to `timeoutMs` total. Returns every agent found, not just the first
// (a device may end up paired with several — ADR 0007), deduplicated by
// agent id.
std::vector<AgentInfo> find(uint32_t timeoutMs, const String &deviceId);

}  // namespace discovery

}  // namespace net
