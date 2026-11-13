# dusrelpdf

Offline PDF toolkit — merge, split, extract, remove, reorder, rotate,
images-to-PDF, watermark, page numbers and signature-image placement.
One Rust core, wrapped as WASM; no network, no uploads: every file is
processed on your own machine.

```sh
npm install dusrelpdf
```

```js
const fs = require("node:fs");
const { mergePdfs, extractPagesRange, addWatermark, getPdfInfo } = require("dusrelpdf");

const a = fs.readFileSync("a.pdf"); // Buffer is a Uint8Array — accepted as-is
const b = fs.readFileSync("b.pdf");

const merged = mergePdfs([a, b]);
console.log(getPdfInfo(merged).pageCount); // pages in a + pages in b

const firstThree = extractPagesRange(merged, "1-3");
const stamped = addWatermark(firstThree, { text: "CONFIDENTIAL" });
fs.writeFileSync("out.pdf", stamped);
```

ESM works too (`import { mergePdfs } from "dusrelpdf"`). The API is
**synchronous** — the WASM calls are synchronous, so there is nothing
to `await` (awaiting a plain value still works, it just does nothing).
Node.js only: the WASM is built with `--target nodejs`.

Every function takes bytes in and returns bytes out (`Uint8Array`;
splits return an array of them). On failure it throws a `DusrelError`
with a stable `code` plus `reasonEn` / `reasonHi` — never a raw
TypeError, never a raw JSON string:

```js
const { extractPages, DusrelError } = require("dusrelpdf");

try {
  extractPages(pdf, [99]);
} catch (err) {
  if (err instanceof DusrelError) {
    console.log(err.code); // "PAGE_OUT_OF_RANGE"
    console.log(err.reasonEn); // "Page 99 is out of range (this PDF has 3 pages). ..."
    console.log(err.reasonHi); // "Page 99 range se bahar hai (is PDF me 3 pages hain). ..."
  }
}
```

## The 10 operations

| Function | What it does |
|---|---|
| `mergePdfs(pdfs, options?)` | Multiple PDFs into one, in input order |
| `splitPdf(pdf, ranges, options?)` | One PDF into a file per page list, e.g. `[[1, 2], [3]]` |
| `splitPdfRanges(pdf, "1-3,4-5", options?)` | One output per comma-separated range chunk |
| `splitEachPage(pdf, options?)` | One PDF per page, in page order |
| `extractPages(pdf, pages, options?)` | Selected pages into a new PDF, in the order given (repeats allowed) |
| `extractPagesRange(pdf, "1-3,5", options?)` | Extract from a range string |
| `removePages(pdf, pages, options?)` | Delete pages (an empty list is a no-op) |
| `reorderPages(pdf, order, options?)` | Change the page order — `order` must list every page exactly once |
| `rotatePages(pdf, options?)` | Rotate pages by 90 / 180 / 270° (default 90, all pages) |
| `imagesToPdf(images, options?)` | Images (PNG/JPEG) into a PDF, one page per image |
| `addWatermark(pdf, options?)` | Text or image watermark, with opacity + rotation |
| `addPageNumbers(pdf, options?)` | Page numbers, any format / position |
| `placeSignature(pdf, signature, options?)` | Handwritten signature image on a page |

Helpers: `version()` (`"0.1.0"`), `getPdfInfo(pdf, options?)`
(`{ pageCount, version, encrypted, title?, pageSizes: [[w, h], …] }`),
`bulkExtract(pdf, selections, options?)` (one result per selection —
a bad selection fails only its own item, with `errorCode` / `error`),
and `makeTestPdf(pages)` (mock multi-page PDF for tests and demos;
every page says `Page <n>`).

## Options

Every function takes `(input, options?)` — there are no separate
`*_with_options` variants; `options` is always optional and omitted
fields use the Rust defaults. The TS API uses **camelCase** keys and
the wrapper translates them to the Rust **snake_case** JSON keys
before calling WASM:

| TS option | Rust JSON key | Type | Default |
|---|---|---|---|
| `password` | `password` | `string` | — (no password) |
| `compress` | `compress` | `boolean` | `true` |
| `preserveMetadata` | `preserve_metadata` | `boolean` | `true` |
| `addBookmarks` | `add_bookmarks` | `boolean` | `true` |
| `angle` | `angle` | `number` | `90` |
| `pages` | `pages` | `number[]` (1-based) | all pages |
| `page` | `page` | `number` (1-based) | `1` |
| `x`, `y` | `x`, `y` | `number` (points) | watermark: — · signature: `36` |
| `width`, `height` | `width`, `height` | `number` (points) | signature: width `144` |
| `text` | `text` | `string` | `"DRAFT"` (watermark) |
| `image` | `image` | `Uint8Array` | — |
| `opacity` | `opacity` | `number` 0–1 | `0.3` |
| `rotation` | `rotation` | `number` (degrees) | `45` |
| `fontSize` | `font_size` | `number` (points) | watermark `48` · numbers `10` |
| `font` | `font` | `"helvetica" \| "helvetica_bold" \| "times_roman" \| "courier" \| "courier_bold"` | `"helvetica"` |
| `position` | `position` | `"top_left" \| "top_center" \| "top_right" \| "bottom_left" \| "bottom_center" \| "bottom_right" \| "center"` | watermark `"center"` · numbers `"bottom_center"` |
| `format` | `format` | `string` with `{n}` / `{total}` | `"{n}"` |
| `startAt` | `start_at` | `number` | `1` |
| `margin` | `margin` | `number` (points) | `36` |
| `pageSize` | `page_size` | `"auto" \| "a4" \| "letter" \| { width, height }` | `"a4"` |
| `fit` | `fit` | `"contain" \| "fill"` | `"contain"` |
| `background` | `background` | `[r, g, b]` 0–1 | `[1, 1, 1]` (white) |
| `imageWidth`, `imageHeight` | `image_width`, `image_height` | `number` (points) | natural size at 96 dpi |

