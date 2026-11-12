"""Options tests: kwargs, options dicts, dataclasses and Pydantic
models — kwargs win over the dict, and both ride the Rust defaults
for anything omitted.
"""

import pytest

import dusrelpdf
from dusrelpdf import DusrelError


@pytest.fixture()
def pdf3():
    return dusrelpdf.make_test_pdf(3)


def test_kwargs_win_over_options_dict(pdf3):
    # options dict says opacity 2.0 (invalid), kwarg says 0.5 (valid):
    # the kwarg wins, so this must succeed.
    out = dusrelpdf.add_watermark(pdf3, opacity=0.5, options={"opacity": 2.0})
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
    # And without the kwarg, the dict value is used — and fails.
    with pytest.raises(DusrelError) as excinfo:
        dusrelpdf.add_watermark(pdf3, options={"opacity": 2.0})
    assert excinfo.value.code == "INVALID_ARGUMENT"


def test_options_dict_only(pdf3):
    out = dusrelpdf.add_watermark(pdf3, options={"text": "SECRET", "rotation": 0.0})
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
    out = dusrelpdf.rotate_pages(pdf3, options={"angle": 180})
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3


def test_options_dataclass(pdf3):
    out = dusrelpdf.add_watermark(
        pdf3, options=dusrelpdf.WatermarkOptions(text="X", opacity=0.2))
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
    out = dusrelpdf.add_page_numbers(
        pdf3, options=dusrelpdf.PageNumberOptions(format="Page {n} of {total}"))
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
    out = dusrelpdf.rotate_pages(pdf3, options=dusrelpdf.RotateOptions(angle=270))
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
    out = dusrelpdf.merge_pdfs(
        [pdf3, pdf3], options=dusrelpdf.MergeOptions(add_bookmarks=False))
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 6


def test_watermark_opacity_bounds(pdf3):
    for opacity in (0.0, 0.1, 0.5, 1.0):
        out = dusrelpdf.add_watermark(pdf3, opacity=opacity)
        assert dusrelpdf.get_pdf_info(out)["page_count"] == 3


def test_watermark_rotation_and_font(pdf3):
    for rotation in (0.0, 45.0, 90.0, 180.0, -45.0):
        out = dusrelpdf.add_watermark(pdf3, rotation=rotation)
        assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
    out = dusrelpdf.add_watermark(pdf3, font="times_roman", font_size=24.0)
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3


def test_page_number_format_variants(pdf3):
    for fmt in ("{n}", "Page {n} of {total}", "{n} / {total}", "p.{n}"):
        out = dusrelpdf.add_page_numbers(pdf3, format=fmt)
        assert dusrelpdf.get_pdf_info(out)["page_count"] == 3


def test_page_number_start_at(pdf3):
    out = dusrelpdf.add_page_numbers(pdf3, start_at=0)
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
    out = dusrelpdf.add_page_numbers(pdf3, start_at=10)
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3


def test_rotate_minus_90_normalisation(pdf3):
    # -90 normalises to 270: four -90 rotations = full circle.
    out = pdf3
    for _ in range(4):
        out = dusrelpdf.rotate_pages(out, angle=-90)
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3


def test_images_page_size_auto(pdf3):
    jpeg = dusrelpdf.make_photo_jpeg()  # 120 x 90 px
    out = dusrelpdf.images_to_pdf([jpeg], page_size="auto")
    assert dusrelpdf.get_pdf_info(out)["page_sizes"] == [(120.0, 90.0)]
    # Default is A4.
    out = dusrelpdf.images_to_pdf([jpeg])
    assert dusrelpdf.get_pdf_info(out)["page_sizes"] == [(595.0, 842.0)]
    # Dataclass options (background as tuple).
    out = dusrelpdf.images_to_pdf(
        [jpeg], options=dusrelpdf.ImagesToPdfOptions(page_size="a4"))
    assert dusrelpdf.get_pdf_info(out)["page_sizes"] == [(595.0, 842.0)]


def test_compress_option(pdf3):
    out = dusrelpdf.merge_pdfs([pdf3], compress=False)
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
    out = dusrelpdf.extract_pages(pdf3, [1], options={"compress": False})
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 1


def test_default_options_shape():
    opts = dusrelpdf.default_options()
    assert opts == {"password": None, "compress": True}


def test_signature_dataclass_options(pdf3):
    out = dusrelpdf.place_signature(
        pdf3, dusrelpdf.make_signature_png(),
        options=dusrelpdf.SignatureOptions(page=2, x=50.0, y=50.0, width=100.0))
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3


def test_watermark_position_xy(pdf3):
    out = dusrelpdf.add_watermark(pdf3, text="X", x=100.0, y=200.0)
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
    out = dusrelpdf.add_watermark(pdf3, text="X", position="top_right")
    assert dusrelpdf.get_pdf_info(out)["page_count"] == 3
