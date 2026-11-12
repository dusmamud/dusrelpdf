//! End-to-end tests for all 10 operations, on fixture PDFs built
//! with lopdf itself. Every output is re-loaded and verified —
//! never just "it returned bytes".

use dusrelpdf::fixtures::{
    make_inherited_mediabox_pdf, make_malformed_pdf, make_photo_jpeg, make_signature_png,
    make_test_pdf,
};
use dusrelpdf::{
    CommonOptions, ErrorCode, ImagesToPdfOptions, PageNumberOptions, PageSetOptions, Position,
    RotateOptions, SignatureOptions, WatermarkOptions, add_page_numbers, add_watermark,
    bulk_extract, extract_pages, extract_pages_range, get_pdf_info, images_to_pdf,
    merge_pdfs_default, place_signature, remove_pages, reorder_pages, rotate_pages,
    split_each_page, split_pdf_ranges,
};
use lopdf::{Document, Object};

fn info(bytes: &[u8]) -> dusrelpdf::PdfInfo {
    get_pdf_info(bytes, &CommonOptions::default()).expect("info loads")
}

fn text_of(bytes: &[u8], page: u32) -> String {
    let doc = Document::load_mem(bytes).expect("output loads");
    doc.extract_text(&[page]).unwrap_or_default()
}

fn has_smask(bytes: &[u8]) -> bool {
    let doc = Document::load_mem(bytes).expect("output loads");
    doc.objects.values().any(|obj| {
        // Image XObjects are Stream objects, not plain dictionaries.
        obj.as_stream()
            .ok()
            .map(|s| {
                s.dict
                    .get(b"Subtype")
                    .map(|v| v == &Object::Name(b"Image".to_vec()))
                    .unwrap_or(false)
                    && s.dict.get(b"SMask").is_ok()
            })
            .unwrap_or(false)
    })
}

#[test]
fn merge_two_pdfs() {
    let a = make_test_pdf(2, Some("Doc A"));
    let b = make_test_pdf(3, None);
    let out = merge_pdfs_default(&[a, b]).expect("merge works");
    assert_eq!(info(&out).page_count, 5);
    assert_eq!(info(&out).title.as_deref(), Some("Doc A"));
    assert!(text_of(&out, 1).contains("Page 1"));
    assert!(text_of(&out, 3).contains("Page 1")); // first page of doc B
    assert!(text_of(&out, 5).contains("Page 3"));
    // Outlines are rebuilt flat (first page of each input) — original
    // outline trees are NOT preserved.
    let doc = Document::load_mem(&out).unwrap();
    assert!(doc.trailer.get(b"Root").is_ok());
}

#[test]
fn merge_single_input_is_a_copy() {
    let a = make_test_pdf(2, None);
    let out = merge_pdfs_default(&[a]).expect("merge works");
    assert_eq!(info(&out).page_count, 2);
}

#[test]
fn merge_rejects_empty_input() {
    let err = merge_pdfs_default(&[]).unwrap_err();
    assert_eq!(err.code, ErrorCode::EmptyInput);
    assert!(!err.reason_hi.is_empty());
}

#[test]
fn merge_inherited_mediabox() {
    let a = make_inherited_mediabox_pdf(2);
    let b = make_test_pdf(1, None);
    let out = merge_pdfs_default(&[a, b]).expect("merge works");
    assert_eq!(info(&out).page_count, 3);
    // Pages that inherited their MediaBox still report A4 size.
    assert_eq!(info(&out).page_sizes[0], (595.0, 842.0));
    assert!(text_of(&out, 1).contains("Page 1"));
}

#[test]
fn split_by_ranges() {
    let pdf = make_test_pdf(5, None);
    let outs =
        split_pdf_ranges(&pdf, "1-2,3,4-5", &PageSetOptions::default()).expect("split works");
    assert_eq!(outs.len(), 3);
    assert_eq!(info(&outs[0]).page_count, 2);
    assert_eq!(info(&outs[1]).page_count, 1);
    assert_eq!(info(&outs[2]).page_count, 2);
    assert!(text_of(&outs[1], 1).contains("Page 3"));
    assert!(text_of(&outs[2], 2).contains("Page 5"));
}

#[test]
fn test_split_each_page() {
    let pdf = make_test_pdf(3, None);
    let outs = split_each_page(&pdf, &PageSetOptions::default()).expect("split works");
    assert_eq!(outs.len(), 3);
    for (i, out) in outs.iter().enumerate() {
        assert_eq!(info(out).page_count, 1);
        assert!(text_of(out, 1).contains(&format!("Page {}", i + 1)));
    }
}