Per-function option types: `MergeOptions`, `SplitOptions` /
`ExtractOptions` / `RemoveOptions` / `ReorderOptions` (all just
`password` + `compress`), `RotateOptions`, `ImagesToPdfOptions`,
`WatermarkOptions`, `PageNumberOptions`, `SignatureOptions`,
`CommonOptions` — all exported from the package.

Two things worth knowing about options:

- **The wrapper always sends complete option objects.** Serde does
  NOT fill missing non-`Option` fields from the Rust `Default` impl —
  calling wasm with a partial object like `{}` fails with
  `INVALID_ARGUMENT: missing field 'angle'` (verified against the
  built wasm). The wrapper fills every missing field with the Rust
  default listed above, so from the outside, omitting a field is
  exactly the default.
- **Watermark `image` is sent as a plain JS number array**, not a
  `Uint8Array`, because serde-wasm-bindgen cannot decode a
  `Uint8Array` into `Vec<u8>`. You pass a `Uint8Array` in TS; the
  conversion is the wrapper's job. `text` and `image` are mutually
  exclusive — set exactly one (neither means the default `"DRAFT"`
  text). For an image watermark pass `rotation: 0` unless you want
  the image rotated 45°.

```js
// Images to PDF, Letter pages, stretched to fill:
imagesToPdf([png, jpeg], { pageSize: "letter", fit: "fill" });

// Page numbers, "Page 1 of 5" style, starting at 0, top-right:
addPageNumbers(pdf, { format: "Page {n} of {total}", startAt: 0, position: "top_right" });

// Signature on page 2, 2 inches wide, bottom-left:
placeSignature(pdf, signaturePng, { page: 2, x: 36, y: 36, width: 144 });

// Rotate only page 1 by 180°:
rotatePages(pdf, { angle: 180, pages: [1] });

// Bulk extract — item 2 fails, the rest still succeed:
const items = bulkExtract(pdf, [[1, 2], [99], [3]]);
// items[1] = { index: 1, errorCode: "PAGE_OUT_OF_RANGE", error: "Page 99 is out of range …" }
```

### Zod schemas

Optional integration — `zod` is an optional peer dependency.
Import from `dusrelpdf/zod`:

```js
const { mergeOptionsSchema, watermarkOptionsSchema, pdfBytesSchema } = require("dusrelpdf/zod");

mergeOptionsSchema.parse({ preserveMetadata: false }); // ok
pdfBytesSchema.parse("not bytes"); // throws ZodError
```

Schemas exist for every Options type (`commonOptionsSchema`,
`mergeOptionsSchema`, `pageSetOptionsSchema`, `rotateOptionsSchema`,
`imagesToPdfOptionsSchema`, `watermarkOptionsSchema`,
`pageNumberOptionsSchema`, `signatureOptionsSchema`) plus inputs
(`pdfBytesSchema`, `pdfArraySchema`, `pageListSchema`,
`rangeStringSchema`, `pageSizeSchema`, `positionSchema`,
`baseFontSchema`, `imageFitSchema`) and `pdfInfoSchema`. They check
shape only — page existence, PDF validity and value ranges are
enforced by the core as `DusrelError`s.

## Error codes

| Code | When | English | Hindi |
|---|---|---|---|
| `INVALID_PDF` | Input is not a PDF, is truncated, or cannot be parsed | `Invalid PDF: …` | `Galat PDF hai: …` |
| `ENCRYPTED_PDF` | The PDF is encrypted and no (or a wrong) password was supplied | `This PDF is encrypted. Supply the password with Options.password.` | `Ye PDF encrypted hai. Options.password ke saath password dein.` |
| `PAGE_OUT_OF_RANGE` | A page number is outside `1..=page_count` | `Page {page} is out of range (this PDF has {total} pages). Pages are numbered from 1.` | `Page {page} range se bahar hai (is PDF me {total} pages hain). Pages 1 se number hote hain.` |
| `EMPTY_INPUT` | An input list / byte buffer is empty where at least one item is required | `{what} is empty.` | `{what} khaali hai.` |
| `INVALID_RANGE` | A range string like `"1-3,5"` is malformed or out of range | `Invalid page range "{range}". Use forms like "1-3,5" with pages numbered from 1.` | `Galat page range "{range}" hai. "1-3,5" jaise form use karein, pages 1 se number hote hain.` |
| `INVALID_ANGLE` | A rotation angle is not a multiple of 90° | `Rotation angle {angle} is not a multiple of 90 degrees. Use 0, 90, 180 or 270.` | `Rotation angle {angle} 90 ka multiple nahi hai. 0, 90, 180 ya 270 use karein.` |
| `INVALID_IMAGE` | An image is missing, corrupt, or in an unsupported format | `Invalid image: …` | `Galat image hai: …` |
| `INVALID_ARGUMENT` | A wrong argument (bad permutation, duplicate pages, bad opacity, format without `{n}`, text + image together, …) | varies | varies |
| `INVALID_TYPE` | A non-`Uint8Array` value reached a typed API (JS-only code — the Rust core takes `&[u8]`) | `… must be a Uint8Array.` | `… Uint8Array hona chahiye.` |
| `OPERATION_FAILED` | The underlying PDF operation failed for another reason | varies | varies |

