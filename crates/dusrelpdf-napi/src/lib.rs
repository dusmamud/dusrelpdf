//! Node.js native bindings for dusrelpdf — thin napi-rs wrappers
//! over the Rust core.
//!
//! All PDF logic lives in the `dusrelpdf` crate; this crate only
//! converts between JavaScript values and Rust types. Build with:
//! `npm run build` in this directory (napi CLI — see README.md), or
//! `cargo build --release -p dusrelpdf-napi` followed by copying
//! `target/release/libdusrelpdf_napi.so` to
//! `dusrelpdf-napi.linux-x64-gnu.node` in this directory.
//!
//! Contract (same as the WASM binding, `crates/dusrelpdf-wasm`):
//! * Binary results are Node `Buffer`s (splits return an Array of them).
//! * On failure the function throws a JS `Error` whose message is
//!   `"CODE: reason in English"`, e.g. `"INVALID_RANGE: Invalid page
//!   range \"9\". …"` — the same stable code the core reports, so JS
//!   callers see identical behaviour to the WASM path.
//! * Options are plain JS objects matching the Rust `*Options`
//!   structs (see `crates/dusrelpdf/src/options.rs`). Unlike serde
//!   directly, missing fields are filled from the Rust `Default`
//!   impl — `{}` means "all defaults" — so callers can pass partial
//!   objects. `undefined` / omitted options also mean all defaults.
//! * `get_pdf_info` / `bulk_extract` return JSON strings in the same
//!   shape as the WASM binding (`bulk_extract` PDFs are base64).

use dusrelpdf::{
    CommonOptions, DusrelError, ImagesToPdfOptions, MergeOptions, PageNumberOptions,
    PageSetOptions, RotateOptions, SignatureOptions, WatermarkOptions,
    add_page_numbers as core_add_page_numbers, add_watermark as core_add_watermark,
    bulk_extract as core_bulk_extract, extract_pages as core_extract_pages,
    extract_pages_range as core_extract_pages_range, get_pdf_info as core_get_pdf_info,
    images_to_pdf as core_images_to_pdf, merge_pdfs as core_merge_pdfs,
    place_signature as core_place_signature, remove_pages as core_remove_pages,
    reorder_pages as core_reorder_pages, rotate_pages as core_rotate_pages,
    split_each_page as core_split_each_page, split_pdf as core_split_pdf,
    split_pdf_ranges as core_split_pdf_ranges,
};
use napi::{Error, Result, Status, bindgen_prelude::Buffer};
use napi_derive::napi;
use serde_json::Value;

/// A core error becomes a JS `Error` whose message carries the stable
/// code: `"INVALID_RANGE: …"`. The JS wrapper parses the code back
/// out of the message, so native and WASM failures look identical.
fn err_napi(e: DusrelError) -> Error {
    Error::new(Status::GenericFailure, e.to_string())
}

/// `INVALID_ARGUMENT` for a bad options object (or a bad argument
/// passed to a binding helper), with EN + HI reasons like the core.
fn invalid_argument(reason_en: &str, reason_hi: &str) -> Error {
    err_napi(DusrelError::invalid_argument(reason_en, reason_hi))
}