#[test]
fn extract_selected_pages_in_given_order() {
    let pdf = make_test_pdf(5, None);
    let out = extract_pages(&pdf, &[5, 1, 3], &PageSetOptions::default()).expect("extract works");
    assert_eq!(info(&out).page_count, 3);
    assert!(text_of(&out, 1).contains("Page 5"));
    assert!(text_of(&out, 2).contains("Page 1"));
    assert!(text_of(&out, 3).contains("Page 3"));
}

#[test]
fn extract_from_range_string() {
    let pdf = make_test_pdf(5, None);
    let out = extract_pages_range(&pdf, "2-3,5", &PageSetOptions::default()).expect("extract");
    assert_eq!(info(&out).page_count, 3);
    assert!(text_of(&out, 3).contains("Page 5"));
}

#[test]
fn extract_duplicate_page_copies_it() {
    let pdf = make_test_pdf(3, None);
    let out = extract_pages(&pdf, &[2, 2], &PageSetOptions::default()).expect("extract works");
    assert_eq!(info(&out).page_count, 2);
    assert!(text_of(&out, 1).contains("Page 2"));
    assert!(text_of(&out, 2).contains("Page 2"));
}

#[test]
fn extract_keeps_info_title() {
    let pdf = make_test_pdf(3, Some("Kept Title"));
    let out = extract_pages(&pdf, &[1, 2], &PageSetOptions::default()).expect("extract");
    assert_eq!(info(&out).title.as_deref(), Some("Kept Title"));
}

#[test]
fn test_remove_pages() {
    let pdf = make_test_pdf(5, None);
    let out = remove_pages(&pdf, &[2, 4], &PageSetOptions::default()).expect("remove works");
    assert_eq!(info(&out).page_count, 3);
    assert!(text_of(&out, 1).contains("Page 1"));
    assert!(text_of(&out, 2).contains("Page 3"));
    assert!(text_of(&out, 3).contains("Page 5"));
}

#[test]
fn remove_all_pages_is_an_error_free_empty_doc() {
    // Removing every page is allowed by lopdf; the result is a
    // 0-page document. We only assert it returns bytes that load.
    let pdf = make_test_pdf(1, None);
    let out = remove_pages(&pdf, &[1], &PageSetOptions::default()).expect("remove works");
    let doc = Document::load_mem(&out).expect("output loads");
    assert_eq!(doc.get_pages().len(), 0);
}

#[test]
fn test_reorder_pages() {
    let pdf = make_test_pdf(3, None);
    let out = reorder_pages(&pdf, &[3, 1, 2], &PageSetOptions::default()).expect("reorder works");
    assert_eq!(info(&out).page_count, 3);
    assert!(text_of(&out, 1).contains("Page 3"));
    assert!(text_of(&out, 2).contains("Page 1"));
    assert!(text_of(&out, 3).contains("Page 2"));
}

#[test]
fn reorder_rejects_bad_permutation() {
    let pdf = make_test_pdf(3, None);
    let err = reorder_pages(&pdf, &[1, 1, 2], &PageSetOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidArgument);
    let err = reorder_pages(&pdf, &[1, 2], &PageSetOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidArgument);
    let err = reorder_pages(&pdf, &[1, 2, 9], &PageSetOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::PageOutOfRange);
}

#[test]
fn rotate_all_pages() {
    let pdf = make_test_pdf(2, None);
    let out = rotate_pages(
        &pdf,
        &RotateOptions {
            angle: 90,
            ..Default::default()
        },
    )
    .expect("rotate works");
    let doc = Document::load_mem(&out).unwrap();
    for (_, id) in doc.get_pages() {
        let dict = doc.get_object(id).unwrap().as_dict().unwrap();
        assert_eq!(dict.get(b"Rotate").unwrap().as_i64().unwrap(), 90);
    }
}

#[test]
fn rotate_negative_angle_normalises() {
    let pdf = make_test_pdf(1, None);
    let out = rotate_pages(
        &pdf,
        &RotateOptions {
            angle: -90,
            ..Default::default()
        },
    )
    .expect("rotate works");
    let doc = Document::load_mem(&out).unwrap();
    let (_, id) = doc.get_pages().into_iter().next().unwrap();
    let dict = doc.get_object(id).unwrap().as_dict().unwrap();
    assert_eq!(dict.get(b"Rotate").unwrap().as_i64().unwrap(), 270);
}

