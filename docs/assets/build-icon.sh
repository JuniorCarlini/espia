#!/bin/sh
# Compiles icon.icon (Apple's Icon Composer / Liquid Glass source format) into
# the files agent/src-tauri actually bundles:
#
#   agent/src-tauri/icons/Assets.car — the Liquid Glass asset catalog macOS
#     26+ reads for the dynamic glass/specular icon rendering.
#   agent/src-tauri/icons/icon.icns — a flattened, backward-compatible icon
#     for macOS 25 and earlier, generated in the same pass.
#
# Requires Xcode (not just the Command Line Tools) for `actool`, since it's
# what turns icon.icns from a plain rounded-square PNG into what Icon
# Composer actually renders (see docs/adr/0008-macos-liquid-glass-icon.md).
#
# Run this after editing docs/assets/icon.icon/, then restart `tauri dev` or
# re-run `tauri build` to pick up the result.

set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ICON_SOURCE="$SCRIPT_DIR/espia.icon"
OUT_DIR="$SCRIPT_DIR/../../agent/src-tauri/icons"
BUILD_DIR="$(mktemp -d)"
trap 'rm -rf "$BUILD_DIR"' EXIT

xcrun actool "$ICON_SOURCE" \
  --compile "$BUILD_DIR" \
  --output-format human-readable-text \
  --notices --warnings --errors \
  --output-partial-info-plist "$BUILD_DIR/partial.plist" \
  --app-icon espia \
  --include-all-app-icons \
  --enable-on-demand-resources NO \
  --development-region en \
  --target-device mac \
  --minimum-deployment-target 26.0 \
  --platform macosx

mkdir -p "$OUT_DIR"
cp "$BUILD_DIR/Assets.car" "$OUT_DIR/Assets.car"
cp "$BUILD_DIR/espia.icns" "$OUT_DIR/icon.icns"

echo "Wrote $OUT_DIR/Assets.car and $OUT_DIR/icon.icns"
