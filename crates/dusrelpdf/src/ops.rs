//! The six document-structure operations: merge, split, extract,
//! remove, reorder, rotate — plus `get_pdf_info` and bulk extract.
//!
//! Only `delete_pages` exists in lopdf; everything else is hand-built
//! from the object graph following the verified flow report
//! (lopdf 0.47.0, README merge example, `examples/rotate.rs`).

use lopdf::{Bookmark, Dictionary, Document, Object, ObjectId};

use crate::common::{
    effective_rotate, load_pdf, load_with_common, materialize_page_attrs, page_size, save_pdf,
};
use crate::error::{DusrelError, Result};
use crate::options::{CommonOptions, MergeOptions, PageSetOptions, RotateOptions};
use crate::range::{parse_page_ranges, parse_page_selection, validate_pages, validate_permutation};
use crate::types::{BulkItem, PdfInfo};

/// Rebuild the `Pages` + `Catalog` of `out` (whose object map is the
/// source's, moved in) around `kids`, then prune unreachable objects,
/// renumber and compress.
fn rebuild_document(out: &mut Document, kids: Vec<ObjectId>) -> Result<()> {
    let count = kids.len() as i64;
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set(
        "Kids",
        Object::Array(kids.iter().map(|id| Object::Reference(*id)).collect()),
    );
    pages_dict.set("Count", Object::Integer(count));
    let pages_id = out.add_object(Object::Dictionary(pages_dict));
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    let catalog_id = out.add_object(Object::Dictionary(catalog_dict));
    out.trailer.set("Root", Object::Reference(catalog_id));
    // Pages must point at the new Pages root, or a stale Parent would
    // reference the pruned source page tree.
    for kid in &kids {
        if let Ok(page) = out.get_object_mut(*kid).and_then(Object::as_dict_mut) {
            page.set("Parent", Object::Reference(pages_id));
        }
    }
    out.prune_objects();
    out.renumber_objects();
    out.adjust_zero_pages();
    Ok(())
}

/// Move `source`'s objects into a fresh document containing only
/// `selected` pages (in the order given). Duplicated page numbers get
/// a cloned page dict so the same page can appear twice.
fn extract_document(mut source: Document, selected: &[u32]) -> Result<Document> {
    for &page in selected {
        let page_id =
            source.get_pages().get(&page).copied().ok_or_else(|| {
                DusrelError::page_out_of_range(page, source.get_pages().len() as u32)
            })?;
        materialize_page_attrs(&mut source, page_id)?;
    }
    let pages_map = source.get_pages();
    let mut kids: Vec<ObjectId> = Vec::with_capacity(selected.len());
    let mut seen: Vec<u32> = Vec::new();
    for &page in selected {
        let page_id = pages_map[&page];
        if seen.contains(&page) {
            // Clone the page dict so the same page can appear twice.
            let cloned = source
                .get_object(page_id)
                .ok()
                .cloned()
                .ok_or_else(|| DusrelError::invalid_pdf("selected page is missing"))?;
            kids.push(source.add_object(cloned));
        } else {
            seen.push(page);
            kids.push(page_id);
        }
    }
    let version = source.version.clone();
    let info = source.trailer.get(b"Info").ok().cloned();
    let mut out = Document::with_version(&version);
    out.max_id = source.max_id;
    out.objects = std::mem::take(&mut source.objects);
    if let Some(info) = info {
        out.trailer.set("Info", info);
    }
    rebuild_document(&mut out, kids)?;
    Ok(out)
}

