//! The four "stamping" operations: images-to-PDF, watermark,
//! page numbers, signature placement.
//!
//! All are hand-built on lopdf content streams — lopdf has no
//! watermark / page-number API. Images use [`crate::common::image_stream`]
//! (transparent PNGs keep their alpha via a `/SMask` soft-mask).

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};

use crate::common::{
    append_image_ops, append_text_ops, ensure_font, ensure_graphics_state, image_stream,
    load_with_common, page_size, register_image, resolve_position, save_pdf,
};
use crate::error::{DusrelError, Result};
use crate::options::{
    BaseFont, ImageFit, ImagesToPdfOptions, PageNumberOptions, PageSize, Position,
    SignatureOptions, WatermarkOptions,
};
use crate::range::validate_pages;

/// One page per image, in input order.
pub fn images_to_pdf(images: &[Vec<u8>], options: &ImagesToPdfOptions) -> Result<Vec<u8>> {
    if images.is_empty() {
        return Err(DusrelError::empty_input("images"));
    }
    let mut doc = Document::with_version("1.5");
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
    let mut kids: Vec<ObjectId> = Vec::with_capacity(images.len());
    for (i, bytes) in images.iter().enumerate() {
        let (stream, iw, ih) = image_stream(&mut doc, bytes).map_err(|e| {
            DusrelError::invalid_image(&format!("image {}: {}", i + 1, e.reason_en))
        })?;
        let img_id = doc.add_object(Object::Stream(stream));
        let img_name = format!("X{}", img_id.0);
        let (page_w, page_h) = match options.page_size {
            PageSize::Auto => (iw as f32, ih as f32),
            PageSize::A4 => (595.0, 842.0),
            PageSize::Letter => (612.0, 792.0),
            PageSize::Custom { width, height } => (width, height),
        };
        let (draw_w, draw_h, draw_x, draw_y) = match options.fit {
            ImageFit::Fill => (page_w, page_h, 0.0, 0.0),
            ImageFit::Contain => {
                let scale = (page_w / iw as f32).min(page_h / ih as f32);
                let w = iw as f32 * scale;
                let h = ih as f32 * scale;
                (w, h, (page_w - w) / 2.0, (page_h - h) / 2.0)
            }
        };
        let [r, g, b] = options.background;
        let content = Content {
            operations: vec![
                Operation::new(
                    "rg",
                    vec![Object::Real(r), Object::Real(g), Object::Real(b)],
                ),
                Operation::new(
                    "re",
                    vec![
                        Object::Real(0.0),
                        Object::Real(0.0),
                        Object::Real(page_w),
                        Object::Real(page_h),
                    ],
                ),
                Operation::new("f", vec![]),
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        Object::Real(draw_w),
                        Object::Real(0.0),
                        Object::Real(0.0),
                        Object::Real(draw_h),
                        Object::Real(draw_x),
                        Object::Real(draw_y),
                    ],
                ),
                Operation::new("Do", vec![Object::Name(img_name.as_bytes().to_vec())]),
                Operation::new("Q", vec![]),
            ],
        };
        let content_bytes = content.encode().map_err(DusrelError::from)?;
        let content_id = doc.add_object(Object::Stream(Stream::new(
            Dictionary::new(),
            content_bytes,
        )));
        let mut xobjects = Dictionary::new();
        xobjects.set(img_name.as_bytes().to_vec(), Object::Reference(img_id));
        let mut resources = Dictionary::new();
        resources.set("XObject", Object::Dictionary(xobjects));
        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(page_w),
                Object::Real(page_h),
            ]),
        );
        page_dict.set("Contents", Object::Reference(content_id));
        page_dict.set("Resources", Object::Dictionary(resources));
        let page_id = doc.add_object(Object::Dictionary(page_dict));
        kids.push(page_id);
    }
    let pages = doc
        .get_object_mut(pages_id)
        .and_then(Object::as_dict_mut)
        .map_err(|_| DusrelError::invalid_pdf("Pages root is not a dictionary"))?;
    pages.set(
        "Kids",
        Object::Array(kids.iter().map(|id| Object::Reference(*id)).collect()),
    );
    pages.set("Count", Object::Integer(kids.len() as i64));
    save_pdf(doc, true)
}

