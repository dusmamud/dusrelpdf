//! WASM bindings for dusrelpdf — thin wrappers over the Rust core.
//!
//! All PDF logic lives in the `dusrelpdf` crate; this crate only
//! converts between JavaScript values and Rust types. Build with:
//! `wasm-pack build --target web` (website playground) or
//! `wasm-pack build --target nodejs` (the npm package's tests).
//!
//! Contract: binary results are `Uint8Array`s. On failure the
//! function throws a JSON string:
//! `{"ok":false,"code":"PAGE_OUT_OF_RANGE","reason_en":"…","reason_hi":"…"}`.
//! Options are plain JS objects matching the Rust `*Options` structs
//! (see `crates/dusrelpdf/src/options.rs`); missing fields use the
//! Rust defaults, and `undefined`/`null` means "all defaults".

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
use js_sys::{Array, Uint8Array};
use wasm_bindgen::prelude::*;

fn err_js(e: DusrelError) -> JsValue {
    JsValue::from_str(
        &serde_json::json!({
            "ok": false,
            "code": e.code_str(),
            "reason_en": e.reason_en,
            "reason_hi": e.reason_hi,
        })
        .to_string(),
    )
}

fn parse_options<T: Default + serde::de::DeserializeOwned>(value: &JsValue) -> Result<T, JsValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(T::default());
    }
    serde_wasm_bindgen::from_value(value.clone()).map_err(|e| {
        err_js(DusrelError::invalid_argument(
            &format!("Invalid options: {e}"),
            &format!("Galat options hain: {e}"),
        ))
    })
}

fn bytes_array(value: &JsValue) -> Result<Vec<Vec<u8>>, JsValue> {
    let arr = Array::from(value);
    let mut out = Vec::with_capacity(arr.length() as usize);
    for item in arr.iter() {
        out.push(Uint8Array::new(&item).to_vec());
    }
    Ok(out)
}

fn u32_array(value: &JsValue) -> Result<Vec<u32>, JsValue> {
    let arr = Array::from(value);
    let mut out = Vec::with_capacity(arr.length() as usize);
    for item in arr.iter() {
        out.push(item.as_f64().unwrap_or(0.0) as u32);
    }
    Ok(out)
}

fn pdf_array(pdfs: Vec<Vec<u8>>) -> Array {
    let arr = Array::new();
    for pdf in pdfs {
        arr.push(&Uint8Array::from(pdf.as_slice()));
    }
    arr
}

fn options_image_bytes(options: &JsValue) -> Option<Vec<u8>> {
    let map = serde_wasm_bindgen::from_value::<serde_json::Map<String, serde_json::Value>>(
        options.clone(),
    )
    .ok()?;
    let arr = map.get("image")?.as_array()?;
    Some(arr.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect())
}

fn common(options: &JsValue) -> Result<CommonOptions, JsValue> {
    parse_options::<CommonOptions>(options)
}

fn page_set(options: &JsValue) -> Result<PageSetOptions, JsValue> {
    // `CommonOptions` is flattened into PageSetOptions in JSON, so
    // { password, compress } parse directly from the options object.
    parse_options::<PageSetOptions>(options)
}

#[wasm_bindgen]
pub fn version() -> String {
    dusrelpdf::VERSION.to_string()
}

/// Merge PDFs (JS Array of Uint8Array) into one Uint8Array.
#[wasm_bindgen]
pub fn merge_pdfs(inputs: JsValue, options: JsValue) -> Result<Vec<u8>, JsValue> {
    let bufs = bytes_array(&inputs)?;
    let mut opts = parse_options::<MergeOptions>(&options)?;
    opts.common = common(&options)?;
    core_merge_pdfs(&bufs, &opts).map_err(err_js)
}

/// Split into one Uint8Array per range (array of page-number arrays).
#[wasm_bindgen]
pub fn split_pdf(pdf: &[u8], ranges: JsValue, options: JsValue) -> Result<Array, JsValue> {
    let parsed: Vec<Vec<u32>> = ranges
        .dyn_into::<Array>()
        .map_err(|_| {
            err_js(DusrelError::invalid_argument(
                "ranges must be an array of page-number arrays",
                "ranges page-number arrays ki list honi chahiye",
            ))
        })?
        .iter()
        .map(|r| u32_array(&r))
        .collect::<Result<Vec<_>, _>>()?;
    let outs = core_split_pdf(pdf, &parsed, &page_set(&options)?).map_err(err_js)?;
    Ok(pdf_array(outs))
}

/// Split from a range string such as "1-3,4-5".
#[wasm_bindgen]
pub fn split_pdf_ranges(pdf: &[u8], ranges: &str, options: JsValue) -> Result<Array, JsValue> {
    let outs = core_split_pdf_ranges(pdf, ranges, &page_set(&options)?).map_err(err_js)?;
    Ok(pdf_array(outs))
}

/// Split into one PDF per page.
#[wasm_bindgen]
pub fn split_each_page(pdf: &[u8], options: JsValue) -> Result<Array, JsValue> {
    let outs = core_split_each_page(pdf, &page_set(&options)?).map_err(err_js)?;
    Ok(pdf_array(outs))
}

/// Extract pages (JS Array of 1-based page numbers) into one PDF.
#[wasm_bindgen]
pub fn extract_pages(pdf: &[u8], pages: JsValue, options: JsValue) -> Result<Vec<u8>, JsValue> {
    let pages = u32_array(&pages)?;
    core_extract_pages(pdf, &pages, &page_set(&options)?).map_err(err_js)
}