/// Merge multiple PDFs into one, in input order.
///
/// Honest limits (from the lopdf merge example): the original outline
/// trees are dropped and rebuilt as flat first-page bookmarks; the
/// first input's Info metadata is kept when `preserve_metadata` is
/// set; AcroForm, named destinations and embedded files are not
/// carried over.
pub fn merge_pdfs(inputs: &[Vec<u8>], options: &MergeOptions) -> Result<Vec<u8>> {
    if inputs.is_empty() {
        return Err(DusrelError::empty_input("merge inputs"));
    }
    let mut docs: Vec<Document> = Vec::with_capacity(inputs.len());
    for bytes in inputs {
        docs.push(load_with_common(bytes, &options.common)?);
    }
    for (i, doc) in docs.iter().enumerate() {
        if doc.get_pages().is_empty() {
            return Err(DusrelError::invalid_pdf(&format!(
                "input {} has no pages",
                i + 1
            )));
        }
    }
    let mut out = Document::new();
    let first_version = docs[0].version.clone();
    out.version = first_version;
    let mut next_id: u32 = 1;
    let mut all_pages: Vec<ObjectId> = Vec::new();
    let mut first_catalog_id: Option<ObjectId> = None;
    let mut first_pages_id: Option<ObjectId> = None;
    let mut first_info: Option<Object> = None;
    let mut page_counter: u32 = 0;
    for (idx, doc) in docs.iter_mut().enumerate() {
        // Materialise inherited attributes before renumbering.
        let page_ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();
        for page_id in &page_ids {
            materialize_page_attrs(doc, *page_id)?;
        }
        if idx == 0 {
            first_info = doc.trailer.get(b"Info").ok().cloned();
        }
        doc.renumber_objects_with(next_id);
        next_id = doc.max_id + 1;
        let pages_map = doc.get_pages();
        let first_page_id = pages_map.values().next().copied();
        for page_id in pages_map.values() {
            all_pages.push(*page_id);
        }
        if options.add_bookmarks {
            if let Some(first) = first_page_id {
                let this_first = page_counter + 1;
                out.add_bookmark(
                    Bookmark::new(format!("Page_{this_first}"), [0.0, 0.0, 1.0], 0, first),
                    None,
                );
            }
        }
        page_counter += pages_map.len() as u32;
        // Re-resolve ids after renumbering.
        let catalog_id = doc
            .trailer
            .get(b"Root")
            .and_then(Object::as_reference)
            .map_err(|_| DusrelError::invalid_pdf("input has no catalog"))?;
        let pages_id = doc
            .get_object(catalog_id)
            .and_then(Object::as_dict)
            .and_then(|c| c.get(b"Pages"))
            .and_then(Object::as_reference)
            .map_err(|_| DusrelError::invalid_pdf("input has no Pages root"))?;
        if idx == 0 {
            first_catalog_id = Some(catalog_id);
            first_pages_id = Some(pages_id);
        }
        let objects = std::mem::take(&mut doc.objects);
        for (id, object) in objects {
            if idx > 0 && (id == catalog_id || id == pages_id) {
                continue;
            }
            if let Ok(dict) = object.as_dict() {
                if let Ok(Object::Name(t)) = dict.get(b"Type") {
                    if t == b"Outlines" || t == b"Outline" {
                        continue;
                    }
                }
            }
            out.objects.insert(id, object);
        }
    }
    let catalog_id = first_catalog_id.expect("first catalog");
    let pages_id = first_pages_id.expect("first pages");
    // Every merged page must point at the new Pages root. Pages from
    // inputs 2..n still reference their own (now skipped) page trees;
    // a stale Parent breaks lopdf's resource lookup (get_page_resources
    // walks the Parent chain and errors on the missing node).
    for page_id in &all_pages {
        if let Ok(page) = out.get_object_mut(*page_id).and_then(Object::as_dict_mut) {
            page.set("Parent", Object::Reference(pages_id));
        }
    }
    // Rebuild the Pages root: all pages, correct Count.
    if let Ok(pages) = out.get_object_mut(pages_id).and_then(Object::as_dict_mut) {
        pages.remove(b"Parent");
        pages.remove(b"MediaBox");
        pages.remove(b"CropBox");
        pages.remove(b"Resources");
        pages.remove(b"Rotate");
        pages.set("Count", Object::Integer(all_pages.len() as i64));
        pages.set(
            "Kids",
            Object::Array(all_pages.iter().map(|id| Object::Reference(*id)).collect()),
        );
    }
    if let Ok(catalog) = out.get_object_mut(catalog_id).and_then(Object::as_dict_mut) {
        catalog.set("Pages", Object::Reference(pages_id));
        catalog.remove(b"Outlines");
    }
    out.trailer.set("Root", Object::Reference(catalog_id));
    if options.preserve_metadata {
        if let Some(info) = first_info {
            out.trailer.set("Info", info);
        }
    }
    out.max_id = out.objects.len() as u32;
    out.renumber_objects();
    out.adjust_zero_pages();
    if let Some(outline_id) = out.build_outline() {
        if let Ok(catalog) = out.get_object_mut(catalog_id).and_then(Object::as_dict_mut) {
            catalog.set("Outlines", Object::Reference(outline_id));
        }
    }
    save_pdf(out, options.common.compress)
}

