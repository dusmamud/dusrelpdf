//! C ABI for dusrelpdf — thin wrappers over the Rust core.
//!
//! # Ownership contract (this is the whole API's memory model)
//!
//! * **Inputs are borrowed.** Every `*const u8` buffer, `DusrelBytes`
//!   array, and `*const c_char` JSON string is read only for the
//!   duration of the call. The caller keeps owning them; the library
//!   never frees caller memory and never holds a pointer after the
//!   call returns.
//! * **Outputs are caller-owned.** Every operation returns a heap
//!   allocated `*mut DusrelResult` (single output) or
//!   `*mut DusrelResultList` (multi-output: split / split-each-page /
//!   split-ranges). The caller **must** free it exactly once with the
//!   matching free function — [`dusrelpdf_free_result`] or
//!   [`dusrelpdf_free_result_list`] — which frees the result struct
//!   itself, the output byte buffer(s) and the error string. Freeing
//!   with the wrong function, freeing twice, or freeing the inner
//!   buffers directly is undefined behaviour.
//! * **Free functions accept NULL** (no-op), so a defensive free is safe.
//! * **On success** (`ok == 1`): `data` points at the output bytes
//!   (`data_len` long) and `error_json` is NULL. For
//!   [`dusrelpdf_get_pdf_info`] the data is UTF-8 JSON of the
//!   `PdfInfo` struct (`page_count`, `version`, `encrypted`, `title`,
//!   `page_sizes`). For [`dusrelpdf_bulk_extract`] the data is UTF-8
//!   JSON: an array of `{"index": n, "pdf": "<base64>"}` on success or
//!   `{"index": n, "error_code": "…", "error": "…"}` per failed item.
//! * **On failure** (`ok == 0`): `data` is NULL, `data_len` is 0, and
//!   `error_json` is a NUL-terminated JSON string
//!   `{"code": "INVALID_RANGE", "message_en": "…", "message_hi": "…"}`.
//!   The `code` is one of the stable dusrelpdf error codes
//!   (`INVALID_PDF`, `ENCRYPTED_PDF`, `PAGE_OUT_OF_RANGE`,
//!   `EMPTY_INPUT`, `INVALID_RANGE`, `INVALID_ANGLE`, `INVALID_IMAGE`,
//!   `INVALID_ARGUMENT`, `INVALID_TYPE`, `OPERATION_FAILED`).
//! * **Functions never return NULL and never panic across the FFI
//!   boundary.** A NULL/invalid pointer argument, a bad options JSON,
//!   or an internal panic all come back as an `ok == 0` result with
//!   `INVALID_ARGUMENT` / `OPERATION_FAILED`. The only exception is
//!   [`dusrelpdf_version`], which returns a static string that must
//!   NOT be freed.
//! * **Options JSON.** The `options_json` argument is a JSON object
//!   using the same snake_case field names as the Rust core options
//!   structs (see `crates/dusrelpdf/src/options.rs`). It is *partial*:
//!   missing keys keep the Rust defaults, so `{}` or NULL means
//!   "all defaults". Pass NULL or an empty string for defaults.
//!   `page_size` also accepts `{"width": w, "height": h}` or
//!   `[w, h]` in addition to the serde shape
//!   `{"custom": {"width": w, "height": h}}`.
//! * **Watermark image.** [`dusrelpdf_add_watermark`] takes the image
//!   bytes as a separate buffer; when non-NULL they override any
//!   `image` field in the options JSON. A text watermark is set with
//!   `{"text": "DRAFT"}` in the options JSON.
//!
//! # Thread safety
//!
//! All functions are pure (bytes in, bytes out) and safe to call from
//! multiple threads concurrently. No shared mutable state exists.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uchar};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

use dusrelpdf::error::{DusrelError, ErrorCode};
use dusrelpdf::{
    ImagesToPdfOptions, MergeOptions, PageNumberOptions, PageSetOptions, RotateOptions,
    SignatureOptions, WatermarkOptions, add_page_numbers, add_watermark, bulk_extract,
    extract_pages, extract_pages_range, get_pdf_info, images_to_pdf, merge_pdfs, place_signature,
    remove_pages, reorder_pages, rotate_pages, split_each_page, split_pdf, split_pdf_ranges,
};

/// One borrowed byte buffer (an input PDF, an image, …).
#[repr(C)]
pub struct DusrelBytes {
    /// Pointer to the bytes. Borrowed for the call only; may be NULL
    /// only when `len` is 0.
    pub ptr: *const c_uchar,
    /// Number of bytes.
    pub len: usize,
}

/// Result of an operation that produces a single output.
#[repr(C)]
pub struct DusrelResult {
    /// 1 on success, 0 on error.
    pub ok: u8,
    /// Output bytes on success; NULL on error. Caller-owned — free
    /// with `dusrelpdf_free_result`, never directly.
    pub data: *mut c_uchar,
    /// Length of `data` in bytes; 0 on error.
    pub data_len: usize,
    /// NULL on success. On error, a NUL-terminated JSON string
    /// `{"code": "…", "message_en": "…", "message_hi": "…"}`.
    /// Caller-owned — freed by `dusrelpdf_free_result`.
    pub error_json: *mut c_char,
}

