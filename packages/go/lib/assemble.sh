#!/bin/sh
# Reassemble the vendored static libraries from their committed chunks.
#
# Why chunks: the GitHub Git Data API (the only push path from this
# environment) rejects blobs larger than ~40 MB, and a Rust staticlib
# is ~55 MB. So linux-amd64 and linux-arm64 are stored as 20 MB parts
# (libdusrelpdf.a.part00, .part01, .part02); windows-amd64 fits whole
# at 40 MB and is stored as one file.
#
# Idempotent: if libdusrelpdf.a already exists and its SHA-256 matches
# SHA256SUMS, nothing is done. Otherwise the parts are concatenated
# and the checksum is verified — a mismatch fails loudly instead of
# producing a silently broken link.
#
# Run once after `go get` (or `go generate ./...` in the package):
#   sh packages/go/lib/assemble.sh
set -eu
cd "$(dirname "$0")"

fail() { echo "assemble.sh: $*" >&2; exit 1; }

for dir in linux-amd64 linux-arm64; do
  out="$dir/libdusrelpdf.a"
  want="$(grep " $dir/libdusrelpdf.a\$" SHA256SUMS | cut -d' ' -f1)"
  [ -n "$want" ] || fail "no checksum for $dir in SHA256SUMS"
  if [ -f "$out" ]; then
    got="$(sha256sum "$out" | cut -d' ' -f1)"
    if [ "$got" = "$want" ]; then
      echo "assemble.sh: $out already present, checksum OK"
      continue
    fi
    echo "assemble.sh: $out exists but checksum mismatch, rebuilding"
  fi
  parts="$dir"/libdusrelpdf.a.part*
  # shellcheck disable=SC2086
  cat $parts > "$out.tmp" || fail "concatenating $dir parts"
  got="$(sha256sum "$out.tmp" | cut -d' ' -f1)"
  [ "$got" = "$want" ] || { rm -f "$out.tmp"; fail "checksum mismatch for $dir (got $got, want $want)"; }
  mv "$out.tmp" "$out"
  echo "assemble.sh: $out assembled, checksum OK"
done

echo "assemble.sh: done"