/// Merge from already-loaded byte slices, default options.
pub fn merge_pdfs_default(inputs: &[Vec<u8>]) -> Result<Vec<u8>> {
    merge_pdfs(inputs, &MergeOptions::default())
}

/// Split one PDF into one output per range. `ranges` is a list of
/// page-number lists (1-based).
pub fn split_pdf(
    pdf: &[u8],
    ranges: &[Vec<u32>],
    options: &PageSetOptions,
) -> Result<Vec<Vec<u8>>> {
    if ranges.is_empty() {
        return Err(DusrelError::empty_input("split ranges"));
    }
    let mut outputs = Vec::with_capacity(ranges.len());
    for range in ranges {
        if range.is_empty() {
            return Err(DusrelError::invalid_range("empty range"));
        }
        let source = load_with_common(pdf, &options.common)?;
        let total = source.get_pages().len() as u32;
        validate_pages(range, total)?;
        let doc = extract_document(source, range)?;
        outputs.push(save_pdf(doc, options.common.compress)?);
    }
    Ok(outputs)
}

/// Split from a range string such as `"1-3,4-5"`.
pub fn split_pdf_ranges(
    pdf: &[u8],
    ranges: &str,
    options: &PageSetOptions,
) -> Result<Vec<Vec<u8>>> {
    let source = load_with_common(pdf, &options.common)?;
    let total = source.get_pages().len() as u32;
    let parsed = parse_page_ranges(ranges, total)?;
    split_pdf(pdf, &parsed, options)
}

/// Split into one PDF per page.
pub fn split_each_page(pdf: &[u8], options: &PageSetOptions) -> Result<Vec<Vec<u8>>> {
    let source = load_with_common(pdf, &options.common)?;
    let total = source.get_pages().len() as u32;
    let ranges: Vec<Vec<u32>> = (1..=total).map(|n| vec![n]).collect();
    split_pdf(pdf, &ranges, options)
}

/// Extract selected pages into a new PDF, in the order given.
pub fn extract_pages(pdf: &[u8], pages: &[u32], options: &PageSetOptions) -> Result<Vec<u8>> {
    if pages.is_empty() {
        return Err(DusrelError::empty_input("page selection"));
    }
    let source = load_with_common(pdf, &options.common)?;
    let total = source.get_pages().len() as u32;
    validate_pages(pages, total)?;
    let doc = extract_document(source, pages)?;
    save_pdf(doc, options.common.compress)
}

/// Extract from a range string such as `"1-3,5"`.
pub fn extract_pages_range(pdf: &[u8], range: &str, options: &PageSetOptions) -> Result<Vec<u8>> {
    let source = load_with_common(pdf, &options.common)?;
    let total = source.get_pages().len() as u32;
    let pages = parse_page_selection(range, total)?;
    extract_pages(pdf, &pages, options)
}

/// Delete pages. Out-of-range numbers are an error (unlike lopdf's
/// `delete_pages`, which silently ignores them).
pub fn remove_pages(pdf: &[u8], pages: &[u32], options: &PageSetOptions) -> Result<Vec<u8>> {
    if pages.is_empty() {
        return Ok(pdf.to_vec());
    }
    let mut doc = load_with_common(pdf, &options.common)?;
    let total = doc.get_pages().len() as u32;
    validate_pages(pages, total)?;
    doc.delete_pages(pages);
    doc.prune_objects();
    doc.renumber_objects();
    save_pdf(doc, options.common.compress)
}

