#include "net/pairing_store.h"

#include <ArduinoJson.h>
#include <Preferences.h>

namespace net {
namespace pairing_store {

namespace {
constexpr const char *kNamespace = "espia-pair";
constexpr const char *kListKey = "list";
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
        pairing.agentId = String((const char *)(entry["agentId"] | ""));
        pairing.name = String((const char *)(entry["name"] | ""));
        pairing.token = String((const char *)(entry["token"] | ""));
        if (!pairing.agentId.isEmpty() && !pairing.token.isEmpty()) {
            pairings.push_back(pairing);
        }
    }
    return pairings;
}

String tokenFor(const String &agentId) {
    for (const auto &pairing : load()) {
        if (pairing.agentId == agentId) return pairing.token;
    }
    return "";
}

void save(const String &agentId, const String &name, const String &token) {
    std::vector<Pairing> pairings = load();

    bool replaced = false;
    for (auto &pairing : pairings) {
        if (pairing.agentId == agentId) {
            pairing.name = name;
            pairing.token = token;
            replaced = true;
            break;
        }
    }
    if (!replaced) {
        pairings.push_back({agentId, name, token});
    }

    JsonDocument doc;
    JsonArray array = doc.to<JsonArray>();
    for (const auto &pairing : pairings) {
        JsonObject entry = array.add<JsonObject>();
        entry["agentId"] = pairing.agentId;
        entry["name"] = pairing.name;
        entry["token"] = pairing.token;
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

}  // namespace pairing_store
}  // namespace net
