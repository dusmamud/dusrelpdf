# Changelog

All notable changes to dusrelpdf are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-11-12

### Added

- First release. One Rust core (`crates/dusrelpdf`, lopdf 0.47,
  edition 2024, Rust 1.85+), with JS (WASM), Python (PyO3) and CLI
  bindings — all version 0.1.0 in lockstep, all named `dusrelpdf`.
- 10 operations, bytes in / bytes out: merge, split, extract, remove,
  reorder, rotate (90/180/270, negative angles normalised), images
  to PDF (A4 / Letter / auto, contain / fill), text + image watermark
  (opacity, rotation, position), page numbers (`{n}` / `{total}`
  templates, any position, start number), signature-image placement
  (transparent PNGs keep their alpha via an `/SMask` soft-mask).
- Pro control layer: `Options` on every operation, stable error codes
  in English + Hindi (`INVALID_PDF`, `ENCRYPTED_PDF`,
  `PAGE_OUT_OF_RANGE`, `EMPTY_INPUT`, `INVALID_RANGE`,
  `INVALID_ANGLE`, `INVALID_IMAGE`, `INVALID_ARGUMENT`,
  `INVALID_TYPE`), page ranges like `"1-3,5"`, bulk extract,
  `get_pdf_info`, mock test-data builders.
- Zod schemas (JS) and Pydantic models (Python).
- Full CLI (`dusrelpdf`): `merge`, `split`, `extract`, `remove`,
  `reorder`, `rotate`, `images-to-pdf`, `watermark`, `numbers`,
  `sign`, `info`, `mock`, `--version` / `version`, `--json` errors.
- Astro Starlight docs website with a client-side WASM playground
  (files never leave the browser).
- release-plz `release-pr` configuration (no publishing yet).

### Known limits

- No PDF rendering, no tolerant repair of malformed PDFs, no full
  encryption support (password via options only).
- Merging drops the original outline trees (rebuilt flat) and does
  not carry AcroForm / named destinations / embedded files.
- Watermark / page-number text is ASCII only.
- Signature placement is an image, not a cryptographic signature.
