# 0016. An access code for the post-connect LAN config page

- **Status:** Accepted
- **Date:** 2026-10-02

## Context

[ADR 0015](0015-device-provisioning.md) gave the device a small web server
on its own LAN address, open for a bounded window after every boot, so a
human can add more agent pairings (ADR 0007) or fix a bad WiFi password
without going back through the AP portal. That ADR's own "Consequences"
section flagged the gap it was deliberately leaving open:

> The device is now, itself, a small web server for as long as it's
> running (...) a follow-up ADR should cover that page's own access
> control before firmware implementation starts.

The firmware implementing that window shipped with no access control at
all: `/connect` (change the stored WiFi network and reboot) and every
`/agents/*` endpoint (pair with a local agent, add a remote one by URL
and token, forget a pairing) accepted any request from anyone who could
reach the device's LAN IP, for as long as the window stayed open. That is
a real gap against this project's own stated threat model
([SECURITY.md](../../SECURITY.md)), which lists "preventing other
computers on the network from taking over a paired device" as explicitly
in scope — unlike the local WebSocket protocol's lack of TLS, which that
same document already accepts as out of scope for v1. Concretely: anyone
else on the same WiFi during that window — or a malicious page open in a
browser on that network, via a plain auto-submitting form, no scripting
needed — could rewrite the device's WiFi credentials, or add an
attacker-controlled "remote agent" that the device would then poll every
few seconds and send its bearer token to indefinitely.

The device has no keyboard and no persistent display content beyond
whatever screen is currently showing — the same constraint ADR 0015
already built around — so whatever access control this page gets has to
work within that.

## Decision

`beginLanConfigWindow()` generates a random 6-digit code each time the
window opens (same shape as the existing local-pairing code, protocol
§4.2) and shows it on the device's own screen, next to the page's
address, for the same few seconds that address is already shown. Every
request that changes state — `POST /connect`, `/agents/pair-local`,
`/agents/add-remote`, `/agents/forget` — must include that code as a
`key` parameter, matched with a plain equality check against the
in-memory value; a missing or wrong code gets a 403. The two read-only
pages (`GET /` and `GET /agents`) require it too, so an unauthenticated
visitor can't even see the WiFi scan or the list of paired agents — they
get a small "enter the code" form instead. Once a visitor's browser has
given the right code once, every link and form on the real pages carries
it forward automatically (as a hidden field or a `?key=` query parameter),
so it's only typed once per visit.

The AP-mode captive portal (`begin()`, first-time setup) keeps requiring
no code: joining the isolated "espia-setup" network is already the
access check for that one, there's nothing else on it to protect, and
requiring a code there would mean showing it on a screen the user hasn't
found yet (they're still looking for the network name).

This is a shared-secret-read-off-a-screen model, not cryptographic
pairing — it stops a passive bystander on the network and a drive-by
CSRF form, which is what the realistic threat here looks like, not a
determined attacker who can also see the device's screen. 1,000,000
possible codes and a window measured in minutes make brute-forcing
impractical without also being right there watching the device.

## Consequences

- Closes the gap ADR 0015 flagged: the page's own access control now
  exists, matching the "preventing other computers on the network from
  taking over a paired device" promise in SECURITY.md.
- One more thing to read off the screen during the window — consistent
  with the address already being shown there (ADR 0015), not a new kind
  of interaction.
- Still not a defense against an attacker who can also see the device's
  screen (shoulder-surfing) or who already controls the network path
  closely enough to watch the phone's traffic for the code in the `key`
  query string — plain HTTP, same already-accepted trade-off as the rest
  of this page and the local WebSocket protocol. Out of scope here, same
  as it is for ADR 0015's page as a whole.
- `/agents/add-remote` also now rejects a name, URL, or token containing
  a CR or LF, closing a header-injection path into the outgoing
  `Authorization` request `net::RemoteAgent` makes — found while adding
  the code check, fixed alongside it since it touches the same handlers.

## Alternatives considered

- **A fixed PIN set during AP-mode setup, reused for every later LAN
  window** — rejected: one more thing to remember long-term, and a code
  that never changes is worth more to capture than one that expires in
  minutes.
- **Requiring physical confirmation (the board's existing button) for
  every sensitive request** — too disruptive for "add a second agent"
  being a routine action, and the button is already the forget-WiFi
  gesture (ADR 0015); overloading it further risks confusing the two.
- **Doing nothing and documenting the window as a known risk** — this is
  the gap SECURITY.md already treats as in scope to prevent, not an
  accepted trade-off like plaintext local WebSocket traffic; leaving it
  open contradicts what the project already promises.
