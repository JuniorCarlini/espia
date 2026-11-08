#include "net/agent_manager.h"

#include "net/agent_link.h"
#include "net/pairing_store.h"
#include "net/remote_agent.h"

namespace net {
namespace agent_manager {

namespace {

// How often to re-run discovery once at least one reconcile has already
// happened — finds newly-added local agents and reconnects any local link
// that gave up on its current agent (protocol §2's "re-discover after 3
// failed attempts").
constexpr uint32_t kRescanIntervalMs = 15000;

String deviceId;

AgentLink localLinks[kMaxLocalLinks];
bool localInUse[kMaxLocalLinks] = {};

RemoteAgent remoteLinks[kMaxRemoteLinks];
bool remoteInUse[kMaxRemoteLinks] = {};

std::vector<AgentInfo> lastScan;
uint32_t nextRescanAt = 0;

String primaryName_;
ui::MetricsView primaryMetrics_;
bool hasNewPrimaryMetrics_ = false;

String pairingCode_;
bool hasNewPairingCode_ = false;

// Local link index already handling `agentId`, or -1.
int localSlotFor(const String &agentId) {
    for (uint8_t i = 0; i < kMaxLocalLinks; i++) {
        if (localInUse[i] && localLinks[i].agentId() == agentId) return i;
    }
    return -1;
}

int freeLocalSlot() {
    for (uint8_t i = 0; i < kMaxLocalLinks; i++) {
        if (!localInUse[i]) return i;
    }
    return -1;
}

int freeRemoteSlot() {
    for (uint8_t i = 0; i < kMaxRemoteLinks; i++) {
        if (!remoteInUse[i]) return i;
    }
    return -1;
}

// Connects every already-paired agent discovery just found (skipping ones
// already connected), and reconnects any link that gave up — leaves
// still-unpaired discoveries alone for unpairedCandidates() to surface.
void reconcileLocal() {
    lastScan = discovery::find(4000, deviceId);

    for (const auto &agent : lastScan) {
        int slot = localSlotFor(agent.agentId);
        if (slot >= 0 && !localLinks[slot].needsRediscovery()) {
            continue;  // already connected (or actively retrying) fine
        }

        String token = pairing_store::tokenFor(agent.agentId);
        if (token.isEmpty()) continue;  // not paired — a candidate, not ours to connect

        if (slot < 0) slot = freeLocalSlot();
        if (slot < 0) continue;  // no room

        localLinks[slot].begin(agent, deviceId, token);
        localInUse[slot] = true;
    }
}

void startStoredRemotes() {
    for (const auto &pairing : pairing_store::load()) {
        if (!pairing.isRemote) continue;
        int slot = freeRemoteSlot();
        if (slot < 0) break;
        remoteLinks[slot].begin(pairing.url, pairing.token, pairing.name);
        remoteInUse[slot] = true;
    }
}

}  // namespace

void begin(const String &deviceIdIn) {
    deviceId = deviceIdIn;
    startStoredRemotes();
    reconcileLocal();
    nextRescanAt = millis() + kRescanIntervalMs;
}

void loop() {
    for (uint8_t i = 0; i < kMaxLocalLinks; i++) {
        if (localInUse[i]) localLinks[i].loop();
    }
    for (uint8_t i = 0; i < kMaxRemoteLinks; i++) {
        if (remoteInUse[i]) remoteLinks[i].loop();
    }

    int primaryLocal = -1;
    for (uint8_t i = 0; i < kMaxLocalLinks; i++) {
        if (localInUse[i] && localLinks[i].isConnected()) {
            primaryLocal = i;
            break;
        }
    }
    int primaryRemote = -1;
    if (primaryLocal < 0) {
        for (uint8_t i = 0; i < kMaxRemoteLinks; i++) {
            if (remoteInUse[i] && remoteLinks[i].isConnected()) {
                primaryRemote = i;
                break;
            }
        }
    }

    if (primaryLocal >= 0) {
        primaryName_ = localLinks[primaryLocal].agentName();
        if (localLinks[primaryLocal].hasNewMetrics()) {
            primaryMetrics_ = localLinks[primaryLocal].lastMetrics();
            hasNewPrimaryMetrics_ = true;
        }
    } else if (primaryRemote >= 0) {
        primaryName_ = remoteLinks[primaryRemote].name();
        if (remoteLinks[primaryRemote].hasNewMetrics()) {
            primaryMetrics_ = remoteLinks[primaryRemote].lastMetrics();
            hasNewPrimaryMetrics_ = true;
        }
    } else {
        primaryName_ = "";
    }

    for (uint8_t i = 0; i < kMaxLocalLinks; i++) {
        if (localInUse[i] && localLinks[i].hasNewPairingCode()) {
            pairingCode_ = localLinks[i].pairingCode();
            hasNewPairingCode_ = true;
        }
    }

    if (millis() >= nextRescanAt) {
        reconcileLocal();
        nextRescanAt = millis() + kRescanIntervalMs;
    }
}

bool isPrimaryConnected() {
    return !primaryName_.isEmpty();
}

bool hasNewPrimaryMetrics() {
    bool had = hasNewPrimaryMetrics_;
    hasNewPrimaryMetrics_ = false;
    return had;
}

const ui::MetricsView &primaryMetrics() {
    return primaryMetrics_;
}

const String &primaryName() {
    return primaryName_;
}

bool hasNewPairingCode() {
    bool had = hasNewPairingCode_;
    hasNewPairingCode_ = false;
    return had;
}

const String &pairingCode() {
    return pairingCode_;
}

std::vector<AgentInfo> unpairedCandidates() {
    std::vector<AgentInfo> out;
    for (const auto &agent : lastScan) {
        if (localSlotFor(agent.agentId) >= 0) continue;
        if (!pairing_store::tokenFor(agent.agentId).isEmpty()) continue;
        out.push_back(agent);
    }
    return out;
}

void pairWithLocal(const String &agentId) {
    for (const auto &agent : lastScan) {
        if (agent.agentId != agentId) continue;
        int slot = freeLocalSlot();
        if (slot < 0) return;  // no room
        localLinks[slot].begin(agent, deviceId, "");
        localInUse[slot] = true;
        return;
    }
}

void addRemote(const String &url, const String &token, const String &name) {
    pairing_store::saveRemote(url, name, token);
    int slot = freeRemoteSlot();
    if (slot < 0) return;  // saved either way; starts on next boot once a slot frees
    remoteLinks[slot].begin(url, token, name);
    remoteInUse[slot] = true;
}

void forgetPairing(const String &agentIdOrUrl) {
    pairing_store::remove(agentIdOrUrl);

    int local = localSlotFor(agentIdOrUrl);
    if (local >= 0) {
        localLinks[local].stop();
        localInUse[local] = false;
    }

    for (uint8_t i = 0; i < kMaxRemoteLinks; i++) {
        if (remoteInUse[i] && remoteLinks[i].url() == agentIdOrUrl) {
            remoteInUse[i] = false;
        }
    }
}

}  // namespace agent_manager
}  // namespace net
