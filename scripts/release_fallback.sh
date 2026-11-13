#!/usr/bin/env bash
# Satellite-only release fallback for dusrelpdf.
#
# THE PROBLEM
#   release-plz only counts commits that touch the publishable crate's
#   own directory (crates/dusrelpdf/). A batch that only touches
#   bindings / packages / website / workflows — e.g. the whole Phase 2
#   bindings batch — makes release-plz say "already up to date" and
#   open no release PR, so the work ships in no released version.
#
#   The same thing happens for the FIRST release: the only commit that
#   ever touched crates/dusrelpdf/ is the non-conventional initial
#   commit, so release-plz computes next version == current version
#   (0.1.0) with an empty changelog and reports "already up-to-date"
#   even though the package has never been published (no tag, no
#   crates.io entry). The fallback therefore also covers the first
#   release — see `decide()`'s no-tag branch.
#
# THE FIX (this script + the satellite-fallback job in
# .github/workflows/release.yml)
#   When release-plz found nothing to do, bump the patch version and
#   open the same kind of release PR for the satellite changes.
#
# SAFETY (how it cannot clobber release-plz's own changes)
#   1. It never runs while a release PR is open. The workflow job runs
#      after the release-plz job and first checks GitHub for any open
#      PR whose head branch starts with `release-plz` — if one exists,
#      the fallback does nothing. release-plz's own PRs use branches
#      `release-plz-YYYY-MM-DDTHH-MM-SSZ`, so this catches them.
#   2. It only acts when ALL of these hold (checked here, in `check`):
#        - EITHER a release tag exists (dusrelpdf-vX.Y.Z, else bare
#          vX.Y.Z) — the satellite case — OR no tag exists at all —
#          the first-release case (see below).
#        - satellite case: the local workspace version EQUALS the
#          latest tag's version (nothing is already bumped or in
#          flight); satellite files changed since that tag, AND the
#          Rust core (crates/dusrelpdf/) did NOT change. Any core
#          change means release-plz will handle the release itself, so
#          the fallback stands down — this is what makes a mixed batch
#          safe.
#        - first-release case: the local version is the version to
#          release (NO bump — 0.1.0 is released as 0.1.0); the PR only
#          moves the CHANGELOG.md [Unreleased] notes into a dated
#          [0.1.0] entry. It runs only because release-plz already ran
#          first in the same workflow and opened nothing (the workflow
#          orders the jobs that way and guard 1 above stops it if a
#          release PR is open). Once the release PR merges and the
#          release step creates the dusrelpdf-v0.1.0 tag, this branch
#          never applies again.
#   3. It runs in its own workflow job with a fresh checkout of main,
#      never in the release-plz job's working tree, so it cannot touch
#      release-plz's in-progress files.
#   4. The PR it opens uses a branch named
#      `release-plz-YYYY-MM-DDTHH-MM-SSZ` — the exact format release-plz
#      uses (see release-plz.toml `pr_branch_prefix`). The next
#      release-plz run recognises that PR as its own and updates it
#      instead of opening a second one. Caveat: when release-plz later
#      updates that PR it regenerates the changelog from core commits,
#      so the hand-written satellite entry below can be dropped from the
#      changelog — the satellite CHANGES still ship in the release, and
#      the PR body records why the bump happened.
#   5. `apply` refuses to run on a dirty working tree (tracked
#      modifications only — untracked files such as CI logs do not
#      block it; the workflow's check step writes its stderr to
#      /tmp for the same reason).
#
# WHAT COUNTS AS "SATELLITE"
#   crates/ (except crates/dusrelpdf/), packages/, bindings/,
#   website/, README.md, CHANGELOG.md. Deliberately excluded:
#   .github/ and scripts/ (release machinery — a workflow-only change
#   must not trigger a release), release-plz.toml, Cargo.lock (a side
#   effect of version bumps, not a release driver).
#
# Usage:
#   bash scripts/release_fallback.sh check   # exit 0 + prints
#     NEW_VERSION=X.Y.Z  (and first_release=true for a first release)
#     if a satellite bump / first release is due, else exit 1 and a
#     SKIP reason on stderr. Safe to run anywhere, never changes files.
#   bash scripts/release_fallback.sh apply   # runs the check, then
#     bumps the patch version everywhere (Cargo.toml, Cargo.lock,
#     scripts/sync-versions.sh for the JS/Python/Flutter/Go/website
#     manifests) and moves the CHANGELOG.md [Unreleased] notes into a
#     dated [X.Y.Z] entry. For a first release there is no bump — the
#     version is already the release version — so only the changelog
#     moves. Prints NEW_VERSION=X.Y.Z (and first_release=true for a
#     first release). Does NOT commit, branch, or open a PR — the
#     workflow does that.
#
# Tests: scripts/test-release-fallback.sh (scratch repos, both the
# bump and the no-bump cases).
set -euo pipefail
cd "$(dirname "$0")/.."

