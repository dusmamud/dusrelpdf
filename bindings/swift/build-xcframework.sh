#!/usr/bin/env bash
# Build the DusrelpdfFFI XCFramework for release distribution.
#
# macOS ONLY — run from the repo root:
#   bash bindings/swift/build-xcframework.sh
#
# What it does:
#   1. Builds the Rust UniFFI crate as a static library (.a) for each
#      Apple slice with `cargo rustc --crate-type staticlib`:
#        iOS device:      aarch64-apple-ios
#        iOS simulator:   aarch64-apple-ios-sim + x86_64-apple-ios-sim (lipo'd fat)
#        macOS:           aarch64-apple-macos + x86_64-apple-macos     (lipo'd fat)
#   2. Stages the C headers (dusrelpdf_uniffiFFI.h + module.modulemap).
#   3. Assembles DusrelpdfFFI.xcframework with `xcodebuild -create-xcframework`.
#
# Output: bindings/swift/DusrelpdfFFI.xcframework (gitignored — a release
# artifact, NOT committed). Zip it and upload it to the tag's GitHub
# Release; then compute its checksum and fill in Package.swift
# (see bindings/swift/README.md, "Releasing the XCFramework").
#
# BUILD_LIBRARY_FOR_DISTRIBUTION=YES: this script builds only C static
# libraries via cargo — the flag has no effect on `xcodebuild
# -create-xcframework` itself. It IS required whenever the Swift module
# is built for distribution as a prebuilt binary (library evolution /
# .swiftinterface): Xcode projects that archive the Dusrelpdf Swift module
# for distribution must enable it (Xcode: Build Settings →
# Build Libraries for Distribution → YES). The generated Swift bindings
# here are distributed as source through SwiftPM, so the flag is not
# needed to assemble this XCFramework — it is documented here so the
# release step does not forget it if the bindings are ever shipped as a
# binary Swift framework.
#
# Requirements (macOS): rustup with the five targets below, Xcode
# (xcodebuild, lipo). No Android/NDK tooling needed.
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "error: this script runs on macOS only (it builds iOS/macOS slices and uses xcodebuild/lipo)." >&2
    exit 1
fi

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT_DIR="$REPO_ROOT/bindings/swift"
BUILD_DIR="$(mktemp -d)"
trap 'rm -rf "$BUILD_DIR"' EXIT

IOS_DEVICE="aarch64-apple-ios"
IOS_SIM_ARM="aarch64-apple-ios-sim"
IOS_SIM_X86="x86_64-apple-ios-sim"
MACOS_ARM="aarch64-apple-macos"
MACOS_X86="x86_64-apple-macos"

echo "Installing Rust targets..."
rustup target add "$IOS_DEVICE" "$IOS_SIM_ARM" "$IOS_SIM_X86" "$MACOS_ARM" "$MACOS_X86"

build_staticlib() {
    local target="$1"
    echo "Building staticlib for $target ..."
    (cd "$REPO_ROOT" && cargo rustc -p dusrelpdf-uniffi --target "$target" --release --crate-type staticlib)
}

# Static .a for each slice (cargo rustc --crate-type staticlib overrides the
# crate's cdylib+rlib declaration without touching Cargo.toml).
build_staticlib "$IOS_DEVICE"
build_staticlib "$IOS_SIM_ARM"
build_staticlib "$IOS_SIM_X86"
build_staticlib "$MACOS_ARM"
build_staticlib "$MACOS_X86"

lib_for() { echo "$REPO_ROOT/target/$1/release/libdusrelpdf_uniffi.a"; }

# Simulator and macOS slices are fat (arm64 + x86_64) via lipo.
echo "Merging simulator slices with lipo..."
lipo -create -output "$BUILD_DIR/libdusrelpdf_uniffi-ios-sim.a" \
    "$(lib_for "$IOS_SIM_ARM")" "$(lib_for "$IOS_SIM_X86")"

echo "Merging macOS slices with lipo..."
lipo -create -output "$BUILD_DIR/libdusrelpdf_uniffi-macos.a" \
    "$(lib_for "$MACOS_ARM")" "$(lib_for "$MACOS_X86")"

# Headers for the XCFramework: the generated C header, and the generated
# modulemap renamed to module.modulemap (the name Xcode expects inside
# an XCFramework's Headers directory).
echo "Staging headers..."
mkdir -p "$BUILD_DIR/Headers"
cp "$REPO_ROOT/bindings/swift/dusrelpdf_uniffiFFI.h" "$BUILD_DIR/Headers/"
cp "$REPO_ROOT/bindings/swift/dusrelpdf_uniffiFFI.modulemap" "$BUILD_DIR/Headers/module.modulemap"

echo "Creating XCFramework..."
xcodebuild -create-xcframework \
    -library "$(lib_for "$IOS_DEVICE")" -headers "$BUILD_DIR/Headers" \
    -library "$BUILD_DIR/libdusrelpdf_uniffi-ios-sim.a" -headers "$BUILD_DIR/Headers" \
    -library "$BUILD_DIR/libdusrelpdf_uniffi-macos.a" -headers "$BUILD_DIR/Headers" \
    -output "$OUT_DIR/DusrelpdfFFI.xcframework"

echo
echo "Built: $OUT_DIR/DusrelpdfFFI.xcframework"
echo
echo "Next steps (release):"
echo "  1. cd bindings/swift && zip -r DusrelpdfFFI.xcframework.zip DusrelpdfFFI.xcframework"
echo "  2. Upload the zip to the tag's GitHub Release (e.g. v0.1.0)."
echo "  3. swift package compute-checksum DusrelpdfFFI.xcframework.zip"
echo "  4. Paste the real URL + checksum into Package.swift, set useLocalFFI = false, tag."
echo "     (See bindings/swift/README.md — never ship the PLACEHOLDER values.)"
