#!/bin/sh
# Builds codex-buddy-ffi for macOS and assembles CodexBuddyFFI.xcframework + the Swift bindings
# consumed by the tray app. Run from anywhere; paths are resolved relative to this script.
set -eu

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
OUT="$ROOT/apps/tray/Sources/CodexBuddyFFI"
XCFRAMEWORK="$ROOT/apps/tray/CodexBuddyFFI.xcframework"
TARGETS="aarch64-apple-darwin x86_64-apple-darwin"

cd "$ROOT"
rm -rf "$XCFRAMEWORK" "$OUT"
mkdir -p "$OUT"

for target in $TARGETS; do
  if rustup target list --installed | grep -q "^$target$"; then
    cargo build -p codex-buddy-ffi --release --target "$target"
  else
    echo "skipping $target (rustup target not installed)" >&2
  fi
done

# Bindgen reads library metadata from a dylib; any built arch works, the API surface is the same.
first_target=""
for target in $TARGETS; do
  lib="target/$target/release/libcodex_buddy_ffi.dylib"
  if [ -f "$lib" ]; then
    first_target="$target"
    break
  fi
done
[ -n "$first_target" ] || {
  echo "no built libcodex_buddy_ffi.dylib found; did the cargo build above fail?" >&2
  exit 1
}
cargo run -p codex-buddy-ffi-bindgen -- generate \
  --library "target/$first_target/release/libcodex_buddy_ffi.dylib" \
  --language swift \
  --out-dir "$OUT"

# A single macOS xcframework slice can only hold one library, so multiple archs must be lipo'd
# into one universal static lib first — xcodebuild rejects two separate "macos" slices as
# "equivalent library definitions", one per arch, that's not what per-slice is for.
staticlibs=""
for target in $TARGETS; do
  lib="target/$target/release/libcodex_buddy_ffi.a"
  [ -f "$lib" ] && staticlibs="$staticlibs $lib"
done
[ -n "$staticlibs" ] || {
  echo "no static libs to package into the xcframework" >&2
  exit 1
}
slice="$ROOT/apps/tray/.xcframework-slice"
rm -rf "$slice"
mkdir -p "$slice/headers"
# shellcheck disable=SC2086
lipo -create $staticlibs -output "$slice/libcodex_buddy_ffi.a"
cp "$OUT"/*.h "$slice/headers/"
cp "$OUT"/*.modulemap "$slice/headers/module.modulemap"

xcodebuild -create-xcframework \
  -library "$slice/libcodex_buddy_ffi.a" -headers "$slice/headers" \
  -output "$XCFRAMEWORK"
rm -rf "$slice"

echo "done: $XCFRAMEWORK"
echo "done: $OUT (Swift bindings)"