/// Stamp a text or image watermark over the selected pages.
///
/// The watermark paints **over** the page content (it is appended to
/// the content stream, not an annotation). Text is ASCII-only
/// (base-14 fonts); use an image watermark for other scripts. The
/// rotation is applied around the stamp's anchor point.
pub fn add_watermark(pdf: &[u8], options: &WatermarkOptions) -> Result<Vec<u8>> {
    if options.opacity < 0.0 || options.opacity > 1.0 {
        return Err(DusrelError::invalid_argument(
            "Opacity must be between 0.0 and 1.0.",
            "Opacity 0.0 se 1.0 ke beech honi chahiye.",
        ));
    }
    let has_text = options
        .text
        .as_ref()
        .map(|t| !t.is_empty())
        .unwrap_or(false);
    let has_image = options.image.is_some();
    if has_text == has_image {
        return Err(DusrelError::invalid_argument(
            "Watermark needs exactly one of `text` or `image`.",
            "Watermark ke liye `text` ya `image` me se exactly ek chahiye.",
        ));
    }
    let mut doc = load_with_common(pdf, &options.common)?;
    let total = doc.get_pages().len() as u32;
    let targets: Vec<u32> = match &options.pages {
        Some(pages) => {
            validate_pages(pages, total)?;
            let mut p = pages.clone();
            p.sort_unstable();
            p.dedup();
            p
        }
        None => (1..=total).collect(),
    };
    for page in targets {
        let page_id = doc
            .get_pages()
            .get(&page)
            .copied()
            .ok_or_else(|| DusrelError::page_out_of_range(page, total))?;
        let (page_w, page_h) = page_size(&doc, page_id);
        if let Some(text) = &options.text {
            if !has_text {
                continue;
            }
            let font_name = ensure_font(&mut doc, page_id, options.font)?;
            let gs = ensure_graphics_state(&mut doc, page_id, options.opacity)?;
            let text_w = options.font.text_width_pt(text, options.font_size);
            let (x, y) = match (options.x, options.y) {
                (Some(x), Some(y)) => (x, y),
                _ => resolve_position(
                    options.position,
                    page_w,
                    page_h,
                    text_w,
                    options.font_size,
                    36.0,
                ),
            };
            append_text_ops(
                &mut doc,
                page_id,
                &font_name,
                options.font_size,
                text,
                x,
                y,
                options.rotation,
                gs.as_deref(),
            )?;
        } else if let Some(image_bytes) = &options.image {
            let (stream, iw, ih) = image_stream(&mut doc, image_bytes)?;
            let (w, h) = match (options.image_width, options.image_height) {
                (Some(w), Some(h)) => (w, h),
                (Some(w), None) => (w, w * ih as f32 / iw as f32),
                (None, Some(h)) => (h * iw as f32 / ih as f32, h),
                (None, None) => (iw as f32 * 0.75, ih as f32 * 0.75),
            };
            let name = register_image(&mut doc, page_id, stream)?;
            let gs = ensure_graphics_state(&mut doc, page_id, options.opacity)?;
            let (x, y) = match (options.x, options.y) {
                (Some(x), Some(y)) => (x, y),
                _ => resolve_position(options.position, page_w, page_h, w, h, 36.0),
            };
            append_image_ops(&mut doc, page_id, &name, x, y, w, h, gs.as_deref())?;
        }
    }
    save_pdf(doc, options.common.compress)
}

/// Stamp page numbers over the selected pages.
///
/// `{n}` is the printed number (`start_at` + position − 1) and
/// `{total}` is the page count. Numbers are baked into the content
/// stream (not annotations); re-running duplicates them.
pub fn add_page_numbers(pdf: &[u8], options: &PageNumberOptions) -> Result<Vec<u8>> {
    if !options.format.contains("{n}") {
        return Err(DusrelError::invalid_argument(
            "Page-number format must contain {n}, e.g. \"{n}\" or \"Page {n} of {total}\".",
            "Page-number format me {n} hona chahiye, jaise \"{n}\" ya \"Page {n} of {total}\".",
        ));
    }
    let mut doc = load_with_common(pdf, &options.common)?;
    let total = doc.get_pages().len() as u32;
    let targets: Vec<u32> = match &options.pages {
        Some(pages) => {
            validate_pages(pages, total)?;
            let mut p = pages.clone();
            p.sort_unstable();
            p.dedup();
            p
        }
        None => (1..=total).collect(),
    };
    let pages_map = doc.get_pages();
    for page in targets {
        let page_id = pages_map
            .get(&page)
            .copied()
            .ok_or_else(|| DusrelError::page_out_of_range(page, total))?;
        let printed = options.start_at + (page - 1);
        let text = options
            .format
            .replace("{n}", &printed.to_string())
            .replace("{total}", &total.to_string());
        let (page_w, page_h) = page_size(&doc, page_id);
        // Font first (needs &mut doc), then position, then append.
        let font_name = ensure_font(&mut doc, page_id, options.font)?;
        let text_w = options.font.text_width_pt(&text, options.font_size);
        let (x, y) = resolve_position(
            options.position,
            page_w,
            page_h,
            text_w,
            options.font_size,
            options.margin,
        );
        append_text_ops(
            &mut doc,
            page_id,
            &font_name,
            options.font_size,
            &text,
            x,
            y,
            0.0,
            None,
        )?;
    }
    save_pdf(doc, options.common.compress)
}

/// Place a handwritten signature IMAGE on one page.
///
/// This is image placement only — **not** a cryptographic digital
/// signature (no signature field, no certificate, nothing verifiable).
/// Transparent PNGs keep their transparency via a `/SMask` soft-mask:
/// without it the signature would render as an opaque rectangle.
pub fn place_signature(
    pdf: &[u8],
    signature_image: &[u8],
    options: &SignatureOptions,
) -> Result<Vec<u8>> {
    let mut doc = load_with_common(pdf, &options.common)?;
    let total = doc.get_pages().len() as u32;
    if options.page == 0 || options.page > total {
        return Err(DusrelError::page_out_of_range(options.page, total));
    }
    let page_id = doc
        .get_pages()
        .get(&options.page)
        .copied()
        .expect("validated page exists");
    let (stream, iw, ih) = image_stream(&mut doc, signature_image)?;
    let (w, h) = match (options.width, options.height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w * ih as f32 / iw as f32),
        (None, Some(h)) => (h * iw as f32 / ih as f32, h),
        (None, None) => (144.0, 144.0 * ih as f32 / iw as f32),
    };
    let name = register_image(&mut doc, page_id, stream)?;
    append_image_ops(&mut doc, page_id, &name, options.x, options.y, w, h, None)?;
    save_pdf(doc, options.common.compress)
}

/// Base-14 font metrics helper, re-exported for the bindings crates.
pub fn default_font() -> BaseFont {
    BaseFont::Helvetica
}

/// Default page-number position, re-exported for the bindings crates.
pub fn default_position() -> Position {
    Position::BottomCenter
}