#[test]
fn rotate_accumulates() {
    let pdf = make_test_pdf(1, None);
    let once = rotate_pages(
        &pdf,
        &RotateOptions {
            angle: 90,
            ..Default::default()
        },
    )
    .unwrap();
    let twice = rotate_pages(
        &once,
        &RotateOptions {
            angle: 90,
            ..Default::default()
        },
    )
    .unwrap();
    let doc = Document::load_mem(&twice).unwrap();
    let (_, id) = doc.get_pages().into_iter().next().unwrap();
    let dict = doc.get_object(id).unwrap().as_dict().unwrap();
    assert_eq!(dict.get(b"Rotate").unwrap().as_i64().unwrap(), 180);
}

#[test]
fn rotate_rejects_non_multiple_of_90() {
    let pdf = make_test_pdf(1, None);
    let err = rotate_pages(
        &pdf,
        &RotateOptions {
            angle: 45,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidAngle);
}

#[test]
fn rotate_selected_pages_only() {
    let pdf = make_test_pdf(3, None);
    let out = rotate_pages(
        &pdf,
        &RotateOptions {
            angle: 180,
            pages: Some(vec![2]),
            ..Default::default()
        },
    )
    .expect("rotate works");
    let doc = Document::load_mem(&out).unwrap();
    let pages = doc.get_pages();
    for (n, id) in &pages {
        let dict = doc.get_object(*id).unwrap().as_dict().unwrap();
        let rot = dict.get(b"Rotate").and_then(|o| o.as_i64()).unwrap_or(0);
        if *n == 2 {
            assert_eq!(rot, 180);
        } else {
            assert_eq!(rot, 0);
        }
    }
}

#[test]
fn images_to_pdf_jpeg_and_png() {
    let jpeg = make_photo_jpeg();
    let png = make_signature_png();
    let out = images_to_pdf(&[jpeg, png], &ImagesToPdfOptions::default()).expect("images work");
    assert_eq!(info(&out).page_count, 2);
    assert_eq!(info(&out).page_sizes[0], (595.0, 842.0)); // A4 default
    // The transparent PNG page must carry an SMask image.
    let split = split_each_page(&out, &PageSetOptions::default()).unwrap();
    assert!(
        has_smask(&split[1]),
        "transparent PNG keeps its alpha via /SMask"
    );
}

#[test]
fn images_to_pdf_auto_size() {
    let jpeg = make_photo_jpeg();
    let out = images_to_pdf(
        &[jpeg],
        &ImagesToPdfOptions {
            page_size: dusrelpdf::PageSize::Auto,
            ..Default::default()
        },
    )
    .expect("images work");
    assert_eq!(info(&out).page_sizes[0], (120.0, 90.0));
}

#[test]
fn images_to_pdf_rejects_empty_and_bad_bytes() {
    let err = images_to_pdf(&[], &ImagesToPdfOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::EmptyInput);
    let err =
        images_to_pdf(&[b"not an image".to_vec()], &ImagesToPdfOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidImage);
}

#[test]
fn watermark_text_paints_over() {
    let pdf = make_test_pdf(2, None);
    let out = add_watermark(
        &pdf,
        &WatermarkOptions {
            text: Some("CONFIDENTIAL".to_string()),
            ..Default::default()
        },
    )
    .expect("watermark works");
    assert_eq!(info(&out).page_count, 2);
    assert!(text_of(&out, 1).contains("CONFIDENTIAL"));
    assert!(text_of(&out, 2).contains("CONFIDENTIAL"));
    assert!(text_of(&out, 1).contains("Page 1")); // original content intact
}

#[test]
fn watermark_image_and_options() {
    let pdf = make_test_pdf(1, None);
    let sig = make_signature_png();
    let out = add_watermark(
        &pdf,
        &WatermarkOptions {
            text: None,
            image: Some(sig),
            opacity: 0.5,
            rotation: 0.0,
            ..Default::default()
        },
    )
    .expect("image watermark works");
    assert_eq!(info(&out).page_count, 1);
    assert!(has_smask(&out), "transparent watermark keeps its alpha");
}

#[test]
fn watermark_rejects_both_and_neither() {
    let pdf = make_test_pdf(1, None);
    let err = add_watermark(
        &pdf,
        &WatermarkOptions {
            text: Some("X".into()),
            image: Some(make_signature_png()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidArgument);
    let err = add_watermark(
        &pdf,
        &WatermarkOptions {
            text: None,
            image: None,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidArgument);
}

#[test]
fn watermark_rejects_non_ascii_text() {
    let pdf = make_test_pdf(1, None);
    let err = add_watermark(
        &pdf,
        &WatermarkOptions {
            text: Some("हिन्दी".to_string()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidArgument);
}

#[test]
fn page_numbers_default() {
    let pdf = make_test_pdf(3, None);
    let out = add_page_numbers(&pdf, &PageNumberOptions::default()).expect("numbers work");
    assert_eq!(info(&out).page_count, 3);
    assert!(text_of(&out, 2).contains('2'));
}

#[test]
fn page_numbers_format_and_start() {
    let pdf = make_test_pdf(2, None);
    let out = add_page_numbers(
        &pdf,
        &PageNumberOptions {
            format: "Page {n} of {total}".to_string(),
            start_at: 5,
            ..Default::default()
        },
    )
    .expect("numbers work");
    assert!(text_of(&out, 1).contains("Page 5 of 2"));
    assert!(text_of(&out, 2).contains("Page 6 of 2"));
}

#[test]
fn page_numbers_position_top_right() {
    let pdf = make_test_pdf(1, None);
    let out = add_page_numbers(
        &pdf,
        &PageNumberOptions {
            position: Position::TopRight,
            ..Default::default()
        },
    )
    .expect("numbers work");
    assert!(text_of(&out, 1).contains('1'));
}

#[test]
fn place_signature_transparent_png() {
    let pdf = make_test_pdf(1, None);
    let sig = make_signature_png();
    let out = place_signature(
        &pdf,
        &sig,
        &SignatureOptions {
            page: 1,
            ..Default::default()
        },
    )
    .expect("signature works");
    assert_eq!(info(&out).page_count, 1);
    assert!(
        has_smask(&out),
        "signature PNG keeps its alpha via /SMask, not an opaque rectangle"
    );
}

#[test]
fn place_signature_rejects_bad_page_and_image() {
    let pdf = make_test_pdf(1, None);
    let err = place_signature(
        &pdf,
        &make_signature_png(),
        &SignatureOptions {
            page: 5,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::PageOutOfRange);
    let err = place_signature(&pdf, b"nope", &SignatureOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidImage);
}

#[test]
fn info_reports_pages_version_and_title() {
    let pdf = make_test_pdf(4, Some("My Doc"));
    let i = info(&pdf);
    assert_eq!(i.page_count, 4);
    assert_eq!(i.version, "1.7");
    assert_eq!(i.title.as_deref(), Some("My Doc"));
    assert!(!i.encrypted);
    assert_eq!(i.page_sizes.len(), 4);
    assert_eq!(i.page_sizes[0], (595.0, 842.0));
}

#[test]
fn errors_are_clean_not_panics() {
    // Empty input.
    let err = extract_pages(&[], &[1], &PageSetOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::EmptyInput);
    // Not a PDF.
    let err = extract_pages(b"hello world", &[1], &PageSetOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidPdf);
    // Malformed PDF (Catalog points at a missing Pages tree, so the
    // lenient loader yields a 0-page document and page 1 is out of
    // range). The point is a clean error code, never a panic — the
    // exact code depends on how far the lenient parser got.
    let bad = make_malformed_pdf();
    let err = extract_pages(&bad, &[1], &PageSetOptions::default()).unwrap_err();
    assert!(
        err.code == ErrorCode::InvalidPdf || err.code == ErrorCode::PageOutOfRange,
        "unexpected code {:?}",
        err.code
    );
    // Out of range.
    let pdf = make_test_pdf(2, None);
    let err = extract_pages(&pdf, &[3], &PageSetOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::PageOutOfRange);
    assert!(!err.reason_hi.is_empty());
    // Bad range string.
    let err = extract_pages_range(&pdf, "5-2", &PageSetOptions::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::InvalidRange);
}

#[test]
fn bulk_extract_keeps_going_on_failure() {
    let pdf = make_test_pdf(3, None);
    let items = bulk_extract(
        &pdf,
        &[vec![1], vec![9], vec![2, 3]],
        &PageSetOptions::default(),
    );
    assert_eq!(items.len(), 3);
    assert!(items[0].pdf.is_some());
    assert_eq!(items[1].error_code.as_deref(), Some("PAGE_OUT_OF_RANGE"));
    assert!(items[2].pdf.is_some());
}
