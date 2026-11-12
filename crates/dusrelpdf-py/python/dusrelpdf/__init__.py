"""dusrelpdf — offline PDF toolkit (Python).

Merge, split, extract, remove, reorder, rotate, images-to-PDF,
watermark, page numbers and signature-image placement. Every operation
takes bytes in and returns bytes out, and every failure raises
:class:`DusrelError` with a stable ``code`` plus ``reason_en`` /
``reason_hi`` (Roman-script Hinglish) — never a bare ``TypeError``.

The heavy lifting is the Rust core, reached through the compiled
``dusrelpdf._native`` extension module. This module is the friendly
Python API on top of it: keyword arguments mirroring the Rust option
fields, an ``options`` dict (or dataclass / Pydantic model) override,
and input-type validation before any native call.

Honest limits (v0.1.0, same as the Rust core): no PDF rendering, no
tolerant repair of malformed PDFs, only empty-password decryption on
load (password PDFs need ``password=...``); merging rebuilds outlines
as flat first-page bookmarks; watermark / page-number text is ASCII
only; signature placement is a raster image, not a cryptographic
digital signature; whole documents live in memory.
"""

from __future__ import annotations

import dataclasses
from dataclasses import dataclass, field

try:
    from . import _native
except ImportError:  # pragma: no cover - built wheel always ships it
    _native = None  # type: ignore[assignment]

__version__ = "0.1.0"

__all__ = [
    "DusrelError",
    "merge_pdfs",
    "split_pdf",
    "split_pdf_ranges",
    "split_each_page",
    "extract_pages",
    "extract_pages_range",
    "remove_pages",
    "reorder_pages",
    "rotate_pages",
    "images_to_pdf",
    "add_watermark",
    "add_page_numbers",
    "place_signature",
    "get_pdf_info",
    "bulk_extract",
    "make_test_pdf",
    "make_signature_png",
    "make_photo_jpeg",
    "default_options",
    "CommonOptions",
    "MergeOptions",
    "PageSetOptions",
    "WatermarkOptions",
    "PageNumberOptions",
    "RotateOptions",
    "ImagesToPdfOptions",
    "SignatureOptions",
    "PdfInfo",
    "__version__",
]


# ---------------------------------------------------------------------------
# Errors
# ---------------------------------------------------------------------------

class DusrelError(Exception):
    """Every dusrelpdf failure, with a stable machine-readable code."""

    def __init__(self, code, reason_en="", reason_hi=""):
        super().__init__(f"{code}: {reason_en}")
        self.code = code
        self.reason_en = reason_en
        self.reason_hi = reason_hi


def _invalid_type(what_en, what_hi):
    return DusrelError("INVALID_TYPE", what_en, what_hi)


def _from_native(exc):
    """Parse the native `CODE\\x1f reason_en \\x1f reason_hi` payload."""
    parts = str(exc).split("\x1f")
    if len(parts) == 3 and parts[0]:
        return DusrelError(parts[0], parts[1], parts[2])
    return DusrelError(
        "OPERATION_FAILED",
        f"Operation failed: {exc}",
        f"Operation fail hua: {exc}",
    )


def _call(fn, *args):
    """Call a native function, translating every failure to DusrelError."""
    if _native is None:  # pragma: no cover
        raise DusrelError(
            "OPERATION_FAILED",
            "The dusrelpdf native module is not available in this install.",
            "Is install me dusrelpdf ka native module nahi hai.",
        )
    try:
        return fn(*args)
    except DusrelError:
        raise
    except ValueError as exc:
        raise _from_native(exc) from None
    except TypeError:
        raise _invalid_type(
            "Wrong argument type passed to the native module.",
            "Native module ko galat type ka argument mila.",
        ) from None
    except Exception as exc:  # native panic / unexpected failure
        raise DusrelError(
            "OPERATION_FAILED",
            f"Operation failed: {exc}",
            f"Operation fail hua: {exc}",
        ) from None