/// Parse an options object: the caller's JSON is merged over the
/// serialised Rust defaults, then deserialised. This is what makes
/// partial objects work — serde on its own rejects a missing
/// non-`Option` field (e.g. `RotateOptions.angle`), while the WASM
/// binding gets away with it only because the TS wrapper always
/// sends complete objects. Native callers get the forgiving path.
///
/// `drop_keys` lists (trigger, key) pairs: when the caller's object
/// contains `trigger`, `key` is dropped from the merged defaults
/// first — used by watermarks, where the default `text: "DRAFT"`
/// must not survive when the caller supplies an `image`.
fn parse_options<T>(value: Option<Value>, drop_keys: &[(&str, &str)]) -> Result<T>
where
    T: Default + serde::Serialize + serde::de::DeserializeOwned,
{
    let mut base = serde_json::to_value(T::default()).map_err(|e| {
        invalid_argument(
            &format!("Could not serialise default options: {e}"),
            &format!("Default options serialise nahi ho paye: {e}"),
        )
    })?;
    if let Some(value) = value {
        let over = value.as_object().ok_or_else(|| {
            invalid_argument("Options must be an object.", "Options object hona chahiye.")
        })?;
        let obj = base.as_object_mut().expect("default serialises to object");
        for (trigger, key) in drop_keys {
            if over.contains_key(*trigger) {
                obj.remove(*key);
            }
        }
        for (key, val) in over {
            obj.insert(key.clone(), val.clone());
        }
    }
    serde_json::from_value(base).map_err(|e| {
        invalid_argument(
            &format!("Invalid options: {e}"),
            &format!("Galat options hain: {e}"),
        )
    })
}

/// Watermark options need one extra step: the Rust default carries
/// `text: "DRAFT"`, so merging an object that sets `image` would
/// leave both `text` and `image` set and the core would reject it.
/// Drop the default text when the caller supplies an image.
fn parse_watermark_options(value: Option<Value>) -> Result<WatermarkOptions> {
    parse_options::<WatermarkOptions>(value, &[("image", "text")])
}

/// `CommonOptions` is flattened into every `*Options` struct, so
/// parsing the full struct already covers it — this is only the
/// standalone parser for `get_pdf_info` / `bulk_extract`.
fn parse_common(value: Option<Value>) -> Result<CommonOptions> {
    parse_options::<CommonOptions>(value, &[])
}

fn buffers_to_vecs(buffers: Vec<Buffer>) -> Vec<Vec<u8>> {
    buffers.iter().map(|b| b.as_ref().to_vec()).collect()
}

fn bytes_to_buffer(bytes: Vec<u8>) -> Buffer {
    Buffer::from(bytes)
}

fn buffers_from_vecs(vecs: Vec<Vec<u8>>) -> Vec<Buffer> {
    vecs.into_iter().map(bytes_to_buffer).collect()
}

#[napi(js_name = "version")]
pub fn version() -> String {
    dusrelpdf::VERSION.to_string()
}

/// Build a mock multi-page PDF (test data only). Pages below 1 are
/// clamped to 1, like the WASM binding.
#[napi(js_name = "makeTestPdf")]
pub fn make_test_pdf(pages: u32) -> Buffer {
    bytes_to_buffer(dusrelpdf::fixtures::make_test_pdf(pages.max(1), None))
}

