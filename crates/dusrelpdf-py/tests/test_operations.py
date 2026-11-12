"""End-to-end tests for all 10 dusrelpdf operations.

Fixtures come from the native module itself (the same builders the
Rust tests use), so every language tests the same bytes. Every output
is re-loaded with get_pdf_info and verified — never just "it returned
bytes".
"""

import dusrelpdf
from dusrelpdf import DusrelError


def info(pdf):
    return dusrelpdf.get_pdf_info(pdf)


def test_merge_two_pdfs():
    a = dusrelpdf.make_test_pdf(2, "Doc A")
    b = dusrelpdf.make_test_pdf(3)
    out = dusrelpdf.merge_pdfs([a, b])
    assert isinstance(out, bytes)
    assert info(out)["page_count"] == 5
    assert info(out)["title"] == "Doc A"


def test_merge_single_pdf():
    pdf = dusrelpdf.make_test_pdf(2)
    assert info(dusrelpdf.merge_pdfs([pdf]))["page_count"] == 2


def test_merge_with_options_dataclass():
    a = dusrelpdf.make_test_pdf(1)
    b = dusrelpdf.make_test_pdf(1)
    out = dusrelpdf.merge_pdfs(
        [a, b],
        options=dusrelpdf.MergeOptions(preserve_metadata=False, add_bookmarks=False),
    )
    assert info(out)["page_count"] == 2


def test_split_pdf_lists():
    pdf = dusrelpdf.make_test_pdf(4)
    parts = dusrelpdf.split_pdf(pdf, [[1, 2], [3], [4]])
    assert len(parts) == 3
    assert [info(p)["page_count"] for p in parts] == [2, 1, 1]


def test_split_pdf_ranges_string():
    pdf = dusrelpdf.make_test_pdf(5)
    parts = dusrelpdf.split_pdf_ranges(pdf, "1-2,3,4-5")
    assert len(parts) == 3
    assert [info(p)["page_count"] for p in parts] == [2, 1, 2]


def test_split_each_page():
    pdf = dusrelpdf.make_test_pdf(3)
    parts = dusrelpdf.split_each_page(pdf)
    assert len(parts) == 3
    assert all(info(p)["page_count"] == 1 for p in parts)


def test_extract_pages_keeps_order():
    pdf = dusrelpdf.make_test_pdf(4)
    out = dusrelpdf.extract_pages(pdf, [3, 1])
    assert info(out)["page_count"] == 2
    # Same source page extracted twice is deterministic.
    assert dusrelpdf.extract_pages(pdf, [3]) == dusrelpdf.extract_pages(pdf, [3])
    # Order check via the uncompressed content stream ("Page N" text).
    first = dusrelpdf.extract_pages(out, [1], compress=False)
    second = dusrelpdf.extract_pages(out, [2], compress=False)
    assert b"(Page 3)" in first
    assert b"(Page 1)" in second


def test_extract_pages_range():
    pdf = dusrelpdf.make_test_pdf(5)
    out = dusrelpdf.extract_pages_range(pdf, "1-3,5")
    assert info(out)["page_count"] == 4


def test_remove_pages():
    pdf = dusrelpdf.make_test_pdf(3)
    out = dusrelpdf.remove_pages(pdf, [2])
    assert info(out)["page_count"] == 2
    page1 = dusrelpdf.extract_pages(out, [1], compress=False)
    page2 = dusrelpdf.extract_pages(out, [2], compress=False)
    assert b"(Page 1)" in page1
    assert b"(Page 3)" in page2


def test_remove_no_pages_returns_input_unchanged():
    pdf = dusrelpdf.make_test_pdf(2)
    assert dusrelpdf.remove_pages(pdf, []) == pdf


def test_reorder_pages():
    pdf = dusrelpdf.make_test_pdf(3)
    out = dusrelpdf.reorder_pages(pdf, [3, 1, 2])
    assert info(out)["page_count"] == 3
    texts = [
        dusrelpdf.extract_pages(out, [n], compress=False) for n in (1, 2, 3)
    ]
    assert b"(Page 3)" in texts[0]
    assert b"(Page 1)" in texts[1]
    assert b"(Page 2)" in texts[2]


def test_rotate_pages():
    pdf = dusrelpdf.make_test_pdf(2)
    out = dusrelpdf.rotate_pages(pdf, angle=90)
    assert info(out)["page_count"] == 2
    # Round-trip: rotating back restores the document shape.
    back = dusrelpdf.rotate_pages(out, angle=270)
    assert info(back)["page_count"] == 2


def test_rotate_negative_angle_normalised():
    pdf = dusrelpdf.make_test_pdf(1)
    out = dusrelpdf.rotate_pages(pdf, angle=-90)
    assert info(out)["page_count"] == 1
    # -90 == 270, so one more 90 brings it back to 0.
    assert info(dusrelpdf.rotate_pages(out, angle=90))["page_count"] == 1


def test_rotate_selected_pages_only():
    pdf = dusrelpdf.make_test_pdf(3)
    out = dusrelpdf.rotate_pages(pdf, angle=180, pages=[2])
    assert info(out)["page_count"] == 3


def test_images_to_pdf_jpeg_and_png():
    jpeg = dusrelpdf.make_photo_jpeg()
    png = dusrelpdf.make_signature_png()
    out = dusrelpdf.images_to_pdf([jpeg, png])
    assert info(out)["page_count"] == 2
    assert info(out)["page_sizes"] == [(595.0, 842.0), (595.0, 842.0)]