TAG_PREFIX="dusrelpdf-v"
# Space-separated, so both bash loops and `git diff -- <list>` use it.
SATELLITE_PATHS="crates packages bindings website README.md CHANGELOG.md"

die() { echo "release_fallback: $*" >&2; exit 2; }

latest_tag() {
  local tag
  tag=$(git tag --list "${TAG_PREFIX}*" --sort=-v:refname | head -n1)
  if [ -z "$tag" ]; then
    tag=$(git tag --list 'v[0-9]*' --sort=-v:refname | head -n1)
  fi
  printf '%s' "$tag"
}

local_version() {
  grep -m1 '^version =' Cargo.toml | sed -E 's/^version = "(.*)"/\1/'
}

tag_version() {
  local tag="$1"
  tag="${tag#"$TAG_PREFIX"}"
  tag="${tag#v}"
  printf '%s' "$tag"
}

bump_patch() {
  local major minor patch
  IFS=. read -r major minor patch <<< "$1"
  printf '%s.%s.%s' "$major" "$minor" "$((patch + 1))"
}

# Prints the changed satellite areas (first two path components) since
# $1, one per line, e.g. "packages/go". With an empty $1 (no release
# tag yet — first release) the diff is against the root commit, i.e.
# everything the repo has ever added.
changed_areas() {
  local base="$1"
  if [ -z "$base" ]; then
    base=$(git rev-list --max-parents=0 HEAD | head -n1)
  fi
  git diff --name-only "$base" HEAD -- $SATELLITE_PATHS ':!crates/dusrelpdf' \
    | awk -F/ '{ if (NF > 1) print $1"/"$2; else print $1 }' \
    | sort -u
}