/// Merge PDFs (JS Array of Buffers) into one Buffer.
#[napi(js_name = "mergePdfs")]
pub fn merge_pdfs(inputs: Vec<Buffer>, options: Option<Value>) -> Result<Buffer> {
    let bufs = buffers_to_vecs(inputs);
    let opts = parse_options::<MergeOptions>(options, &[])?;
    core_merge_pdfs(&bufs, &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// Split into one Buffer per range (array of page-number arrays).
#[napi(js_name = "splitPdf")]
pub fn split_pdf(
    pdf: Buffer,
    ranges: Vec<Vec<u32>>,
    options: Option<Value>,
) -> Result<Vec<Buffer>> {
    let opts = parse_options::<PageSetOptions>(options, &[])?;
    core_split_pdf(pdf.as_ref(), &ranges, &opts)
        .map(buffers_from_vecs)
        .map_err(err_napi)
}

/// Split from a range string such as "1-3,4-5".
#[napi(js_name = "splitPdfRanges")]
pub fn split_pdf_ranges(
    pdf: Buffer,
    ranges: String,
    options: Option<Value>,
) -> Result<Vec<Buffer>> {
    let opts = parse_options::<PageSetOptions>(options, &[])?;
    core_split_pdf_ranges(pdf.as_ref(), &ranges, &opts)
        .map(buffers_from_vecs)
        .map_err(err_napi)
}

/// Split into one PDF per page.
#[napi(js_name = "splitEachPage")]
pub fn split_each_page(pdf: Buffer, options: Option<Value>) -> Result<Vec<Buffer>> {
    let opts = parse_options::<PageSetOptions>(options, &[])?;
    core_split_each_page(pdf.as_ref(), &opts)
        .map(buffers_from_vecs)
        .map_err(err_napi)
}

/// Extract pages (JS Array of 1-based page numbers) into one PDF.
#[napi(js_name = "extractPages")]
pub fn extract_pages(pdf: Buffer, pages: Vec<u32>, options: Option<Value>) -> Result<Buffer> {
    let opts = parse_options::<PageSetOptions>(options, &[])?;
    core_extract_pages(pdf.as_ref(), &pages, &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// Extract from a range string such as "1-3,5".
#[napi(js_name = "extractPagesRange")]
pub fn extract_pages_range(pdf: Buffer, range: String, options: Option<Value>) -> Result<Buffer> {
    let opts = parse_options::<PageSetOptions>(options, &[])?;
    core_extract_pages_range(pdf.as_ref(), &range, &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// Delete pages (JS Array of 1-based page numbers).
#[napi(js_name = "removePages")]
pub fn remove_pages(pdf: Buffer, pages: Vec<u32>, options: Option<Value>) -> Result<Buffer> {
    let opts = parse_options::<PageSetOptions>(options, &[])?;
    core_remove_pages(pdf.as_ref(), &pages, &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// Reorder pages (JS Array forming a full permutation, 1-based).
#[napi(js_name = "reorderPages")]
pub fn reorder_pages(pdf: Buffer, order: Vec<u32>, options: Option<Value>) -> Result<Buffer> {
    let opts = parse_options::<PageSetOptions>(options, &[])?;
    core_reorder_pages(pdf.as_ref(), &order, &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// Rotate pages. `options` is a RotateOptions object
/// ({ angle, pages?, password? }); `pages` omitted = all pages.
#[napi(js_name = "rotatePages")]
pub fn rotate_pages(pdf: Buffer, options: Option<Value>) -> Result<Buffer> {
    let opts = parse_options::<RotateOptions>(options, &[])?;
    core_rotate_pages(pdf.as_ref(), &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// Make a PDF from images (JS Array of Buffers, one page each).
#[napi(js_name = "imagesToPdf")]
pub fn images_to_pdf(images: Vec<Buffer>, options: Option<Value>) -> Result<Buffer> {
    let bufs = buffers_to_vecs(images);
    let opts = parse_options::<ImagesToPdfOptions>(options, &[])?;
    core_images_to_pdf(&bufs, &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// Stamp a text or image watermark.
#[napi(js_name = "addWatermark")]
pub fn add_watermark(pdf: Buffer, options: Option<Value>) -> Result<Buffer> {
    let opts = parse_watermark_options(options)?;
    core_add_watermark(pdf.as_ref(), &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// Stamp page numbers.
#[napi(js_name = "addPageNumbers")]
pub fn add_page_numbers(pdf: Buffer, options: Option<Value>) -> Result<Buffer> {
    let opts = parse_options::<PageNumberOptions>(options, &[])?;
    core_add_page_numbers(pdf.as_ref(), &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// Place a signature image on one page.
#[napi(js_name = "placeSignature")]
pub fn place_signature(pdf: Buffer, signature: Buffer, options: Option<Value>) -> Result<Buffer> {
    let opts = parse_options::<SignatureOptions>(options, &[])?;
    core_place_signature(pdf.as_ref(), signature.as_ref(), &opts)
        .map(bytes_to_buffer)
        .map_err(err_napi)
}

/// PDF summary as a JSON string: { page_count, version, encrypted,
/// title?, page_sizes: [[w, h], ...] }.
#[napi(js_name = "getPdfInfo")]
pub fn get_pdf_info(pdf: Buffer, options: Option<Value>) -> Result<String> {
    let common = parse_common(options)?;
    let info = core_get_pdf_info(pdf.as_ref(), &common).map_err(err_napi)?;
    serde_json::to_string(&info).map_err(|_| {
        invalid_argument(
            "Could not serialise PDF info.",
            "PDF info serialise nahi ho paya.",
        )
    })
}

/// Extract many selections at once; returns a JSON string of
/// [{ index, pdf?: base64, error_code?, error? }].
#[napi(js_name = "bulkExtract")]
pub fn bulk_extract(
    pdf: Buffer,
    selections: Vec<Vec<u32>>,
    options: Option<Value>,
) -> Result<String> {
    let opts = parse_options::<PageSetOptions>(options, &[])?;
    let items = core_bulk_extract(pdf.as_ref(), &selections, &opts);
    let json: Vec<Value> = items
        .into_iter()
        .map(|item| {
            serde_json::json!({
                "index": item.index,
                "pdf": item.pdf.as_ref().map(|b| base64_encode(b)),
                "error_code": item.error_code,
                "error": item.error,
            })
        })
        .collect();
    Ok(serde_json::to_string(&json).expect("bulk result serialises"))
}

/// Minimal base64 (standard alphabet) — same encoder as the WASM
/// binding, so both backends produce identical `bulk_extract` JSON.
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(CHARS[(n >> 18 & 63) as usize] as char);
        out.push(CHARS[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            CHARS[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            CHARS[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use dusrelpdf::{MergeOptions, RotateOptions};

    #[test]
    fn parse_options_fills_defaults_from_partial_object() {
        // `{}` means "all defaults": angle 90, compress true.
        let opts: RotateOptions = parse_options(Some(serde_json::json!({})), &[]).unwrap();
        assert_eq!(opts, RotateOptions::default());
        // A partial object keeps the given field and fills the rest.
        let opts: RotateOptions =
            parse_options(Some(serde_json::json!({ "angle": -90 })), &[]).unwrap();
        assert_eq!(opts.angle, -90);
        assert!(opts.common.compress);
        assert_eq!(opts.pages, None);
    }

    #[test]
    fn parse_options_none_means_defaults() {
        let opts: MergeOptions = parse_options(None, &[]).unwrap();
        assert_eq!(opts, MergeOptions::default());
    }

    #[test]
    fn parse_options_rejects_non_object() {
        let err = parse_options::<RotateOptions>(Some(serde_json::json!("nope")), &[]).unwrap_err();
        assert!(
            err.reason.contains("INVALID_ARGUMENT"),
            "expected INVALID_ARGUMENT, got: {}",
            err.reason
        );
    }

    #[test]
    fn parse_options_rejects_wrong_type() {
        let err = parse_options::<RotateOptions>(Some(serde_json::json!({ "angle": "90" })), &[])
            .unwrap_err();
        assert!(
            err.reason.contains("INVALID_ARGUMENT"),
            "expected INVALID_ARGUMENT, got: {}",
            err.reason
        );
    }

    #[test]
    fn parse_watermark_options_image_drops_default_text() {
        let opts = parse_watermark_options(Some(serde_json::json!({
            "image": [137, 80, 78, 71],
            "opacity": 0.5,
        })))
        .unwrap();
        assert_eq!(opts.text, None);
        assert_eq!(opts.image, Some(vec![137, 80, 78, 71]));
        assert_eq!(opts.opacity, 0.5);
    }

    #[test]
    fn parse_watermark_options_text_default_is_draft() {
        let opts = parse_watermark_options(Some(serde_json::json!({}))).unwrap();
        assert_eq!(opts.text.as_deref(), Some("DRAFT"));
        assert_eq!(opts.image, None);
    }

    #[test]
    fn error_message_carries_code_prefix() {
        let err = err_napi(DusrelError::invalid_range("9"));
        assert!(err.reason.starts_with("INVALID_RANGE: "), "{}", err.reason);
        assert_eq!(err.status, Status::GenericFailure);
    }

    #[test]
    fn base64_encode_matches_standard_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
