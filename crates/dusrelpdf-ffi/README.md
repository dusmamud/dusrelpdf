# dusrelpdf-ffi — C ABI for dusrelpdf

Thin C wrappers over the Rust core (`crates/dusrelpdf`). No PDF logic
lives here: every function converts C buffers / JSON options, calls the
core, and converts the result back. Go (`packages/go`, cgo) and Flutter
(`packages/flutter/dusrelpdf`, dart:ffi) both load the cdylib this crate
builds.

## Build

```sh
# Debug (fast, for development)
cargo build -p dusrelpdf-ffi
# Release — the file Go and Flutter load
cargo build -p dusrelpdf-ffi --release
# Linux:   target/release/libdusrelpdf.so
# macOS:   target/release/libdusrelpdf.dylib
# Windows: target/release/dusrelpdf.dll
```

The lib target is deliberately named `dusrelpdf` (see `Cargo.toml`):
the core crate builds only an rlib, so the cdylib file is
`libdusrelpdf.{so,dylib,dll}`.

## Regenerate the header

`dusrelpdf.h` is committed. Regenerate after changing the exported API:

```sh
# from the workspace root, with cbindgen on PATH
cbindgen --config crates/dusrelpdf-ffi/cbindgen.toml \
  --crate dusrelpdf-ffi \
  --output crates/dusrelpdf-ffi/dusrelpdf.h
```

## API at a glance

| Function | Returns |
| --- | --- |
| `dusrelpdf_version()` | static version string — **do NOT free** |
| `dusrelpdf_merge(inputs, count, options_json)` | `DusrelResult*` |
| `dusrelpdf_split(pdf, len, ranges_json, options_json)` | `DusrelResultList*` (`[[1,2],[3]]`) |
| `dusrelpdf_split_ranges(pdf, len, ranges, options_json)` | `DusrelResultList*` (`"1-3,4-5"`) |
| `dusrelpdf_split_each_page(pdf, len, options_json)` | `DusrelResultList*` |
| `dusrelpdf_extract_pages(pdf, len, pages_json, options_json)` | `DusrelResult*` (`"[1,3]"`) |
| `dusrelpdf_extract_pages_range(pdf, len, range, options_json)` | `DusrelResult*` (`"1-3,5"`) |
| `dusrelpdf_remove_pages(pdf, len, pages_json, options_json)` | `DusrelResult*` |
| `dusrelpdf_reorder_pages(pdf, len, order_json, options_json)` | `DusrelResult*` |
| `dusrelpdf_rotate_pages(pdf, len, options_json)` | `DusrelResult*` (`{"angle": 90}`) |
| `dusrelpdf_get_pdf_info(pdf, len, options_json)` | `DusrelResult*`, data = `PdfInfo` JSON |
| `dusrelpdf_images_to_pdf(images, count, options_json)` | `DusrelResult*` |
| `dusrelpdf_add_watermark(pdf, len, image, image_len, options_json)` | `DusrelResult*` |
| `dusrelpdf_add_page_numbers(pdf, len, options_json)` | `DusrelResult*` |
| `dusrelpdf_place_signature(pdf, len, image, image_len, options_json)` | `DusrelResult*` |
| `dusrelpdf_bulk_extract(pdf, len, selections_json, options_json)` | `DusrelResult*`, data = JSON array (base64 PDFs / per-item errors) |
| `dusrelpdf_free_result(r)` / `dusrelpdf_free_result_list(r)` | free exactly once; NULL is a no-op |

## Ownership contract

- **Inputs are borrowed.** Byte buffers, `DusrelBytes` arrays and JSON
  strings are read only for the duration of the call. The caller keeps
  owning them; the library never frees caller memory and never holds a
  pointer after returning.
- **Outputs are caller-owned.** Every operation returns a heap
  allocated `DusrelResult*` / `DusrelResultList*`. Free it **exactly
  once** with the matching free function, which frees the struct, the
  output byte buffer(s) and the error string. Do not free the inner
  buffers yourself, do not free twice, do not mix the two free
  functions.
- **On success** (`ok == 1`): `data` holds the output bytes and
  `error_json` is NULL. `get_pdf_info` and `bulk_extract` return
  UTF-8 JSON in `data` instead of PDF bytes (see `src/lib.rs` for the
  exact shapes).
- **On failure** (`ok == 0`): `data` is NULL and `error_json` is a
  NUL-terminated JSON string
  `{"code": "INVALID_RANGE", "message_en": "…", "message_hi": "…"}`.
  Codes are the stable dusrelpdf codes (`INVALID_PDF`,
  `ENCRYPTED_PDF`, `PAGE_OUT_OF_RANGE`, `EMPTY_INPUT`,
  `INVALID_RANGE`, `INVALID_ANGLE`, `INVALID_IMAGE`,
  `INVALID_ARGUMENT`, `INVALID_TYPE`, `OPERATION_FAILED`).
- **Functions never return NULL and never panic across the FFI
  boundary.** A NULL/invalid pointer, bad options JSON, or an internal
  panic all come back as an `ok == 0` result. The only exception is
  `dusrelpdf_version()`, which returns a static string.
- **Options JSON is partial** — same snake_case field names as the
  Rust core options (`crates/dusrelpdf/src/options.rs`), missing keys
  keep Rust defaults, NULL or `""` means all defaults. `page_size`
  also accepts `{"width": w, "height": h}` or `[w, h]`.
- **Watermark image:** pass the image bytes as the separate buffer
  argument; they override any `image` field in the options JSON and
  clear the default `text` (the core needs exactly one of them). For a
  text watermark pass NULL for the image and `{"text": "DRAFT"}` in
  the options JSON.

## Tests

```sh
cargo test -p dusrelpdf-ffi   # 18 tests: every operation + error paths + NULL safety
cargo clippy -p dusrelpdf-ffi --all-targets
cargo fmt -p dusrelpdf-ffi -- --check
```

## Honest limits

- Whole documents live in memory; no streaming.
- Same v0.1.0 core limits: no rendering, no tolerant repair, merge
  drops original outlines (rebuilt flat), watermark / page-number
  text is ASCII only, signature is a raster image, not a
  cryptographic digital signature.
