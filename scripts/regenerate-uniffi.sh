#!/usr/bin/env bash
# Regenerate the Kotlin and Swift UniFFI bindings for dusrelpdf.
#
# Usage:
#   scripts/regenerate-uniffi.sh           regenerate bindings/ in place
#   scripts/regenerate-uniffi.sh --check   generate to a temp dir and diff
#                                           against the committed bindings;
#                                           exit non-zero on drift (for CI)
#
# Requires UniFFI 0.28.3. The `uniffi-bindgen` binary is not published
# as a cargo-installable crate in 0.28.x, so this script builds the CLI
# from a small throwaway crate (cached at
# $HOME/.cache/dusrelpdf-uniffi-bindgen/uniffi-cli) when it is missing.
# Set UNIFFI_BINDGEN to the path of an existing 0.28.3 bindgen binary
# to skip that step. Generation is deterministic: re-running this script
# on an unchanged crate produces byte-identical output.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

UNIFFI_VERSION="0.28.3"
CLI_BIN="${UNIFFI_BINDGEN:-$HOME/.cache/dusrelpdf-uniffi-bindgen/uniffi-cli}"

if [[ ! -x "$CLI_BIN" ]]; then
    echo "Building uniffi-bindgen $UNIFFI_VERSION CLI (one-off, cached at $CLI_BIN)..."
    WORK="$(mktemp -d)"
    trap 'rm -rf "$WORK"' EXIT
    mkdir -p "$WORK/src"
    cat > "$WORK/Cargo.toml" <<EOF
[package]
name = "uniffi-cli"
version = "0.1.0"
edition = "2021"

[dependencies]
uniffi = { version = "=$UNIFFI_VERSION", features = ["cli"] }
EOF
    printf 'fn main() { uniffi::uniffi_bindgen_main() }\n' > "$WORK/src/main.rs"
    (cd "$WORK" && cargo build --release)
    mkdir -p "$(dirname "$CLI_BIN")"
    cp "$WORK/target/release/uniffi-cli" "$CLI_BIN"
    trap - EXIT
    rm -rf "$WORK"
fi

# Build the cdylib the bindings are generated from.
cargo build -p dusrelpdf-uniffi

case "$(uname -s)" in
    Darwin) LIB="target/debug/libdusrelpdf_uniffi.dylib" ;;
    MINGW*|MSYS*|CYGWIN*) LIB="target/debug/dusrelpdf_uniffi.dll" ;;
    *) LIB="target/debug/libdusrelpdf_uniffi.so" ;;
esac
[[ -f "$LIB" ]] || { echo "error: shared library not found at $LIB" >&2; exit 1; }

OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT

"$CLI_BIN" generate --library "$LIB" --language kotlin --out-dir "$OUT/kotlin"
"$CLI_BIN" generate --library "$LIB" --language swift --out-dir "$OUT/swift"

if [[ "${1:-}" == "--check" ]]; then
    if diff -r bindings/kotlin "$OUT/kotlin" && diff -r bindings/swift "$OUT/swift"; then
        echo "Bindings are up to date (byte-identical)."
    else
        echo "error: committed bindings differ from generated output." >&2
        echo "Run scripts/regenerate-uniffi.sh to refresh them." >&2
        exit 1
    fi
else
    rm -rf bindings/kotlin bindings/swift
    mkdir -p bindings
    cp -r "$OUT/kotlin" bindings/kotlin
    cp -r "$OUT/swift" bindings/swift
    trap - EXIT
    rm -rf "$OUT"
    echo "Regenerated bindings/kotlin and bindings/swift."
    echo "Verify with: scripts/regenerate-uniffi.sh --check"
fi