# ---------------------------------------------------------------------------
# Option resolution: kwargs win over the `options` dict, which wins over
# the Rust defaults. A keyword left at _UNSET means "not specified".
# ---------------------------------------------------------------------------

class _Unset:
    def __repr__(self):
        return "<unset>"


_UNSET = _Unset()


def default_options():
    """Default shared options: no password, compressed output."""
    return {"password": None, "compress": True}


def _options_to_dict(options):
    """Accept a dict, a dataclass instance or a Pydantic model."""
    if options is None:
        return {}
    if isinstance(options, dict):
        return dict(options)
    if dataclasses.is_dataclass(options) and not isinstance(options, type):
        if hasattr(options, "to_dict"):
            return dict(options.to_dict())
        return dataclasses.asdict(options)
    if hasattr(options, "model_dump"):  # Pydantic v2 model
        try:
            return dict(options.model_dump(mode="json"))
        except TypeError:
            return dict(options.model_dump())
    raise _invalid_type(
        "options must be a dict, an options dataclass, or a Pydantic model.",
        "options dict, options dataclass ya Pydantic model hona chahiye.",
    )


def _resolve_kwargs(kwargs, options):
    """Merge explicit kwargs over the options dict. _UNSET = not given."""
    resolved = _options_to_dict(options)
    for key, value in kwargs.items():
        if value is not _UNSET:
            resolved[key] = value
    return resolved


# ---------------------------------------------------------------------------
# Input validation (Python layer — the indycheck v0.2.1 lesson: wrong
# input types raise DusrelError("INVALID_TYPE"), never TypeError).
# ---------------------------------------------------------------------------

def _as_bytes(value, name):
    if isinstance(value, (bytes, bytearray, memoryview)):
        return bytes(value)
    raise _invalid_type(
        f"{name} must be bytes, bytearray or memoryview.",
        f"{name} bytes, bytearray ya memoryview hona chahiye.",
    )


def _as_bytes_list(value, name):
    if not isinstance(value, (list, tuple)):
        raise _invalid_type(
            f"{name} must be a list of bytes.",
            f"{name} bytes ki list honi chahiye.",
        )
    return [_as_bytes(item, name) for item in value]


def _is_int(value):
    return isinstance(value, int) and not isinstance(value, bool)


def _as_page_list(value, name):
    if isinstance(value, str) or not isinstance(value, (list, tuple)):
        raise _invalid_type(
            f"{name} must be a list of page numbers (integers).",
            f"{name} page numbers (integers) ki list honi chahiye.",
        )
    for item in value:
        if not _is_int(item):
            raise _invalid_type(
                f"{name} must contain only integers, got {type(item).__name__}.",
                f"{name} me sirf integers hone chahiye, {type(item).__name__} mila.",
            )
    return list(value)


def _as_str(value, name):
    if value is None or isinstance(value, str):
        return value
    raise _invalid_type(
        f"{name} must be a string.",
        f"{name} string hona chahiye.",
    )