/// Result of an operation that produces several outputs (split).
#[repr(C)]
pub struct DusrelResultList {
    /// 1 on success, 0 on error.
    pub ok: u8,
    /// Array of `count` output buffers on success; NULL on error.
    /// Each `items[i].ptr` is caller-owned — freed by
    /// `dusrelpdf_free_result_list`, never directly.
    pub items: *mut DusrelBytes,
    /// Number of items; 0 on error.
    pub count: usize,
    /// NULL on success. On error, a NUL-terminated JSON string
    /// `{"code": "…", "message_en": "…", "message_hi": "…"}`.
    /// Caller-owned — freed by `dusrelpdf_free_result_list`.
    pub error_json: *mut c_char,
}

// ---------------------------------------------------------------------------
// Helpers (not exported)
// ---------------------------------------------------------------------------

fn invalid_argument(detail: &str) -> DusrelError {
    DusrelError::new(
        ErrorCode::InvalidArgument,
        format!("Invalid argument: {detail}"),
        format!("Galat argument hai: {detail}"),
    )
}

fn error_json(e: &DusrelError) -> *mut c_char {
    let json = serde_json::json!({
        "code": e.code_str(),
        "message_en": e.reason_en,
        "message_hi": e.reason_hi,
    });
    CString::new(json.to_string())
        .map(|s| s.into_raw())
        .unwrap_or(ptr::null_mut())
}

fn ok_result(bytes: Vec<u8>) -> *mut DusrelResult {
    // `into_boxed_slice` gives an allocation whose capacity is exactly
    // `len`, so the free side can reconstruct it from (ptr, len) alone.
    let boxed_slice = bytes.into_boxed_slice();
    let len = boxed_slice.len();
    let ptr = Box::into_raw(boxed_slice) as *mut c_uchar;
    Box::into_raw(Box::new(DusrelResult {
        ok: 1,
        data: ptr,
        data_len: len,
        error_json: ptr::null_mut(),
    }))
}

fn err_result(e: &DusrelError) -> *mut DusrelResult {
    Box::into_raw(Box::new(DusrelResult {
        ok: 0,
        data: ptr::null_mut(),
        data_len: 0,
        error_json: error_json(e),
    }))
}

fn ok_result_list(items: Vec<Vec<u8>>) -> *mut DusrelResultList {
    let boxed: Vec<DusrelBytes> = items
        .into_iter()
        .map(|bytes| {
            let slice = bytes.into_boxed_slice();
            let len = slice.len();
            DusrelBytes {
                ptr: Box::into_raw(slice) as *const c_uchar,
                len,
            }
        })
        .collect();
    // Boxed slice again: exact capacity, so the free side can rebuild
    // it from (ptr, count) alone.
    let boxed_slice = boxed.into_boxed_slice();
    let count = boxed_slice.len();
    let ptr = Box::into_raw(boxed_slice) as *mut DusrelBytes;
    Box::into_raw(Box::new(DusrelResultList {
        ok: 1,
        items: ptr,
        count,
        error_json: ptr::null_mut(),
    }))
}

fn err_result_list(e: &DusrelError) -> *mut DusrelResultList {
    Box::into_raw(Box::new(DusrelResultList {
        ok: 0,
        items: ptr::null_mut(),
        count: 0,
        error_json: error_json(e),
    }))
}

/// Read a borrowed `*const u8 + len` pair into a slice. NULL with a
/// non-zero length, or a NULL options pointer where one is required,
/// is an `INVALID_ARGUMENT` error — never undefined behaviour.
fn borrow_bytes<'a>(ptr: *const c_uchar, len: usize) -> Result<&'a [u8], DusrelError> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(invalid_argument("NULL buffer pointer with non-zero length"));
    }
    // SAFETY: ptr is non-NULL and the caller guarantees `len` bytes
    // are readable for the duration of the call (C ABI contract).
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

/// Read a borrowed `*const DusrelBytes + count` array.
fn borrow_bytes_array<'a>(
    ptr: *const DusrelBytes,
    count: usize,
) -> Result<Vec<&'a [u8]>, DusrelError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if ptr.is_null() {
        return Err(invalid_argument(
            "NULL buffer-array pointer with non-zero count",
        ));
    }
    // SAFETY: same contract as `borrow_bytes`.
    let items = unsafe { std::slice::from_raw_parts(ptr, count) };
    let mut out = Vec::with_capacity(count);
    for item in items {
        out.push(borrow_bytes(item.ptr, item.len)?);
    }
    Ok(out)
}

/// Read an optional NUL-terminated C string. NULL or empty = `None`.
fn borrow_opt_str(ptr: *const c_char) -> Result<Option<String>, DusrelError> {
    if ptr.is_null() {
        return Ok(None);
    }
    // SAFETY: caller guarantees a NUL-terminated string (C ABI contract).
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|_| invalid_argument("string argument is not valid UTF-8"))?;
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(s.to_string()))
    }
}

/// Read a required NUL-terminated C string.
fn borrow_str(ptr: *const c_char, what: &str) -> Result<String, DusrelError> {
    match borrow_opt_str(ptr)? {
        Some(s) => Ok(s),
        None => Err(invalid_argument(&format!(
            "{what} must not be NULL or empty"
        ))),
    }
}

