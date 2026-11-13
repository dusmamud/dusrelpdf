# dusrelpdf-go — Go bindings for dusrelpdf

Idiomatic Go (cgo) wrapper over the dusrelpdf Rust core, via the C ABI
in `crates/dusrelpdf-ffi`. All PDF logic lives in Rust; this package
only converts Go values to C buffers / JSON options and back.

Module: `github.com/dusmamud/dusrelpdf-go` (`go.mod`: `go 1.24`, no
dependencies).

## Install

Once the first tag exists, install it like any Go module:

```sh
go get github.com/dusmamud/dusrelpdf-go@v0.1.0
```

That needs the git tag **`packages/go/v0.1.0`** on the repo (a
subdirectory module is tagged by its path, not by a bare `v0.1.0`).
**The tag does not exist yet** — nothing is released — so today that
command fails with "module not found". Until Dus cuts the tag, use a
`replace` directive (below).

No Rust toolchain is needed to *use* the module: the compiled Rust
core is vendored as a static library under `lib/<GOOS>-<GOARCH>/`
(see `lib/README.md`), and the C header is vendored as
`dusrelpdf.h` next to the Go source. The `#cgo` lines in
`dusrelpdf.go` link the archive for your platform and pull in the
system libraries the static Rust library needs (`-ldl -lpthread
-lm` on Linux, CoreFoundation/Security on macOS, ws2_32/userenv/
bcrypt/ntdll on Windows).

One one-time step after install: the Linux archives are stored as
20 MB chunks (the GitHub API rejects blobs over ~40 MB), so
reassemble them once — no Rust, no network:

```sh
go generate github.com/dusmamud/dusrelpdf-go
```

That runs `lib/assemble.sh`, concatenates the parts and verifies
their SHA-256 checksums. CI does the same before `go test`.

Vendored platforms today:

| Platform | Status |
| --- | --- |
| Linux x86_64 | vendored (`lib/linux-amd64/libdusrelpdf.a`) |
| Linux aarch64 | vendored (`lib/linux-arm64/libdusrelpdf.a`) |
| Windows x86_64 | vendored (`lib/windows-amd64/libdusrelpdf.a`, GNU/MinGW build) |
| macOS arm64 / x86_64 | **not vendored yet** — built by `.github/workflows/release-assets.yml` (manual dispatch), then committed by a maintainer |

On a platform with no vendored archive the build fails at link
time with a missing-file error. That is deliberate: it is better
than silently linking a stale or wrong library. See "Build the
library yourself" below for the escape hatch.

## Build the library yourself (fallback)

Only needed if you are developing the Rust core, your platform has
no vendored archive, or you want to check the vendored archive
against a fresh build. From the repo root:

```sh
cargo build -p dusrelpdf-ffi --release
# Linux x86_64: target/release/libdusrelpdf.a
# Linux aarch64: target/aarch64-unknown-linux-gnu/release/libdusrelpdf.a
#   (rustup target add aarch64-unknown-linux-gnu + gcc-aarch64-linux-gnu;
#    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc)
# Windows x86_64 (GNU): target/x86_64-pc-windows-gnu/release/libdusrelpdf.a
#   (rustup target add x86_64-pc-windows-gnu + gcc-mingw-w64-x86-64;
#    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc)
# macOS: target/release/libdusrelpdf.a (built on a Mac; x86_64 needs
#   --target x86_64-apple-darwin, then lipo for a universal archive)
```

This builds the **staticlib** (`crate-type` in
`crates/dusrelpdf-ffi/Cargo.toml` includes `staticlib` for exactly
this reason). Then drop the fresh `.a` into the matching
`lib/<GOOS>-<GOARCH>/` directory and rebuild — the `#cgo` lines pick
it up with no other change. Do not point cgo at the shared library
(`libdusrelpdf.so` / `.dylib` / `.dll`): the module no longer links
the cdylib, because a shared library would need to be found at
runtime (`LD_LIBRARY_PATH` / rpath / next-to-the-binary) by every
`go get` user.

If you also changed the exported C API, regenerate the header and
refresh the vendored copy (see `lib/README.md`) before rebuilding
the archives — a stale `.a` or header silently keeps old behaviour.

Until a tag exists, depend on it with a replace directive:

```go
// go.mod
require github.com/dusmamud/dusrelpdf-go v0.1.0
replace github.com/dusmamud/dusrelpdf-go => /path/to/dusrelpdf/packages/go
```

## Use it

