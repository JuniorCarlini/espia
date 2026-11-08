#include "net/pairing_store.h"

#include <ArduinoJson.h>
#include <Preferences.h>

namespace net {
namespace pairing_store {

namespace {
constexpr const char *kNamespace = "espia-pair";
constexpr const char *kListKey = "list";

// The identity key: `url` for a remote pairing, `agentId` for a local one
// — matches whichever field `remove()`/the save functions key off of.
const String &keyOf(const Pairing &pairing) {
    return pairing.isRemote ? pairing.url : pairing.agentId;
}

void writeAll(const std::vector<Pairing> &pairings) {
    JsonDocument doc;
    JsonArray array = doc.to<JsonArray>();
    for (const auto &pairing : pairings) {
        JsonObject entry = array.add<JsonObject>();
        entry["remote"] = pairing.isRemote;
        entry["agentId"] = pairing.agentId;
        entry["name"] = pairing.name;
        entry["token"] = pairing.token;
        entry["url"] = pairing.url;
    }

    String out;
    serializeJson(doc, out);

    Preferences prefs;
    if (!prefs.begin(kNamespace, /*readOnly=*/false)) {
        Serial.println("espia: could not open flash storage to save a pairing");
        return;
    }
    prefs.putString(kListKey, out);
    prefs.end();
}

void upsert(const Pairing &pairing) {
    std::vector<Pairing> pairings = load();

    bool replaced = false;
    for (auto &existing : pairings) {
        if (existing.isRemote == pairing.isRemote && keyOf(existing) == keyOf(pairing)) {
            existing = pairing;
            replaced = true;
            break;
        }
    }
    if (!replaced) pairings.push_back(pairing);

    writeAll(pairings);
}

}  // namespace

std::vector<Pairing> load() {
    std::vector<Pairing> pairings;

    Preferences prefs;
    if (!prefs.begin(kNamespace, /*readOnly=*/true)) {
        return pairings;  // nothing stored yet — not an error
    }
    String raw = prefs.getString(kListKey, "");
    prefs.end();

    if (raw.isEmpty()) return pairings;

    JsonDocument doc;
    if (deserializeJson(doc, raw) != DeserializationError::Ok) {
        Serial.println("espia: stored pairings were corrupt, ignoring");
        return pairings;
    }

    for (JsonObject entry : doc.as<JsonArray>()) {
        Pairing pairing;
        pairing.isRemote = entry["remote"] | false;
        pairing.agentId = String((const char *)(entry["agentId"] | ""));
        pairing.name = String((const char *)(entry["name"] | ""));
        pairing.token = String((const char *)(entry["token"] | ""));
        pairing.url = String((const char *)(entry["url"] | ""));
        bool hasIdentity = pairing.isRemote ? !pairing.url.isEmpty() : !pairing.agentId.isEmpty();
        if (hasIdentity && !pairing.token.isEmpty()) {
            pairings.push_back(pairing);
        }
    }
    return pairings;
}

String tokenFor(const String &agentId) {
    for (const auto &pairing : load()) {
        if (!pairing.isRemote && pairing.agentId == agentId) return pairing.token;
    }
    return "";
}

void save(const String &agentId, const String &name, const String &token) {
    Pairing pairing;
    pairing.isRemote = false;
    pairing.agentId = agentId;
    pairing.name = name;
    pairing.token = token;
    upsert(pairing);
}

void saveRemote(const String &url, const String &name, const String &token) {
    Pairing pairing;
    pairing.isRemote = true;
    pairing.name = name;
    pairing.token = token;
    pairing.url = url;
    upsert(pairing);
}

void remove(const String &agentIdOrUrl) {
    std::vector<Pairing> pairings = load();
    std::vector<Pairing> kept;
    kept.reserve(pairings.size());
    for (const auto &pairing : pairings) {
        if (keyOf(pairing) != agentIdOrUrl) kept.push_back(pairing);
    }
    writeAll(kept);
}

}  // namespace pairing_store
}  // namespace net
