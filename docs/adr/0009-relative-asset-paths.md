# 0009. Frontend asset references must be relative, not absolute

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

A production build (`tauri build`) opened to a completely blank window —
correct background color, but no header, no cards, nothing. The identical
code ran perfectly under `tauri dev`. Neither Rust nor the JavaScript logic
was at fault: `agent/src/index.html` referenced its own script and image
with a leading slash (`/main.js`, `/assets/wordmark.png`).

Absolute root-relative paths like that resolve against whatever server is
in front of the page. `tauri dev` and `tauri build` don't serve the frontend
the same way: `tauri build` embeds `frontendDist` and serves it through the
`tauri://` custom protocol, which does not treat a leading `/` as "this
app's root" the way a real web server would — it resolves to the wrong
location, the script and stylesheet never load, and the page renders
blank. This only reproduces in a real bundled `.app`, which is why it went
unnoticed through everything built and checked under `tauri dev` in this
project so far.

(While chasing this, a resize-triggered relayout also appeared to fix a
blank window once — a coincidence, not the cause: that pointed at a
[known, unrelated WKWebView compositor bug](https://github.com/tauri-apps/wry/issues/1848)
that intermittently affects macOS 26. The actual, reproducible cause here
was the absolute paths.)

## Decision

Every asset reference in `agent/src/index.html` (`<script src>`,
`<link href>`, `<img src>`, and any added later) is relative: `./main.js`,
not `/main.js`.

## Consequences

- Verifying frontend changes work in a bundled app requires actually
  running `tauri build` and opening the resulting `.app` occasionally —
  `tauri dev` alone would not have caught this, and won't catch anything in
  the same class.
- A comment in `index.html`'s `<head>` flags the rule at the point someone
  would break it, since the failure mode (silent, blank window, only in
  release builds) is expensive to debug from scratch.

## Alternatives considered

- **Set a `<base>` tag** to make absolute paths resolve correctly — adds a
  layer of indirection to work around a footgun, rather than removing it.