```go
import dusrelpdf "github.com/dusmamud/dusrelpdf-go"

// Merge two PDFs; nil options = Rust defaults.
merged, err := dusrelpdf.Merge([][]byte{a, b}, nil)
if err != nil {
    // err is *dusrelpdf.Error with .Code, .MessageEN, .MessageHI
    var derr *dusrelpdf.Error
    errors.As(err, &derr)
    fmt.Println(derr.Code) // e.g. INVALID_RANGE
}

// Rotate page 1 by 90 degrees.
rotated, err := dusrelpdf.Rotate(pdf, &dusrelpdf.RotateOptions{
    Angle: dusrelpdf.Int(90),
    Pages: []uint32{1},
})

// Summary without modifying the PDF.
info, err := dusrelpdf.GetPDFInfo(pdf, nil)
fmt.Println(info.PageCount, info.Version, info.PageSizes)
```

Until a tag exists, depend on it with a replace directive (see
"Install" above).

## API

| Function | Notes |
| --- | --- |
| `Version()` | library version string |
| `Merge(pdfs, *MergeOptions)` | `[][]byte` in, `[]byte` out |
| `Split(pdf, [][]uint32, *PageSetOptions)` | one output per range |
| `SplitRanges(pdf, "1-3,4-5", *PageSetOptions)` | one output per chunk |
| `SplitEachPage(pdf, *PageSetOptions)` | one output per page |
| `ExtractPages(pdf, []uint32, *PageSetOptions)` | `[]byte` out |
| `ExtractPagesRange(pdf, "1-3,5", *PageSetOptions)` | |
| `RemovePages(pdf, []uint32, *PageSetOptions)` | |
| `ReorderPages(pdf, []uint32, *PageSetOptions)` | every page exactly once |
| `Rotate(pdf, *RotateOptions)` | angle must be a multiple of 90 |
| `GetPDFInfo(pdf, *CommonOptions)` | `*PdfInfo` |
| `ImagesToPDF(images [][]byte, *ImagesToPdfOptions)` | PNG/JPEG in |
| `AddWatermark(pdf, image []byte, *WatermarkOptions)` | image nil = text watermark |
| `AddPageNumbers(pdf, *PageNumberOptions)` | format must contain `{n}` |
| `PlaceSignature(pdf, image, *SignatureOptions)` | image placement, not a digital signature |
| `BulkExtract(pdf, [][]uint32, *PageSetOptions)` | `[]BulkItem`; one bad selection fails only its item |

All functions return `([]byte, error)` / `([][]byte, error)` /
`(*PdfInfo, error)` / `([]BulkItem, error)`. Errors are
`*dusrelpdf.Error` carrying the stable dusrelpdf code (`INVALID_PDF`,
`ENCRYPTED_PDF`, `PAGE_OUT_OF_RANGE`, `EMPTY_INPUT`, `INVALID_RANGE`,
`INVALID_ANGLE`, `INVALID_IMAGE`, `INVALID_ARGUMENT`, `OPERATION_FAILED`)
plus English and Hindi messages.

**Options:** every options struct mirrors the Rust core options
(same snake_case JSON field names). All fields are pointers with
`omitempty` — a nil field is left out and the Rust default applies.
Pass a nil options pointer for all defaults, or use the
`Default*Options()` constructors and change what you need.
`ImagesToPdfOptions.PageSize` accepts `"auto"` / `"a4"` / `"letter"`
or `map[string]any{"width": w, "height": h}` for a custom size.

## Tests

```sh
go test ./...   # 18 tests, real fixture PDFs in testdata/
go vet ./...
gofmt -l .
```

Fixtures (`testdata/three.pdf`, `testdata/five.pdf`) were generated by
the repo's own CLI:

```sh
cargo run -p dusrelpdf-cli -- mock --pages 3 -o packages/go/testdata/three.pdf
cargo run -p dusrelpdf-cli -- mock --pages 5 -o packages/go/testdata/five.pdf
```

Image fixtures (PNG/JPEG) are generated in-test with the Go stdlib.

## Honest limits

- Same v0.1.0 core limits: no rendering, no tolerant repair, merge
  drops original outlines (rebuilt flat), watermark / page-number
  text is ASCII only, signature is a raster image.
- Whole documents live in memory; no streaming.
- macOS has no vendored archive yet (see "Install"), so the module
  does not build there until one is committed.
- No Go module release/tag yet — `go get` will not work until the
  `packages/go/v0.1.0` tag is cut (deliberate: this piece does not
  publish).
