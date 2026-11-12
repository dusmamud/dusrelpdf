//! Shared low-level helpers built on lopdf 0.47.
//!
//! None of the 10 operations has a dedicated lopdf API except
//! `delete_pages`, so everything here is the hand-built layer the
//! research report describes: inherited-attribute materialisation,
//! base-14 fonts, ExtGState opacity, content-stream text, and image
//! XObjects — including a soft-mask (`/SMask`) so transparent PNG
//! signatures do not render as opaque rectangles.

use flate2::Compression;
use flate2::write::ZlibEncoder;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream, StringFormat};
use std::io::Write;

use crate::error::{DusrelError, Result};
use crate::options::{BaseFont, CommonOptions};

/// Load a PDF from bytes. Empty input and non-PDF bytes are proper
/// errors, never panics. `password` opens encrypted PDFs.
pub fn load_pdf(bytes: &[u8], password: Option<&str>) -> Result<Document> {
    if bytes.is_empty() {
        return Err(DusrelError::empty_input("PDF bytes"));
    }
    let opts = lopdf::LoadOptions {
        password: password.map(|p| p.to_string()),
        ..Default::default()
    };
    Document::load_mem_with_options(bytes, opts).map_err(|e| e.into())
}

/// Load with the shared [`CommonOptions`].
pub fn load_with_common(bytes: &[u8], common: &CommonOptions) -> Result<Document> {
    load_pdf(bytes, common.password.as_deref())
}

/// Compress (unless disabled) and serialise a document to bytes.
pub fn save_pdf(mut doc: Document, compress: bool) -> Result<Vec<u8>> {
    if compress {
        doc.compress();
    }
    let mut out = Vec::new();
    doc.save_to(&mut out).map_err(DusrelError::from)?;
    Ok(out)
}

/// Walk the `Parent` chain of a page and return the first value found
/// for `key` (e.g. `MediaBox`, `Resources`, `Rotate`).
pub fn inherited_attr(doc: &Document, page_id: ObjectId, key: &[u8]) -> Option<Object> {
    let mut current = Some(page_id);
    let mut depth = 0;
    while let Some(id) = current {
        if depth > 256 {
            break;
        }
        depth += 1;
        let dict = doc.get_object(id).ok()?.as_dict().ok()?;
        if let Ok(value) = dict.get(key) {
            return Some(value.clone());
        }
        current = dict.get(b"Parent").and_then(Object::as_reference).ok();
    }
    None
}

/// Page size in points, from the inherited `MediaBox`.
/// Default is A4 (595 × 842) when no MediaBox exists anywhere.
pub fn page_size(doc: &Document, page_id: ObjectId) -> (f32, f32) {
    if let Some(Object::Array(arr)) = inherited_attr(doc, page_id, b"MediaBox") {
        if arr.len() >= 4 {
            let nums: Vec<f32> = arr
                .iter()
                .map(|o| match o {
                    Object::Real(r) => *r,
                    Object::Integer(i) => *i as f32,
                    _ => 0.0,
                })
                .collect();
            let w = (nums[2] - nums[0]).max(1.0);
            let h = (nums[3] - nums[1]).max(1.0);
            return (w, h);
        }
    }
    (595.0, 842.0)
}

/// Effective rotation of a page (inherited `Rotate`, default 0).
pub fn effective_rotate(doc: &Document, page_id: ObjectId) -> i64 {
    match inherited_attr(doc, page_id, b"Rotate") {
        Some(Object::Integer(i)) => i,
        Some(Object::Real(r)) => r as i64,
        _ => 0,
    }
}

/// Copy inherited `MediaBox` / `CropBox` / `Resources` / `Rotate` onto
/// the page dict itself, exactly like the lopdf README merge example.
/// Required before a page is moved to another document, otherwise the
/// inherited values are lost when the page is re-parented.
pub fn materialize_page_attrs(doc: &mut Document, page_id: ObjectId) -> Result<()> {
    let media = inherited_attr(doc, page_id, b"MediaBox");
    let crop = inherited_attr(doc, page_id, b"CropBox");
    let resources = inherited_attr(doc, page_id, b"Resources");
    let rotate = inherited_attr(doc, page_id, b"Rotate");
    let page = doc
        .get_object_mut(page_id)
        .and_then(Object::as_dict_mut)
        .map_err(|_| DusrelError::invalid_pdf("page object is not a dictionary"))?;
    if !page.has(b"MediaBox") {
        if let Some(m) = &media {
            page.set(b"MediaBox", m.clone());
        }
    }
    if !page.has(b"CropBox") {
        if let Some(c) = crop {
            page.set(b"CropBox", c);
        } else if let Some(m) = &media {
            page.set(b"CropBox", m.clone());
        }
    }
    if !page.has(b"Resources") {
        page.set(
            b"Resources",
            resources.unwrap_or_else(|| Object::Dictionary(Dictionary::new())),
        );
    }
    if !page.has(b"Rotate") {
        page.set(b"Rotate", rotate.unwrap_or(Object::Integer(0)));
    }
    Ok(())
}

