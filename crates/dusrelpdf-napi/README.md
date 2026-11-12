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
in Node.

## Status

Part of dusrelpdf Phase 2. Not published to npm — there is no
separate npm package for the addon; it is loaded directly from
this directory by `packages/js`.
