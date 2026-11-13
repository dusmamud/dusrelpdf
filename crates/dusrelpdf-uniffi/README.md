# dusrelpdf-uniffi

UniFFI bindings (Kotlin/Swift) for
[dusrelpdf](https://github.com/dusmamud/dusrelpdf) — offline PDF toolkit
(merge, split, extract, remove, reorder, rotate, images-to-PDF,
watermark, page numbers, signature-image placement).

This is a thin wrapper: all PDF logic lives in the core `dusrelpdf`
crate. Every function here calls the matching core operation and
returns bytes in / bytes out. Errors are a `DusrelError` enum — one
variant per stable core error code (`InvalidRange`, `PageOutOfRange`,
…), each carrying `code`, `reason_en` and `reason_hi`.

This crate is never published to crates.io (`publish = false`); only
the core `dusrelpdf` crate is published.

## API

| Function | Returns |
| --- | --- |
| `mergePdfs(inputs, options)` | `ByteArray` / `Data` (merged PDF) |
| `splitPdf(pdf, ranges, options)` | `List<ByteArray>` / `[Data]` |
| `splitPdfRanges(pdf, ranges, options)` | `List<ByteArray>` / `[Data]` |
| `splitEachPage(pdf, options)` | `List<ByteArray>` / `[Data]` |
| `extractPages(pdf, pages, options)` | `ByteArray` / `Data` |
| `extractPagesRange(pdf, range, options)` | `ByteArray` / `Data` |
| `removePages(pdf, pages, options)` | `ByteArray` / `Data` |
| `reorderPages(pdf, order, options)` | `ByteArray` / `Data` |
| `rotatePages(pdf, options)` | `ByteArray` / `Data` |
| `getPdfInfo(pdf, common)` | `PdfInfo` (`pageCount`, `version`, `encrypted`, `title`, `pageSizes`) |
| `bulkExtract(pdf, selections, options)` | `List<BulkItem>` / `[BulkItem]` (per-item errors, never aborts the batch) |
| `imagesToPdf(images, options)` | `ByteArray` / `Data` |
| `addWatermark(pdf, options)` | `ByteArray` / `Data` |
| `addPageNumbers(pdf, options)` | `ByteArray` / `Data` |
| `placeSignature(pdf, signatureImage, options)` | `ByteArray` / `Data` |
| `version()` | `String` (`"0.1.0"`) |

Options are records with defaults matching the core, e.g.
`MergeOptions` (default: preserve metadata, add bookmarks),
`WatermarkOptions` (default text `"DRAFT"`, opacity `0.3`,
rotation `45`), `PageNumberOptions` (default format `"{n}"`,
position `BottomCenter`), `RotateOptions` (default angle `90`).
`ImagesToPdfOptions.background` is `[r, g, b]` (each 0.0–1.0,
default white) and must have exactly 3 values.

Honest limits (same as the core): no PDF rendering, no tolerant
repair of malformed PDFs, watermark / page-number text is ASCII
only, and signature placement is a raster image, not a
cryptographic digital signature.

## Kotlin usage

The generated bindings live in `bindings/kotlin/` at the repo root
(package `uniffi.dusrelpdf_uniffi`, file `dusrelpdf_uniffi.kt`).
Build the Rust core as a shared library for your target (e.g.
Android `libdusrelpdf_uniffi.so` via
`cargo build -p dusrelpdf-uniffi`), put the `.kt` file in your
project, and load the library (the generated code does this via
JNA).

```kotlin
import uniffi.dusrelpdf_uniffi.*

val merged = mergePdfs(listOf(pdfA, pdfB), MergeOptions())
println(getPdfInfo(merged, CommonOptions()).pageCount)  // 5

val rotated = rotatePages(pdf, RotateOptions(angle = 90))

try {
    extractPages(pdf, listOf(9u), PageSetOptions())
} catch (e: DusrelException.PageOutOfRange) {
    println(e.code)      // PAGE_OUT_OF_RANGE
    println(e.reasonEn)  // Page 9 is out of range ...
    println(e.reasonHi)  // Page 9 range se bahar hai ...
}
```

## Swift usage

The generated bindings live in `bindings/swift/` at the repo root:
`dusrelpdf_uniffi.swift`, plus the C header
`dusrelpdf_uniffiFFI.h` and `dusrelpdf_uniffiFFI.modulemap`, which
expose the Rust shared library (build it with
`cargo build -p dusrelpdf-uniffi` and link the resulting
`libdusrelpdf_uniffi` / `.xcframework` into your Xcode project).

```swift
import dusrelpdf_uniffi

let merged = try mergePdfs(inputs: [pdfA, pdfB], options: MergeOptions())
print(try getPdfInfo(pdf: merged, common: CommonOptions()).pageCount)  // 5

let rotated = try rotatePages(pdf: pdf, options: RotateOptions(angle: 90))

do {
    _ = try extractPages(pdf: pdf, pages: [9], options: PageSetOptions())
} catch let e as DusrelError {
    print(e)  // PAGE_OUT_OF_RANGE: Page 9 is out of range ...
}
```

## Packaging

The generated sources are wrapped in real packages — see
`bindings/kotlin/README.md` (Gradle Android library, Maven coordinates
`io.github.dusmamud:dusrelpdf:0.1.0`) and `bindings/swift/README.md`
(SwiftPM package `Dusrelpdf`, with an XCFramework binary target wired at
release time). Compilation of both packages is verified by
`.github/workflows/mobile.yml`; neither package is published anywhere yet.

## Regenerating the bindings

Requires UniFFI **0.28.3**. From the repo root:

```sh
scripts/regenerate-uniffi.sh            # regenerate bindings/ in place
scripts/regenerate-uniffi.sh --check    # verify committed bindings match
```

The `uniffi-bindgen` binary is not published as a cargo-installable
crate in 0.28.x, so the script builds the CLI from a small throwaway
crate and caches it at
`$HOME/.cache/dusrelpdf-uniffi-bindgen/uniffi-cli` (set
`UNIFFI_BINDGEN` to use an existing 0.28.3 binary instead).

Generation is deterministic: re-running the script on an unchanged
crate produces byte-identical output — `--check` proves it with
`diff -r` and exits non-zero on drift. (`ktlint` / `swiftformat`
are optional formatters; if absent, uniffi-bindgen prints a warning
and continues.)

## Honest gaps

Kotlin/Swift **compilation was not verified on this machine**: the
build VM has no `kotlinc` and no `swiftc`. The generated sources
were verified only for shape (all 16 functions, records, enums and
error variants present, defaults rendered correctly) and for
byte-identical regeneration. CI / host compilation is the
verification step for the generated code.
