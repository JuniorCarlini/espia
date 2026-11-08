// Persists this device's pairing tokens across reboots (protocol §4.2,
// ADR 0007) — a list, not a single slot, so pairing with more than one
// agent later doesn't need this storage rearchitected.

#pragma once

#include <Arduino.h>

#include <vector>

namespace net {
namespace pairing_store {

struct Pairing {
    bool isRemote = false;
    String agentId;  // local pairings only — from the WebSocket handshake.
    String name;
    String token;
    String url;  // remote pairings only (protocol §1.4) — empty for local.
};

// Loads every stored pairing from flash — local (WebSocket) and remote
// (HTTP-polled) alike.
std::vector<Pairing> load();

// Returns the stored token for a local agent's `agentId`, or an empty
// string if this device hasn't paired with it (yet, or ever).
String tokenFor(const String &agentId);

// Saves (or replaces, if `agentId` already had one) a local pairing.
void save(const String &agentId, const String &name, const String &token);

// Saves (or replaces, if `url` already had one) a remote agent — added
// directly by URL and token (protocol §1.4), no on-device pairing code:
// both are typed by a human who already has them, not read off the
// device's screen.
void saveRemote(const String &url, const String &name, const String &token);

// Forgets one pairing, local or remote, by whichever of agentId/url
// identifies it.
void remove(const String &agentIdOrUrl);

}  // namespace pairing_store
}  // namespace net