/// Change the page order. `order` must list every page exactly once.
pub fn reorder_pages(pdf: &[u8], order: &[u32], options: &PageSetOptions) -> Result<Vec<u8>> {
    let mut doc = load_with_common(pdf, &options.common)?;
    let total = doc.get_pages().len() as u32;
    validate_permutation(order, total)?;
    let pages = doc.get_pages();
    let kids: Vec<Object> = order.iter().map(|n| Object::Reference(pages[n])).collect();
    let catalog = doc.catalog().map_err(DusrelError::from)?;
    let pages_id = catalog
        .get(b"Pages")
        .and_then(Object::as_reference)
        .map_err(|_| DusrelError::invalid_pdf("document has no Pages root"))?;
    let pages_dict = doc
        .get_dictionary_mut(pages_id)
        .map_err(DusrelError::from)?;
    pages_dict.set("Kids", Object::Array(kids));
    save_pdf(doc, options.common.compress)
}

/// Rotate pages by a multiple of 90 degrees. Negative angles are
/// normalised (-90 becomes 270). The rotation is display rotation:
/// the page dict's `/Rotate` entry is updated, content streams are
/// not redrawn. Inherited rotation is honoured — a page inheriting
/// 90° rotated by another 90° ends up at 180°, not 90°.
pub fn rotate_pages(pdf: &[u8], options: &RotateOptions) -> Result<Vec<u8>> {
    if options.angle % 90 != 0 {
        return Err(DusrelError::invalid_angle(options.angle));
    }
    let delta = ((options.angle % 360) + 360) % 360;
    let mut doc = load_with_common(pdf, &options.common)?;
    let total = doc.get_pages().len() as u32;
    let targets: Vec<u32> = match &options.pages {
        Some(pages) => {
            validate_pages(pages, total)?;
            pages.clone()
        }
        None => (1..=total).collect(),
    };
    for page in targets {
        let page_id = doc
            .get_pages()
            .get(&page)
            .copied()
            .ok_or_else(|| DusrelError::page_out_of_range(page, total))?;
        let current = effective_rotate(&doc, page_id);
        let new = (current + delta as i64) % 360;
        let dict = doc
            .get_object_mut(page_id)
            .and_then(Object::as_dict_mut)
            .map_err(|_| DusrelError::invalid_pdf("page object is not a dictionary"))?;
        dict.set("Rotate", Object::Integer(new));
    }
    save_pdf(doc, options.common.compress)
}

/// Read a PDF's summary without modifying it.
pub fn get_pdf_info(pdf: &[u8], common: &CommonOptions) -> Result<PdfInfo> {
    let doc = load_pdf(pdf, common.password.as_deref())?;
    let pages = doc.get_pages();
    let page_count = pages.len() as u32;
    let mut page_sizes = Vec::with_capacity(pages.len());
    for page_id in pages.values() {
        page_sizes.push(page_size(&doc, *page_id));
    }
    let title = doc
        .trailer
        .get(b"Info")
        .and_then(Object::as_reference)
        .ok()
        .and_then(|id| doc.get_object(id).ok())
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"Title").ok())
        .and_then(|o| match o {
            Object::String(bytes, _) => Some(String::from_utf8_lossy(bytes).into_owned()),
            _ => None,
        });
    Ok(PdfInfo {
        page_count,
        version: doc.version.clone(),
        encrypted: doc.is_encrypted(),
        title,
        page_sizes,
    })
}

/// Extract many selections from one PDF in a single call. One bad
/// selection fails only its own item, never the whole batch.
pub fn bulk_extract(
    pdf: &[u8],
    selections: &[Vec<u32>],
    options: &PageSetOptions,
) -> Vec<BulkItem> {
    selections
        .iter()
        .enumerate()
        .map(|(index, pages)| match extract_pages(pdf, pages, options) {
            Ok(out) => BulkItem {
                index,
                pdf: Some(out),
                error_code: None,
                error: None,
            },
            Err(e) => BulkItem {
                index,
                pdf: None,
                error_code: Some(e.code_str().to_string()),
                error: Some(e.reason_en),
            },
        })
        .collect()
}