/// Ensure a base-14 font is in the page's `Resources → Font` and return
/// its resource name (`F1`, `F2`, …). Never clobbers an existing font:
/// the first unused `F<n>` name wins.
pub fn ensure_font(doc: &mut Document, page_id: ObjectId, font: BaseFont) -> Result<String> {
    // Pick an unused name first (read-only pass).
    let mut name = String::from("F1");
    {
        let resources = doc
            .get_object(page_id)
            .and_then(Object::as_dict)
            .ok()
            .and_then(|page| page.get(b"Resources").ok())
            .and_then(|r| match r {
                Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok().cloned(),
                Object::Dictionary(d) => Some(d.clone()),
                _ => None,
            });
        let mut used: Vec<String> = Vec::new();
        if let Some(res) = resources {
            if let Ok(Object::Dictionary(fonts)) = res.get(b"Font") {
                for (k, _) in fonts.iter() {
                    used.push(String::from_utf8_lossy(k).into_owned());
                }
            } else if let Ok(Object::Reference(fid)) = res.get(b"Font") {
                if let Ok(fonts) = doc.get_object(*fid).and_then(Object::as_dict) {
                    for (k, _) in fonts.iter() {
                        used.push(String::from_utf8_lossy(k).into_owned());
                    }
                }
            }
        }
        for n in 1..100u32 {
            let candidate = format!("F{n}");
            if !used.contains(&candidate) {
                name = candidate;
                break;
            }
        }
    }
    let mut font_dict = Dictionary::new();
    font_dict.set("Type", Object::Name(b"Font".to_vec()));
    font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
    font_dict.set(
        "BaseFont",
        Object::Name(font.base_name().as_bytes().to_vec()),
    );
    font_dict.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
    let font_id = doc.add_object(Object::Dictionary(font_dict));
    // Insert into Resources → Font (create the Font dict if missing).
    let resources = doc
        .get_or_create_resources(page_id)
        .map_err(DusrelError::from)?
        .as_dict_mut()
        .map_err(|_| DusrelError::invalid_pdf("page Resources is not a dictionary"))?;
    if !resources.has(b"Font") {
        resources.set("Font", Dictionary::new());
    }
    let fonts_obj = resources.get_mut(b"Font").map_err(DusrelError::from)?;
    match fonts_obj {
        Object::Dictionary(fonts) => {
            fonts.set(name.as_bytes().to_vec(), Object::Reference(font_id));
        }
        Object::Reference(fid) => {
            let fid = *fid;
            let fonts = doc
                .get_object_mut(fid)
                .and_then(Object::as_dict_mut)
                .map_err(|_| DusrelError::invalid_pdf("Font dict is not a dictionary"))?;
            fonts.set(name.as_bytes().to_vec(), Object::Reference(font_id));
        }
        _ => {
            return Err(DusrelError::invalid_pdf(
                "page Font resources are not a dictionary",
            ));
        }
    }
    Ok(name)
}

/// Ensure an `ExtGState` with the given opacity exists and return its
/// resource name (`GS1`, `GS2`, …). Returns `None` for opacity >= 1.0
/// (fully opaque needs no graphics state).
pub fn ensure_graphics_state(
    doc: &mut Document,
    page_id: ObjectId,
    opacity: f32,
) -> Result<Option<String>> {
    if !(0.0..1.0).contains(&opacity) {
        if (opacity - 1.0).abs() < f32::EPSILON {
            return Ok(None);
        }
        return Err(DusrelError::invalid_argument(
            "Opacity must be between 0.0 and 1.0.",
            "Opacity 0.0 se 1.0 ke beech honi chahiye.",
        ));
    }
    let name = "GS1".to_string();
    let mut gs_dict = Dictionary::new();
    gs_dict.set("Type", Object::Name(b"ExtGState".to_vec()));
    gs_dict.set("ca", Object::Real(opacity));
    gs_dict.set("CA", Object::Real(opacity));
    let gs_id = doc.add_object(Object::Dictionary(gs_dict));
    doc.add_graphics_state(page_id, name.as_bytes(), gs_id)
        .map_err(DusrelError::from)?;
    Ok(Some(name))
}

