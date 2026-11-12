# dusrelpdf (Python)

Offline PDF toolkit — merge, split, extract, remove, reorder, rotate,
watermark, page numbers, images-to-PDF and signature-image placement.
One Rust core, no network, no uploads: every file is processed on your
own machine.

```sh
pip install dusrelpdf
```

```python
import dusrelpdf

merged = dusrelpdf.merge_pdfs([pdf_a, pdf_b])
pages = dusrelpdf.extract_pages_range(pdf, "1-3,5")
stamped = dusrelpdf.add_watermark(pdf, text="CONFIDENTIAL", opacity=0.3)
info = dusrelpdf.get_pdf_info(pdf)  # {"page_count": 5, "version": "1.7", ...}
```

## Operations (v0.1.0)

| Function | What it does |
|---|---|
| `merge_pdfs(pdfs, ...)` | Multiple PDFs into one, in order |
| `split_pdf(pdf, ranges, ...)` | One PDF into a file per page list |
| `split_pdf_ranges(pdf, ranges, ...)` | Split from a string like `"1-3,4-5"` |
| `split_each_page(pdf, ...)` | One PDF per page |
| `extract_pages(pdf, pages, ...)` | Selected pages into a new PDF |
| `extract_pages_range(pdf, range, ...)` | Extract from a string like `"1-3,5"` |
| `remove_pages(pdf, pages, ...)` | Delete unwanted pages |
| `reorder_pages(pdf, order, ...)` | Change the page order |
| `rotate_pages(pdf, angle=90, ...)` | Rotate pages by 90 / 180 / 270° |
| `images_to_pdf(images, ...)` | Images into a PDF, one page each |
| `add_watermark(pdf, text=..., ...)` | Text or image watermark |
| `add_page_numbers(pdf, format="{n}", ...)` | Page numbers, any format / position |
| `place_signature(pdf, signature, ...)` | Handwritten signature image on a page |
| `get_pdf_info(pdf, ...)` | Page count, version, title, page sizes |
| `bulk_extract(pdf, selections, ...)` | Many extracts in one call; one bad selection fails only its own item |

Every function takes bytes in and returns bytes out. Every operation
accepts keyword arguments mirroring the Rust option fields, plus an
`options` dict override (kwargs win). Option dataclasses
(`MergeOptions`, `WatermarkOptions`, `PageNumberOptions`,
`RotateOptions`, `ImagesToPdfOptions`, `SignatureOptions`) can be
passed as `options` too:

```python
from dusrelpdf import WatermarkOptions

out = dusrelpdf.add_watermark(pdf, options=WatermarkOptions(text="DRAFT", opacity=0.2))
```

Pydantic v2 models for every options struct live in
`dusrelpdf.pydantic` (requires `pip install dusrelpdf[pydantic]`):

```python
from dusrelpdf.pydantic import WatermarkOptions

opts = WatermarkOptions(text="DRAFT", opacity=0.2)
out = dusrelpdf.add_watermark(pdf, options=opts)
```

## Errors

Every failure raises `dusrelpdf.DusrelError` with stable attributes —
never a bare `TypeError` or `ValueError`:

```python
try:
    dusrelpdf.extract_pages(pdf, [99])
except dusrelpdf.DusrelError as exc:
    print(exc.code)       # "PAGE_OUT_OF_RANGE"
    print(exc.reason_en)  # "Page 99 is out of range (this PDF has 3 pages). ..."
    print(exc.reason_hi)  # Roman-script Hindi reason
```

Error codes: `INVALID_PDF`, `ENCRYPTED_PDF`, `PAGE_OUT_OF_RANGE`,
`EMPTY_INPUT`, `INVALID_RANGE`, `INVALID_ANGLE`, `INVALID_IMAGE`,
`INVALID_ARGUMENT`, `INVALID_TYPE`, `OPERATION_FAILED`.

## Honest limits (v0.1.0)

- No PDF rendering (PDF → image), no tolerant repair of malformed
  PDFs, and only empty-password decryption on load — password PDFs
  need `password=...`.
- Merging drops the original outline trees and rebuilds flat
  first-page bookmarks; AcroForm, named destinations and embedded
  files are not carried over.
- Watermark / page-number text is ASCII only (base-14 fonts).
- Signature placement is a raster **image** on the page, not a
  cryptographic digital signature.
- Whole documents live in memory; there is no streaming mode.