/// Parse an options JSON string onto the Rust defaults (missing keys
/// keep defaults; an explicit JSON null means `None` for `Option`
/// fields). NULL / empty JSON = all defaults. Same merge rule as the
/// Python binding (`parse_options` in dusrelpdf-py).
fn parse_options<T>(json: Option<&str>) -> Result<T, DusrelError>
where
    T: Default + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    let mut value =
        serde_json::to_value(T::default()).map_err(|e| invalid_argument(&e.to_string()))?;
    if let Some(text) = json {
        let overlay: serde_json::Value =
            serde_json::from_str(text).map_err(|e| invalid_argument(&e.to_string()))?;
        if let (serde_json::Value::Object(base), serde_json::Value::Object(over)) =
            (&mut value, overlay)
        {
            for (k, v) in over {
                base.insert(k, v);
            }
        } else {
            return Err(invalid_argument("options JSON must be an object"));
        }
    }
    normalize_page_size(&mut value);
    serde_json::from_value(value).map_err(|e| invalid_argument(&e.to_string()))
}

/// Accept `page_size` as `{"width": w, "height": h}` or `[w, h]` in
/// addition to the serde shape `{"custom": {"width": w, "height": h}}`
/// (mirrors dusrelpdf-py).
fn normalize_page_size(value: &mut serde_json::Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    let Some(size) = obj.get("page_size").cloned() else {
        return;
    };
    match &size {
        serde_json::Value::Object(map)
            if map.contains_key("width") && map.contains_key("height") =>
        {
            obj.insert(
                "page_size".to_string(),
                serde_json::json!({ "custom": size }),
            );
        }
        serde_json::Value::Array(items) if items.len() == 2 => {
            obj.insert(
                "page_size".to_string(),
                serde_json::json!({ "custom": { "width": items[0], "height": items[1] } }),
            );
        }
        _ => {}
    }
}

/// Parse a JSON array of page numbers, e.g. `"[1, 3]"`.
fn parse_pages_json(json: &str) -> Result<Vec<u32>, DusrelError> {
    serde_json::from_str::<Vec<u32>>(json)
        .map_err(|e| invalid_argument(&format!("pages JSON is not an array of page numbers: {e}")))
}

fn to_page_set_options(json: Option<&str>) -> Result<PageSetOptions, DusrelError> {
    parse_options::<PageSetOptions>(json)
}

/// Run a fallible core call and turn it into a `*mut DusrelResult`.
/// A Rust panic inside the core is caught and reported as
/// `OPERATION_FAILED` instead of crossing the FFI boundary.
fn run<F>(f: F) -> *mut DusrelResult
where
    F: FnOnce() -> Result<Vec<u8>, DusrelError>,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(bytes)) => ok_result(bytes),
        Ok(Err(e)) => err_result(&e),
        Err(_) => err_result(&DusrelError::new(
            ErrorCode::OperationFailed,
            "Internal error: the PDF operation panicked.",
            "Internal error hai: PDF operation panic ho gaya.",
        )),
    }
}

fn run_list<F>(f: F) -> *mut DusrelResultList
where
    F: FnOnce() -> Result<Vec<Vec<u8>>, DusrelError>,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(items)) => ok_result_list(items),
        Ok(Err(e)) => err_result_list(&e),
        Err(_) => err_result_list(&DusrelError::new(
            ErrorCode::OperationFailed,
            "Internal error: the PDF operation panicked.",
            "Internal error hai: PDF operation panic ho gaya.",
        )),
    }
}

/// Minimal standard-base64 encoder (no new dependency; used only for
/// the per-item PDF payloads in `dusrelpdf_bulk_extract`).
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(TABLE[(n >> 18 & 63) as usize] as char);
        out.push(TABLE[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

// ---------------------------------------------------------------------------
// Exported C API
// ---------------------------------------------------------------------------

/// Library version, e.g. `"0.1.0"`. Static string — do NOT free it.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_version() -> *const c_char {
    const VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    // SAFETY: VERSION is NUL-terminated by construction.
    unsafe { CStr::from_bytes_with_nul_unchecked(VERSION) }.as_ptr()
}

/// Free a `DusrelResult` returned by any single-output operation,
/// including its output bytes and error string. Accepts NULL (no-op).
/// Must be called exactly once per result.
///
/// # Safety
/// `result` must be a pointer previously returned by this library
/// (or NULL), and must not have been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dusrelpdf_free_result(result: *mut DusrelResult) {
    if result.is_null() {
        return;
    }
    // SAFETY: `result` came from `Box::into_raw` in this crate.
    let boxed = unsafe { Box::from_raw(result) };
    if !boxed.data.is_null() && boxed.data_len > 0 {
        // SAFETY: `data` came from `Box::<[u8]>::into_raw` with this len.
        let slice = std::ptr::slice_from_raw_parts_mut(boxed.data, boxed.data_len);
        let _ = unsafe { Box::from_raw(slice) };
    }
    if !boxed.error_json.is_null() {
        // SAFETY: `error_json` came from `CString::into_raw`.
        let _ = unsafe { CString::from_raw(boxed.error_json) };
    }
    // `boxed` drops here, freeing the struct itself.
}

