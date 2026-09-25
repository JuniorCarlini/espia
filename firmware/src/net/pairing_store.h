// Persists this device's pairing tokens across reboots (protocol §4.2,
// ADR 0007) — a list, not a single slot, so pairing with more than one
// agent later doesn't need this storage rearchitected.

#pragma once

#include <Arduino.h>

#include <vector>

namespace net {
namespace pairing_store {

struct Pairing {
    String agentId;
    String name;
    String token;
};

// Loads every stored pairing from flash.
std::vector<Pairing> load();

// Returns the stored token for `agentId`, or an empty string if this
// device hasn't paired with it (yet, or ever).
String tokenFor(const String &agentId);

// Saves (or replaces, if `agentId` already had one) a pairing.
void save(const String &agentId, const String &name, const String &token);

}  // namespace pairing_store
}  // namespace net
