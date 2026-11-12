#!/usr/bin/env python3
"""Push the whole dusrelpdf working tree to GitHub in ONE commit via the
Git Data API (create blobs -> tree -> commit -> update ref).

Why not `git push`? The PAT lives in Muse's Secure Vault and is only
reachable through the surrogate mechanism in
/opt/muse/skills/custom-connector-setup/bin/dynamic_credentials.py —
there is no raw token to hand to git. This script uses the same
surrogate the github_rest.py helper uses, so no secret ever appears
in a file, a log or a command line.

Usage:
    python3 scripts/push_via_api.py [--message "msg"] [--dry-run]

Excludes build outputs (target/, node_modules/, dist/, wasm-pkg/,
.venv/, .git/) and anything in .gitignore that is generated.
"""
import argparse
import base64
import json
import os
import sys
import urllib.error
import urllib.request

sys.path.insert(0, "/opt/muse/skills/custom-connector-setup/bin")
from dynamic_credentials import add_surrogate_to_request, read_json_response

BASE = "https://api.github.com"
OWNER = "dusmamud"
REPO = "dusrelpdf"
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

EXCLUDE_DIRS = {
    "target", "node_modules", ".git", ".venv", "__pycache__",
    "dist", "wasm-pkg", ".astro", ".pytest_cache", ".dart_tool",
}
EXCLUDE_SUFFIXES = (".whl", ".pyc", ".wasm", ".so", ".node")
EXCLUDE_FILES = {".DS_Store"}
# Generated paths (wasm-pack copies LICENSE next to its output; the
# website build script copies the WASM bundle into public/wasm/).
EXCLUDE_PATHS = {"website/src/LICENSE", "packages/LICENSE"}
EXCLUDE_PREFIXES = ("website/public/wasm/",)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args):
        raise RuntimeError("redirect refused, verify the endpoint")


def call(path, method="GET", data=None):
    url = BASE + path
    body = json.dumps(data).encode() if data is not None else None
    req = urllib.request.Request(url, data=body, method=method)
    req.add_header("Accept", "application/vnd.github+json")
    req.add_header("X-GitHub-Api-Version", "2022-11-28")
    if body is not None:
        req.add_header("Content-Type", "application/json")
    add_surrogate_to_request(req, "custom.github-rest",
                             allowed_hosts=["api.github.com"])
    try:
        with urllib.request.build_opener(NoRedirect()).open(req, timeout=60) as r:
            return read_json_response(r)
    except urllib.error.HTTPError as e:
        detail = e.read().decode(errors="replace")
        raise SystemExit(f"GitHub API {method} {path} -> HTTP {e.code}: {detail}")


def collect_files():
    out = []
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]
        for name in filenames:
            if name in EXCLUDE_FILES or name.endswith(EXCLUDE_SUFFIXES):
                continue
            full = os.path.join(dirpath, name)
            rel = os.path.relpath(full, ROOT)
            if rel in EXCLUDE_PATHS:
                continue
            if any(rel.startswith(p) for p in EXCLUDE_PREFIXES):
                continue
            out.append((rel, full))
    return sorted(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--message", default="dusrelpdf v0.1.0 — initial commit")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    files = collect_files()
    print(f"{len(files)} files to push")
    for rel, _ in files:
        print("  " + rel)
    if args.dry_run:
        return

    # 1. blobs
    entries = []
    for rel, full in files:
        with open(full, "rb") as f:
            content = base64.b64encode(f.read()).decode()
        blob = call(f"/repos/{OWNER}/{REPO}/git/blobs", "POST",
                    {"content": content, "encoding": "base64"})
        entries.append({"path": rel, "mode": "100644",
                        "type": "blob", "sha": blob["sha"]})
        print(f"blob {rel}: {blob['sha'][:7]}")

    # 2. tree (base_tree = current main tip if it exists, else empty)
    base_tree = None
    try:
        ref = call(f"/repos/{OWNER}/{REPO}/git/ref/heads/main")
        head = call(f"/repos/{OWNER}/{REPO}/git/commits/"
                    f"{ref['object']['sha']}")
        base_tree = head["tree"]["sha"]
        parents = [ref["object"]["sha"]]
    except SystemExit:
        parents = []
    tree_data = {"tree": entries}
    if base_tree:
        tree_data["base_tree"] = base_tree
    tree = call(f"/repos/{OWNER}/{REPO}/git/trees", "POST", tree_data)
    print(f"tree: {tree['sha']}")

    # 3. commit
    commit = call(f"/repos/{OWNER}/{REPO}/git/commits", "POST",
                  {"message": args.message, "tree": tree["sha"],
                   "parents": parents,
                   "author": {"name": "dusmamud",
                              "email": "dusmamud@dusyn.in"}})
    print(f"commit: {commit['sha']}")

    # 4. ref
    if parents:
        call(f"/repos/{OWNER}/{REPO}/git/refs/heads/main", "PATCH",
             {"sha": commit["sha"]})
    else:
        call(f"/repos/{OWNER}/{REPO}/git/refs", "POST",
             {"ref": "refs/heads/main", "sha": commit["sha"]})
    print(f"pushed: https://github.com/{OWNER}/{REPO}/commit/{commit['sha']}")


if __name__ == "__main__":
    main()