/// Free a `DusrelResultList` returned by any multi-output operation,
/// including every output buffer and the error string. Accepts NULL
/// (no-op). Must be called exactly once per result.
///
/// # Safety
/// `result` must be a pointer previously returned by this library
/// (or NULL), and must not have been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dusrelpdf_free_result_list(result: *mut DusrelResultList) {
    if result.is_null() {
        return;
    }
    // SAFETY: `result` came from `Box::into_raw` in this crate.
    let boxed = unsafe { Box::from_raw(result) };
    if !boxed.items.is_null() && boxed.count > 0 {
        // SAFETY: `items` came from `Box::<[DusrelBytes]>::into_raw`.
        let slice = std::ptr::slice_from_raw_parts_mut(boxed.items, boxed.count);
        let items = unsafe { Box::from_raw(slice) };
        for item in items.iter() {
            if !item.ptr.is_null() && item.len > 0 {
                // SAFETY: each ptr came from `Box::<[u8]>::into_raw`.
                let s = std::ptr::slice_from_raw_parts_mut(item.ptr as *mut c_uchar, item.len);
                let _ = unsafe { Box::from_raw(s) };
            }
        }
    }
    if !boxed.error_json.is_null() {
        // SAFETY: `error_json` came from `CString::into_raw`.
        let _ = unsafe { CString::from_raw(boxed.error_json) };
    }
}

/// Merge several PDFs into one, in input order.
///
/// `inputs` is an array of `count` byte buffers. `options_json` is a
/// partial `MergeOptions` JSON object (NULL/empty = defaults:
/// `preserve_metadata` and `add_bookmarks` both true).
///
/// # Safety
/// `inputs` must point at `count` readable `DusrelBytes`, each
/// pointing at `len` readable bytes (or NULL when `len` is 0).
/// `options_json` must be NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_merge(
    inputs: *const DusrelBytes,
    count: usize,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let borrowed = borrow_bytes_array(inputs, count)?;
        let owned: Vec<Vec<u8>> = borrowed.iter().map(|b| b.to_vec()).collect();
        let options: MergeOptions = parse_options(options_text.as_deref())?;
        merge_pdfs(&owned, &options)
    })
}

/// Split one PDF into one output per range. `ranges_json` is a JSON
/// array of page-number arrays, e.g. `"[[1, 2], [3]]"`.
/// `options_json` is a partial `PageSetOptions` JSON object.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `ranges_json` must
/// be a NUL-terminated string. `options_json` must be NULL or a
/// NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_split(
    pdf: *const c_uchar,
    pdf_len: usize,
    ranges_json: *const c_char,
    options_json: *const c_char,
) -> *mut DusrelResultList {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result_list(&e),
    };
    let ranges_text = match borrow_str(ranges_json, "ranges_json") {
        Ok(t) => t,
        Err(e) => return err_result_list(&e),
    };
    run_list(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let ranges: Vec<Vec<u32>> = serde_json::from_str(&ranges_text).map_err(|e| {
            invalid_argument(&format!("ranges JSON is not an array of page arrays: {e}"))
        })?;
        let options = to_page_set_options(options_text.as_deref())?;
        split_pdf(pdf, &ranges, &options)
    })
}

/// Split one PDF from a range string such as `"1-3,4-5"` — one output
/// per comma-separated chunk. `options_json` is a partial
/// `PageSetOptions` JSON object.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `ranges` and
/// `options_json` must be NULL (options only) or NUL-terminated strings.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_split_ranges(
    pdf: *const c_uchar,
    pdf_len: usize,
    ranges: *const c_char,
    options_json: *const c_char,
) -> *mut DusrelResultList {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result_list(&e),
    };
    let ranges_text = match borrow_str(ranges, "ranges") {
        Ok(t) => t,
        Err(e) => return err_result_list(&e),
    };
    run_list(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let options = to_page_set_options(options_text.as_deref())?;
        split_pdf_ranges(pdf, &ranges_text, &options)
    })
}

/// Split one PDF into one output per page.
/// `options_json` is a partial `PageSetOptions` JSON object.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `options_json` must
/// be NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_split_each_page(
    pdf: *const c_uchar,
    pdf_len: usize,
    options_json: *const c_char,
) -> *mut DusrelResultList {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result_list(&e),
    };
    run_list(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let options = to_page_set_options(options_text.as_deref())?;
        split_each_page(pdf, &options)
    })
}

/// Extract selected pages into a new PDF, in the order given.
/// `pages_json` is a JSON array of 1-based page numbers, e.g.
/// `"[1, 3]"`. `options_json` is a partial `PageSetOptions` JSON object.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `pages_json` must be
/// a NUL-terminated string. `options_json` must be NULL or a
/// NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_extract_pages(
    pdf: *const c_uchar,
    pdf_len: usize,
    pages_json: *const c_char,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    let pages_text = match borrow_str(pages_json, "pages_json") {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let pages = parse_pages_json(&pages_text)?;
        let options = to_page_set_options(options_text.as_deref())?;
        extract_pages(pdf, &pages, &options)
    })
}

/// Extract pages from a range string such as `"1-3,5"`.
/// `options_json` is a partial `PageSetOptions` JSON object.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `range` must be a
/// NUL-terminated string. `options_json` must be NULL or a
/// NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_extract_pages_range(
    pdf: *const c_uchar,
    pdf_len: usize,
    range: *const c_char,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    let range_text = match borrow_str(range, "range") {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let options = to_page_set_options(options_text.as_deref())?;
        extract_pages_range(pdf, &range_text, &options)
    })
}

