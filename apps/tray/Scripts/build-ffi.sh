#!/bin/sh
# Builds codex-buddy-ffi for macOS and assembles CodexBuddyFFI.xcframework + the Swift bindings
# consumed by the tray app. Run from anywhere; paths are resolved relative to this script.
set -eu

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
OUT="$ROOT/apps/tray/Sources/CodexBuddyFFI"
XCFRAMEWORK="$ROOT/apps/tray/CodexBuddyFFI.xcframework"
TARGETS="aarch64-apple-darwin x86_64-apple-darwin"

cd "$ROOT"

# Track what this run actually built: lipo'ing whatever .a files happen to exist on disk could
# silently package a stale slice from an earlier run of a since-removed target.
built=""
for target in $TARGETS; do
  if rustup target list --installed | grep -q "^$target$"; then
    cargo build -p codex-buddy-ffi --release --target "$target"
    built="$built $target"
  else
    echo "skipping $target (rustup target not installed)" >&2
  fi
done
[ -n "$built" ] || {
  echo "no rustup targets installed; rustup target add aarch64-apple-darwin" >&2
  exit 1
}

# Stage everything first; the live OUT/XCFRAMEWORK are replaced only once the whole pipeline
# succeeded, so a mid-script failure can't leave the package without bindings.
stage="$ROOT/apps/tray/.ffi-stage"
rm -rf "$stage"
mkdir -p "$stage/slice/headers"

# Bindgen reads library metadata from a dylib; any built arch works, the API surface is the same.
first_target="${built# }"
first_target="${first_target%% *}"
cargo run -p codex-buddy-ffi-bindgen -- generate \
  --library "target/$first_target/release/libcodex_buddy_ffi.dylib" \
  --language swift \
  --out-dir "$stage/bindings"

# A single macOS xcframework slice can only hold one library, so multiple archs must be lipo'd
# into one universal static lib first — xcodebuild rejects two separate "macos" slices as
# "equivalent library definitions", one per arch, that's not what per-slice is for.
staticlibs=""
for target in $built; do
  staticlibs="$staticlibs target/$target/release/libcodex_buddy_ffi.a"
done
# shellcheck disable=SC2086
lipo -create $staticlibs -output "$stage/slice/libcodex_buddy_ffi.a"
cp "$stage/bindings"/*.h "$stage/slice/headers/"
cp "$stage/bindings"/*.modulemap "$stage/slice/headers/module.modulemap"

xcodebuild -create-xcframework \
  -library "$stage/slice/libcodex_buddy_ffi.a" -headers "$stage/slice/headers" \
  -output "$stage/CodexBuddyFFI.xcframework"

rm -rf "$XCFRAMEWORK" "$OUT"
mv "$stage/CodexBuddyFFI.xcframework" "$XCFRAMEWORK"
mv "$stage/bindings" "$OUT"
rm -rf "$stage"

echo "done: $XCFRAMEWORK"
echo "done: $OUT (Swift bindings)"