def _validate_options(resolved):
    """Type-check a merged options dict before it reaches native code."""
    for key, value in resolved.items():
        if value is None:
            continue
        if key in ("password", "text", "format", "font", "position", "fit"):
            if not isinstance(value, str):
                raise _invalid_type(
                    f"Option {key!r} must be a string.",
                    f"Option {key!r} string hona chahiye.",
                )
        elif key in ("compress", "preserve_metadata", "add_bookmarks"):
            if not isinstance(value, bool):
                raise _invalid_type(
                    f"Option {key!r} must be a bool.",
                    f"Option {key!r} bool hona chahiye.",
                )
        elif key in ("opacity", "rotation", "font_size", "margin", "x", "y",
                     "width", "height", "image_width", "image_height"):
            if not (isinstance(value, (int, float)) and not isinstance(value, bool)):
                raise _invalid_type(
                    f"Option {key!r} must be a number.",
                    f"Option {key!r} number hona chahiye.",
                )
        elif key in ("start_at", "angle", "page"):
            if not _is_int(value):
                raise _invalid_type(
                    f"Option {key!r} must be an integer.",
                    f"Option {key!r} integer hona chahiye.",
                )
        elif key == "pages":
            _as_page_list(value, "Option 'pages'")
        elif key == "image":
            if not isinstance(value, (bytes, bytearray, memoryview)):
                raise _invalid_type(
                    "Option 'image' must be bytes, bytearray or memoryview.",
                    "Option 'image' bytes, bytearray ya memoryview hona chahiye.",
                )
            resolved[key] = bytes(value)
        elif key == "background":
            if isinstance(value, (list, tuple)) and len(value) == 3 and all(
                isinstance(c, (int, float)) and not isinstance(c, bool) for c in value
            ):
                resolved[key] = list(value)
            else:
                raise _invalid_type(
                    "Option 'background' must be 3 numbers, e.g. [1.0, 1.0, 1.0].",
                    "Option 'background' 3 numbers hone chahiye, jaise [1.0, 1.0, 1.0].",
                )
        elif key == "page_size":
            if isinstance(value, str):
                if value not in ("auto", "a4", "letter"):
                    raise DusrelError(
                        "INVALID_ARGUMENT",
                        "Option 'page_size' must be 'auto', 'a4', 'letter', or a custom size.",
                        "Option 'page_size' 'auto', 'a4', 'letter' ya custom size hona chahiye.",
                    )
            elif isinstance(value, dict):
                if "custom" in value:
                    inner = value["custom"]
                    if not (isinstance(inner, dict) and "width" in inner
                            and "height" in inner):
                        raise DusrelError(
                            "INVALID_ARGUMENT",
                            "Custom page_size needs {'custom': {'width': w, 'height': h}}.",
                            "Custom page_size ke liye {'custom': {'width': w, 'height': h}} chahiye.",
                        )
                elif "width" in value and "height" in value:
                    resolved[key] = {"custom": dict(value)}
                else:
                    raise DusrelError(
                        "INVALID_ARGUMENT",
                        "Custom page_size needs {'width': w, 'height': h}.",
                        "Custom page_size ke liye {'width': w, 'height': h} chahiye.",
                    )
            elif isinstance(value, (list, tuple)) and len(value) == 2 and all(
                isinstance(c, (int, float)) and not isinstance(c, bool) for c in value
            ):
                resolved[key] = {"custom": {"width": value[0], "height": value[1]}}
            else:
                raise _invalid_type(
                    "Option 'page_size' must be 'auto', 'a4', 'letter', a dict, or [width, height].",
                    "Option 'page_size' 'auto', 'a4', 'letter', dict ya [width, height] hona chahiye.",
                )
        # Unknown keys are ignored (forward compatibility with new options).


def _final_options(kwargs, options):
    resolved = _resolve_kwargs(kwargs, options)
    _validate_options(resolved)
    return resolved


# ---------------------------------------------------------------------------
# Option dataclasses — one clear way to build options objects.
# Defaults mirror the Rust core. Pass an instance as `options=...`.
# ---------------------------------------------------------------------------

@dataclass
class CommonOptions:
    password: str | None = None
    compress: bool = True

    def to_dict(self):
        return {"password": self.password, "compress": self.compress}


@dataclass
class MergeOptions:
    password: str | None = None
    compress: bool = True
    preserve_metadata: bool = True
    add_bookmarks: bool = True

    def to_dict(self):
        return dataclasses.asdict(self)


@dataclass
class PageSetOptions:
    password: str | None = None
    compress: bool = True

    def to_dict(self):
        return dataclasses.asdict(self)


@dataclass
class WatermarkOptions:
    password: str | None = None
    compress: bool = True
    text: str | None = "DRAFT"
    image: bytes | None = None
    opacity: float = 0.3
    rotation: float = 45.0
    font_size: float = 48.0
    font: str = "helvetica"
    position: str = "center"
    x: float | None = None
    y: float | None = None
    pages: list | None = None
    image_width: float | None = None
    image_height: float | None = None

    def to_dict(self):
        return dataclasses.asdict(self)


