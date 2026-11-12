//! dusrelpdf — offline PDF toolkit.
//!
//! Merge, split, extract, remove, reorder, rotate, images-to-PDF,
//! watermark, page numbers and signature-image placement, in one
//! package. Every operation takes bytes in and returns bytes out,
//! never panics on bad input, and reports stable error codes in
//! English and Hindi — the same professional standard as indycheck.
//!
//! Honest limits of v0.1.0:
//! * No PDF rendering (PDF → image), no tolerant repair of malformed
//!   PDFs, and only empty-password decryption on load — password
//!   PDFs need `CommonOptions.password`.
//! * Merging drops the original outline trees and rebuilds flat
//!   first-page bookmarks; AcroForm, named destinations and embedded
//!   files are not carried over.
//! * Watermark / page-number text is ASCII only (base-14 fonts).
//! * Signature placement is a raster **image** on the page, not a
//!   cryptographic digital signature.
//! * Whole documents live in memory; there is no streaming mode.

pub mod common;
pub mod error;
pub mod fixtures;
pub mod ops;
pub mod options;
pub mod range;
pub mod stamp;
pub mod types;

pub use error::{DusrelError, ErrorCode, Result};
pub use ops::{
    bulk_extract, extract_pages, extract_pages_range, get_pdf_info, merge_pdfs, merge_pdfs_default,
    remove_pages, reorder_pages, rotate_pages, split_each_page, split_pdf, split_pdf_ranges,
};
pub use options::{
    BaseFont, CommonOptions, ImageFit, ImagesToPdfOptions, MergeOptions, PageNumberOptions,
    PageSetOptions, PageSize, Position, RotateOptions, SignatureOptions, WatermarkOptions,
};
pub use range::{parse_page_ranges, parse_page_selection};
pub use stamp::{add_page_numbers, add_watermark, images_to_pdf, place_signature};
pub use types::{BulkItem, PdfInfo};

/// Crate version (lockstep with the npm / PyPI packages).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
