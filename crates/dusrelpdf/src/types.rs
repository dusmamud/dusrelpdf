//! Public result / info types.

use serde::{Deserialize, Serialize};

/// Summary of a PDF document, from [`crate::get_pdf_info`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfInfo {
    /// Number of pages.
    pub page_count: u32,
    /// PDF version string, e.g. `"1.4"`.
    pub version: String,
    /// True when the trailer has an `/Encrypt` entry.
    pub encrypted: bool,
    /// Document title from the Info dict, if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Page sizes in points (width, height), in page order.
    pub page_sizes: Vec<(f32, f32)>,
}

/// Result of a bulk operation: one output per input, in input order.
/// Failed items keep their error instead of aborting the whole batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BulkItem {
    /// Input index (0-based).
    pub index: usize,
    /// Output bytes on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf: Option<Vec<u8>>,
    /// Error code on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    /// Human-readable error (English) on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