@dataclass
class PageNumberOptions:
    password: str | None = None
    compress: bool = True
    format: str = "{n}"
    position: str = "bottom_center"
    start_at: int = 1
    font_size: float = 10.0
    font: str = "helvetica"
    margin: float = 36.0
    pages: list | None = None

    def to_dict(self):
        return dataclasses.asdict(self)


@dataclass
class RotateOptions:
    password: str | None = None
    compress: bool = True
    angle: int = 90
    pages: list | None = None

    def to_dict(self):
        return dataclasses.asdict(self)


@dataclass
class ImagesToPdfOptions:
    page_size: str | dict | list = "a4"
    fit: str = "contain"
    background: list | tuple = (1.0, 1.0, 1.0)

    def to_dict(self):
        d = dataclasses.asdict(self)
        d["background"] = list(self.background)
        return d


@dataclass
class SignatureOptions:
    password: str | None = None
    compress: bool = True
    page: int = 1
    x: float = 36.0
    y: float = 36.0
    width: float | None = None
    height: float | None = None

    def to_dict(self):
        return dataclasses.asdict(self)


@dataclass
class PdfInfo:
    page_count: int
    version: str
    encrypted: bool
    title: str | None = None
    page_sizes: list = field(default_factory=list)

    def to_dict(self):
        return dataclasses.asdict(self)


# Backwards-friendly alias: the Rust core calls these "CommonOptions".
PageOptions = PageSetOptions


# ---------------------------------------------------------------------------
# Operations
# ---------------------------------------------------------------------------

def merge_pdfs(pdfs, *, preserve_metadata=_UNSET, add_bookmarks=_UNSET,
               password=_UNSET, compress=_UNSET, options=None):
    """Merge multiple PDFs into one, in input order.

    ``pdfs`` is a list of PDF bytes. Returns the merged PDF bytes.
    """
    inputs = _as_bytes_list(pdfs, "pdfs")
    opts = _final_options({
        "preserve_metadata": preserve_metadata,
        "add_bookmarks": add_bookmarks,
        "password": password,
        "compress": compress,
    }, options)
    return _call(_native.merge_pdfs, inputs, opts)


def split_pdf(pdf, ranges, *, password=_UNSET, compress=_UNSET, options=None):
    """Split one PDF into one output per page list.

    ``ranges`` is a list of page-number lists, e.g. ``[[1, 2], [3]]``.
    Returns a list of PDF bytes, one per range.
    """
    data = _as_bytes(pdf, "pdf")
    if not isinstance(ranges, (list, tuple)):
        raise _invalid_type(
            "ranges must be a list of page-number lists.",
            "ranges page-number lists ki list honi chahiye.",
        )
    lists = [_as_page_list(item, "ranges") for item in ranges]
    opts = _final_options({"password": password, "compress": compress}, options)
    return _call(_native.split_pdf, data, lists, opts)


def split_pdf_ranges(pdf, ranges, *, password=_UNSET, compress=_UNSET, options=None):
    """Split one PDF from a range string like ``"1-3,4-5"``.

    Each comma-separated chunk becomes one output file. Returns a list
    of PDF bytes.
    """
    data = _as_bytes(pdf, "pdf")
    spec = _as_str(ranges, "ranges")
    if spec is None:
        raise _invalid_type(
            "ranges must be a string like '1-3,4-5'.",
            "ranges '1-3,4-5' jaisi string honi chahiye.",
        )
    opts = _final_options({"password": password, "compress": compress}, options)
    return _call(_native.split_pdf_ranges, data, spec, opts)


def split_each_page(pdf, *, password=_UNSET, compress=_UNSET, options=None):
    """Split into one PDF per page. Returns a list of PDF bytes."""
    data = _as_bytes(pdf, "pdf")
    opts = _final_options({"password": password, "compress": compress}, options)
    return _call(_native.split_each_page, data, opts)