/// Delete pages (1-based numbers in `pages_json`, e.g. `"[2]"`).
/// `options_json` is a partial `PageSetOptions` JSON object.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `pages_json` must be
/// a NUL-terminated string. `options_json` must be NULL or a
/// NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_remove_pages(
    pdf: *const c_uchar,
    pdf_len: usize,
    pages_json: *const c_char,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    let pages_text = match borrow_str(pages_json, "pages_json") {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let pages = parse_pages_json(&pages_text)?;
        let options = to_page_set_options(options_text.as_deref())?;
        remove_pages(pdf, &pages, &options)
    })
}

/// Change the page order. `order_json` is a JSON array listing every
/// page exactly once, e.g. `"[3, 1, 2]"`. `options_json` is a partial
/// `PageSetOptions` JSON object.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `order_json` must be
/// a NUL-terminated string. `options_json` must be NULL or a
/// NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_reorder_pages(
    pdf: *const c_uchar,
    pdf_len: usize,
    order_json: *const c_char,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    let order_text = match borrow_str(order_json, "order_json") {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let order = parse_pages_json(&order_text)?;
        let options = to_page_set_options(options_text.as_deref())?;
        reorder_pages(pdf, &order, &options)
    })
}

/// Rotate pages by a multiple of 90 degrees. The angle (and the
/// optional `pages` list) come from `options_json`, a partial
/// `RotateOptions` JSON object, e.g. `{"angle": 90}` or
/// `{"angle": -90, "pages": [1]}`. NULL/empty = rotate every page
/// by 90°.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `options_json` must
/// be NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_rotate_pages(
    pdf: *const c_uchar,
    pdf_len: usize,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let options: RotateOptions = parse_options(options_text.as_deref())?;
        rotate_pages(pdf, &options)
    })
}

/// Read a PDF's summary without modifying it. On success `data` is
/// UTF-8 JSON of the `PdfInfo` struct: `{"page_count": 3,
/// "version": "1.7", "encrypted": false, "title": "…",
/// "page_sizes": [[595.0, 842.0], …]}`. `options_json` is a partial
/// `CommonOptions` JSON object (only `password` / `compress` apply).
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `options_json` must
/// be NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_get_pdf_info(
    pdf: *const c_uchar,
    pdf_len: usize,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let common = parse_options::<dusrelpdf::CommonOptions>(options_text.as_deref())?;
        let info = get_pdf_info(pdf, &common)?;
        serde_json::to_vec(&info)
            .map_err(|e| invalid_argument(&format!("could not serialise PdfInfo: {e}")))
    })
}

/// Build a PDF with one page per image, in input order. `images` is
/// an array of `count` image buffers (PNG/JPEG). `options_json` is a
/// partial `ImagesToPdfOptions` JSON object, e.g.
/// `{"page_size": "a4", "fit": "contain"}`.
///
/// # Safety
/// `images` must point at `count` readable `DusrelBytes`, each
/// pointing at `len` readable bytes (or NULL when `len` is 0).
/// `options_json` must be NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_images_to_pdf(
    images: *const DusrelBytes,
    count: usize,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let borrowed = borrow_bytes_array(images, count)?;
        let owned: Vec<Vec<u8>> = borrowed.iter().map(|b| b.to_vec()).collect();
        let options: ImagesToPdfOptions = parse_options(options_text.as_deref())?;
        images_to_pdf(&owned, &options)
    })
}

/// Stamp a text or image watermark over the selected pages.
/// `options_json` is a partial `WatermarkOptions` JSON object, e.g.
/// `{"text": "DRAFT", "opacity": 0.3, "rotation": 45}`. When `image`
/// is non-NULL it is the watermark image bytes (PNG/JPEG): it
/// overrides any `image` field in the options JSON and clears the
/// default `text` (the core needs exactly one of them); pass NULL +
/// a `text` in the options for a text watermark.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `image` must be NULL
/// or point at `image_len` readable bytes. `options_json` must be
/// NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_add_watermark(
    pdf: *const c_uchar,
    pdf_len: usize,
    image: *const c_uchar,
    image_len: usize,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let image_bytes = borrow_bytes(image, image_len)?;
        let mut options: WatermarkOptions = parse_options(options_text.as_deref())?;
        if !image_bytes.is_empty() {
            // An image buffer wins over any `text` in the options JSON
            // (the Rust default is `text: "DRAFT"`, which would
            // otherwise make the core reject the call as "both set").
            options.image = Some(image_bytes.to_vec());
            options.text = None;
        }
        add_watermark(pdf, &options)
    })
}

/// Stamp page numbers over the selected pages. `options_json` is a
/// partial `PageNumberOptions` JSON object, e.g.
/// `{"format": "Page {n} of {total}", "position": "bottom_center"}`.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `options_json` must
/// be NULL or a NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_add_page_numbers(
    pdf: *const c_uchar,
    pdf_len: usize,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let options: PageNumberOptions = parse_options(options_text.as_deref())?;
        add_page_numbers(pdf, &options)
    })
}

