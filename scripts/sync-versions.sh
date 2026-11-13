#!/usr/bin/env bash
# Sync the lockstep version of every non-Cargo version string from the
# Rust workspace version in the root Cargo.toml.
#
# The single source of truth is [workspace.package] version in
# Cargo.toml. release-plz bumps that; this script propagates the same
# version to the JS / Python / Flutter / Go / website manifests that
# release-plz never touches. The satellite fallback
# (scripts/release_fallback.sh) calls this after it bumps Cargo.toml.
#
# Usage: bash scripts/sync-versions.sh [VERSION]
#   No argument: read the version from the root Cargo.toml.
#   With an argument: set every file to that version (must be X.Y.Z).
# Safe to run by hand either way: it is idempotent, a second run is a
# no-op, and it only rewrites a file when that file's version differs.
#
# Files synced (all must exist, except the optional Flutter plugin):
#   packages/js/package.json, packages/js/package-lock.json
#   crates/dusrelpdf-napi/package.json, crates/dusrelpdf-napi/package-lock.json
#   crates/dusrelpdf-py/pyproject.toml
#   packages/flutter/dusrelpdf/pubspec.yaml
#   packages/flutter/dusrelpdf_flutter/pubspec.yaml (optional — the
#     Flutter plugin scaffolding may not exist yet; skipped silently)
#   website/package.json, website/astro.config.mjs (softwareVersion)
#   packages/go/dusrelpdf_test.go (Version() assertion — Version()
#     returns the Rust core version, so the literal must track it or
#     `go test` fails after a bump)
#   packages/go/dusrelpdf.go (Version() doc comment)
#
# Deliberately NOT synced (prose, not machine-checked version fields):
#   README.md / website docs (*.mdx) version mentions, and the Go
#   `require ... vX.Y.Z` example in packages/go/README.md — those are
#   updated by hand when the corresponding tag (e.g. packages/go/vX.Y.Z)
#   actually exists. bindings/kotlin and bindings/swift carry no
#   version string (generated UniFFI sources, no Gradle / Package.swift
#   manifest), so there is nothing to sync there.
set -euo pipefail
cd "$(dirname "$0")/.."

if [ $# -gt 1 ]; then
  echo "usage: $0 [VERSION]" >&2
  exit 2
fi

if [ $# -eq 1 ]; then
  version="$1"
else
  version=$(grep -m1 '^version =' Cargo.toml | sed -E 's/^version = "(.*)"/\1/')
fi
if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "sync-versions: not a valid X.Y.Z version: '$version'" >&2
  exit 2
fi
echo "syncing lockstep version to $version"

require_file() {
  if [ ! -f "$1" ]; then
    echo "sync-versions: required file missing: $1" >&2
    exit 1
  fi
}

# --- Single-line JSON manifests (exactly one "version" field each) ---
for f in packages/js/package.json website/package.json; do
  require_file "$f"
  # \1 preserves the file's own spacing (these files are compact JSON:
  # "version":"0.1.0", no space after the colon).
  sed -i -E "s/(\"version\": *)\"[0-9]+\.[0-9]+\.[0-9]+\"/\1\"$version\"/" "$f"
  echo "$f: $(grep -o '"version": *"[^"]*"' "$f")"
done

# --- Indented JSON manifests (python3: exact keys only) ---
# A global sed on package-lock.json would rewrite every dependency's
# version too (there are ~100 "version" lines in packages/js's lock).
# python3 json round-trips these three files byte-identically except
# for the keys changed here (verified: load + dump(indent=2) of the
# untouched file is a no-op).
python3 - "$version" <<'PYEOF'
import json
import sys

version = sys.argv[1]
updates = [
    ("crates/dusrelpdf-napi/package.json", ["version"]),
    ("packages/js/package-lock.json", ["version"]),
    ("packages/js/package-lock.json", ["packages", "", "version"]),
    ("crates/dusrelpdf-napi/package-lock.json", ["version"]),
    ("crates/dusrelpdf-napi/package-lock.json", ["packages", "", "version"]),
]
for path, keys in updates:
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    node = data
    for key in keys[:-1]:
        node = node[key]
    node[keys[-1]] = version
    with open(path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")
    print(f"{path}: {'/'.join(keys)} = {version}")
PYEOF

# --- TOML / YAML / JS-config / Go version strings ---
require_file crates/dusrelpdf-py/pyproject.toml
sed -i -E "s/^version = \"[0-9]+\.[0-9]+\.[0-9]+\"/version = \"$version\"/" crates/dusrelpdf-py/pyproject.toml
echo "crates/dusrelpdf-py/pyproject.toml: $(grep -m1 '^version' crates/dusrelpdf-py/pyproject.toml)"

require_file packages/flutter/dusrelpdf/pubspec.yaml
sed -i -E "s/^version: [0-9]+\.[0-9]+\.[0-9]+/version: $version/" packages/flutter/dusrelpdf/pubspec.yaml
echo "packages/flutter/dusrelpdf/pubspec.yaml: $(grep -m1 '^version:' packages/flutter/dusrelpdf/pubspec.yaml)"

if [ -f packages/flutter/dusrelpdf_flutter/pubspec.yaml ]; then
  sed -i -E "s/^version: [0-9]+\.[0-9]+\.[0-9]+/version: $version/" packages/flutter/dusrelpdf_flutter/pubspec.yaml
  echo "packages/flutter/dusrelpdf_flutter/pubspec.yaml: $(grep -m1 '^version:' packages/flutter/dusrelpdf_flutter/pubspec.yaml)"
else
  echo "packages/flutter/dusrelpdf_flutter/pubspec.yaml: not present, skipped"
fi

require_file website/astro.config.mjs
sed -i -E "s/softwareVersion: '[0-9]+\.[0-9]+\.[0-9]+'/softwareVersion: '$version'/" website/astro.config.mjs
echo "website/astro.config.mjs: $(grep -m1 'softwareVersion' website/astro.config.mjs)"

require_file packages/go/dusrelpdf_test.go
sed -i -E "s/v != \"[0-9]+\.[0-9]+\.[0-9]+\"/v != \"$version\"/" packages/go/dusrelpdf_test.go
sed -i -E "s/want [0-9]+\.[0-9]+\.[0-9]+/want $version/" packages/go/dusrelpdf_test.go
echo "packages/go/dusrelpdf_test.go: $(grep -m1 'v !=' packages/go/dusrelpdf_test.go | sed 's/^[[:space:]]*//')"

require_file packages/go/dusrelpdf.go
sed -i -E "s/e\.g\. \"[0-9]+\.[0-9]+\.[0-9]+\"/e.g. \"$version\"/" packages/go/dusrelpdf.go
echo "packages/go/dusrelpdf.go: $(grep -m1 'Version returns' packages/go/dusrelpdf.go | sed 's/^[[:space:]]*//')"
