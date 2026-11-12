//! Python bindings for dusrelpdf — thin PyO3 wrappers over the Rust core.
//!
//! All PDF logic lives in the `dusrelpdf` crate; this crate only
//! converts bytes / option dicts to and from Python. Build with
//! `maturin develop` / `maturin build` (from this directory).

// clippy 1.99 flags `PyResult<PyObject>` returns from #[pyfunction]s
// as useless conversions — a PyO3 false positive (the conversion is
// what turns the value into a Python object). Allowed crate-wide,
// same as indycheck-py.
#![allow(clippy::useless_conversion)]
//!
//! Error contract: every Rust `DusrelError` is raised as a Python
//! `ValueError` whose message is `CODE\x1f reason_en \x1f reason_hi`.
//! The Python layer (`python/dusrelpdf/__init__.py`) parses that into
//! `dusrelpdf.DusrelError`. Options arrive as a Python dict matching
//! the Rust Options JSON shape (snake_case keys); missing keys fall
//! back to the Rust `Default` for that options struct, so callers can
//! pass a partial dict.

use dusrelpdf::{
    ImagesToPdfOptions, MergeOptions, PageNumberOptions, PageSetOptions, RotateOptions,
    SignatureOptions, WatermarkOptions,
};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

/// Separator for the native error payload. `\x1f` never appears in
/// the Rust reason strings.
const SEP: char = '\x1f';

fn native_err(e: &dusrelpdf::DusrelError) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
        "{}{}{}{}{}",
        e.code_str(),
        SEP,
        e.reason_en,
        SEP,
        e.reason_hi
    ))
}

fn invalid_argument(detail: &str) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
        "INVALID_ARGUMENT{}Invalid argument: {}{}Galat argument hai: {}",
        SEP, detail, SEP, detail
    ))
}

/// Convert a Python value to a JSON value. Bytes become an array of
/// byte values (so a watermark `image` field survives the trip).
fn py_to_json(obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if obj.is_none() {
        return Ok(serde_json::Value::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        // extract::<bool> only succeeds for real bools in PyO3.
        return Ok(serde_json::Value::Bool(b));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(serde_json::Value::Number(i.into()));
    }
    if let Ok(u) = obj.extract::<u64>() {
        return Ok(serde_json::Value::Number(u.into()));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(serde_json::Number::from_f64(f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(serde_json::Value::String(s));
    }
    // bytes / bytearray only (the Python layer normalises memoryview
    // to bytes first, so the native side never sees it).
    if obj.is_instance_of::<PyBytes>() || obj.downcast::<pyo3::types::PyByteArray>().is_ok() {
        let bytes: Vec<u8> = obj.extract()?;
        return Ok(serde_json::Value::Array(
            bytes
                .into_iter()
                .map(|b| serde_json::Value::Number(b.into()))
                .collect(),
        ));
    }
    if let Ok(items) = obj.extract::<Vec<PyObject>>() {
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(py_to_json(item.bind(obj.py()))?);
        }
        return Ok(serde_json::Value::Array(out));
    }
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut out = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            out.insert(key, py_to_json(&v)?);
        }
        return Ok(serde_json::Value::Object(out));
    }
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "cannot convert {} to a JSON value",
        obj.get_type()
    )))
}

/// Parse an options dict onto the Rust defaults: start from
/// `T::default()` serialised to JSON, overlay the caller's dict, then
/// deserialise. Missing keys therefore keep Rust defaults, and an
/// explicit JSON null means `None` for `Option` fields.
fn parse_options<T>(options: Option<&Bound<'_, PyDict>>) -> PyResult<T>
where
    T: Default + serde::Serialize + for<'de> serde::Deserialize<'de>,
{
    let mut value =
        serde_json::to_value(T::default()).map_err(|e| invalid_argument(&e.to_string()))?;
    if let Some(dict) = options {
        let overlay = py_to_json(dict.as_any())?;
        if let (serde_json::Value::Object(base), serde_json::Value::Object(over)) =
            (&mut value, overlay)
        {
            for (k, v) in over {
                base.insert(k, v);
            }
        }
    }
    normalize_page_size(&mut value);
    serde_json::from_value(value).map_err(|e| invalid_argument(&e.to_string()))
}

/// Accept `page_size` as `{"width": w, "height": h}` or `[w, h]` in
/// addition to the serde shape `{"custom": {"width": w, "height": h}}`
/// (the Pydantic `CustomPageSize` model dumps the flat form).
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

fn value_to_py(py: Python<'_>, value: &serde_json::Value) -> PyObject {
    match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(b) => b.to_object(py),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_object(py)
            } else if let Some(u) = n.as_u64() {
                u.to_object(py)
            } else if let Some(f) = n.as_f64() {
                f.to_object(py)
            } else {
                py.None()
            }
        }
        serde_json::Value::String(s) => s.to_object(py),
        serde_json::Value::Array(items) => {
            let list = PyList::empty_bound(py);
            for item in items {
                list.append(value_to_py(py, item)).expect("list append");
            }
            list.into()
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new_bound(py);
            for (k, v) in map {
                dict.set_item(k, value_to_py(py, v)).expect("dict set");
            }
            dict.into()
        }
    }
}