def extract_pages(pdf, pages, *, password=_UNSET, compress=_UNSET, options=None):
    """Extract selected pages (1-based, in the order given) into a new PDF."""
    data = _as_bytes(pdf, "pdf")
    sel = _as_page_list(pages, "pages")
    opts = _final_options({"password": password, "compress": compress}, options)
    return _call(_native.extract_pages, data, sel, opts)


def extract_pages_range(pdf, range, *, password=_UNSET, compress=_UNSET, options=None):
    """Extract from a range string like ``"1-3,5"``. Returns PDF bytes."""
    data = _as_bytes(pdf, "pdf")
    spec = _as_str(range, "range")
    if spec is None:
        raise _invalid_type(
            "range must be a string like '1-3,5'.",
            "range '1-3,5' jaisi string honi chahiye.",
        )
    opts = _final_options({"password": password, "compress": compress}, options)
    return _call(_native.extract_pages_range, data, spec, opts)


def remove_pages(pdf, pages, *, password=_UNSET, compress=_UNSET, options=None):
    """Delete pages (1-based). An empty list returns the PDF unchanged."""
    data = _as_bytes(pdf, "pdf")
    sel = _as_page_list(pages, "pages")
    opts = _final_options({"password": password, "compress": compress}, options)
    return _call(_native.remove_pages, data, sel, opts)


def reorder_pages(pdf, order, *, password=_UNSET, compress=_UNSET, options=None):
    """Change the page order. ``order`` must list every page exactly once."""
    data = _as_bytes(pdf, "pdf")
    perm = _as_page_list(order, "order")
    opts = _final_options({"password": password, "compress": compress}, options)
    return _call(_native.reorder_pages, data, perm, opts)


def rotate_pages(pdf, *, angle=_UNSET, pages=_UNSET,
                 password=_UNSET, compress=_UNSET, options=None):
    """Rotate pages by a multiple of 90 degrees (default 90).

    Negative angles are normalised (-90 becomes 270). ``pages`` limits
    the rotation to selected pages; ``None`` means every page.
    """
    data = _as_bytes(pdf, "pdf")
    opts = _final_options({
        "angle": angle, "pages": pages,
        "password": password, "compress": compress,
    }, options)
    return _call(_native.rotate_pages, data, opts)


def images_to_pdf(images, *, page_size=_UNSET, fit=_UNSET, background=_UNSET,
                  options=None):
    """Build a PDF with one page per image, in input order.

    ``page_size`` is ``"auto"`` (page sized to the image), ``"a4"``,
    ``"letter"``, or a custom size as ``{"width": w, "height": h}`` or
    ``[w, h]``. ``fit`` is ``"contain"`` (default) or ``"fill"``.
    """
    imgs = _as_bytes_list(images, "images")
    opts = _final_options({
        "page_size": page_size, "fit": fit, "background": background,
    }, options)
    return _call(_native.images_to_pdf, imgs, opts)


def add_watermark(pdf, *, text=_UNSET, image=_UNSET, opacity=_UNSET,
                  rotation=_UNSET, font_size=_UNSET, font=_UNSET,
                  position=_UNSET, x=_UNSET, y=_UNSET, pages=_UNSET,
                  image_width=_UNSET, image_height=_UNSET,
                  password=_UNSET, compress=_UNSET, options=None):
    """Stamp a text or image watermark over the selected pages.

    Exactly one of ``text`` or ``image``. Defaults: text ``"DRAFT"``,
    opacity ``0.3``, rotation ``45``, font size ``48``. When ``image``
    is given and ``text`` is not, no text watermark is added.
    """
    data = _as_bytes(pdf, "pdf")
    opts = _final_options({
        "text": text, "image": image, "opacity": opacity,
        "rotation": rotation, "font_size": font_size, "font": font,
        "position": position, "x": x, "y": y, "pages": pages,
        "image_width": image_width, "image_height": image_height,
        "password": password, "compress": compress,
    }, options)
    if "text" not in opts:
        # No text given: an image watermark stands alone, otherwise the
        # Rust default text ("DRAFT") applies.
        opts["text"] = None if opts.get("image") is not None else "DRAFT"
    return _call(_native.add_watermark, data, opts)