/// Escape text for a literal PDF string and reject non-ASCII text:
/// base-14 fonts only cover WinAnsi, so anything else would render
/// as mojibake. The caller gets a clear error instead.
pub fn pdf_literal_string(text: &str) -> Result<Vec<u8>> {
    if !text.is_ascii() {
        return Err(DusrelError::invalid_argument(
            "Watermark / page-number text must be ASCII (base-14 fonts only support WinAnsi). Use an image watermark for other scripts.",
            "Watermark / page-number text sirf ASCII ho sakta hai (base-14 fonts sirf WinAnsi support karte hain). Doosri scripts ke liye image watermark use karein.",
        ));
    }
    let mut out = Vec::with_capacity(text.len());
    for b in text.bytes() {
        match b {
            b'(' | b')' | b'\\' => {
                out.push(b'\\');
                out.push(b);
            }
            _ => out.push(b),
        }
    }
    Ok(out)
}

fn flate(data: &[u8]) -> Vec<u8> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    let _ = enc.write_all(data);
    enc.finish().unwrap_or_else(|_| data.to_vec())
}

/// Build an image XObject stream from image bytes.
///
/// JPEG is passed through untouched (DCTDecode, via lopdf) so it stays
/// small. Every other format is decoded with the `image` crate:
/// fully-opaque images become a plain RGB Flate stream, and images
/// with any transparent pixel get a greyscale `/SMask` soft-mask
/// XObject built from the alpha channel — otherwise a transparent PNG
/// signature would render as an opaque rectangle (lopdf's
/// `image_from` silently discards alpha).
///
/// Returns the stream plus its pixel dimensions.
pub fn image_stream(doc: &mut Document, bytes: &[u8]) -> Result<(Stream, u32, u32)> {
    if bytes.is_empty() {
        return Err(DusrelError::invalid_image("image bytes are empty"));
    }
    let decoded = image::load_from_memory(bytes)
        .map_err(|e| DusrelError::invalid_image(&format!("cannot decode image: {e}")))?;
    let rgba = decoded.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    if width == 0 || height == 0 {
        return Err(DusrelError::invalid_image("image has zero width or height"));
    }
    // JPEG: keep the original bytes (DCTDecode passthrough).
    if image::guess_format(bytes)
        .map(|f| f == image::ImageFormat::Jpeg)
        .unwrap_or(false)
    {
        let stream = lopdf::xobject::image_from(bytes.to_vec())
            .map_err(|e| DusrelError::invalid_image(&e.to_string()))?;
        return Ok((stream, width, height));
    }
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    let mut alpha = Vec::with_capacity((width * height) as usize);
    let mut has_transparency = false;
    for px in rgba.pixels() {
        rgb.extend_from_slice(&px.0[..3]);
        alpha.push(px.0[3]);
        if px.0[3] < 255 {
            has_transparency = true;
        }
    }
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"XObject".to_vec()));
    dict.set("Subtype", Object::Name(b"Image".to_vec()));
    dict.set("Width", Object::Integer(width as i64));
    dict.set("Height", Object::Integer(height as i64));
    dict.set("BitsPerComponent", Object::Integer(8));
    dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));
    if has_transparency {
        let mut smask_dict = Dictionary::new();
        smask_dict.set("Type", Object::Name(b"XObject".to_vec()));
        smask_dict.set("Subtype", Object::Name(b"Image".to_vec()));
        smask_dict.set("Width", Object::Integer(width as i64));
        smask_dict.set("Height", Object::Integer(height as i64));
        smask_dict.set("BitsPerComponent", Object::Integer(8));
        smask_dict.set("ColorSpace", Object::Name(b"DeviceGray".to_vec()));
        smask_dict.set("Filter", Object::Name(b"FlateDecode".to_vec()));
        let smask = Stream::new(smask_dict, flate(&alpha));
        let smask_id = doc.add_object(Object::Stream(smask));
        dict.set("SMask", Object::Reference(smask_id));
    }
    let stream = Stream::new(dict, flate(&rgb));
    Ok((stream, width, height))
}