Range quirks worth knowing (they come from the core, not the
wrapper): page ranges are validated against the real page count —
`"1-9"` on a 3-page PDF is `INVALID_RANGE`, while `[99]` as an
explicit list is `PAGE_OUT_OF_RANGE`. In `reorderPages`, an
out-of-range entry is `PAGE_OUT_OF_RANGE` (checked before the
duplicate check); a wrong-length list or a duplicate page is
`INVALID_ARGUMENT`. `removePages(pdf, [])` is a no-op returning the
input bytes unchanged; `extractPages(pdf, [])` is `EMPTY_INPUT`.

## Honest limits (v0.1.0)

- **No PDF rendering** (PDF → image), no tolerant repair of malformed
  PDFs, and only empty-password decryption on load — encrypted PDFs
  need `options.password`.
- **Merging drops the original outline trees** and rebuilds flat
  first-page bookmarks; AcroForm, named destinations and embedded
  files are not carried over.
- **Watermark / page-number text is ASCII only** (base-14 fonts). Use
  an image watermark for other scripts.
- **Signature placement is a raster image on the page, not a
  cryptographic digital signature** — no signature field, no
  certificate, nothing verifiable. Transparent PNGs keep their
  transparency via an `/SMask` soft-mask (without it the signature
  would render as an opaque rectangle).
- Whole documents live in memory; there is no streaming mode.

## Native addon (optional, faster)

`npm install dusrelpdf` also installs a prebuilt native addon for
your platform when one exists — the `dusrelpdf-<triple>`
`optionalDependencies` (`linux-x64-gnu`, `linux-x64-musl`,
`linux-arm64-gnu`, `darwin-x64`, `darwin-arm64`, `win32-x64-msvc`),
each a tiny package holding one `.node` file. The loader picks it
up automatically and `backend()` reports `"native"`; when no
prebuild matches your platform (or `DUSREL_PDF_FORCE_WASM=1` is
set) it falls back to the bundled WASM build and `backend()`
reports `"wasm"`. Same API, same errors, either way — the native
path is just faster. The prebuilds are built by
`.github/workflows/napi.yml` from `crates/dusrelpdf-napi`.

## Building from source

The WASM is built from the Rust core in this repo
(`crates/dusrelpdf-wasm`) and is **not** checked in. Building needs
Rust + `wasm-pack` (0.15.x) and Node.js:

```sh
npm ci            # or: npm install
npm run build     # wasm-pack build --target nodejs + tsc
npm run typecheck # tsc --noEmit
npm test          # vitest run (pretest rebuilds the wasm if missing)
```

`npm run build` writes the wasm-pack output to `wasm-pkg/` (gitignored)
and the compiled TypeScript to `dist/` (gitignored). The website uses
`--target web` separately; this package only owns `--target nodejs`.
The published tarball ships the WASM files (`wasm-pkg/dusrelpdf_wasm.js`
+ `.wasm`) alongside `dist/` — the fallback only works on an
installed package because of that (listed file-by-file in
`package.json` `files`: a bare `"wasm-pkg"` directory entry does not
survive the `*` in wasm-pack's generated `wasm-pkg/.gitignore`).

To build the native addon locally instead of (or as well as) the
WASM build:

```sh
npm run native:build   # napi build --platform --release, in crates/dusrelpdf-napi
npm test               # native tests run when the .node exists, else they skip
DUSREL_PDF_FORCE_WASM=1 npm test   # forced-WASM run, same suite
```

## Lockfile note (platform packages)

`package-lock.json` lists the six `dusrelpdf-<triple>` platform
packages as `optionalDependencies`. They are **not published yet**,
so their lockfile entries carry placeholder `integrity` hashes
(npm tolerates the unresolvable optional deps — `npm ci` skips
them). **After the first npm publish of the platform packages**,
regenerate the lockfile with `npm install` so the entries carry
real `resolved` URLs + integrity hashes, and commit the result.
Until then, do not delete the entries — `npm ci` (npm 11) fails
when `package.json` and `package-lock.json` are out of sync.

## Licence

MIT — author `dusmamud`.
