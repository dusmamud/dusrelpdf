"""Pydantic v2 model tests.

The models are pure Python (no native import) and mirror the Rust
options exactly, with the same defaults. ``pydantic`` is an optional
extra — these tests are skipped when it is not installed.
"""

import pytest

pydantic = pytest.importorskip("pydantic")
from pydantic import ValidationError  # noqa: E402

import dusrelpdf  # noqa: E402
from dusrelpdf import pydantic as dpy  # noqa: E402


def test_defaults_match_rust():
    assert dpy.WatermarkOptions().model_dump() == {
        "password": None,
        "compress": True,
        "text": "DRAFT",
        "image": None,
        "opacity": 0.3,
        "rotation": 45.0,
        "font_size": 48.0,
        "font": "helvetica",
        "position": "center",
        "x": None,
        "y": None,
        "pages": None,
        "image_width": None,
        "image_height": None,
    }
    assert dpy.PageNumberOptions().format == "{n}"
    assert dpy.PageNumberOptions().position == "bottom_center"
    assert dpy.PageNumberOptions().start_at == 1
    assert dpy.RotateOptions().angle == 90
    assert dpy.MergeOptions().preserve_metadata is True
    assert dpy.MergeOptions().add_bookmarks is True
    assert dpy.ImagesToPdfOptions().page_size == "a4"
    assert dpy.ImagesToPdfOptions().fit == "contain"
    assert dpy.ImagesToPdfOptions().background == (1.0, 1.0, 1.0)
    assert dpy.SignatureOptions().page == 1
    assert dpy.SignatureOptions().x == 36.0
    assert dpy.SignatureOptions().y == 36.0


def test_bad_opacity_rejected():
    with pytest.raises(ValidationError):
        dpy.WatermarkOptions(opacity=1.5)
    with pytest.raises(ValidationError):
        dpy.WatermarkOptions(opacity=-0.1)
    assert dpy.WatermarkOptions(opacity=1.0).opacity == 1.0
    assert dpy.WatermarkOptions(opacity=0.0).opacity == 0.0


def test_bad_enum_rejected():
    with pytest.raises(ValidationError):
        dpy.WatermarkOptions(position="middle")
    with pytest.raises(ValidationError):
        dpy.ImagesToPdfOptions(fit="stretch")
    with pytest.raises(ValidationError):
        dpy.WatermarkOptions(font="comic_sans")


def test_custom_page_size():
    opts = dpy.ImagesToPdfOptions(page_size=dpy.CustomPageSize(width=200.0, height=300.0))
    assert opts.page_size.width == 200.0
    # And it works end-to-end (the native side accepts the flat form).
    jpeg = dusrelpdf.make_photo_jpeg()
    out = dusrelpdf.images_to_pdf([jpeg], options=opts)
    assert dusrelpdf.get_pdf_info(out)["page_sizes"] == [(200.0, 300.0)]


def test_pydantic_model_as_options():
    pdf = dusrelpdf.make_test_pdf(2)
    out = dusrelpdf.add_watermark(
        pdf, options=dpy.WatermarkOptions(text="SECRET", opacity=0.2))
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 2
    out = dusrelpdf.add_page_numbers(
        pdf, options=dpy.PageNumberOptions(format="Page {n} of {total}"))
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 2
    out = dusrelpdf.rotate_pages(pdf, options=dpy.RotateOptions(angle=180))
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 2


def test_pdf_info_model():
    pdf = dusrelpdf.make_test_pdf(3, "Hello")
    raw = dusrelpdf.get_pdf_info(pdf)
    model = dpy.PdfInfo(**raw)
    assert model.page_count == 3
    assert model.version == "1.7"
    assert model.encrypted is False
    assert model.title == "Hello"
    assert model.page_sizes == [(595.0, 842.0)] * 3


def test_models_do_not_need_native():
    # pydantic.py imports no native code: the models build fine even if
    # the extension were absent — here we just assert the module has no
    # _native attribute and constructs models standalone.
    assert not hasattr(dpy, "_native")
    assert dpy.CommonOptions().compress is True
    assert dpy.MergeOptions(password="secret").password == "secret"