def add_page_numbers(pdf, *, format=_UNSET, position=_UNSET, start_at=_UNSET,
                     font_size=_UNSET, font=_UNSET, margin=_UNSET,
                     pages=_UNSET, password=_UNSET, compress=_UNSET,
                     options=None):
    """Stamp page numbers over the selected pages.

    ``format`` uses ``{n}`` for the page number and ``{total}`` for the
    page count, e.g. ``"Page {n} of {total}"`` (default ``"{n}"``).
    """
    data = _as_bytes(pdf, "pdf")
    opts = _final_options({
        "format": format, "position": position, "start_at": start_at,
        "font_size": font_size, "font": font, "margin": margin,
        "pages": pages, "password": password, "compress": compress,
    }, options)
    return _call(_native.add_page_numbers, data, opts)


def place_signature(pdf, signature, *, page=_UNSET, x=_UNSET, y=_UNSET,
                    width=_UNSET, height=_UNSET,
                    password=_UNSET, compress=_UNSET, options=None):
    """Place a handwritten signature IMAGE on one page (default page 1).

    This is image placement only — not a cryptographic digital
    signature. Transparent PNGs keep their transparency.
    """
    data = _as_bytes(pdf, "pdf")
    sig = _as_bytes(signature, "signature")
    opts = _final_options({
        "page": page, "x": x, "y": y, "width": width, "height": height,
        "password": password, "compress": compress,
    }, options)
    return _call(_native.place_signature, data, sig, opts)


def get_pdf_info(pdf, *, password=_UNSET, compress=_UNSET, options=None):
    """Read a PDF's summary without modifying it.

    Returns ``{"page_count", "version", "encrypted", "title",
    "page_sizes"}`` where ``page_sizes`` is a list of ``(width,
    height)`` tuples in points, in page order.
    """
    data = _as_bytes(pdf, "pdf")
    opts = _final_options({"password": password, "compress": compress}, options)
    info = _call(_native.get_pdf_info, data, opts)
    info["page_sizes"] = [tuple(size) for size in info.get("page_sizes", [])]
    return info


def bulk_extract(pdf, selections, *, password=_UNSET, compress=_UNSET, options=None):
    """Extract many selections from one PDF in a single call.

    ``selections`` is a list of page-number lists. Returns one dict per
    input, in order: ``{"index", "pdf", "error_code", "error"}``. One
    bad selection fails only its own item, never the whole batch.
    """
    data = _as_bytes(pdf, "pdf")
    if not isinstance(selections, (list, tuple)):
        raise _invalid_type(
            "selections must be a list of page-number lists.",
            "selections page-number lists ki list honi chahiye.",
        )
    sels = [_as_page_list(item, "selections") for item in selections]
    opts = _final_options({"password": password, "compress": compress}, options)
    return _call(_native.bulk_extract, data, sels, opts)


# ---------------------------------------------------------------------------
# Fixtures — test data built by the Rust core (same builders the Rust
# tests use). For tests, demos and form previews only.
# ---------------------------------------------------------------------------

def make_test_pdf(pages=1, title=None):
    """Build an N-page PDF; every page says ``Page <n>`` (A4)."""
    if not _is_int(pages):
        raise _invalid_type(
            "pages must be an integer.",
            "pages integer hona chahiye.",
        )
    if pages < 1:
        raise DusrelError(
            "INVALID_ARGUMENT",
            "make_test_pdf needs at least 1 page.",
            "make_test_pdf ke liye kam se kam 1 page chahiye.",
        )
    if title is not None and not isinstance(title, str):
        raise _invalid_type(
            "title must be a string or None.",
            "title string ya None hona chahiye.",
        )
    return _call(_native.make_test_pdf, pages, title)


def make_signature_png():
    """A small PNG that looks like a handwritten signature: an opaque
    dark stroke on a fully transparent background."""
    return _call(_native.make_signature_png)


def make_photo_jpeg():
    """A small opaque JPEG (a red/blue photo-like image)."""
    return _call(_native.make_photo_jpeg)