/// Extract from a range string such as "1-3,5".
#[wasm_bindgen]
pub fn extract_pages_range(pdf: &[u8], range: &str, options: JsValue) -> Result<Vec<u8>, JsValue> {
    core_extract_pages_range(pdf, range, &page_set(&options)?).map_err(err_js)
}

/// Delete pages (JS Array of 1-based page numbers).
#[wasm_bindgen]
pub fn remove_pages(pdf: &[u8], pages: JsValue, options: JsValue) -> Result<Vec<u8>, JsValue> {
    let pages = u32_array(&pages)?;
    core_remove_pages(pdf, &pages, &page_set(&options)?).map_err(err_js)
}

/// Reorder pages (JS Array forming a full permutation, 1-based).
#[wasm_bindgen]
pub fn reorder_pages(pdf: &[u8], order: JsValue, options: JsValue) -> Result<Vec<u8>, JsValue> {
    let order = u32_array(&order)?;
    core_reorder_pages(pdf, &order, &page_set(&options)?).map_err(err_js)
}

/// Rotate pages. `options` is a RotateOptions object
/// ({ angle, pages?, password? }); `pages` omitted = all pages.
#[wasm_bindgen]
pub fn rotate_pages(pdf: &[u8], options: JsValue) -> Result<Vec<u8>, JsValue> {
    let mut opts = parse_options::<RotateOptions>(&options)?;
    opts.common = common(&options)?;
    core_rotate_pages(pdf, &opts).map_err(err_js)
}

/// Make a PDF from images (JS Array of Uint8Array, one page each).
#[wasm_bindgen]
pub fn images_to_pdf(images: JsValue, options: JsValue) -> Result<Vec<u8>, JsValue> {
    let bufs = bytes_array(&images)?;
    let opts = parse_options::<ImagesToPdfOptions>(&options)?;
    core_images_to_pdf(&bufs, &opts).map_err(err_js)
}

/// Stamp a text or image watermark.
#[wasm_bindgen]
pub fn add_watermark(pdf: &[u8], options: JsValue) -> Result<Vec<u8>, JsValue> {
    let mut opts = parse_options::<WatermarkOptions>(&options)?;
    opts.common = common(&options)?;
    // `image` arrives as a plain JS number array inside the options
    // object (the TS wrapper converts Uint8Array -> Array.from);
    // serde-wasm-bindgen cannot decode a Uint8Array into Vec<u8>,
    // so pull a number array out by hand when present.
    if opts.image.is_none() {
        opts.image = options_image_bytes(&options);
    }
    core_add_watermark(pdf, &opts).map_err(err_js)
}

/// Stamp page numbers.
#[wasm_bindgen]
pub fn add_page_numbers(pdf: &[u8], options: JsValue) -> Result<Vec<u8>, JsValue> {
    let mut opts = parse_options::<PageNumberOptions>(&options)?;
    opts.common = common(&options)?;
    core_add_page_numbers(pdf, &opts).map_err(err_js)
}

/// Place a signature image on one page.
#[wasm_bindgen]
pub fn place_signature(pdf: &[u8], signature: &[u8], options: JsValue) -> Result<Vec<u8>, JsValue> {
    let mut opts = parse_options::<SignatureOptions>(&options)?;
    opts.common = common(&options)?;
    core_place_signature(pdf, signature, &opts).map_err(err_js)
}

/// PDF summary as a JSON string: { page_count, version, encrypted,
/// title?, page_sizes: [[w, h], ...] }.
#[wasm_bindgen]
pub fn get_pdf_info(pdf: &[u8], options: JsValue) -> Result<String, JsValue> {
    let info = core_get_pdf_info(pdf, &common(&options)?).map_err(err_js)?;
    serde_json::to_string(&info).map_err(|_| {
        err_js(DusrelError::invalid_argument(
            "Could not serialise PDF info.",
            "PDF info serialise nahi ho paya.",
        ))
    })
}

/// Extract many selections at once; returns a JSON string of
/// [{ index, pdf?: base64, error_code?, error? }].
#[wasm_bindgen]
pub fn bulk_extract(pdf: &[u8], selections: JsValue, options: JsValue) -> Result<String, JsValue> {
    use base64_for_wasm::encode as b64;
    let parsed: Vec<Vec<u32>> = selections
        .dyn_into::<Array>()
        .map_err(|_| {
            err_js(DusrelError::invalid_argument(
                "selections must be an array of page-number arrays",
                "selections page-number arrays ki list honi chahiye",
            ))
        })?
        .iter()
        .map(|r| u32_array(&r))
        .collect::<Result<Vec<_>, _>>()?;
    let items = core_bulk_extract(pdf, &parsed, &page_set(&options)?);
    let json: Vec<serde_json::Value> = items
        .into_iter()
        .map(|item| {
            serde_json::json!({
                "index": item.index,
                "pdf": item.pdf.as_ref().map(|b| b64(b)),
                "error_code": item.error_code,
                "error": item.error,
            })
        })
        .collect();
    Ok(serde_json::to_string(&json).unwrap())
}

/// Build a mock multi-page PDF (test data only).
#[wasm_bindgen]
pub fn make_test_pdf(pages: u32) -> Vec<u8> {
    dusrelpdf::fixtures::make_test_pdf(pages.max(1), None)
}

mod base64_for_wasm {
    const CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    pub fn encode(data: &[u8]) -> String {
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
}
