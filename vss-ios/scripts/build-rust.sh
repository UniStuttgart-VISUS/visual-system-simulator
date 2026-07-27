#!/bin/sh
set -eu
cd "$SRCROOT/.."
case "$PLATFORM_NAME" in
  iphoneos) target=aarch64-apple-ios ;;
  iphonesimulator) target=aarch64-apple-ios-sim ;;
  *) echo "Unsupported platform: $PLATFORM_NAME" >&2; exit 1 ;;
esac
profile=debug
if [ "$CONFIGURATION" = Release ]; then profile=release; release=--release; else release=; fi
cargo build -p vss-ios --target "$target" $release
mkdir -p "$SRCROOT/build/rust/$PLATFORM_NAME"
cp "target/$target/$profile/libvss_ios.a" "$SRCROOT/build/rust/$PLATFORM_NAME/"