fn pdf_info_to_py(py: Python<'_>, info: &dusrelpdf::PdfInfo) -> PyResult<PyObject> {
    let value = serde_json::to_value(info).map_err(|e| invalid_argument(&e.to_string()))?;
    Ok(value_to_py(py, &value))
}

fn bulk_to_py(py: Python<'_>, items: &[dusrelpdf::BulkItem]) -> PyObject {
    let list = PyList::empty_bound(py);
    for item in items {
        let dict = PyDict::new_bound(py);
        dict.set_item("index", item.index).expect("dict set");
        match &item.pdf {
            Some(bytes) => dict
                .set_item("pdf", PyBytes::new_bound(py, bytes))
                .expect("dict set"),
            None => dict.set_item("pdf", py.None()).expect("dict set"),
        }
        match &item.error_code {
            Some(code) => dict.set_item("error_code", code).expect("dict set"),
            None => dict.set_item("error_code", py.None()).expect("dict set"),
        }
        match &item.error {
            Some(msg) => dict.set_item("error", msg).expect("dict set"),
            None => dict.set_item("error", py.None()).expect("dict set"),
        }
        list.append(dict).expect("list append");
    }
    list.into()
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(signature = (inputs, options=None))]
fn merge_pdfs(
    py: Python<'_>,
    inputs: Vec<Vec<u8>>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: MergeOptions = parse_options(options)?;
    let out = dusrelpdf::merge_pdfs(&inputs, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (pdf, ranges, options=None))]
fn split_pdf(
    py: Python<'_>,
    pdf: Vec<u8>,
    ranges: Vec<Vec<u32>>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageSetOptions = parse_options(options)?;
    let out = dusrelpdf::split_pdf(&pdf, &ranges, &opts).map_err(|e| native_err(&e))?;
    let list = PyList::empty_bound(py);
    for bytes in out {
        list.append(PyBytes::new_bound(py, &bytes))
            .expect("list append");
    }
    Ok(list.into())
}

#[pyfunction]
#[pyo3(signature = (pdf, ranges, options=None))]
fn split_pdf_ranges(
    py: Python<'_>,
    pdf: Vec<u8>,
    ranges: String,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageSetOptions = parse_options(options)?;
    let out = dusrelpdf::split_pdf_ranges(&pdf, &ranges, &opts).map_err(|e| native_err(&e))?;
    let list = PyList::empty_bound(py);
    for bytes in out {
        list.append(PyBytes::new_bound(py, &bytes))
            .expect("list append");
    }
    Ok(list.into())
}

#[pyfunction]
#[pyo3(signature = (pdf, options=None))]
fn split_each_page(
    py: Python<'_>,
    pdf: Vec<u8>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageSetOptions = parse_options(options)?;
    let out = dusrelpdf::split_each_page(&pdf, &opts).map_err(|e| native_err(&e))?;
    let list = PyList::empty_bound(py);
    for bytes in out {
        list.append(PyBytes::new_bound(py, &bytes))
            .expect("list append");
    }
    Ok(list.into())
}

#[pyfunction]
#[pyo3(signature = (pdf, pages, options=None))]
fn extract_pages(
    py: Python<'_>,
    pdf: Vec<u8>,
    pages: Vec<u32>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageSetOptions = parse_options(options)?;
    let out = dusrelpdf::extract_pages(&pdf, &pages, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (pdf, range, options=None))]
fn extract_pages_range(
    py: Python<'_>,
    pdf: Vec<u8>,
    range: String,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageSetOptions = parse_options(options)?;
    let out = dusrelpdf::extract_pages_range(&pdf, &range, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (pdf, pages, options=None))]
fn remove_pages(
    py: Python<'_>,
    pdf: Vec<u8>,
    pages: Vec<u32>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageSetOptions = parse_options(options)?;
    let out = dusrelpdf::remove_pages(&pdf, &pages, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (pdf, order, options=None))]
