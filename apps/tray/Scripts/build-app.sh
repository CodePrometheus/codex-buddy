#!/bin/sh
# Builds "Codex Buddy.app": a release CodexBuddyTray binary + an .icns rendered from the app's own
# BuddyWordmark shapes, assembled into an LSUIElement bundle (menu-bar only, no Dock icon).
#
# Not signed or notarized — this is a local-use bundle. Gatekeeper will require a right-click →
# Open the first time on another Mac. Run Scripts/build-ffi.sh first if the xcframework is stale.
set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/build/Codex Buddy.app"
# Version and arch flags are overridable so CI can stamp the tag and build universal; the
# defaults keep a plain local `build-app.sh` fast (native arch) and working.
VERSION="${CODEX_BUDDY_VERSION:-0.2.1}"
SWIFT_ARCH_FLAGS="${SWIFT_ARCH_FLAGS:-}"

cd "$ROOT"

if [ ! -d "$ROOT/CodexBuddyFFI.xcframework" ]; then
  echo "CodexBuddyFFI.xcframework missing — run Scripts/build-ffi.sh first" >&2
  exit 1
fi

echo "==> swift build (release)"
# shellcheck disable=SC2086
swift build -c release --product CodexBuddyTray $SWIFT_ARCH_FLAGS
# Universal (--arch a --arch b) lands in .build/apple/…, not .build/release; ask SwiftPM directly.
# shellcheck disable=SC2086
BUILD="$(swift build -c release --product CodexBuddyTray $SWIFT_ARCH_FLAGS --show-bin-path)"

echo "==> rendering app icon"
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
PNG1024="$(mktemp -d)/icon-1024.png"
"$BUILD/CodexBuddyTray" --render-icon "$PNG1024"

# Standard macOS iconset: each logical size at 1x and 2x.
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$PNG1024" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  d=$((size * 2))
  sips -z "$d" "$d" "$PNG1024" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
ICNS="$(mktemp -d)/AppIcon.icns"
iconutil -c icns "$ICONSET" -o "$ICNS"

echo "==> assembling bundle"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$BUILD/CodexBuddyTray" "$APP/Contents/MacOS/CodexBuddyTray"
cp "$ICNS" "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key><string>Codex Buddy</string>
	<key>CFBundleDisplayName</key><string>Codex Buddy</string>
	<key>CFBundleExecutable</key><string>CodexBuddyTray</string>
	<key>CFBundleIdentifier</key><string>dev.codeprometheus.codex-buddy.tray</string>
	<key>CFBundlePackageType</key><string>APPL</string>
	<key>CFBundleShortVersionString</key><string>$VERSION</string>
	<key>CFBundleVersion</key><string>$VERSION</string>
	<key>CFBundleIconFile</key><string>AppIcon</string>
	<key>LSMinimumSystemVersion</key><string>13.0</string>
	<key>LSUIElement</key><true/>
	<key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# Ad-hoc sign so the bundle runs without "damaged app" prompts on the build machine.
codesign --force --deep --sign - "$APP" 2>/dev/null || true

echo "==> done: $APP"
