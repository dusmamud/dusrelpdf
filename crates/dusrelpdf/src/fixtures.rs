//! Test-data builders (also used by the TS / Python / CLI test
//! suites via the bindings): small fixture PDFs built with lopdf
//! itself, a transparent-PNG signature, and a malformed-PDF sample.
//!
//! These are public on purpose — downstream tests and the website
//! playground build the same fixtures, so every language tests the
//! same bytes.

use image::{ImageBuffer, Rgba};
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, Stream};

/// Build an N-page PDF; every page says `Page <n>` in Courier and
/// has its own MediaBox (A4). `title` sets the Info title when given.
pub fn make_test_pdf(pages: u32, title: Option<&str>) -> Vec<u8> {
    assert!(pages >= 1, "fixture needs at least one page");
    let mut doc = Document::with_version("1.7");
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(vec![]));
    pages_dict.set("Count", Object::Integer(0));
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog_dict));
    doc.trailer.set("Root", Object::Reference(catalog_id));
    let mut kids = Vec::new();
    for n in 1..=pages {
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), Object::Real(24.0)]),
                Operation::new("Td", vec![Object::Real(100.0), Object::Real(700.0)]),
                Operation::new(
                    "Tj",
                    vec![Object::String(
                        format!("Page {n}").into_bytes(),
                        lopdf::StringFormat::Literal,
                    )],
                ),
                Operation::new("ET", vec![]),
            ],
        };
        let bytes = content.encode().expect("fixture content encodes");
        let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), bytes)));
        let mut font_dict = Dictionary::new();
        font_dict.set("Type", Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", Object::Name(b"Courier".to_vec()));
        font_dict.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
        let font_id = doc.add_object(Object::Dictionary(font_dict));
        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Reference(font_id));
        let mut resources = Dictionary::new();
        resources.set("Font", Object::Dictionary(fonts));
        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(595.0),
                Object::Real(842.0),
            ]),
        );
        page_dict.set("Contents", Object::Reference(content_id));
        page_dict.set("Resources", Object::Dictionary(resources));
        let page_id = doc.add_object(Object::Dictionary(page_dict));
        kids.push(page_id);
    }
    if let Some(title) = title {
        let mut info = Dictionary::new();
        info.set(
            "Title",
            Object::String(title.as_bytes().to_vec(), lopdf::StringFormat::Literal),
        );
        let info_id = doc.add_object(Object::Dictionary(info));
        doc.trailer.set("Info", Object::Reference(info_id));
    }
    let pages = doc
        .get_object_mut(pages_id)
        .and_then(Object::as_dict_mut)
        .expect("pages dict");
    pages.set(
        "Kids",
        Object::Array(kids.iter().map(|id| Object::Reference(*id)).collect()),
    );
    pages.set("Count", Object::Integer(kids.len() as i64));
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("fixture saves");
    out
}

/// Build a PDF whose pages do NOT carry their own MediaBox — it lives
/// on the parent `Pages` dict, so the operation must materialise it
/// (the merge/extract path). Each page still says `Page <n>`.
pub fn make_inherited_mediabox_pdf(pages: u32) -> Vec<u8> {
    let mut doc = Document::with_version("1.7");
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Kids", Object::Array(vec![]));
    pages_dict.set("Count", Object::Integer(0));
    pages_dict.set(
        "MediaBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(595.0),
            Object::Real(842.0),
        ]),
    );
    let pages_id = doc.add_object(Object::Dictionary(pages_dict));
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog_dict));
    doc.trailer.set("Root", Object::Reference(catalog_id));
    let mut kids = Vec::new();
    for n in 1..=pages {
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec![Object::Name(b"F1".to_vec()), Object::Real(24.0)]),
                Operation::new("Td", vec![Object::Real(100.0), Object::Real(700.0)]),
                Operation::new(
                    "Tj",
                    vec![Object::String(
                        format!("Page {n}").into_bytes(),
                        lopdf::StringFormat::Literal,
                    )],
                ),
                Operation::new("ET", vec![]),
            ],
        };
        let bytes = content.encode().expect("fixture content encodes");
        let content_id = doc.add_object(Object::Stream(Stream::new(Dictionary::new(), bytes)));
        let mut font_dict = Dictionary::new();
        font_dict.set("Type", Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", Object::Name(b"Courier".to_vec()));
        font_dict.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
        let font_id = doc.add_object(Object::Dictionary(font_dict));
        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Reference(font_id));
        let mut resources = Dictionary::new();
        resources.set("Font", Object::Dictionary(fonts));
        // NOTE: no MediaBox here — it is inherited from Pages.
        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set("Contents", Object::Reference(content_id));
        page_dict.set("Resources", Object::Dictionary(resources));
        let page_id = doc.add_object(Object::Dictionary(page_dict));
        kids.push(page_id);
    }
    let pages = doc
        .get_object_mut(pages_id)
        .and_then(Object::as_dict_mut)
        .expect("pages dict");
    pages.set(
        "Kids",
        Object::Array(kids.iter().map(|id| Object::Reference(*id)).collect()),
    );
    pages.set("Count", Object::Integer(kids.len() as i64));
    let mut out = Vec::new();
    doc.save_to(&mut out).expect("fixture saves");
    out
}

/// A small PNG that looks like a handwritten signature: an opaque dark
/// stroke on a fully transparent background. If the alpha channel is
/// dropped, the image renders as an opaque rectangle — exactly the bug
/// the signature tests guard against.
pub fn make_signature_png() -> Vec<u8> {
    let (w, h) = (200u32, 80u32);
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(w, h);
    for (x, y, px) in img.enumerate_pixels_mut() {
        // A diagonal "stroke" plus a wavy underline, opaque black.
        let on_stroke = (x as i32 - y as i32 * 2).abs() < 3
            || (y as i32 - (40 + ((x as f32 / 9.0).sin() * 8.0) as i32)).abs() < 2;
        if on_stroke {
            *px = Rgba([20, 20, 20, 255]);
        } else {
            *px = Rgba([0, 0, 0, 0]);
        }
    }
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .expect("png encodes");
    out
}

/// A small opaque JPEG (a red/blue photo-like image).
pub fn make_photo_jpeg() -> Vec<u8> {
    // JPEG has no alpha: use an RGB buffer (the encoder rejects RGBA).
    let (w, h) = (120u32, 90u32);
    let mut img: ImageBuffer<image::Rgb<u8>, Vec<u8>> = ImageBuffer::new(w, h);
    for (x, y, px) in img.enumerate_pixels_mut() {
        let r = (x * 255 / w) as u8;
        let b = (y * 255 / h) as u8;
        *px = image::Rgb([r, 60, b]);
    }
    let mut out = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut out),
        image::ImageFormat::Jpeg,
    )
    .expect("jpeg encodes");
    out
}

/// Bytes that are not a PDF at all (truncated header + garbage).
pub fn make_malformed_pdf() -> Vec<u8> {
    b"%PDF-1.4\n%garbage\n1 0 obj\n<< /Type /Catalog /Pages 99 0 R >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n".to_vec()
}
