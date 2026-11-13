# dusrelpdf-napi

Node.js native bindings for dusrelpdf — thin
[napi-rs](https://napi.rs) (napi 2) wrappers over the Rust core
(`crates/dusrelpdf`). No PDF logic lives here; this crate only
converts between JavaScript values and Rust types.

## Build

A fresh clone **must** build this first — the `.node` binary is
**not committed** (gitignored via `*.node` in the repo-root
`.gitignore`):

```sh
cd crates/dusrelpdf-napi
npm install   # installs @napi-rs/cli (devDependency)
npm run build # napi build --platform --release
```

That produces `dusrelpdf-napi.linux-x64-gnu.node` (name varies by
platform) in this directory, which `packages/js` lazy-loads when
present. Without a build tool, the fallback also works:

```sh
cargo build --release -p dusrelpdf-napi
cp target/release/libdusrelpdf_napi.so \
   dusrelpdf-napi.linux-x64-gnu.node
```

(`.so` on Linux, `.dylib` on macOS, `.dll` on Windows — rename to
`<name>.<platform>-<arch>-<abi>.node` for the platform.)

Cross-target builds use `--target` (the `--platform` flag is what
appends the triple to the filename, so both flags are needed):

```sh
npm run build -- --target x86_64-unknown-linux-musl # needs musl-tools (apt install musl-tools)
# aarch64 needs gcc-aarch64-linux-gnu AND the linker pinned explicitly:
# without it rustc links with host `cc`, whose rust-lld runs in x86-64
# mode and rejects the aarch64-only `--fix-cortex-a53-843419` flag.
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  npm run build -- --target aarch64-unknown-linux-gnu
npm run build -- --target aarch64-apple-darwin      # macOS only
npm run build -- --target x86_64-pc-windows-msvc   # Windows only
```

## Platform packages (npm prebuilds)

An npm-installed user never builds anything: `packages/js` lists
one `optionalDependency` per platform —
`dusrelpdf-linux-x64-gnu`, `dusrelpdf-linux-x64-musl`,
`dusrelpdf-linux-arm64-gnu`, `dusrelpdf-darwin-x64`,
`dusrelpdf-darwin-arm64`, `dusrelpdf-win32-x64-msvc` — each a tiny
package holding one `.node` file. npm picks the matching one by
the `os` / `cpu` / `libc` fields in its `package.json`, whose
`main` points straight at the `.node`; `packages/js/src/index.ts`
resolves that package by name (`require.resolve`) and falls back
to WASM when no prebuild matches.

The platform packages live in `npm/<triple>/` here (manifests
committed, `.node` files gitignored — they are staged at release
time, never committed). They are managed with the standard napi-rs
CLI commands, configured in this directory's `package.json`
(`napi.name` is the binary name, `napi.package.name` is the
platform-package prefix, `napi.triples` is the target list —
note: this CLI version (2.18) reads `triples`, not the legacy
`targets` / `packageName` fields):

```sh
npm run create-npm-dir  # (re)generate npm/<triple>/package.json + README.md
npm run artifacts       # copy built .node files into npm/<triple>/ (CI: .github/workflows/napi.yml)
node scripts/verify-platforms.mjs            # manifests + optionalDependencies agree
node scripts/verify-platforms.mjs --staged  # ...and the .node files are staged
npm run version           # sync versions into npm/<triple>/ after a bump
```

This `package.json` is a **private build-tool manifest**
(`"private": true`) — it is never published, so it has no `main`
field: the `.node` filename is per-platform, and the real
per-platform `main` fields live in `npm/<triple>/package.json`.
`napi build` also writes a generated `index.js` / `index.d.ts`
here (the CLI's own loader + types); they are build outputs, not
the loader `packages/js` uses.

Publishing order (release flow — nothing here publishes on its
own): platform packages first, root last. `npm run prepublishOnly`
here runs `napi prepublish -t npm --skip-gh-release`, which
updates `optionalDependencies` to the exact current version and
`npm publish`es each `npm/<triple>` package (the GitHub-release
upload is skipped: releases are the release workflow's job).
`packages/js` has a `prepublishOnly` that delegates to this one,
so `npm publish` in `packages/js` publishes the platforms first
and the root package last. Keep the two `optionalDependencies`
lists identical — `verify-platforms.mjs` checks that.

## Contract

Same as the WASM binding (`crates/dusrelpdf-wasm`):

* Binary results are Node `Buffer`s (splits return an Array of them).
* On failure the function throws a JS `Error` whose message is
  `"CODE: reason in English"`, e.g. `"INVALID_RANGE: Invalid page
  range \"9\". …"` — the same stable code the core reports, so JS
  callers see identical behaviour to the WASM path.
* Options are plain JS objects matching the Rust `*Options` structs
  (see `crates/dusrelpdf/src/options.rs`). Missing fields are filled
  from the Rust `Default` impl — `{}` means "all defaults" — so
  callers can pass partial objects. `undefined` / omitted options
  also mean all defaults.
* `get_pdf_info` / `bulk_extract` return JSON strings in the same
  shape as the WASM binding (`bulk_extract` PDFs are base64).

Exported functions (camelCase `js_name`s): `version`,
`makeTestPdf`, `mergePdfs`, `splitPdf`, `splitPdfRanges`,
`splitEachPage`, `extractPages`, `extractPagesRange`, `removePages`,
`reorderPages`, `rotatePages`, `imagesToPdf`, `addWatermark`,
`addPageNumbers`, `placeSignature`, `getPdfInfo`, `bulkExtract`.

## Tests

```sh
cargo test -p dusrelpdf-napi   # Rust unit tests (option merging,
                               # watermark image/text handling,
                               # base64, error-code formatting)
cargo clippy -p dusrelpdf-napi # 0 warnings
```

The binding surface itself is tested end-to-end from Node in
`packages/js/src/native.test.ts` (native path when the `.node`
exists, WASM fallback with `DUSREL_PDF_FORCE_WASM=1`).

Note: the `#[napi]` functions cannot be called from Rust
integration tests — the napi symbols they reference are provided
by the Node runtime, so a plain `cargo test` binary cannot link
them. That is why the Rust tests live as unit tests in
`src/lib.rs` (pure helpers only) and the end-to-end coverage is
in Node. The same link failure bites `cargo test --workspace` on
macOS/Windows (their linkers refuse the undefined napi symbols;
Linux tolerates them), so the CI `rust` job excludes this crate
(`--exclude dusrelpdf-napi`) and its 8 unit tests run on Linux in
the `phase2` workflow instead.

## Status

Part of dusrelpdf Phase 2. Not published to npm yet — neither the
root `dusrelpdf` package nor the `dusrelpdf-<triple>` platform
packages. The wiring above (platform manifests, `optionalDependencies`,
`napi.yml`, `prepublishOnly`) is ready; the first publish follows
the release flow (platform packages first, root last).