def test_images_to_pdf_page_size_auto():
    jpeg = dusrelpdf.make_photo_jpeg()  # 120 x 90 px
    out = dusrelpdf.images_to_pdf([jpeg], page_size="auto")
    assert info(out)["page_sizes"] == [(120.0, 90.0)]


def test_images_to_pdf_letter_and_custom():
    jpeg = dusrelpdf.make_photo_jpeg()
    assert info(dusrelpdf.images_to_pdf([jpeg], page_size="letter"))["page_sizes"] == [
        (612.0, 792.0)
    ]
    out = dusrelpdf.images_to_pdf([jpeg], page_size={"width": 200.0, "height": 300.0})
    assert info(out)["page_sizes"] == [(200.0, 300.0)]
    out = dusrelpdf.images_to_pdf([jpeg], page_size=[200.0, 300.0])
    assert info(out)["page_sizes"] == [(200.0, 300.0)]


def test_images_to_pdf_fill_fit():
    jpeg = dusrelpdf.make_photo_jpeg()
    out = dusrelpdf.images_to_pdf([jpeg], fit="fill")
    assert info(out)["page_count"] == 1


def test_add_watermark_text():
    pdf = dusrelpdf.make_test_pdf(2)
    out = dusrelpdf.add_watermark(pdf, text="DRAFT", opacity=0.3, rotation=45.0)
    assert info(out)["page_count"] == 2


def test_add_watermark_defaults():
    pdf = dusrelpdf.make_test_pdf(1)
    out = dusrelpdf.add_watermark(pdf)
    assert info(out)["page_count"] == 1


def test_add_watermark_image():
    pdf = dusrelpdf.make_test_pdf(1)
    sig = dusrelpdf.make_signature_png()
    out = dusrelpdf.add_watermark(pdf, image=sig, opacity=0.5, rotation=0.0)
    assert info(out)["page_count"] == 1


def test_add_watermark_selected_pages():
    pdf = dusrelpdf.make_test_pdf(3)
    out = dusrelpdf.add_watermark(pdf, text="X", pages=[1, 3])
    assert info(out)["page_count"] == 3


def test_add_page_numbers_default():
    pdf = dusrelpdf.make_test_pdf(3)
    out = dusrelpdf.add_page_numbers(pdf)
    assert info(out)["page_count"] == 3


def test_add_page_numbers_format():
    pdf = dusrelpdf.make_test_pdf(3)
    out = dusrelpdf.add_page_numbers(pdf, format="Page {n} of {total}")
    assert info(out)["page_count"] == 3
    out = dusrelpdf.add_page_numbers(pdf, format="{n} / {total}", start_at=5)
    assert info(out)["page_count"] == 3


def test_add_page_numbers_position_and_font():
    pdf = dusrelpdf.make_test_pdf(1)
    out = dusrelpdf.add_page_numbers(
        pdf, position="top_right", font="courier_bold", font_size=12.0, margin=24.0
    )
    assert info(out)["page_count"] == 1


def test_place_signature():
    pdf = dusrelpdf.make_test_pdf(1)
    sig = dusrelpdf.make_signature_png()
    out = dusrelpdf.place_signature(pdf, sig)
    result = info(out)
    assert result["page_count"] == 1
    # Output is a valid 1-page PDF (deep SMask check is covered by the
    # Rust tests; here we assert the document re-loads cleanly).
    assert result["version"] == "1.7"


def test_place_signature_options():
    pdf = dusrelpdf.make_test_pdf(2)
    sig = dusrelpdf.make_signature_png()
    out = dusrelpdf.place_signature(pdf, sig, page=2, x=100.0, y=100.0, width=72.0)
    assert info(out)["page_count"] == 2


def test_get_pdf_info():
    pdf = dusrelpdf.make_test_pdf(3, "Hello")
    result = info(pdf)
    assert result["page_count"] == 3
    assert result["version"] == "1.7"
    assert result["encrypted"] is False
    assert result["title"] == "Hello"
    assert result["page_sizes"] == [(595.0, 842.0)] * 3


def test_bulk_extract_partial_failure():
    pdf = dusrelpdf.make_test_pdf(3)
    items = dusrelpdf.bulk_extract(pdf, [[1], [99], [2, 3]])
    assert len(items) == 3
    assert items[0]["index"] == 0
    assert info(items[0]["pdf"])["page_count"] == 1
    assert items[0]["error_code"] is None
    assert items[1]["pdf"] is None
    assert items[1]["error_code"] == "PAGE_OUT_OF_RANGE"
    assert items[1]["error"] != ""
    assert info(items[2]["pdf"])["page_count"] == 2


def test_fixtures():
    pdf = dusrelpdf.make_test_pdf(1)
    assert pdf[:5] == b"%PDF-"
    png = dusrelpdf.make_signature_png()
    assert png[:8] == b"\x89PNG\r\n\x1a\n"
    jpeg = dusrelpdf.make_photo_jpeg()
    assert jpeg[:2] == b"\xff\xd8"


def test_make_test_pdf_title_none():
    pdf = dusrelpdf.make_test_pdf(2)
    # No title set: the key is absent (serde skip_serializing_if).
    assert info(pdf).get("title") is None