/// Register an image stream on a page's `Resources → XObject` and
/// return its resource name (`X<object_id>`, same scheme as lopdf's
/// `insert_image`).
pub fn register_image(doc: &mut Document, page_id: ObjectId, stream: Stream) -> Result<String> {
    let img_id = doc.add_object(Object::Stream(stream));
    let name = format!("X{}", img_id.0);
    doc.add_xobject(page_id, name.as_bytes(), img_id)
        .map_err(DusrelError::from)?;
    Ok(name)
}

/// Append `q [gs] cm w 0 0 h x y /Name Do Q` to a page — paints an
/// already-registered image over the page content.
#[allow(clippy::too_many_arguments)]
pub fn append_image_ops(
    doc: &mut Document,
    page_id: ObjectId,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    gs_name: Option<&str>,
) -> Result<()> {
    let mut ops = vec![Operation::new("q", vec![])];
    if let Some(gs) = gs_name {
        ops.push(Operation::new(
            "gs",
            vec![Object::Name(gs.as_bytes().to_vec())],
        ));
    }
    ops.push(Operation::new(
        "cm",
        vec![
            Object::Real(w),
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(h),
            Object::Real(x),
            Object::Real(y),
        ],
    ));
    ops.push(Operation::new(
        "Do",
        vec![Object::Name(name.as_bytes().to_vec())],
    ));
    ops.push(Operation::new("Q", vec![]));
    doc.add_to_page_content(page_id, Content { operations: ops })
        .map_err(DusrelError::from)?;
    Ok(())
}

/// Append text ops (`BT Tf Tm Tj ET`, optionally inside `q gs … Q`)
/// to a page. Coordinates are points, origin bottom-left.
#[allow(clippy::too_many_arguments)]
pub fn append_text_ops(
    doc: &mut Document,
    page_id: ObjectId,
    font_name: &str,
    font_size: f32,
    text: &str,
    x: f32,
    y: f32,
    rotation_deg: f32,
    gs_name: Option<&str>,
) -> Result<()> {
    let escaped = pdf_literal_string(text)?;
    let rad = rotation_deg.to_radians();
    let (cos, sin) = (rad.cos(), rad.sin());
    let mut ops = vec![Operation::new("q", vec![])];
    if let Some(gs) = gs_name {
        ops.push(Operation::new(
            "gs",
            vec![Object::Name(gs.as_bytes().to_vec())],
        ));
    }
    ops.push(Operation::new("BT", vec![]));
    ops.push(Operation::new(
        "Tf",
        vec![
            Object::Name(font_name.as_bytes().to_vec()),
            Object::Real(font_size),
        ],
    ));
    ops.push(Operation::new(
        "Tm",
        vec![
            Object::Real(cos),
            Object::Real(sin),
            Object::Real(-sin),
            Object::Real(cos),
            Object::Real(x),
            Object::Real(y),
        ],
    ));
    ops.push(Operation::new(
        "Tj",
        vec![Object::String(escaped, StringFormat::Literal)],
    ));
    ops.push(Operation::new("ET", vec![]));
    ops.push(Operation::new("Q", vec![]));
    doc.add_to_page_content(page_id, Content { operations: ops })
        .map_err(DusrelError::from)?;
    Ok(())
}

/// Resolve a position option to (x, y) in points for a stamp of the
/// given width/height on a page of the given size.
pub fn resolve_position(
    position: crate::options::Position,
    page_w: f32,
    page_h: f32,
    stamp_w: f32,
    stamp_h: f32,
    margin: f32,
) -> (f32, f32) {
    use crate::options::Position::*;
    match position {
        TopLeft => (margin, page_h - margin - stamp_h),
        TopCenter => ((page_w - stamp_w) / 2.0, page_h - margin - stamp_h),
        TopRight => (page_w - margin - stamp_w, page_h - margin - stamp_h),
        BottomLeft => (margin, margin),
        BottomCenter => ((page_w - stamp_w) / 2.0, margin),
        BottomRight => (page_w - margin - stamp_w, margin),
        Center => ((page_w - stamp_w) / 2.0, (page_h - stamp_h) / 2.0),
    }
}