/// Place a handwritten signature IMAGE on one page (image placement
/// only — not a cryptographic digital signature). `image` is the
/// signature image bytes (transparent PNG recommended).
/// `options_json` is a partial `SignatureOptions` JSON object, e.g.
/// `{"page": 1, "x": 36.0, "y": 36.0, "width": 144.0}`.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `image` must point
/// at `image_len` readable bytes. `options_json` must be NULL or a
/// NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_place_signature(
    pdf: *const c_uchar,
    pdf_len: usize,
    image: *const c_uchar,
    image_len: usize,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let image_bytes = borrow_bytes(image, image_len)?;
        if image_bytes.is_empty() {
            return Err(invalid_argument("place_signature needs a signature image"));
        }
        let options: SignatureOptions = parse_options(options_text.as_deref())?;
        place_signature(pdf, image_bytes, &options)
    })
}

/// Extract many selections from one PDF in a single call. One bad
/// selection fails only its own item, never the whole batch.
/// `selections_json` is a JSON array of page-number arrays, e.g.
/// `"[[1], [2, 3], [9]]"`. On success `data` is UTF-8 JSON: an array
/// of `{"index": n, "pdf": "<base64>"}` per success or
/// `{"index": n, "error_code": "…", "error": "…"}` per failure, in
/// input order. `options_json` is a partial `PageSetOptions` JSON object.
///
/// # Safety
/// `pdf` must point at `pdf_len` readable bytes. `selections_json`
/// must be a NUL-terminated string. `options_json` must be NULL or a
/// NUL-terminated string.
#[unsafe(no_mangle)]
pub extern "C" fn dusrelpdf_bulk_extract(
    pdf: *const c_uchar,
    pdf_len: usize,
    selections_json: *const c_char,
    options_json: *const c_char,
) -> *mut DusrelResult {
    let options_text = match borrow_opt_str(options_json) {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    let selections_text = match borrow_str(selections_json, "selections_json") {
        Ok(t) => t,
        Err(e) => return err_result(&e),
    };
    run(|| {
        let pdf = borrow_bytes(pdf, pdf_len)?;
        let selections: Vec<Vec<u32>> = serde_json::from_str(&selections_text).map_err(|e| {
            invalid_argument(&format!(
                "selections JSON is not an array of page arrays: {e}"
            ))
        })?;
        let options = to_page_set_options(options_text.as_deref())?;
        let items = bulk_extract(pdf, &selections, &options);
        let json_items: Vec<serde_json::Value> = items
            .into_iter()
            .map(|item| {
                if let Some(bytes) = item.pdf {
                    serde_json::json!({ "index": item.index, "pdf": base64_encode(&bytes) })
                } else {
                    serde_json::json!({
                        "index": item.index,
                        "error_code": item.error_code,
                        "error": item.error,
                    })
                }
            })
            .collect();
        serde_json::to_vec(&json_items)
            .map_err(|e| invalid_argument(&format!("could not serialise bulk results: {e}")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dusrelpdf::fixtures::{make_photo_jpeg, make_signature_png, make_test_pdf};

    /// Call an FFI function the way a C caller would: build the raw
    /// pointers, read the result, then free it with the matching
    /// free function. The ASan-style checks here are the ownership
    /// contract itself — every test frees exactly once.
    fn call_result(f: impl FnOnce() -> *mut DusrelResult) -> (u8, Vec<u8>, Option<String>) {
        let r = f();
        assert!(!r.is_null(), "FFI must never return NULL");
        let boxed = unsafe { &*r };
        let ok = boxed.ok;
        let data = if boxed.data.is_null() {
            Vec::new()
        } else {
            // SAFETY: data is valid until freed below.
            unsafe { std::slice::from_raw_parts(boxed.data, boxed.data_len) }.to_vec()
        };
        let err = if boxed.error_json.is_null() {
            None
        } else {
            // SAFETY: error_json is a valid NUL-terminated string.
            Some(
                unsafe { CStr::from_ptr(boxed.error_json) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        unsafe { dusrelpdf_free_result(r) };
        (ok, data, err)
    }

    fn call_list(f: impl FnOnce() -> *mut DusrelResultList) -> (u8, Vec<Vec<u8>>, Option<String>) {
        let r = f();
        assert!(!r.is_null(), "FFI must never return NULL");
        let boxed = unsafe { &*r };
        let ok = boxed.ok;
        let items = if boxed.items.is_null() {
            Vec::new()
        } else {
            // SAFETY: items are valid until freed below.
            let slice = unsafe { std::slice::from_raw_parts(boxed.items, boxed.count) };
            slice
                .iter()
                .map(|b| {
                    if b.ptr.is_null() {
                        Vec::new()
                    } else {
                        // SAFETY: each item is valid until freed below.
                        unsafe { std::slice::from_raw_parts(b.ptr, b.len) }.to_vec()
                    }
                })
                .collect()
        };
        let err = if boxed.error_json.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(boxed.error_json) }
                    .to_string_lossy()
                    .into_owned(),
            )
        };
        unsafe { dusrelpdf_free_result_list(r) };
        (ok, items, err)
    }

    fn bytes_of(pdf: &[u8]) -> DusrelBytes {
        DusrelBytes {
            ptr: pdf.as_ptr(),
            len: pdf.len(),
        }
    }

    fn cstr(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    fn info_of(pdf: &[u8]) -> serde_json::Value {
        let (ok, data, _) =
            call_result(|| dusrelpdf_get_pdf_info(pdf.as_ptr(), pdf.len(), ptr::null()));
        assert_eq!(ok, 1);
        serde_json::from_slice(&data).unwrap()
    }

    #[test]
    fn version_is_static_and_not_freed() {
        let v = dusrelpdf_version();
        assert!(!v.is_null());
        // SAFETY: version() returns a static NUL-terminated string.
        assert_eq!(unsafe { CStr::from_ptr(v) }.to_str().unwrap(), "0.1.0");
    }

    #[test]
    fn merge_two_pdfs() {
        let a = make_test_pdf(3, Some("A"));
        let b = make_test_pdf(2, Some("B"));
        let inputs = [bytes_of(&a), bytes_of(&b)];
        let (ok, out, err) =
            call_result(|| dusrelpdf_merge(inputs.as_ptr(), inputs.len(), ptr::null()));
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 5);
    }

    #[test]
    fn merge_with_partial_options() {
        let a = make_test_pdf(1, None);
        let inputs = [bytes_of(&a)];
        let opts = cstr(r#"{"add_bookmarks": false}"#);
        let (ok, out, err) =
            call_result(|| dusrelpdf_merge(inputs.as_ptr(), inputs.len(), opts.as_ptr()));
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 1);
    }

    #[test]
    fn merge_empty_input_is_error() {
        let (ok, _, err) = call_result(|| dusrelpdf_merge(ptr::null(), 0, ptr::null()));
        assert_eq!(ok, 0);
        let err = err.unwrap();
        assert!(err.contains("EMPTY_INPUT"), "{err}");
        assert!(err.contains("message_hi"), "{err}");
    }

    #[test]
    fn split_returns_one_pdf_per_range() {
        let pdf = make_test_pdf(5, None);
        let ranges = cstr("[[1, 2], [3, 4, 5]]");
        let (ok, outs, err) =
            call_list(|| dusrelpdf_split(pdf.as_ptr(), pdf.len(), ranges.as_ptr(), ptr::null()));
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(outs.len(), 2);
        assert_eq!(info_of(&outs[0])["page_count"], 2);
        assert_eq!(info_of(&outs[1])["page_count"], 3);
    }

    #[test]
    fn split_ranges_string() {
        let pdf = make_test_pdf(4, None);
        let ranges = cstr("1-2,3-4");
        let (ok, outs, err) = call_list(|| {
            dusrelpdf_split_ranges(pdf.as_ptr(), pdf.len(), ranges.as_ptr(), ptr::null())
        });
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(outs.len(), 2);
        assert_eq!(info_of(&outs[0])["page_count"], 2);
    }

    #[test]
    fn split_each_page() {
        let pdf = make_test_pdf(3, None);
        let (ok, outs, err) =
            call_list(|| dusrelpdf_split_each_page(pdf.as_ptr(), pdf.len(), ptr::null()));
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(outs.len(), 3);
        for out in &outs {
            assert_eq!(info_of(out)["page_count"], 1);
        }
    }

    #[test]
    fn extract_pages_and_range() {
        let pdf = make_test_pdf(5, None);
        let pages = cstr("[2, 4]");
        let (ok, out, err) = call_result(|| {
            dusrelpdf_extract_pages(pdf.as_ptr(), pdf.len(), pages.as_ptr(), ptr::null())
        });
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 2);

        let range = cstr("1-3,5");
        let (ok, out, err) = call_result(|| {
            dusrelpdf_extract_pages_range(pdf.as_ptr(), pdf.len(), range.as_ptr(), ptr::null())
        });
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 4);
    }

    #[test]
    fn extract_out_of_range_is_page_out_of_range() {
        let pdf = make_test_pdf(3, None);
        let pages = cstr("[9]");
        let (ok, _, err) = call_result(|| {
            dusrelpdf_extract_pages(pdf.as_ptr(), pdf.len(), pages.as_ptr(), ptr::null())
        });
        assert_eq!(ok, 0);
        assert!(err.unwrap().contains("PAGE_OUT_OF_RANGE"));
    }

    #[test]
    fn extract_bad_range_string_is_invalid_range() {
        let pdf = make_test_pdf(3, None);
        let range = cstr("9");
        let (ok, _, err) = call_result(|| {
            dusrelpdf_extract_pages_range(pdf.as_ptr(), pdf.len(), range.as_ptr(), ptr::null())
        });
        assert_eq!(ok, 0);
        assert!(err.unwrap().contains("INVALID_RANGE"));
    }

    #[test]
    fn remove_and_reorder() {
        let pdf = make_test_pdf(4, None);
        let pages = cstr("[2]");
        let (ok, out, err) = call_result(|| {
            dusrelpdf_remove_pages(pdf.as_ptr(), pdf.len(), pages.as_ptr(), ptr::null())
        });
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 3);

        let order = cstr("[4, 3, 2, 1]");
        let (ok, out, err) = call_result(|| {
            dusrelpdf_reorder_pages(pdf.as_ptr(), pdf.len(), order.as_ptr(), ptr::null())
        });
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 4);
    }

    #[test]
    fn rotate_with_angle_options() {
        let pdf = make_test_pdf(2, None);
        let opts = cstr(r#"{"angle": 180}"#);
        let (ok, out, err) =
            call_result(|| dusrelpdf_rotate_pages(pdf.as_ptr(), pdf.len(), opts.as_ptr()));
        assert_eq!(ok, 1, "{err:?}");
        assert!(out.windows(11).any(|w| w == b"/Rotate 180"));

        let bad = cstr(r#"{"angle": 45}"#);
        let (ok, _, err) =
            call_result(|| dusrelpdf_rotate_pages(pdf.as_ptr(), pdf.len(), bad.as_ptr()));
        assert_eq!(ok, 0);
        assert!(err.unwrap().contains("INVALID_ANGLE"));
    }

    #[test]
    fn get_pdf_info_json() {
        let pdf = make_test_pdf(3, Some("Hello"));
        let (ok, data, err) =
            call_result(|| dusrelpdf_get_pdf_info(pdf.as_ptr(), pdf.len(), ptr::null()));
        assert_eq!(ok, 1, "{err:?}");
        let info: serde_json::Value = serde_json::from_slice(&data).unwrap();
        assert_eq!(info["page_count"], 3);
        assert_eq!(info["version"], "1.7");
        assert_eq!(info["encrypted"], false);
        assert_eq!(info["title"], "Hello");
        assert_eq!(info["page_sizes"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn get_pdf_info_bad_input() {
        let bad = b"not a pdf at all";
        let (ok, _, err) =
            call_result(|| dusrelpdf_get_pdf_info(bad.as_ptr(), bad.len(), ptr::null()));
        assert_eq!(ok, 0);
        assert!(err.unwrap().contains("INVALID_PDF"));
    }

    #[test]
    fn images_to_pdf_and_watermark_and_numbers_and_signature() {
        let jpeg = make_photo_jpeg();
        let images = [bytes_of(&jpeg)];
        let (ok, out, err) =
            call_result(|| dusrelpdf_images_to_pdf(images.as_ptr(), images.len(), ptr::null()));
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 1);

        let pdf = make_test_pdf(2, None);
        let opts = cstr(r#"{"text": "DRAFT", "opacity": 0.3, "rotation": 45.0}"#);
        let (ok, out, err) = call_result(|| {
            dusrelpdf_add_watermark(pdf.as_ptr(), pdf.len(), ptr::null(), 0, opts.as_ptr())
        });
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 2);

        let opts = cstr(r#"{"format": "Page {n} of {total}"}"#);
        let (ok, out, err) =
            call_result(|| dusrelpdf_add_page_numbers(pdf.as_ptr(), pdf.len(), opts.as_ptr()));
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 2);

        let sig = make_signature_png();
        let opts = cstr(r#"{"page": 1, "x": 36.0, "y": 36.0}"#);
        let (ok, out, err) = call_result(|| {
            dusrelpdf_place_signature(
                pdf.as_ptr(),
                pdf.len(),
                sig.as_ptr(),
                sig.len(),
                opts.as_ptr(),
            )
        });
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 2);
    }

    #[test]
    fn watermark_image_via_buffer_overrides_json() {
        let pdf = make_test_pdf(1, None);
        let png = make_signature_png();
        let opts = cstr(r#"{"opacity": 0.5, "rotation": 0.0}"#);
        let (ok, out, err) = call_result(|| {
            dusrelpdf_add_watermark(
                pdf.as_ptr(),
                pdf.len(),
                png.as_ptr(),
                png.len(),
                opts.as_ptr(),
            )
        });
        assert_eq!(ok, 1, "{err:?}");
        assert_eq!(info_of(&out)["page_count"], 1);
    }

    #[test]
    fn bulk_extract_keeps_per_item_errors() {
        let pdf = make_test_pdf(3, None);
        let sel = cstr("[[1], [2, 3], [9]]");
        let (ok, data, err) = call_result(|| {
            dusrelpdf_bulk_extract(pdf.as_ptr(), pdf.len(), sel.as_ptr(), ptr::null())
        });
        assert_eq!(ok, 1, "{err:?}");
        let items: serde_json::Value = serde_json::from_slice(&data).unwrap();
        let items = items.as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert!(items[0]["pdf"].as_str().unwrap().starts_with("JVBERi0"));
        assert!(items[1]["pdf"].as_str().is_some());
        assert_eq!(items[2]["error_code"], "PAGE_OUT_OF_RANGE");
    }

    #[test]
    fn null_pointers_never_panic() {
        // NULL pdf pointer with non-zero length.
        let (ok, _, err) = call_result(|| dusrelpdf_get_pdf_info(ptr::null(), 4, ptr::null()));
        assert_eq!(ok, 0);
        assert!(err.unwrap().contains("INVALID_ARGUMENT"));

        // NULL ranges string.
        let pdf = make_test_pdf(2, None);
        let (ok, _, err) =
            call_list(|| dusrelpdf_split(pdf.as_ptr(), pdf.len(), ptr::null(), ptr::null()));
        assert_eq!(ok, 0);
        assert!(err.unwrap().contains("INVALID_ARGUMENT"));

        // Bad options JSON.
        let (ok, _, err) = call_result(|| {
            let bad = cstr("{not json");
            dusrelpdf_rotate_pages(pdf.as_ptr(), pdf.len(), bad.as_ptr())
        });
        assert_eq!(ok, 0);
        assert!(err.unwrap().contains("INVALID_ARGUMENT"));

        // Freeing NULL is a safe no-op.
        unsafe {
            dusrelpdf_free_result(ptr::null_mut());
            dusrelpdf_free_result_list(ptr::null_mut());
        }
    }
}
