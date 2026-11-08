#include "net/provisioning.h"

#include <DNSServer.h>
#include <Preferences.h>
#include <WebServer.h>
#include <WiFi.h>

#include "net/agent_manager.h"
#include "net/pairing_store.h"
#include "net/portal_assets.h"

namespace net {
namespace provisioning {

namespace {

constexpr const char *kNamespace = "espia-wifi";
constexpr const char *kSsidKey = "ssid";
constexpr const char *kPasswordKey = "pass";
constexpr const char *kApSsid = "espia-setup";
constexpr uint16_t kDnsPort = 53;

DNSServer dnsServer;
WebServer server(80);

// True while running as the "espia-setup" AP; false during the bounded
// post-connect LAN config window. Only affects which IP the page and the
// not-found redirect report, and whether the DNS catch-all runs (real
// networks already have real DNS).
bool apMode = false;
bool lanConfigActive = false;
uint32_t lanConfigDeadline = 0;

// One-time code the LAN config window requires (ADR 0016). Empty outside
// that window — the AP portal needs no code, since joining the isolated
// "espia-setup" network is already the access check for that one.
String lanConfigCode;

IPAddress portalAddress() {
    return apMode ? WiFi.softAPIP() : WiFi.localIP();
}

// True unless the LAN config window is open and the request's `key` param
// doesn't match lanConfigCode — always true in AP mode and outside any
// window, since nothing there requires a code.
bool configKeyOk() {
    return !lanConfigActive || server.arg("key") == lanConfigCode;
}

// Shared by every state-changing handler (POST /connect, /agents/*) —
// ADR 0016 requires the code for all of them alike.
bool requireConfigKey() {
    if (configKeyOk()) return true;
    server.send(403, "text/plain", "Wrong or missing code.");
    return false;
}

// `key=<code>` to append to a link, or "" outside the LAN config window —
// keeps every link/form on the page working without asking for the code
// again once the visitor has already typed it in once.
String keyQuery() {
    return lanConfigActive ? "?key=" + lanConfigCode : "";
}

String keyHiddenField() {
    return lanConfigActive ? "<input type=\"hidden\" name=\"key\" value=\"" + lanConfigCode + "\">" : "";
}

// True if `text` contains a CR or LF — both are valid in an HTML form
// field but not in an HTTP header value, and `token`/`url`/`name` from
// "Add a remote agent" end up in one (net::RemoteAgent's `Authorization`
// header). Rejecting them here keeps a pasted value from smuggling extra
// headers into that outgoing request.
bool hasControlChars(const String &text) {
    for (size_t i = 0; i < text.length(); i++) {
        char c = text.charAt(i);
        if (c == '\r' || c == '\n') return true;
    }
    return false;
}

// Shown instead of the real page when the LAN config window is open and
// no valid code has been given yet (ADR 0016) — `path` is where to submit
// it (the page the visitor actually asked for).
void sendKeyGate(const String &path) {
    String page = R"HTML(<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>espia-setup</title>
<style>body{margin:0;min-height:100vh;background:#0a0a0c;color:#f5f6f2;
font-family:-apple-system,"Segoe UI",sans-serif;display:flex;align-items:center;
justify-content:center;padding:16px}
main{width:100%;max-width:320px;text-align:center}
input{width:100%;box-sizing:border-box;height:2.5rem;margin-top:12px;text-align:center;
font-size:1.1rem;border-radius:0.5rem;border:1px solid #ffffff2a;background:#17181b;color:#f5f6f2}
button{width:100%;height:2.5rem;margin-top:12px;border-radius:0.5rem;border:0;
background:#5cec01;color:#0a0a0c;font-weight:600;cursor:pointer}</style></head>
<body><main>
<p>Enter the code shown on the device's own screen.</p>
<form method="GET" action=")HTML";
    page += path;
    page += R"HTML("><input name="key" inputmode="numeric" autofocus>
<button type="submit">Continue</button></form></main></body></html>
)HTML";
    server.send(403, "text/html", page);
}

// Same field layout, copy, and Tucano styling as
// firmware/simulator/setup.html's Wi-Fi tab (ADR 0012) — vendored locally
// (net/portal_assets.h) rather than loaded from a CDN, since the device's
// isolated access point has no path to the internet. `{{ADDR}}` is filled
// in per-request with this device's own portal address.
constexpr const char *kPageHead = R"HTML(<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>espia-setup</title>
<link rel="stylesheet" href="/tucano.min.css">
<style>
  @font-face {
    font-family: 'Bitcount Grid Single';
    src: url('/bitcount.woff2') format('woff2');
    font-weight: 400;
    font-display: swap;
  }
  :root {
    --tuc-accent: #5cec01;
    --tuc-accent-hover: #4abe01;
    --tuc-accent-fg: #0a0a0c;
    --tuc-accent-text: #5cec01;
    --tuc-bg: #17181b;
    --tuc-fg: #f5f6f2;
    --tuc-muted: #9a9ca3;
    --tuc-subtle: #6b6d74;
    --tuc-border: #ffffff1a;
    --tuc-hover: #1e1f23;
    --tuc-elevated: #1e1f23;
    --tuc-success: #5cec01;
    --tuc-warning: #ff8a00;
    --tuc-danger: #ef4444;
    --tuc-danger-fill: #ef4444;
    --tuc-font: -apple-system, "Segoe UI", sans-serif;
    --tuc-radius: 1rem;
    --tuc-radius-md: 0.75rem;
    --tuc-radius-sm: 0.5rem;
  }
  body { margin:0; min-height:100vh; background:#0a0a0c; color:#f5f6f2;
    font-family:var(--tuc-font); display:flex; justify-content:center;
    padding:28px 16px 60px; }
  main { width:100%; max-width:420px; }
  .brand { display:flex; align-items:center; gap:10px; margin-bottom:4px; }
  .brand .dot { width:9px; height:9px; border-radius:999px; background:#5cec01;
    box-shadow:0 0 10px 1px #5cec0126; }
  .brand span { font-family:'Bitcount Grid Single', ui-monospace, monospace;
    font-size:1.15rem; color:#5cec01; }
  p.hint { color:#6b6d74; font-size:0.82rem; line-height:1.5; margin:4px 0 24px; }
  p.hint a { color:inherit; }
  label { display:block; font-size:0.75rem; color:#9a9ca3; margin:14px 0 6px; }
</style>
</head>
<body>
<main>
<div class="brand"><span class="dot"></span><span>espia-setup</span></div>
<p class="hint">Choose your Wi-Fi network and enter its password. The device will restart and connect once you submit. This page: <a href="http://{{ADDR}}/">{{ADDR}}</a>{{NAV}}</p>
<form method="POST" action="/connect">
{{KEYFIELD}}
<label for="ssid">Network</label>
<select id="ssid" name="ssid" class="tuc-input">
)HTML";

constexpr const char *kPageTail = R"HTML(</select>
<label for="password">Password</label>
<input type="password" id="password" name="password" class="tuc-input" autocomplete="off">
<button type="submit" class="tuc-btn is-primary is-block" style="margin-top:16px">Connect</button>
</form>
</main>
</body>
</html>
)HTML";

constexpr const char *kConnectingPage = R"HTML(<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>espia-setup</title>
<style>body{margin:0;min-height:100vh;background:#0a0a0c;color:#f5f6f2;
font-family:-apple-system,"Segoe UI",sans-serif;display:flex;align-items:center;
justify-content:center;text-align:center;padding:16px}</style></head>
<body><p>Saved. The device is restarting and will connect to your network&hellip;</p></body></html>
)HTML";

String htmlEscape(const String &text) {
    String out;
    out.reserve(text.length());
    for (size_t i = 0; i < text.length(); i++) {
        char c = text.charAt(i);
        switch (c) {
            case '&': out += "&amp;"; break;
            case '<': out += "&lt;"; break;
            case '>': out += "&gt;"; break;
            case '"': out += "&quot;"; break;
            case '\'': out += "&#39;"; break;
            default: out += c;
        }
    }
    return out;
}

void handleRoot() {
    if (!configKeyOk()) {
        sendKeyGate("/");
        return;
    }

    int found = WiFi.scanNetworks();
    String page = kPageHead;
    page.replace("{{ADDR}}", portalAddress().toString());
    page.replace("{{NAV}}", apMode ? "" : " &middot; <a href=\"/agents" + keyQuery() + "\">Agents</a>");
    page.replace("{{KEYFIELD}}", keyHiddenField());
    if (found <= 0) {
        page += "<option value=\"\">No networks found — reload to scan again</option>";
    } else {
        for (int i = 0; i < found; i++) {
            String ssid = htmlEscape(WiFi.SSID(i));
            page += "<option value=\"" + ssid + "\">" + ssid + " (" + String(WiFi.RSSI(i)) + " dBm)</option>";
        }
    }
    page += kPageTail;
    server.send(200, "text/html", page);
}

void handleConnect() {
    if (!requireConfigKey()) return;

    String ssid = server.arg("ssid");
    if (ssid.isEmpty()) {
        server.send(400, "text/plain", "Choose a network first.");
        return;
    }
    String password = server.arg("password");

    Preferences prefs;
    if (prefs.begin(kNamespace, /*readOnly=*/false)) {
        prefs.putString(kSsidKey, ssid);
        prefs.putString(kPasswordKey, password);
        prefs.end();
    }

    server.send(200, "text/html", kConnectingPage);
    server.client().flush();
    delay(500);  // let the response actually reach the phone before rebooting
    ESP.restart();
}

void handleTucanoCss() {
    // send_P (not send): the plain-`send` overload would copy all 82KB
    // into a heap `String` first — this streams straight from flash.
    server.send_P(200, "text/css", TUCANO_MIN_CSS);
}

void handleBitcountFont() {
    server.send_P(200, "font/woff2", reinterpret_cast<const char *>(BITCOUNT_WOFF2), BITCOUNT_WOFF2_LEN);
}

// Shared by the Wi-Fi and Agents pages — same brand header/fonts/tokens,
// factored out so the two don't drift out of sync visually.
constexpr const char *kSharedHead = R"HTML(<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>espia-setup</title>
<link rel="stylesheet" href="/tucano.min.css">
<style>
  @font-face { font-family:'Bitcount Grid Single'; src:url('/bitcount.woff2') format('woff2');
    font-weight:400; font-display:swap; }
  :root {
    --tuc-accent:#5cec01; --tuc-accent-hover:#4abe01; --tuc-accent-fg:#0a0a0c; --tuc-accent-text:#5cec01;
    --tuc-bg:#17181b; --tuc-fg:#f5f6f2; --tuc-muted:#9a9ca3; --tuc-subtle:#6b6d74; --tuc-border:#ffffff1a;
    --tuc-hover:#1e1f23; --tuc-elevated:#1e1f23; --tuc-success:#5cec01; --tuc-warning:#ff8a00;
    --tuc-danger:#ef4444; --tuc-danger-fill:#ef4444; --tuc-font:-apple-system,"Segoe UI",sans-serif;
    --tuc-radius:1rem; --tuc-radius-md:.75rem; --tuc-radius-sm:.5rem;
  }
  body { margin:0; min-height:100vh; background:#0a0a0c; color:#f5f6f2; font-family:var(--tuc-font);
    display:flex; justify-content:center; padding:28px 16px 60px; }
  main { width:100%; max-width:420px; }
  .brand { display:flex; align-items:center; gap:10px; margin-bottom:4px; }
  .brand .dot { width:9px; height:9px; border-radius:999px; background:#5cec01; box-shadow:0 0 10px 1px #5cec0126; }
  .brand span { font-family:'Bitcount Grid Single', ui-monospace, monospace; font-size:1.15rem; color:#5cec01; }
  h2 { font-size:0.85rem; margin:22px 0 3px; }
  p.hint { color:#6b6d74; font-size:0.8rem; line-height:1.5; margin:2px 0 12px; }
  p.hint a { color:inherit; }
  label { display:block; font-size:0.75rem; color:#9a9ca3; margin:12px 0 6px; }
  .row { display:flex; align-items:center; justify-content:space-between; gap:10px; padding:10px 12px;
    border-radius:10px; background:#17181b; margin-bottom:6px; font-size:0.85rem; }
  .row form { margin:0; }
  .tuc-btn.is-sm { height:2rem; padding:0 0.75rem; font-size:0.8rem; }
</style></head><body><main>
<div class="brand"><span class="dot"></span><span>espia-setup</span></div>
)HTML";

void handleAgents() {
    if (!configKeyOk()) {
        sendKeyGate("/agents");
        return;
    }

    String page = kSharedHead;
    page += "<p class=\"hint\">Agents this device talks to. This page: <a href=\"http://" + portalAddress().toString() +
            "/agents\">" + portalAddress().toString() + "/agents</a> &middot; <a href=\"/" + keyQuery() + "\">Wi-Fi</a></p>";

    String keyField = keyHiddenField();

    page += "<h2>Paired</h2>";
    auto pairings = pairing_store::load();
    if (pairings.empty()) {
        page += "<p class=\"hint\">None yet.</p>";
    } else {
        for (const auto &p : pairings) {
            String id = p.isRemote ? p.url : p.agentId;
            String label = p.name.isEmpty() ? id : p.name;
            page += "<div class=\"row\"><span>" + htmlEscape(label) + (p.isRemote ? " (remote)" : "") + "</span>"
                    "<form method=\"POST\" action=\"/agents/forget\">" + keyField + "<input type=\"hidden\" name=\"id\" value=\"" +
                    htmlEscape(id) + "\"><button class=\"tuc-btn is-outline is-sm\" type=\"submit\">Forget</button></form></div>";
        }
    }

    page += "<h2>Found nearby</h2>";
    auto candidates = agent_manager::unpairedCandidates();
    if (candidates.empty()) {
        page += "<p class=\"hint\">None right now — reload to scan again.</p>";
    } else {
        for (const auto &c : candidates) {
            String label = c.name.isEmpty() ? c.agentId : c.name;
            page += "<div class=\"row\"><span>" + htmlEscape(label) + "</span>"
                    "<form method=\"POST\" action=\"/agents/pair-local\">" + keyField + "<input type=\"hidden\" name=\"agentId\" value=\"" +
                    htmlEscape(c.agentId) + "\"><button class=\"tuc-btn is-primary is-sm\" type=\"submit\">Pair</button></form></div>";
        }
        page += "<p class=\"hint\">Pairing shows a code on this device's own screen — type it into the agent when asked.</p>";
    }

    page += "<h2>Add a remote agent</h2>"
            "<p class=\"hint\">Not on this network — a VPS or Docker host running the headless agent.</p>"
            "<form method=\"POST\" action=\"/agents/add-remote\">" + keyField +
            R"HTML(<label for="name">Name</label>
<input type="text" id="name" name="name" class="tuc-input" placeholder="e.g. checksafe-prod">
<label for="url">Agent URL</label>
<input type="text" id="url" name="url" class="tuc-input" placeholder="https://your-vps.example.com">
<label for="token">Token</label>
<input type="password" id="token" name="token" class="tuc-input" autocomplete="off">
<button type="submit" class="tuc-btn is-primary is-block" style="margin-top:12px">Add agent</button>
</form>
</main></body></html>
)HTML";

    server.send(200, "text/html", page);
}

void redirectToAgents() {
    server.sendHeader("Location", "/agents" + keyQuery(), true);
    server.send(303, "text/plain", "");
}

void handlePairLocal() {
    if (!requireConfigKey()) return;
    String agentId = server.arg("agentId");
    if (!agentId.isEmpty()) agent_manager::pairWithLocal(agentId);
    redirectToAgents();
}

void handleAddRemote() {
    if (!requireConfigKey()) return;

    String url = server.arg("url");
    String token = server.arg("token");
    String name = server.arg("name");
    if (name.isEmpty()) name = url;

    // url/token/name end up in an outgoing Authorization header
    // (net::RemoteAgent) — a CR or LF in any of them could smuggle extra
    // headers into that request, so reject rather than store them.
    if (hasControlChars(url) || hasControlChars(token) || hasControlChars(name)) {
        server.send(400, "text/plain", "Name, URL, and token can't contain line breaks.");
        return;
    }

    if (!url.isEmpty() && !token.isEmpty()) {
        agent_manager::addRemote(url, token, name);
    }
    redirectToAgents();
}

void handleForgetAgent() {
    if (!requireConfigKey()) return;
    String id = server.arg("id");
    if (!id.isEmpty()) agent_manager::forgetPairing(id);
    redirectToAgents();
}

// Every OS probes a specific URL to decide whether to pop up the captive
// portal (Apple: hotspot-detect.html, Android: generate_204, Windows:
// connecttest.txt/ncsi.txt) — rather than matching each one, redirecting
// every unmatched request to `/` covers all of them the same way.
void handleNotFound() {
    server.sendHeader("Location", "http://" + portalAddress().toString() + "/", true);
    server.send(302, "text/plain", "");
}

}  // namespace

bool hasStoredCredentials() {
    Preferences prefs;
    if (!prefs.begin(kNamespace, /*readOnly=*/true)) return false;
    bool has = !prefs.getString(kSsidKey, "").isEmpty();
    prefs.end();
    return has;
}

bool connectStored(uint32_t timeoutMs) {
    Preferences prefs;
    if (!prefs.begin(kNamespace, /*readOnly=*/true)) return false;
    String ssid = prefs.getString(kSsidKey, "");
    String password = prefs.getString(kPasswordKey, "");
    prefs.end();

    if (ssid.isEmpty()) return false;

    Serial.printf("espia: connecting to stored WiFi \"%s\"...\n", ssid.c_str());
    WiFi.mode(WIFI_STA);
    WiFi.begin(ssid.c_str(), password.c_str());

    uint32_t deadline = millis() + timeoutMs;
    while (WiFi.status() != WL_CONNECTED && millis() < deadline) {
        delay(100);
    }
    return WiFi.status() == WL_CONNECTED;
}

void begin() {
    apMode = true;
    WiFi.mode(WIFI_AP);
    WiFi.softAP(kApSsid);
    IPAddress apIP = WiFi.softAPIP();
    Serial.printf("espia: provisioning portal \"%s\" at %s\n", kApSsid, apIP.toString().c_str());

    dnsServer.start(kDnsPort, "*", apIP);

    server.on("/", HTTP_GET, handleRoot);
    server.on("/connect", HTTP_POST, handleConnect);
    server.on("/tucano.min.css", HTTP_GET, handleTucanoCss);
    server.on("/bitcount.woff2", HTTP_GET, handleBitcountFont);
    server.onNotFound(handleNotFound);
    server.begin();
}

void beginLanConfigWindow(uint32_t windowMs) {
    apMode = false;
    lanConfigActive = true;
    lanConfigDeadline = millis() + windowMs;

    // Same 6-digit-code shape as AgentLink's pairing code (protocol §4.2)
    // — plenty for "read it off the screen in front of you", not meant to
    // resist sustained brute-forcing (ADR 0016).
    uint32_t code = random(0, 1000000);
    char buf[7];
    snprintf(buf, sizeof(buf), "%06lu", static_cast<unsigned long>(code));
    lanConfigCode = buf;

    server.on("/", HTTP_GET, handleRoot);
    server.on("/connect", HTTP_POST, handleConnect);
    server.on("/agents", HTTP_GET, handleAgents);
    server.on("/agents/pair-local", HTTP_POST, handlePairLocal);
    server.on("/agents/add-remote", HTTP_POST, handleAddRemote);
    server.on("/agents/forget", HTTP_POST, handleForgetAgent);
    server.on("/tucano.min.css", HTTP_GET, handleTucanoCss);
    server.on("/bitcount.woff2", HTTP_GET, handleBitcountFont);
    server.onNotFound(handleNotFound);
    server.begin();

    Serial.printf("espia: temporary config page open at http://%s/ for %lus\n", WiFi.localIP().toString().c_str(),
                  static_cast<unsigned long>(windowMs / 1000));
}

bool isLanConfigWindowOpen() {
    return lanConfigActive;
}

const String &lanConfigAccessCode() {
    return lanConfigCode;
}

void poll() {
    if (apMode) {
        dnsServer.processNextRequest();
    }
    if (apMode || lanConfigActive) {
        server.handleClient();
    }
    if (lanConfigActive && millis() >= lanConfigDeadline) {
        server.stop();
        lanConfigActive = false;
        lanConfigCode = "";
        Serial.println("espia: temporary config page closed");
    }
}

void forget() {
    Preferences prefs;
    if (prefs.begin(kNamespace, /*readOnly=*/false)) {
        prefs.clear();
        prefs.end();
    }
}

}  // namespace provisioning
}  // namespace net
