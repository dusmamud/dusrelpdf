# dusrelpdf

Offline PDF toolkit — merge, split, extract, remove, reorder, rotate,
watermark, page numbers, images-to-PDF and signature-image placement.
One Rust core, one package, no network, no uploads: every file is
processed on your own machine.

```sh
npm install dusrelpdf      # JavaScript / TypeScript (WASM)
pip install dusrelpdf      # Python (native wheel)
cargo add dusrelpdf        # Rust
```

```rust
use dusrelpdf::{extract_pages_range, fixtures::make_test_pdf, merge_pdfs_default, CommonOptions, PageSetOptions};

fn main() -> Result<(), dusrelpdf::DusrelError> {
    let pdf_a = make_test_pdf(2, None);
    let pdf_b = make_test_pdf(3, None);
    let merged = merge_pdfs_default(&[pdf_a, pdf_b])?;
    let pages = extract_pages_range(&merged, "1-3,5", &PageSetOptions::default())?;
    assert_eq!(dusrelpdf::get_pdf_info(&pages, &CommonOptions::default())?.page_count, 4);
    Ok(())
}
```

```js
import { mergePdfs, extractPagesRange } from "dusrelpdf";

const merged = await mergePdfs([pdfA, pdfB]);
const pages = await extractPagesRange(pdf, "1-3,5");
```

```python
import dusrelpdf

merged = dusrelpdf.merge_pdfs([pdf_a, pdf_b])
pages = dusrelpdf.extract_pages_range(pdf, "1-3,5")
```

```sh
dusrelpdf merge a.pdf b.pdf -o merged.pdf
dusrelpdf extract doc.pdf --pages "1-3,5" -o out.pdf
dusrelpdf watermark doc.pdf --text CONFIDENTIAL -o out.pdf
```

## Features (v0.1.0)

| Operation | What it does |
|---|---|
| `merge_pdfs` | Multiple PDFs into one, in order |
| `split_pdf` | One PDF into a file per page range |
| `extract_pages` | Selected pages into a new PDF |
| `remove_pages` | Delete unwanted pages |
| `reorder_pages` | Change the page order |
| `rotate_pages` | Rotate pages by 90 / 180 / 270° |
| `images_to_pdf` | Selected images into a PDF, one page each |
| `add_watermark` | Text or image watermark, with opacity + rotation |
| `add_page_numbers` | Page numbers, any format / position |
| `place_signature` | Handwritten signature image on a page |

Plus the pro control layer: every operation takes bytes in and bytes
out, `Options` everywhere (opacity, angle, font, position, page size,
fit, start number, password), stable error codes in English and Hindi
(`INVALID_PDF`, `ENCRYPTED_PDF`, `PAGE_OUT_OF_RANGE`, `EMPTY_INPUT`,
`INVALID_RANGE`, `INVALID_ANGLE`, `INVALID_IMAGE`, `INVALID_ARGUMENT`,
`INVALID_TYPE`), page ranges like `"1-3,5"`, bulk extraction, mock
test-data builders, Zod schemas (JS) and Pydantic models (Python),
and a full CLI.

## More languages (Phase 2, same 0.1.0 core)

One Rust core, thin wrappers only — behaviour never drifts between
languages. Nothing here is published yet; these bindings live in the
repo and are tested in CI.

| Language | Package | How it works |
|---|---|---|
| Node.js (native) | `dusrelpdf` (npm) | napi-rs addon (`crates/dusrelpdf-napi`); lazy-loads the native `.node` when built, falls back to the WASM build otherwise. Force WASM with `DUSREL_PDF_FORCE_WASM=1`. A fresh clone must run `npm run build` in `crates/dusrelpdf-napi` first — `.node` binaries are not committed. |
| Kotlin | UniFFI (`bindings/kotlin`) | Generated from `crates/dusrelpdf-uniffi` (UniFFI 0.28.3). Regenerate with `scripts/regenerate-uniffi.sh` — CI checks the output is byte-identical. |
| Swift | UniFFI (`bindings/swift`) | Same source as Kotlin, generated alongside it. |
| Go | `github.com/dusmamud/dusrelpdf-go` | cgo wrapper over the C API (`crates/dusrelpdf-ffi`, header `dusrelpdf.h` via cbindgen). Build the shared library first: `cargo build -p dusrelpdf-ffi --release`. No module release/tag yet, so `go get` needs a `replace` directive for now. |
| Flutter / Dart | `dusrelpdf` (pub.dev name, not published) | `packages/flutter/dusrelpdf`, dart:ffi over the same `libdusrelpdf` shared library. |

The C API (`dusrelpdf.h`) is the shared FFI surface for Go and
Flutter: inputs are borrowed, outputs are caller-owned and freed
exactly once with the matching `dusrelpdf_free_*` function — see
`crates/dusrelpdf-ffi/README.md`. Kotlin/Swift compilation is not
verified on this machine (no kotlinc/swiftc here); the generated
sources are shape-checked and regeneration-checked in CI.

## Why dusrelpdf

- **One core, every language.** The logic lives once in Rust
  (`crates/dusrelpdf`); JS (WASM), Python (PyO3) and the CLI all call
  the same code, so behaviour never drifts between languages.
- **MIT licence, light install.** PyMuPDF is fast but AGPL — it blocks
  closed-source / SaaS use without a commercial licence. pypdf is pure
  Python and slow. pdf-lib cannot render. dusrelpdf is fast native
  Rust under a permissive licence.
- **Private by design.** Nothing is uploaded anywhere. The website
  playground runs the WASM build entirely in your browser.

## Honest limits (v0.1.0)

- **No PDF rendering** (PDF → image), **no tolerant repair** of
  malformed PDFs, and **no full encryption support**: encrypted PDFs
  need `password` in the options; only empty-password PDFs decrypt
  automatically on load.
- **Merging drops the original outline trees** and rebuilds flat
  first-page bookmarks. AcroForm, named destinations and embedded
  files are not carried over.
- **Watermark / page-number text is ASCII only** (base-14 fonts).
  Use an image watermark for other scripts.
- **Signature placement is a raster image** on the page — not a
  cryptographic digital signature. Nothing about it is verifiable.
- Whole documents live in memory; there is no streaming mode.
- Transparent PNG signatures keep their alpha via a `/SMask`
  soft-mask (verified by test with a real transparent PNG fixture).

## Layout

```
crates/dusrelpdf/       Rust core (the only place with PDF logic)
crates/dusrelpdf-cli/   `dusrelpdf` binary (publish = false)
crates/dusrelpdf-py/    Python bindings, PyO3 (publish = false)
crates/dusrelpdf-wasm/  WASM bindings (publish = false)
packages/js/            npm package `dusrelpdf` (WASM wrapper)
website/                Astro Starlight docs + WASM playground
```

## Development

```sh
cargo test --workspace          # Rust core + CLI (38 tests)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check

cd packages/js && npm ci && npm run build && npm test
cd crates/dusrelpdf-py && maturin develop && pytest

dusrelpdf --version
dusrelpdf mock --pages 3 -o test.pdf
```

## Licence

MIT — see [LICENSE](LICENSE).
