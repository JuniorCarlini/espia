# 0008. Build the macOS app icon as a real Liquid Glass icon

- **Status:** Accepted
- **Date:** 2026-09-24

## Context

macOS 26 (Tahoe) unified its app icon system across Apple's platforms under
"Liquid Glass": a layered format (background + glyph layers) the OS
composites at render time with dynamic specular light, shadow, and material
effects. Icons still shipped as a classic flat `.icns` (which is what Tauri
produces from a plain PNG, and what `agent/src-tauri/icons/icon.icns`
originally was) do not get this treatment — they render as a flat image
with no OS-applied shadow, margin, or material, so they visibly stand out
next to icons that do, even at the same pixel dimensions and a hand-drawn
"squircle" shape and safe-area margin (which is what espia's icon looked
like before this ADR).

Building a proper Liquid Glass icon normally means designing it in Apple's
**Icon Composer** app and building through **Xcode** — a workflow with no
place in a Tauri project. But the underlying format turns out to be usable
without either:

- Icon Composer's `.icon` bundle format (`icon.json` manifest + an `Assets/`
  folder of layer images) is a plain, hand-authorable JSON + image files —
  no GUI required to produce one.
- **`actool`**, Xcode's asset-catalog compiler, accepts a `.icon` bundle
  directly and compiles it to an `Assets.car` (what macOS 26+ actually reads
  for the Liquid Glass rendering) *and* a backward-compatible `.icns` for
  macOS 25 and earlier, in one command — no Xcode *project* needed, just the
  `actool` binary Xcode installs.
- Tauri has no built-in support for any of this, but doesn't need to: its
  existing `bundle.macOS.files` config can copy an arbitrary file
  (`Assets.car`) into `Contents/Resources`, and a `src-tauri/Info.plist`
  merges in the one extra key (`CFBundleIconName`) macOS needs to find it.

## Decision

- The source of truth is `docs/assets/espia.icon/` — a hand-authored `.icon`
  bundle (dark gradient background fill + the wordmark as a single glyph
  layer, `glass: false` since it's a flat brand mark, not meant to look
  frosted).
- `docs/assets/build-icon.sh` compiles it with `actool` and copies the
  result into `agent/src-tauri/icons/`: `Assets.car` (the Liquid Glass
  catalog) and `icon.icns` (the compiled fallback — replacing espia's
  earlier hand-drawn one, since `actool`'s own flattened rendering already
  includes the correct margin and corner curve).
- `agent/src-tauri/tauri.conf.json`'s `bundle.macOS.files` copies
  `Assets.car` into the bundle's `Resources/`; `agent/src-tauri/Info.plist`
  sets `CFBundleIconName` so macOS 26+ finds it there. Both merge with
  Tauri's own generated bundle rather than replacing it.
- Requires **Xcode** (not just the Command Line Tools) to run `actool` —
  only to rebuild the icon after a design change, not for `tauri dev` or
  ordinary `tauri build`, since the compiled output is committed like any
  other generated icon file.

## Consequences

- The Dock, Cmd+Tab, and Finder now render espia's icon with the same
  dynamic shadow and glass material every other Tahoe-updated app gets,
  instead of a flat image that stands out next to them.
- Verifying this requires an actual bundled `.app` (`tauri build`): `tauri
  dev`'s raw, unbundled binary does not register with LaunchServices the
  way a real bundle does, so it cannot show the real icon rendering.
- One more required tool (Xcode) for contributors who touch the icon,
  though not for anyone building or developing the app normally.
- The icon.json schema is Apple's, undocumented in any official reference at
  the time of writing; this was hand-authored from a real example fixture
  found in a third-party open-source project (linked below) and verified by
  actually compiling it, not by guessing from prose descriptions.
- `agent/src-tauri/icons/32x32.png`, `128x128.png`, and `128x128@2x.png` and
  `icon.ico` still come from the older plain-PNG pipeline
  (`docs/assets/icon-source.png` /`generate-icon.py`) — they cover Windows
  and the sizes Tauri's own bundler asks for directly, which have no
  Liquid Glass equivalent to target.

## Alternatives considered

- **Keep the hand-drawn flat `.icns`** (a hand-computed superellipse with
  safe-area padding, no OS material) — closer to correct than the original
  edge-to-edge artwork, but still visibly different from every
  natively-updated icon, which was the actual complaint driving this ADR.
- **Design in Icon Composer's GUI, build through a real Xcode project** —
  the "intended" workflow, but would mean maintaining a second, parallel
  Xcode project alongside the Tauri one just for icon compilation. The
  `actool`-directly approach gets the identical compiled output without it.

## References

- [Updating application icons for macOS 26 Tahoe and Liquid Glass — Successful Software](https://successfulsoftware.net/2025/09/26/updating-application-icons-for-macos-26-tahoe-and-liquid-glass/)
- [How to Export a Mac .icon File With the Proper Margins — Michael Tsai](https://mjtsai.com/blog/2025/10/02/how-to-export-a-mac-icon-file-with-the-proper-margins/)
- [icon-composer-template (icon.json fixtures) — peterpoliwoda](https://github.com/peterpoliwoda/icon-composer-template)
- [Tauri: macOS Application Bundle — `bundle.macOS.files`, Info.plist merging](https://v2.tauri.app/distribute/macos-application-bundle/)
