"""Optional Pydantic v2 models for dusrelpdf.

Pydantic is **never** required to import or use the main ``dusrelpdf``
package. This module is pure Python — it does not import the native
extension — and it imports cleanly without Pydantic installed; the
models below raise an informative ``ImportError`` only when actually
used without Pydantic.

Usage (requires ``pip install dusrelpdf[pydantic]``)::

    from dusrelpdf.pydantic import WatermarkOptions, PageNumberOptions

    opts = WatermarkOptions(text="CONFIDENTIAL", opacity=0.2)
    out = dusrelpdf.add_watermark(pdf, options=opts)

    class StampRequest(BaseModel):
        watermark: WatermarkOptions
        page_numbers: PageNumberOptions = PageNumberOptions()

Field names are snake_case and match the Rust core options exactly,
with the same defaults. The shared ``password`` / ``compress`` fields
are flattened into each options model, exactly like the Rust
``#[serde(flatten)] CommonOptions``.
"""

try:
    from pydantic import BaseModel, Field
except ImportError as _exc:  # pragma: no cover - depends on env
    raise ImportError(
        "dusrelpdf Pydantic models need Pydantic v2: "
        "pip install 'dusrelpdf[pydantic]'"
    ) from _exc

from typing import Literal

__all__ = [
    "CommonOptions",
    "MergeOptions",
    "PageSetOptions",
    "WatermarkOptions",
    "PageNumberOptions",
    "RotateOptions",
    "ImagesToPdfOptions",
    "CustomPageSize",
    "SignatureOptions",
    "PdfInfo",
]

# ---------------------------------------------------------------------------
# Shared field types
# ---------------------------------------------------------------------------

Position = Literal[
    "top_left", "top_center", "top_right",
    "bottom_left", "bottom_center", "bottom_right",
    "center",
]
BaseFont = Literal[
    "helvetica", "helvetica_bold", "times_roman", "courier", "courier_bold",
]
ImageFit = Literal["contain", "fill"]
PageSizeName = Literal["auto", "a4", "letter"]


# ---------------------------------------------------------------------------
# Models — defaults mirror crates/dusrelpdf/src/options.rs exactly.
# ---------------------------------------------------------------------------

class CommonOptions(BaseModel):
    """Options shared by every operation that reads a PDF."""

    password: str | None = None
    compress: bool = True


class MergeOptions(BaseModel):
    """Options for ``merge_pdfs``."""

    password: str | None = None
    compress: bool = True
    preserve_metadata: bool = True
    add_bookmarks: bool = True


class PageSetOptions(BaseModel):
    """Options for split / extract / remove / reorder / info."""

    password: str | None = None
    compress: bool = True


class WatermarkOptions(BaseModel):
    """Options for ``add_watermark`` (exactly one of text / image)."""

    password: str | None = None
    compress: bool = True
    text: str | None = "DRAFT"
    image: bytes | None = None
    opacity: float = Field(default=0.3, ge=0.0, le=1.0)
    rotation: float = 45.0
    font_size: float = 48.0
    font: BaseFont = "helvetica"
    position: Position = "center"
    x: float | None = None
    y: float | None = None
    pages: list[int] | None = None
    image_width: float | None = None
    image_height: float | None = None


class PageNumberOptions(BaseModel):
    """Options for ``add_page_numbers``."""

    password: str | None = None
    compress: bool = True
    format: str = "{n}"
    position: Position = "bottom_center"
    start_at: int = 1
    font_size: float = 10.0
    font: BaseFont = "helvetica"
    margin: float = 36.0
    pages: list[int] | None = None


class RotateOptions(BaseModel):
    """Options for ``rotate_pages`` (angle must be a multiple of 90)."""

    password: str | None = None
    compress: bool = True
    angle: int = 90
    pages: list[int] | None = None


class CustomPageSize(BaseModel):
    """Custom page size in points (``page_size`` for images-to-PDF)."""

    width: float
    height: float


class ImagesToPdfOptions(BaseModel):
    """Options for ``images_to_pdf``."""

    page_size: PageSizeName | CustomPageSize = "a4"
    fit: ImageFit = "contain"
    background: tuple[float, float, float] = (1.0, 1.0, 1.0)


class SignatureOptions(BaseModel):
    """Options for ``place_signature``."""

    password: str | None = None
    compress: bool = True
    page: int = 1
    x: float = 36.0
    y: float = 36.0
    width: float | None = None
    height: float | None = None


class PdfInfo(BaseModel):
    """Summary of a PDF document, from ``get_pdf_info``."""

    page_count: int
    version: str
    encrypted: bool
    title: str | None = None
    page_sizes: list[tuple[float, float]] = []
