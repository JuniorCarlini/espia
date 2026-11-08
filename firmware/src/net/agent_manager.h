// Orchestrates every agent this device talks to — local (WebSocket,
// `net::AgentLink`) and remote (HTTP-polled, `net::RemoteAgent`) alike
// (ADR 0007, protocol §1.3-§1.4). `main.cpp` drives this instead of a
// single link directly, so pairing with more than one agent is additive,
// not a rearchitecture.
//
// Fixed-size link arrays, not `std::vector`, deliberately — see
// net/agent_link.h's own reasoning.

#pragma once

#include <Arduino.h>

#include <vector>

#include "net/discovery.h"
#include "ui/ui.h"

namespace net {
namespace agent_manager {

constexpr uint8_t kMaxLocalLinks = 3;
constexpr uint8_t kMaxRemoteLinks = 2;

// Loads stored pairings (net::pairing_store), starts remote agents right
// away, and runs one discovery pass to reconnect any local agents already
// paired. Call once, after WiFi connects.
void begin(const String &deviceId);

// Pumps every active link, and periodically re-runs discovery to find new
// local agents (and reconnect any local link that gave up — protocol §2).
// Call every loop().
void loop();

// The device's screen shows one agent at a time — the first connected
// local link, or the first connected remote link if none is local (no
// screen-cycling yet, see firmware/README.md's roadmap).
bool isPrimaryConnected();
bool hasNewPrimaryMetrics();
const ui::MetricsView &primaryMetrics();
const String &primaryName();

// Pairing-code passthrough (protocol §4.2) from whichever local link is
// actively pairing right now, if any.
bool hasNewPairingCode();
const String &pairingCode();

// Local agents the most recent discovery scan found that this device
// hasn't paired with — what the "Agents" page offers to pair with.
std::vector<AgentInfo> unpairedCandidates();

// Starts pairing with a discovered-but-unpaired local agent — must be one
// unpairedCandidates() returned since the last scan (discovery results
// aren't persisted). No-op if every local link slot is already in use.
void pairWithLocal(const String &agentId);

// Adds (or replaces) a remote agent directly — no on-device pairing code,
// see net/remote_agent.h.
void addRemote(const String &url, const String &token, const String &name);

// Forgets a pairing, local or remote, and stops its link if active.
void forgetPairing(const String &agentIdOrUrl);

}  // namespace agent_manager
}  // namespace net