fn reorder_pages(
    py: Python<'_>,
    pdf: Vec<u8>,
    order: Vec<u32>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageSetOptions = parse_options(options)?;
    let out = dusrelpdf::reorder_pages(&pdf, &order, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (pdf, options=None))]
fn rotate_pages(
    py: Python<'_>,
    pdf: Vec<u8>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: RotateOptions = parse_options(options)?;
    let out = dusrelpdf::rotate_pages(&pdf, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (images, options=None))]
fn images_to_pdf(
    py: Python<'_>,
    images: Vec<Vec<u8>>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: ImagesToPdfOptions = parse_options(options)?;
    let out = dusrelpdf::images_to_pdf(&images, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (pdf, options=None))]
fn add_watermark(
    py: Python<'_>,
    pdf: Vec<u8>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: WatermarkOptions = parse_options(options)?;
    let out = dusrelpdf::add_watermark(&pdf, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (pdf, options=None))]
fn add_page_numbers(
    py: Python<'_>,
    pdf: Vec<u8>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageNumberOptions = parse_options(options)?;
    let out = dusrelpdf::add_page_numbers(&pdf, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (pdf, signature_image, options=None))]
fn place_signature(
    py: Python<'_>,
    pdf: Vec<u8>,
    signature_image: Vec<u8>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: SignatureOptions = parse_options(options)?;
    let out =
        dusrelpdf::place_signature(&pdf, &signature_image, &opts).map_err(|e| native_err(&e))?;
    Ok(PyBytes::new_bound(py, &out).into())
}

#[pyfunction]
#[pyo3(signature = (pdf, options=None))]
fn get_pdf_info(
    py: Python<'_>,
    pdf: Vec<u8>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageSetOptions = parse_options(options)?;
    let info = dusrelpdf::get_pdf_info(&pdf, &opts.common).map_err(|e| native_err(&e))?;
    pdf_info_to_py(py, &info)
}

#[pyfunction]
#[pyo3(signature = (pdf, selections, options=None))]
fn bulk_extract(
    py: Python<'_>,
    pdf: Vec<u8>,
    selections: Vec<Vec<u32>>,
    options: Option<&Bound<'_, PyDict>>,
) -> PyResult<PyObject> {
    let opts: PageSetOptions = parse_options(options)?;
    let items = dusrelpdf::bulk_extract(&pdf, &selections, &opts);
    Ok(bulk_to_py(py, &items))
}

// ---------------------------------------------------------------------------
// Fixtures (test data — same builders the Rust tests use)
// ---------------------------------------------------------------------------

#[pyfunction]
#[pyo3(signature = (pages, title=None))]
fn make_test_pdf(py: Python<'_>, pages: u32, title: Option<String>) -> PyObject {
    let out = dusrelpdf::fixtures::make_test_pdf(pages, title.as_deref());
    PyBytes::new_bound(py, &out).into()
}

#[pyfunction]
fn make_signature_png(py: Python<'_>) -> PyObject {
    let out = dusrelpdf::fixtures::make_signature_png();
    PyBytes::new_bound(py, &out).into()
}

#[pyfunction]
fn make_photo_jpeg(py: Python<'_>) -> PyObject {
    let out = dusrelpdf::fixtures::make_photo_jpeg();
    PyBytes::new_bound(py, &out).into()
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(merge_pdfs, m)?)?;
    m.add_function(wrap_pyfunction!(split_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(split_pdf_ranges, m)?)?;
    m.add_function(wrap_pyfunction!(split_each_page, m)?)?;
    m.add_function(wrap_pyfunction!(extract_pages, m)?)?;
    m.add_function(wrap_pyfunction!(extract_pages_range, m)?)?;
    m.add_function(wrap_pyfunction!(remove_pages, m)?)?;
    m.add_function(wrap_pyfunction!(reorder_pages, m)?)?;
    m.add_function(wrap_pyfunction!(rotate_pages, m)?)?;
    m.add_function(wrap_pyfunction!(images_to_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(add_watermark, m)?)?;
    m.add_function(wrap_pyfunction!(add_page_numbers, m)?)?;
    m.add_function(wrap_pyfunction!(place_signature, m)?)?;
    m.add_function(wrap_pyfunction!(get_pdf_info, m)?)?;
    m.add_function(wrap_pyfunction!(bulk_extract, m)?)?;
    m.add_function(wrap_pyfunction!(make_test_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(make_signature_png, m)?)?;
    m.add_function(wrap_pyfunction!(make_photo_jpeg, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
