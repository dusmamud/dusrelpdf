"""Error-code tests: every failure is a DusrelError with the right
code and English + Hindi reasons — including wrong input types, which
must raise INVALID_TYPE and never a bare TypeError (indycheck v0.2.1
lesson).
"""

import pytest

import dusrelpdf
from dusrelpdf import DusrelError


def check_code(call, code):
    with pytest.raises(DusrelError) as excinfo:
        call()
    err = excinfo.value
    assert err.code == code, f"expected {code}, got {err.code}: {err.reason_en}"
    assert err.reason_en, "reason_en must not be empty"
    assert err.reason_hi, "reason_hi must not be empty"
    return err


@pytest.fixture()
def pdf3():
    return dusrelpdf.make_test_pdf(3)


@pytest.fixture()
def sig():
    return dusrelpdf.make_signature_png()


@pytest.fixture()
def photo():
    return dusrelpdf.make_photo_jpeg()


# --- INVALID_PDF -----------------------------------------------------------

def test_invalid_pdf_garbage(pdf3):
    check_code(lambda: dusrelpdf.merge_pdfs([b"not a pdf at all"]),
               "INVALID_PDF")
    check_code(lambda: dusrelpdf.extract_pages(b"not a pdf at all", [1]),
               "INVALID_PDF")
    check_code(lambda: dusrelpdf.get_pdf_info(b""),
               "EMPTY_INPUT")
    check_code(lambda: dusrelpdf.get_pdf_info(b"garbage"),
               "INVALID_PDF")


def test_invalid_pdf_truncated(pdf3):
    check_code(lambda: dusrelpdf.get_pdf_info(pdf3[:20]),
               "INVALID_PDF")


# --- EMPTY_INPUT -----------------------------------------------------------

def test_empty_input():
    check_code(lambda: dusrelpdf.merge_pdfs([]), "EMPTY_INPUT")
    check_code(lambda: dusrelpdf.images_to_pdf([]), "EMPTY_INPUT")
    check_code(lambda: dusrelpdf.split_pdf(dusrelpdf.make_test_pdf(1), []),
               "EMPTY_INPUT")
    check_code(lambda: dusrelpdf.extract_pages(dusrelpdf.make_test_pdf(1), []),
               "EMPTY_INPUT")
    check_code(lambda: dusrelpdf.get_pdf_info(b""), "EMPTY_INPUT")


# --- PAGE_OUT_OF_RANGE -----------------------------------------------------

def test_page_out_of_range(pdf3):
    check_code(lambda: dusrelpdf.extract_pages(pdf3, [99]), "PAGE_OUT_OF_RANGE")
    check_code(lambda: dusrelpdf.extract_pages(pdf3, [0]), "PAGE_OUT_OF_RANGE")
    check_code(lambda: dusrelpdf.remove_pages(pdf3, [4]), "PAGE_OUT_OF_RANGE")
    check_code(lambda: dusrelpdf.reorder_pages(pdf3, [1, 2, 4]), "PAGE_OUT_OF_RANGE")
    check_code(lambda: dusrelpdf.rotate_pages(pdf3, angle=90, pages=[99]),
               "PAGE_OUT_OF_RANGE")

    def _sig_page():
        return dusrelpdf.place_signature(pdf3, dusrelpdf.make_signature_png(), page=99)

    check_code(_sig_page, "PAGE_OUT_OF_RANGE")
    check_code(lambda: dusrelpdf.add_watermark(pdf3, pages=[0]), "PAGE_OUT_OF_RANGE")


# --- INVALID_RANGE ---------------------------------------------------------

def test_invalid_range(pdf3):
    check_code(lambda: dusrelpdf.extract_pages_range(pdf3, "5-2"), "INVALID_RANGE")
    check_code(lambda: dusrelpdf.extract_pages_range(pdf3, "0"), "INVALID_RANGE")
    check_code(lambda: dusrelpdf.extract_pages_range(pdf3, "abc"), "INVALID_RANGE")
    check_code(lambda: dusrelpdf.extract_pages_range(pdf3, ""), "INVALID_RANGE")
    check_code(lambda: dusrelpdf.extract_pages_range(pdf3, "1,,2"), "INVALID_RANGE")
    check_code(lambda: dusrelpdf.split_pdf_ranges(pdf3, ""), "INVALID_RANGE")
    check_code(lambda: dusrelpdf.split_pdf(pdf3, [[]]), "INVALID_RANGE")


# --- INVALID_ANGLE ---------------------------------------------------------

def test_invalid_angle(pdf3):
    check_code(lambda: dusrelpdf.rotate_pages(pdf3, angle=45), "INVALID_ANGLE")
    check_code(lambda: dusrelpdf.rotate_pages(pdf3, angle=30), "INVALID_ANGLE")
    # 0 and multiples of 90 are fine.
    assert dusrelpdf.get_pdf_info(
        dusrelpdf.rotate_pages(pdf3, angle=0))["page_count"] == 3
    assert dusrelpdf.get_pdf_info(
        dusrelpdf.rotate_pages(pdf3, angle=360))["page_count"] == 3


# --- INVALID_IMAGE ---------------------------------------------------------