# Decide whether a satellite bump / first release is due. On success
# prints NEW_VERSION=X.Y.Z (plus first_release=true for a first
# release) to stdout and exits 0; otherwise prints a SKIP reason to
# stderr and exits 1. Never changes files.
decide() {
  [ -f Cargo.toml ] || { echo "SKIP: no Cargo.toml here" >&2; return 1; }
  local tag local_v tag_v
  tag=$(latest_tag)
  local_v=$(local_version || true)
  if ! [[ "$local_v" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "SKIP: unparsable local version ($local_v)" >&2
    return 1
  fi
  if [ -z "$tag" ]; then
    # FIRST RELEASE: no tag exists yet. release-plz already ran in
    # this workflow and opened nothing (it needs a conventional commit
    # touching crates/dusrelpdf/ to see a first release, and the only
    # such commit is the non-conventional initial commit). The version
    # to release is the local version as-is — NO bump. The PR only
    # moves the CHANGELOG.md [Unreleased] notes into a dated entry.
    # The workflow's guard (no open release-plz PR) is what stops a
    # duplicate while this PR is open.
    printf 'NEW_VERSION=%s\nfirst_release=true\n' "$local_v"
    return 0
  fi
  tag_v=$(tag_version "$tag")
  if ! [[ "$tag_v" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "SKIP: unparsable version (local=$local_v tag=$tag)" >&2
    return 1
  fi
  if [ "$local_v" != "$tag_v" ]; then
    echo "SKIP: local version $local_v is already ahead of latest tag $tag — a bump is in flight or release-plz owns it" >&2
    return 1
  fi
  if ! git diff --quiet "$tag" HEAD -- crates/dusrelpdf ':!crates/dusrelpdf/CHANGELOG.md'; then
    echo "SKIP: crates/dusrelpdf/ changed since $tag — release-plz handles this release" >&2
    return 1
  fi
  if git diff --quiet "$tag" HEAD -- $SATELLITE_PATHS ':!crates/dusrelpdf'; then
    echo "SKIP: no satellite changes since $tag" >&2
    return 1
  fi
  printf 'NEW_VERSION=%s\n' "$(bump_patch "$local_v")"
  return 0
}

# Bump [workspace.package] version in the root Cargo.toml. The
# version line lives in that section; anchor the edit to it so no
# dependency's `version = "..."` line is ever touched.
bump_cargo_toml() {
  local new="$1"
  python3 - "$new" <<'PYEOF'
import sys

new = sys.argv[1]
path = "Cargo.toml"
lines = open(path, encoding="utf-8").read().splitlines(keepends=True)
in_section = False
changed = False
for i, line in enumerate(lines):
    if line.startswith("["):
        in_section = line.strip() == "[workspace.package]"
        continue
    if in_section and line.startswith("version = "):
        lines[i] = f'version = "{new}"\n'
        changed = True
        break
if not changed:
    raise SystemExit("release_fallback: [workspace.package] version line not found in Cargo.toml")
open(path, "w", encoding="utf-8").write("".join(lines))
PYEOF
}

# Update Cargo.lock for the changed workspace version. Only the
# workspace members (from the root Cargo.toml members list) are
# touched — never third-party crates that happen to share the old
# version (e.g. byteorder-lite and simd_helpers are also 0.1.0).
# No cargo toolchain needed: workspace members have no checksums in
# the lock, so rewriting their version field is exactly what cargo
# itself would write.
bump_cargo_lock() {
  local new="$1"
  python3 - "$new" <<'PYEOF'
import sys
import tomllib

new = sys.argv[1]
with open("Cargo.toml", "rb") as f:
    members = {m.rsplit("/", 1)[-1] for m in tomllib.load(f)["workspace"]["members"]}
lines = open("Cargo.lock", encoding="utf-8").read().splitlines(keepends=True)
current = None
changed = set()
for i, line in enumerate(lines):
    if line.startswith("[[package]]"):
        current = None
        continue
    if line.startswith('name = "'):
        current = line.split('"')[1]
        continue
    if current in members and line.startswith("version = "):
        lines[i] = f'version = "{new}"\n'
        changed.add(current)
        current = None
if changed != members:
    raise SystemExit(
        f"release_fallback: Cargo.lock update incomplete: changed {sorted(changed)}, members {sorted(members)}"
    )
open("Cargo.lock", "w", encoding="utf-8").write("".join(lines))
PYEOF
}

# Move the CHANGELOG.md [Unreleased] notes into a dated [X.Y.Z]
# entry, exactly as release-plz does on a release. If [Unreleased] is
# empty, generate the entry from the satellite areas changed since the
# tag instead. The [Unreleased] heading is left in place (empty) for
# the next release's notes.
update_changelog() {
  local new="$1" tag="$2" date="$3"
  changed_areas "$tag" > /tmp/dusrelpdf-fallback-areas.txt
  python3 - "$new" "$date" <<'PYEOF'
import sys

new, date = sys.argv[1], sys.argv[2]
areas = [line.strip() for line in open("/tmp/dusrelpdf-fallback-areas.txt", encoding="utf-8") if line.strip()]
path = "CHANGELOG.md"
text = open(path, encoding="utf-8").read()
lines = text.splitlines(keepends=True)

def heading_index(prefix, start=0):
    for i in range(start, len(lines)):
        if lines[i].startswith(prefix):
            return i
    return None

unrel = heading_index("## [Unreleased]")
if unrel is not None:
    nxt = heading_index("## [", unrel + 1)
    end = nxt if nxt is not None else len(lines)
    body = lines[unrel + 1 : end]
    while body and body[-1].strip() == "":
        body.pop()
    while body and body[0].strip() == "":
        body.pop(0)
    if body:
        section = [f"## [{new}] - {date}\n", "\n", *body, "\n"]
    else:
        section = [f"## [{new}] - {date}\n", "\n",
                   "### Changed\n", "\n",
                   "- Satellite-only release: no changes to the Rust core\n"
                   "  (`crates/dusrelpdf/`); bindings, packages and docs moved.\n",
                   *("- " + a + "\n" for a in areas), "\n"]
    lines = lines[: unrel + 1] + ["\n", *section] + lines[end:]
else:
    first = heading_index("## [")
    section = [f"## [{new}] - {date}\n", "\n",
               "### Changed\n", "\n",
               "- Satellite-only release: no changes to the Rust core\n"
               "  (`crates/dusrelpdf/`); bindings, packages and docs moved.\n",
               *("- " + a + "\n" for a in areas), "\n"]
    if first is not None:
        lines = lines[:first] + section + lines[first:]
    else:
        lines = lines + ["\n", *section]
open(path, "w", encoding="utf-8").write("".join(lines))
PYEOF
  rm -f /tmp/dusrelpdf-fallback-areas.txt
}

apply_bump() {
  # Untracked files (e.g. CI logs) must not block the bump — only
  # tracked modifications do. The workflow's check step used to write
  # check.err into the worktree, which tripped this guard on the first
  # real apply (release run 43388953112).
  if [ -n "$(git status --porcelain | grep -v '^??' || true)" ]; then
    die "working tree is dirty — commit or stash first (the fallback never bumps on top of uncommitted changes)"
  fi
  local out new tag date first_release="false"
  # `decide` already printed the SKIP reason to stderr.
  if ! out=$(decide); then
    return 1
  fi
  new="${out#NEW_VERSION=}"
  new="${new%%$'\n'*}"
  if grep -q '^first_release=true$' <<< "$out"; then
    first_release="true"
  fi
  tag=$(latest_tag)
  date=$(date -u +%F)
  if [ "$first_release" = "true" ]; then
    # First release: the version IS the release version — no bump,
    # no sync. Only the changelog moves.
    update_changelog "$new" "$tag" "$date"
  else
    bump_cargo_toml "$new"
    bump_cargo_lock "$new"
    bash scripts/sync-versions.sh "$new" > /dev/null
    update_changelog "$new" "$tag" "$date"
  fi

  # Verify: the only files changed are the ones this script owns.
  # (Untracked files are not ours — same exception as the dirty
  # check above.)
  local allowed='^(Cargo\.toml|Cargo\.lock|CHANGELOG\.md|packages/js/package\.json|packages/js/package-lock\.json|crates/dusrelpdf-napi/package\.json|crates/dusrelpdf-napi/package-lock\.json|crates/dusrelpdf-py/pyproject\.toml|packages/flutter/dusrelpdf/pubspec\.yaml|packages/flutter/dusrelpdf_flutter/pubspec\.yaml|website/package\.json|website/astro\.config\.mjs|packages/go/dusrelpdf_test\.go|packages/go/dusrelpdf\.go)$'
  local unexpected
  unexpected=$(git status --porcelain | grep -v '^??' | awk '{print $2}' | grep -vE "$allowed" || true)
  if [ -n "$unexpected" ]; then
    die "unexpected files changed, aborting (nothing committed): $unexpected"
  fi
  if ! grep -q "## \[$new\] - $date" CHANGELOG.md; then
    die "changelog entry for $new not found after update"
  fi
  if [ "$first_release" = "true" ]; then
    echo "first release: $new (no tag yet; changelog only, no version bump)" >&2
    printf 'NEW_VERSION=%s\nfirst_release=true\n' "$new"
  else
    echo "satellite fallback: $tag -> $new (areas changed since $tag: $(changed_areas "$tag" | tr '\n' ' '))" >&2
    printf 'NEW_VERSION=%s\n' "$new"
  fi
}

case "${1:-}" in
  check)
    decide
    ;;
  apply)
    apply_bump
    ;;
  *)
    echo "usage: $0 {check|apply}" >&2
    exit 2
    ;;
esac