def test_invalid_image(pdf3, sig):
    check_code(lambda: dusrelpdf.images_to_pdf([b"not an image"]),
               "INVALID_IMAGE")
    check_code(lambda: dusrelpdf.images_to_pdf([b"", sig]),
               "INVALID_IMAGE")

    def _bad_sig():
        return dusrelpdf.place_signature(pdf3, b"garbage bytes")

    check_code(_bad_sig, "INVALID_IMAGE")

    def _bad_wm():
        return dusrelpdf.add_watermark(pdf3, image=b"garbage bytes")

    check_code(_bad_wm, "INVALID_IMAGE")


# --- INVALID_ARGUMENT ------------------------------------------------------

def test_invalid_argument(pdf3, sig):
    check_code(lambda: dusrelpdf.add_watermark(pdf3, opacity=2.0),
               "INVALID_ARGUMENT")
    check_code(lambda: dusrelpdf.add_watermark(pdf3, opacity=-0.1),
               "INVALID_ARGUMENT")
    check_code(lambda: dusrelpdf.add_watermark(pdf3, text=None, image=None),
               "INVALID_ARGUMENT")
    check_code(lambda: dusrelpdf.add_watermark(pdf3, text="X", image=sig),
               "INVALID_ARGUMENT")
    check_code(lambda: dusrelpdf.add_page_numbers(pdf3, format="hello"),
               "INVALID_ARGUMENT")
    check_code(lambda: dusrelpdf.reorder_pages(pdf3, [1, 1, 2]),
               "INVALID_ARGUMENT")
    check_code(lambda: dusrelpdf.reorder_pages(pdf3, [1, 2]),
               "INVALID_ARGUMENT")
    check_code(lambda: dusrelpdf.images_to_pdf(
        [dusrelpdf.make_photo_jpeg()], page_size="a3"), "INVALID_ARGUMENT")
    check_code(lambda: dusrelpdf.make_test_pdf(0), "INVALID_ARGUMENT")


# --- INVALID_TYPE ----------------------------------------------------------
# Every function, given a string / None / int where bytes are expected,
# raises DusrelError(INVALID_TYPE) — never TypeError.

def test_invalid_type_pdf_argument(pdf3, sig):
    cases = [
        lambda: dusrelpdf.merge_pdfs("not bytes"),
        lambda: dusrelpdf.merge_pdfs([None]),
        lambda: dusrelpdf.merge_pdfs([123]),
        lambda: dusrelpdf.split_pdf("not bytes", [[1]]),
        lambda: dusrelpdf.split_pdf(None, [[1]]),
        lambda: dusrelpdf.split_pdf_ranges(123, "1-2"),
        lambda: dusrelpdf.split_each_page("not bytes"),
        lambda: dusrelpdf.extract_pages("not bytes", [1]),
        lambda: dusrelpdf.extract_pages_range("not bytes", "1"),
        lambda: dusrelpdf.remove_pages("not bytes", [1]),
        lambda: dusrelpdf.reorder_pages("not bytes", [1]),
        lambda: dusrelpdf.rotate_pages("not bytes"),
        lambda: dusrelpdf.images_to_pdf("not bytes"),
        lambda: dusrelpdf.images_to_pdf([123]),
        lambda: dusrelpdf.add_watermark("not bytes"),
        lambda: dusrelpdf.add_page_numbers("not bytes"),
        lambda: dusrelpdf.place_signature("not bytes", sig),
        lambda: dusrelpdf.place_signature(pdf3, "not bytes"),
        lambda: dusrelpdf.get_pdf_info("not bytes"),
        lambda: dusrelpdf.get_pdf_info(None),
        lambda: dusrelpdf.bulk_extract("not bytes", [[1]]),
        lambda: dusrelpdf.make_test_pdf("3"),
        lambda: dusrelpdf.make_test_pdf(None),
        lambda: dusrelpdf.make_test_pdf(1, title=123),
    ]
    for call in cases:
        with pytest.raises(DusrelError) as excinfo:
            call()
        assert excinfo.value.code == "INVALID_TYPE", (
            f"{call} raised {excinfo.value.code}, expected INVALID_TYPE")


def test_invalid_type_other_arguments(pdf3):
    check_code(lambda: dusrelpdf.extract_pages(pdf3, "1"), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.extract_pages(pdf3, [1.5]), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.extract_pages(pdf3, [True]), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.split_pdf(pdf3, "1-2"), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.split_pdf(pdf3, [[1, "2"]]), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.extract_pages_range(pdf3, 3), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.split_pdf_ranges(pdf3, 3), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.reorder_pages(pdf3, 3), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.bulk_extract(pdf3, [[1], "2"]), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.add_watermark(pdf3, opacity="high"),
               "INVALID_TYPE")
    check_code(lambda: dusrelpdf.add_watermark(pdf3, options="nope"),
               "INVALID_TYPE")
    check_code(lambda: dusrelpdf.rotate_pages(pdf3, angle="90"),
               "INVALID_TYPE")
    check_code(lambda: dusrelpdf.images_to_pdf(
        [dusrelpdf.make_photo_jpeg()], page_size=42), "INVALID_TYPE")
    check_code(lambda: dusrelpdf.place_signature(pdf3, dusrelpdf.make_signature_png(),
                                                 page="1"), "INVALID_TYPE")


def test_dusrel_error_is_exception():
    err = DusrelError("INVALID_PDF", "Bad PDF", "Galat PDF")
    assert isinstance(err, Exception)
    assert err.code == "INVALID_PDF"
    assert "INVALID_PDF" in str(err)
