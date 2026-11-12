//! UniFFI bindings (Kotlin/Swift) for dusrelpdf.
//!
//! Thin wrappers over the core crate — no PDF logic lives here.
//! Every exported function calls the matching core operation and
//! returns bytes in / bytes out, exactly like the core API. Errors
//! are a [`DusrelError`] enum: one variant per stable core error
//! code, each carrying the English and Hindi messages.
//!
//! The generated Kotlin and Swift sources are checked in under
//! `bindings/` at the repo root — see the crate README for how to
//! regenerate them.
//!
//! Honest limits (same as the core): no PDF rendering, no tolerant
//! repair of malformed PDFs, watermark / page-number text is ASCII
//! only, and signature placement is a raster image, not a
//! cryptographic digital signature.

use dusrelpdf::ErrorCode as CoreCode;

// ---------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------

/// One variant per stable core error code. Kotlin receives this as a
/// sealed class (`DusrelError.InvalidRange`, …), Swift as an enum.
/// `code` is the stable machine-readable code (`"INVALID_RANGE"`,
/// …); `reason_en` / `reason_hi` are human-readable explanations.
#[derive(Debug, Clone, thiserror::Error, uniffi::Error)]
pub enum DusrelError {
    #[error("{code}: {reason_en}")]
    InvalidPdf {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
    #[error("{code}: {reason_en}")]
    EncryptedPdf {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
    #[error("{code}: {reason_en}")]
    PageOutOfRange {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
    #[error("{code}: {reason_en}")]
    EmptyInput {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
    #[error("{code}: {reason_en}")]
    InvalidRange {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
    #[error("{code}: {reason_en}")]
    InvalidAngle {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
    #[error("{code}: {reason_en}")]
    InvalidImage {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
    #[error("{code}: {reason_en}")]
    InvalidArgument {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
    #[error("{code}: {reason_en}")]
    InvalidType {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
    #[error("{code}: {reason_en}")]
    OperationFailed {
        code: String,
        reason_en: String,
        reason_hi: String,
    },
}

impl DusrelError {
    fn from_core(err: dusrelpdf::DusrelError) -> Self {
        let code = err.code_str().to_string();
        let reason_en = err.reason_en;
        let reason_hi = err.reason_hi;
        match err.code {
            CoreCode::InvalidPdf => Self::InvalidPdf {
                code,
                reason_en,
                reason_hi,
            },
            CoreCode::EncryptedPdf => Self::EncryptedPdf {
                code,
                reason_en,
                reason_hi,
            },
            CoreCode::PageOutOfRange => Self::PageOutOfRange {
                code,
                reason_en,
                reason_hi,
            },
            CoreCode::EmptyInput => Self::EmptyInput {
                code,
                reason_en,
                reason_hi,
            },
            CoreCode::InvalidRange => Self::InvalidRange {
                code,
                reason_en,
                reason_hi,
            },
            CoreCode::InvalidAngle => Self::InvalidAngle {
                code,
                reason_en,
                reason_hi,
            },
            CoreCode::InvalidImage => Self::InvalidImage {
                code,
                reason_en,
                reason_hi,
            },
            CoreCode::InvalidArgument => Self::InvalidArgument {
                code,
                reason_en,
                reason_hi,
            },
            CoreCode::InvalidType => Self::InvalidType {
                code,
                reason_en,
                reason_hi,
            },
            CoreCode::OperationFailed => Self::OperationFailed {
                code,
                reason_en,
                reason_hi,
            },
        }
    }

    /// The stable error code, e.g. `"INVALID_RANGE"`.
    pub fn code(&self) -> &str {
        match self {
            Self::InvalidPdf { code, .. }
            | Self::EncryptedPdf { code, .. }
            | Self::PageOutOfRange { code, .. }
            | Self::EmptyInput { code, .. }
            | Self::InvalidRange { code, .. }
            | Self::InvalidAngle { code, .. }
            | Self::InvalidImage { code, .. }
            | Self::InvalidArgument { code, .. }
            | Self::InvalidType { code, .. }
            | Self::OperationFailed { code, .. } => code,
        }
    }

    /// Human-readable reason in English.
    pub fn reason_en(&self) -> &str {
        match self {
            Self::InvalidPdf { reason_en, .. }
            | Self::EncryptedPdf { reason_en, .. }
            | Self::PageOutOfRange { reason_en, .. }
            | Self::EmptyInput { reason_en, .. }
            | Self::InvalidRange { reason_en, .. }
            | Self::InvalidAngle { reason_en, .. }
            | Self::InvalidImage { reason_en, .. }
            | Self::InvalidArgument { reason_en, .. }
            | Self::InvalidType { reason_en, .. }
            | Self::OperationFailed { reason_en, .. } => reason_en,
        }
    }
}

impl From<dusrelpdf::DusrelError> for DusrelError {
    fn from(err: dusrelpdf::DusrelError) -> Self {
        Self::from_core(err)
    }
}

/// Convenience result type for the exported functions.
pub type Result<T> = std::result::Result<T, DusrelError>;

// ---------------------------------------------------------------------
// Shared option types
// ---------------------------------------------------------------------

/// Options shared by every operation that reads a PDF.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct CommonOptions {
    /// Password for an encrypted PDF. Empty-password PDFs open without it.
    #[uniffi(default = None)]
    pub password: Option<String>,
    /// Compress output streams (default true). Set false for faster saves.
    #[uniffi(default = true)]
    pub compress: bool,
}

impl Default for CommonOptions {
    fn default() -> Self {
        Self {
            password: None,
            compress: true,
        }
    }
}

impl From<CommonOptions> for dusrelpdf::CommonOptions {
    fn from(value: CommonOptions) -> Self {
        Self {
            password: value.password,
            compress: value.compress,
        }
    }
}

/// Where on the page a stamp (watermark / page number) goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Enum)]
pub enum Position {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
    #[default]
    Center,
}

impl From<Position> for dusrelpdf::Position {
    fn from(value: Position) -> Self {
        match value {
            Position::TopLeft => Self::TopLeft,
            Position::TopCenter => Self::TopCenter,
            Position::TopRight => Self::TopRight,
            Position::BottomLeft => Self::BottomLeft,
            Position::BottomCenter => Self::BottomCenter,
            Position::BottomRight => Self::BottomRight,
            Position::Center => Self::Center,
        }
    }
}

/// A base-14 (built-in) PDF font. No embedding needed, so these always
/// work; only WinAnsi (Latin) text is supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Enum)]
pub enum BaseFont {
    #[default]
    Helvetica,
    HelveticaBold,
    TimesRoman,
    Courier,
    CourierBold,
}

impl From<BaseFont> for dusrelpdf::BaseFont {
    fn from(value: BaseFont) -> Self {
        match value {
            BaseFont::Helvetica => Self::Helvetica,
            BaseFont::HelveticaBold => Self::HelveticaBold,
            BaseFont::TimesRoman => Self::TimesRoman,
            BaseFont::Courier => Self::Courier,
            BaseFont::CourierBold => Self::CourierBold,
        }
    }
}

/// Page size for [`images_to_pdf`].
#[derive(Debug, Clone, Copy, PartialEq, Default, uniffi::Enum)]
pub enum PageSize {
    /// One page per image, sized to the image (1 pixel = 1 point).
    Auto,
    /// A4 (595 × 842 pt), image fitted inside.
    #[default]
    A4,
    /// US Letter (612 × 792 pt), image fitted inside.
    Letter,
    /// Custom size in points.
    Custom { width: f32, height: f32 },
}

impl From<PageSize> for dusrelpdf::PageSize {
    fn from(value: PageSize) -> Self {
        match value {
            PageSize::Auto => Self::Auto,
            PageSize::A4 => Self::A4,
            PageSize::Letter => Self::Letter,
            PageSize::Custom { width, height } => Self::Custom { width, height },
        }
    }
}

/// How an image is fitted into its page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, uniffi::Enum)]
pub enum ImageFit {
    /// Scale to fit inside the page, preserving aspect ratio (default).
    #[default]
    Contain,
    /// Stretch to fill the whole page (aspect ratio not preserved).
    Fill,
}

impl From<ImageFit> for dusrelpdf::ImageFit {
    fn from(value: ImageFit) -> Self {
        match value {
            ImageFit::Contain => Self::Contain,
            ImageFit::Fill => Self::Fill,
        }
    }
}

// ---------------------------------------------------------------------
// Per-operation option records
// ---------------------------------------------------------------------

/// Options for [`merge_pdfs`].
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MergeOptions {
    pub common: CommonOptions,
    /// Copy the first input's document info (title/author) into the
    /// output. Default true.
    #[uniffi(default = true)]
    pub preserve_metadata: bool,
    /// Add a flat outline bookmark for the first page of each input.
    /// Default true.
    #[uniffi(default = true)]
    pub add_bookmarks: bool,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions::default(),
            preserve_metadata: true,
            add_bookmarks: true,
        }
    }
}

impl From<MergeOptions> for dusrelpdf::MergeOptions {
    fn from(value: MergeOptions) -> Self {
        Self {
            common: value.common.into(),
            preserve_metadata: value.preserve_metadata,
            add_bookmarks: value.add_bookmarks,
        }
    }
}

/// Options for split / extract / remove.
#[derive(Debug, Clone, PartialEq, Default, uniffi::Record)]
pub struct PageSetOptions {
    pub common: CommonOptions,
}

impl From<PageSetOptions> for dusrelpdf::PageSetOptions {
    fn from(value: PageSetOptions) -> Self {
        Self {
            common: value.common.into(),
        }
    }
}

/// Options for [`rotate_pages`].
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct RotateOptions {
    pub common: CommonOptions,
    /// Rotation in degrees; must be a multiple of 90 (default 90).
    /// Negative angles are normalised (e.g. -90 becomes 270).
    #[uniffi(default = 90)]
    pub angle: i32,
    /// Pages to rotate, 1-based. `None` = every page.
    #[uniffi(default = None)]
    pub pages: Option<Vec<u32>>,
}

impl Default for RotateOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions::default(),
            angle: 90,
            pages: None,
        }
    }
}

impl From<RotateOptions> for dusrelpdf::RotateOptions {
    fn from(value: RotateOptions) -> Self {
        Self {
            common: value.common.into(),
            angle: value.angle,
            pages: value.pages,
        }
    }
}

/// Options for [`add_watermark`].
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct WatermarkOptions {
    pub common: CommonOptions,
    /// Watermark text. Mutually exclusive with `image` — set one.
    /// Default `"DRAFT"`.
    #[uniffi(default = Some("DRAFT"))]
    pub text: Option<String>,
    /// Watermark image bytes (PNG/JPEG). Mutually exclusive with `text`.
    #[uniffi(default = None)]
    pub image: Option<Vec<u8>>,
    /// Opacity 0.0–1.0 (default 0.3).
    #[uniffi(default = 0.3)]
    pub opacity: f32,
    /// Rotation in degrees, clockwise (default 45). Pass 0 explicitly
    /// for image watermarks that should stay upright.
    #[uniffi(default = 45.0)]
    pub rotation: f32,
    /// Font size in points (text only, default 48).
    #[uniffi(default = 48.0)]
    pub font_size: f32,
    /// Font (text only, default Helvetica).
    pub font: BaseFont,
    /// Where to place it (default Center).
    pub position: Position,
    /// Custom position in points from the bottom-left corner.
    /// Used only when `position` is not enough — set `x`/`y` directly.
    #[uniffi(default = None)]
    pub x: Option<f32>,
    #[uniffi(default = None)]
    pub y: Option<f32>,
    /// Pages to watermark, 1-based. `None` = every page.
    #[uniffi(default = None)]
    pub pages: Option<Vec<u32>>,
    /// Image width/height in points (image watermarks only).
    /// `None` = natural size at 96 dpi.
    #[uniffi(default = None)]
    pub image_width: Option<f32>,
    #[uniffi(default = None)]
    pub image_height: Option<f32>,
}

impl Default for WatermarkOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions::default(),
            text: Some("DRAFT".to_string()),
            image: None,
            opacity: 0.3,
            rotation: 45.0,
            font_size: 48.0,
            font: BaseFont::Helvetica,
            position: Position::Center,
            x: None,
            y: None,
            pages: None,
            image_width: None,
            image_height: None,
        }
    }
}

impl From<WatermarkOptions> for dusrelpdf::WatermarkOptions {
    fn from(value: WatermarkOptions) -> Self {
        Self {
            common: value.common.into(),
            text: value.text,
            image: value.image,
            opacity: value.opacity,
            rotation: value.rotation,
            font_size: value.font_size,
            font: value.font.into(),
            position: value.position.into(),
            x: value.x,
            y: value.y,
            pages: value.pages,
            image_width: value.image_width,
            image_height: value.image_height,
        }
    }
}

/// Options for [`add_page_numbers`].
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PageNumberOptions {
    pub common: CommonOptions,
    /// Template with `{n}` for the page number and `{total}` for the
    /// total, e.g. `"{n}"`, `"Page {n} of {total}"`. Default `"{n}"`.
    #[uniffi(default = "{n}")]
    pub format: String,
    /// Where to place the number (default BottomCenter).
    pub position: Position,
    /// First printed number (default 1). Use 0 to start from 0.
    #[uniffi(default = 1)]
    pub start_at: u32,
    /// Font size in points (default 10).
    #[uniffi(default = 10.0)]
    pub font_size: f32,
    /// Font (default Helvetica).
    pub font: BaseFont,
    /// Margin from the page edge in points (default 36).
    #[uniffi(default = 36.0)]
    pub margin: f32,
    /// Pages to number, 1-based. `None` = every page.
    #[uniffi(default = None)]
    pub pages: Option<Vec<u32>>,
}

impl Default for PageNumberOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions::default(),
            format: "{n}".to_string(),
            position: Position::BottomCenter,
            start_at: 1,
            font_size: 10.0,
            font: BaseFont::Helvetica,
            margin: 36.0,
            pages: None,
        }
    }
}

impl From<PageNumberOptions> for dusrelpdf::PageNumberOptions {
    fn from(value: PageNumberOptions) -> Self {
        Self {
            common: value.common.into(),
            format: value.format,
            position: value.position.into(),
            start_at: value.start_at,
            font_size: value.font_size,
            font: value.font.into(),
            margin: value.margin,
            pages: value.pages,
        }
    }
}

/// Options for [`images_to_pdf`].
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct ImagesToPdfOptions {
    /// Page size (default A4 with Contain fit).
    pub page_size: PageSize,
    /// Fit mode (default Contain).
    pub fit: ImageFit,
    /// Background colour as [r, g, b], each 0.0–1.0, for letterboxed
    /// areas. Default white `[1.0, 1.0, 1.0]`. Exactly 3 values are
    /// required — anything else is an `InvalidArgument` error.
    pub background: Vec<f32>,
}

impl Default for ImagesToPdfOptions {
    fn default() -> Self {
        Self {
            page_size: PageSize::A4,
            fit: ImageFit::Contain,
            background: vec![1.0, 1.0, 1.0],
        }
    }
}

impl ImagesToPdfOptions {
    fn into_core(self) -> Result<dusrelpdf::ImagesToPdfOptions> {
        let [r, g, b] = self.background.as_slice() else {
            return Err(DusrelError::InvalidArgument {
                code: "INVALID_ARGUMENT".to_string(),
                reason_en: "background must have exactly 3 values [r, g, b], each 0.0-1.0."
                    .to_string(),
                reason_hi: "background me exactly 3 values [r, g, b] hone chahiye, har ek 0.0-1.0."
                    .to_string(),
            });
        };
        Ok(dusrelpdf::ImagesToPdfOptions {
            page_size: self.page_size.into(),
            fit: self.fit.into(),
            background: [*r, *g, *b],
        })
    }
}

/// Options for [`place_signature`].
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SignatureOptions {
    pub common: CommonOptions,
    /// Target page, 1-based (default 1).
    #[uniffi(default = 1)]
    pub page: u32,
    /// Position in points from the bottom-left corner (default 36, 36).
    #[uniffi(default = 36.0)]
    pub x: f32,
    #[uniffi(default = 36.0)]
    pub y: f32,
    /// Display size in points. `None` width = 144 pt (2 inches) with
    /// height from the image aspect ratio.
    #[uniffi(default = None)]
    pub width: Option<f32>,
    #[uniffi(default = None)]
    pub height: Option<f32>,
}

impl Default for SignatureOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions::default(),
            page: 1,
            x: 36.0,
            y: 36.0,
            width: None,
            height: None,
        }
    }
}

impl From<SignatureOptions> for dusrelpdf::SignatureOptions {
    fn from(value: SignatureOptions) -> Self {
        Self {
            common: value.common.into(),
            page: value.page,
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

// ---------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------

/// One page's size in points.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PageDimensions {
    pub width: f32,
    pub height: f32,
}

/// Summary of a PDF document, from [`get_pdf_info`].
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct PdfInfo {
    /// Number of pages.
    pub page_count: u32,
    /// PDF version string, e.g. `"1.4"`.
    pub version: String,
    /// True when the trailer has an `/Encrypt` entry.
    pub encrypted: bool,
    /// Document title from the Info dict, if present.
    pub title: Option<String>,
    /// Page sizes in points (width, height), in page order.
    pub page_sizes: Vec<PageDimensions>,
}

impl From<dusrelpdf::PdfInfo> for PdfInfo {
    fn from(value: dusrelpdf::PdfInfo) -> Self {
        Self {
            page_count: value.page_count,
            version: value.version,
            encrypted: value.encrypted,
            title: value.title,
            page_sizes: value
                .page_sizes
                .into_iter()
                .map(|(width, height)| PageDimensions { width, height })
                .collect(),
        }
    }
}

/// Result of a bulk operation: one output per input, in input order.
/// Failed items keep their error instead of aborting the whole batch.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct BulkItem {
    /// Input index (0-based).
    pub index: u64,
    /// Output bytes on success.
    pub pdf: Option<Vec<u8>>,
    /// Error code on failure.
    pub error_code: Option<String>,
    /// Human-readable error (English) on failure.
    pub error: Option<String>,
}

impl From<dusrelpdf::BulkItem> for BulkItem {
    fn from(value: dusrelpdf::BulkItem) -> Self {
        Self {
            index: value.index as u64,
            pdf: value.pdf,
            error_code: value.error_code,
            error: value.error,
        }
    }
}

// ---------------------------------------------------------------------
// Exported functions — thin wrappers over the core, one per operation
// ---------------------------------------------------------------------

/// Crate version (lockstep with the npm / PyPI packages).
#[uniffi::export]
pub fn version() -> String {
    dusrelpdf::VERSION.to_string()
}

/// Merge multiple PDFs into one, in input order.
///
/// Honest limits (from the core): the original outline trees are
/// dropped and rebuilt as flat first-page bookmarks; the first
/// input's Info metadata is kept when `preserve_metadata` is set;
/// AcroForm, named destinations and embedded files are not carried
/// over.
#[uniffi::export]
pub fn merge_pdfs(inputs: Vec<Vec<u8>>, options: MergeOptions) -> Result<Vec<u8>> {
    dusrelpdf::merge_pdfs(&inputs, &options.into()).map_err(DusrelError::from)
}

/// Split one PDF into one output per range. `ranges` is a list of
/// page-number lists (1-based).
#[uniffi::export]
pub fn split_pdf(
    pdf: Vec<u8>,
    ranges: Vec<Vec<u32>>,
    options: PageSetOptions,
) -> Result<Vec<Vec<u8>>> {
    dusrelpdf::split_pdf(&pdf, &ranges, &options.into()).map_err(DusrelError::from)
}

/// Split from a range string such as `"1-3,4-5"`.
#[uniffi::export]
pub fn split_pdf_ranges(
    pdf: Vec<u8>,
    ranges: String,
    options: PageSetOptions,
) -> Result<Vec<Vec<u8>>> {
    dusrelpdf::split_pdf_ranges(&pdf, &ranges, &options.into()).map_err(DusrelError::from)
}

/// Split into one PDF per page.
#[uniffi::export]
pub fn split_each_page(pdf: Vec<u8>, options: PageSetOptions) -> Result<Vec<Vec<u8>>> {
    dusrelpdf::split_each_page(&pdf, &options.into()).map_err(DusrelError::from)
}

/// Extract selected pages into a new PDF, in the order given.
#[uniffi::export]
pub fn extract_pages(pdf: Vec<u8>, pages: Vec<u32>, options: PageSetOptions) -> Result<Vec<u8>> {
    dusrelpdf::extract_pages(&pdf, &pages, &options.into()).map_err(DusrelError::from)
}

/// Extract from a range string such as `"1-3,5"`.
#[uniffi::export]
pub fn extract_pages_range(
    pdf: Vec<u8>,
    range: String,
    options: PageSetOptions,
) -> Result<Vec<u8>> {
    dusrelpdf::extract_pages_range(&pdf, &range, &options.into()).map_err(DusrelError::from)
}

/// Delete pages. Out-of-range numbers are an error.
#[uniffi::export]
pub fn remove_pages(pdf: Vec<u8>, pages: Vec<u32>, options: PageSetOptions) -> Result<Vec<u8>> {
    dusrelpdf::remove_pages(&pdf, &pages, &options.into()).map_err(DusrelError::from)
}

/// Change the page order. `order` must list every page exactly once.
#[uniffi::export]
pub fn reorder_pages(pdf: Vec<u8>, order: Vec<u32>, options: PageSetOptions) -> Result<Vec<u8>> {
    dusrelpdf::reorder_pages(&pdf, &order, &options.into()).map_err(DusrelError::from)
}

/// Rotate pages by a multiple of 90 degrees. Negative angles are
/// normalised (-90 becomes 270). The rotation is display rotation:
/// the page dict's `/Rotate` entry is updated, content streams are
/// not redrawn.
#[uniffi::export]
pub fn rotate_pages(pdf: Vec<u8>, options: RotateOptions) -> Result<Vec<u8>> {
    dusrelpdf::rotate_pages(&pdf, &options.into()).map_err(DusrelError::from)
}

/// Read a PDF's summary without modifying it.
#[uniffi::export]
pub fn get_pdf_info(pdf: Vec<u8>, common: CommonOptions) -> Result<PdfInfo> {
    dusrelpdf::get_pdf_info(&pdf, &common.into())
        .map(PdfInfo::from)
        .map_err(DusrelError::from)
}

/// Extract many selections from one PDF in a single call. One bad
/// selection fails only its own item, never the whole batch.
#[uniffi::export]
pub fn bulk_extract(
    pdf: Vec<u8>,
    selections: Vec<Vec<u32>>,
    options: PageSetOptions,
) -> Vec<BulkItem> {
    dusrelpdf::bulk_extract(&pdf, &selections, &options.into())
        .into_iter()
        .map(BulkItem::from)
        .collect()
}

/// One page per image, in input order.
#[uniffi::export]
pub fn images_to_pdf(images: Vec<Vec<u8>>, options: ImagesToPdfOptions) -> Result<Vec<u8>> {
    let core_options = options.into_core()?;
    dusrelpdf::images_to_pdf(&images, &core_options).map_err(DusrelError::from)
}

/// Add a text or image watermark to pages.
#[uniffi::export]
pub fn add_watermark(pdf: Vec<u8>, options: WatermarkOptions) -> Result<Vec<u8>> {
    dusrelpdf::add_watermark(&pdf, &options.into()).map_err(DusrelError::from)
}

/// Add page numbers to pages, using a `{n}` / `{total}` template.
#[uniffi::export]
pub fn add_page_numbers(pdf: Vec<u8>, options: PageNumberOptions) -> Result<Vec<u8>> {
    dusrelpdf::add_page_numbers(&pdf, &options.into()).map_err(DusrelError::from)
}

/// Place a handwritten signature IMAGE on one page.
///
/// This is image placement only — **not** a cryptographic digital
/// signature. Transparent PNGs keep their transparency via a
/// `/SMask` soft-mask.
#[uniffi::export]
pub fn place_signature(
    pdf: Vec<u8>,
    signature_image: Vec<u8>,
    options: SignatureOptions,
) -> Result<Vec<u8>> {
    dusrelpdf::place_signature(&pdf, &signature_image, &options.into()).map_err(DusrelError::from)
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::*;
    use dusrelpdf::fixtures::{
        make_malformed_pdf, make_photo_jpeg, make_signature_png, make_test_pdf,
    };

    fn three_pages() -> Vec<u8> {
        make_test_pdf(3, Some("Test Doc"))
    }

    fn page_count(pdf: &[u8]) -> u32 {
        get_pdf_info(pdf.to_vec(), CommonOptions::default())
            .expect("fixture is a valid PDF")
            .page_count
    }

    #[test]
    fn version_matches_core() {
        assert_eq!(version(), dusrelpdf::VERSION);
        assert_eq!(version(), "0.1.0");
    }

    #[test]
    fn merge_two_pdfs() {
        let a = make_test_pdf(2, Some("A"));
        let b = make_test_pdf(3, Some("B"));
        let out = merge_pdfs(vec![a, b], MergeOptions::default()).expect("merge works");
        let info = get_pdf_info(out, CommonOptions::default()).expect("merged is valid");
        assert_eq!(info.page_count, 5);
        // First input's title is preserved by default.
        assert_eq!(info.title.as_deref(), Some("A"));
        // Every page is A4 (595 x 842).
        assert_eq!(info.page_sizes.len(), 5);
        assert_eq!(info.page_sizes[0].width, 595.0);
        assert_eq!(info.page_sizes[0].height, 842.0);
    }

    #[test]
    fn merge_empty_input_errors() {
        let err = merge_pdfs(vec![], MergeOptions::default()).unwrap_err();
        assert_eq!(err.code(), "EMPTY_INPUT");
        assert!(matches!(err, DusrelError::EmptyInput { .. }));
        assert!(!err.reason_en().is_empty());
    }

    #[test]
    fn merge_malformed_pdf_errors() {
        let err = merge_pdfs(vec![make_malformed_pdf()], MergeOptions::default()).unwrap_err();
        assert_eq!(err.code(), "INVALID_PDF");
    }

    #[test]
    fn extract_pages_and_ranges() {
        let pdf = three_pages();
        let one =
            extract_pages(pdf.clone(), vec![2], PageSetOptions::default()).expect("extract works");
        assert_eq!(page_count(&one), 1);

        let two = extract_pages_range(pdf.clone(), "1,3".to_string(), PageSetOptions::default())
            .expect("range extract works");
        assert_eq!(page_count(&two), 2);
    }

    #[test]
    fn extract_out_of_range_page_errors() {
        let err = extract_pages(three_pages(), vec![9], PageSetOptions::default()).unwrap_err();
        assert_eq!(err.code(), "PAGE_OUT_OF_RANGE");
        assert!(matches!(err, DusrelError::PageOutOfRange { .. }));
        assert!(err.reason_en().contains('9'));
    }

    #[test]
    fn malformed_range_string_errors_invalid_range() {
        let err = extract_pages_range(three_pages(), "abc".to_string(), PageSetOptions::default())
            .unwrap_err();
        assert_eq!(err.code(), "INVALID_RANGE");
        assert!(matches!(err, DusrelError::InvalidRange { .. }));
        // Both languages are carried through.
        let DusrelError::InvalidRange {
            reason_en,
            reason_hi,
            ..
        } = &err
        else {
            unreachable!()
        };
        assert!(reason_en.contains("abc"));
        assert!(!reason_hi.is_empty());
    }

    #[test]
    fn split_each_page_and_ranges() {
        let pdf = three_pages();
        let per_page =
            split_each_page(pdf.clone(), PageSetOptions::default()).expect("split works");
        assert_eq!(per_page.len(), 3);
        for part in &per_page {
            assert_eq!(page_count(part), 1);
        }

        let ranged = split_pdf_ranges(pdf, "1-2,3".to_string(), PageSetOptions::default())
            .expect("range split works");
        assert_eq!(ranged.len(), 2);
        assert_eq!(page_count(&ranged[0]), 2);
        assert_eq!(page_count(&ranged[1]), 1);
    }

    #[test]
    fn remove_and_reorder_pages() {
        let pdf = three_pages();
        let removed =
            remove_pages(pdf.clone(), vec![2], PageSetOptions::default()).expect("remove works");
        assert_eq!(page_count(&removed), 2);

        let reordered =
            reorder_pages(pdf, vec![3, 1, 2], PageSetOptions::default()).expect("reorder works");
        assert_eq!(page_count(&reordered), 3);
        // First page is now the old page 3 ("Page 3" in its content).
        let doc = lopdf::Document::load_mem(&reordered).expect("reordered parses");
        let first_id = *doc.get_pages().values().next().expect("has pages");
        let bytes = doc.get_page_content(first_id);
        let content = lopdf::content::Content::decode(&bytes).expect("page content decodes");
        let text: Vec<u8> = content
            .operations
            .iter()
            .flat_map(|op| {
                op.operands.iter().filter_map(|o| match o {
                    lopdf::Object::String(bytes, _) => Some(bytes.clone()),
                    _ => None,
                })
            })
            .flatten()
            .collect();
        assert_eq!(text, b"Page 3");
    }

    #[test]
    fn reorder_bad_permutation_errors() {
        let err =
            reorder_pages(three_pages(), vec![1, 1, 2], PageSetOptions::default()).unwrap_err();
        assert_eq!(err.code(), "INVALID_ARGUMENT");
    }

    #[test]
    fn rotate_pages_sets_rotate_entry() {
        let pdf = three_pages();
        let opts = RotateOptions {
            angle: 90,
            pages: Some(vec![1]),
            ..Default::default()
        };
        let out = rotate_pages(pdf, opts).expect("rotate works");
        assert_eq!(page_count(&out), 3);
        let doc = lopdf::Document::load_mem(&out).expect("rotated parses");
        let first_id = *doc.get_pages().values().next().expect("has pages");
        let rotate = doc
            .get_object(first_id)
            .and_then(lopdf::Object::as_dict)
            .and_then(|d| d.get(b"Rotate"))
            .and_then(lopdf::Object::as_i64)
            .expect("page has /Rotate");
        assert_eq!(rotate, 90);
    }

    #[test]
    fn rotate_negative_angle_normalises() {
        let opts = RotateOptions {
            angle: -90,
            ..Default::default()
        };
        let out = rotate_pages(three_pages(), opts).expect("rotate -90 works");
        let doc = lopdf::Document::load_mem(&out).expect("rotated parses");
        let first_id = *doc.get_pages().values().next().expect("has pages");
        let rotate = doc
            .get_object(first_id)
            .and_then(lopdf::Object::as_dict)
            .and_then(|d| d.get(b"Rotate"))
            .and_then(lopdf::Object::as_i64)
            .expect("page has /Rotate");
        assert_eq!(rotate, 270);
    }

    #[test]
    fn rotate_bad_angle_errors_invalid_angle() {
        let opts = RotateOptions {
            angle: 45,
            ..Default::default()
        };
        let err = rotate_pages(three_pages(), opts).unwrap_err();
        assert_eq!(err.code(), "INVALID_ANGLE");
        assert!(matches!(err, DusrelError::InvalidAngle { .. }));
    }

    #[test]
    fn get_pdf_info_reports_title_and_version() {
        let info = get_pdf_info(three_pages(), CommonOptions::default()).expect("info works");
        assert_eq!(info.page_count, 3);
        assert_eq!(info.version, "1.7");
        assert!(!info.encrypted);
        assert_eq!(info.title.as_deref(), Some("Test Doc"));
        assert_eq!(info.page_sizes.len(), 3);
    }

    #[test]
    fn get_pdf_info_on_garbage_errors() {
        // Bytes that are not a PDF at all (the "malformed" fixture still
        // loads in lopdf — it yields a 0-page document — so use garbage).
        let err = get_pdf_info(b"not a pdf at all".to_vec(), CommonOptions::default()).unwrap_err();
        assert_eq!(err.code(), "INVALID_PDF");
    }

    #[test]
    fn bulk_extract_keeps_per_item_errors() {
        let items = bulk_extract(
            three_pages(),
            vec![vec![1, 2], vec![9], vec![3]],
            PageSetOptions::default(),
        );
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].index, 0);
        assert_eq!(page_count(items[0].pdf.as_ref().expect("item 0 ok")), 2);
        assert!(items[1].pdf.is_none());
        assert_eq!(items[1].error_code.as_deref(), Some("PAGE_OUT_OF_RANGE"));
        assert!(items[1].error.as_ref().is_some_and(|e| !e.is_empty()));
        assert_eq!(page_count(items[2].pdf.as_ref().expect("item 2 ok")), 1);
    }

    #[test]
    fn images_to_pdf_and_background_validation() {
        let out = images_to_pdf(vec![make_photo_jpeg()], ImagesToPdfOptions::default())
            .expect("images to pdf works");
        assert_eq!(page_count(&out), 1);

        let opts = ImagesToPdfOptions {
            background: vec![1.0, 0.0], // wrong length
            ..Default::default()
        };
        let err = images_to_pdf(vec![make_photo_jpeg()], opts).unwrap_err();
        assert_eq!(err.code(), "INVALID_ARGUMENT");
    }

    #[test]
    fn watermark_and_page_numbers_keep_pages() {
        let pdf = three_pages();
        let wm = WatermarkOptions {
            text: Some("CONFIDENTIAL".to_string()),
            ..Default::default()
        };
        let out = add_watermark(pdf.clone(), wm).expect("watermark works");
        assert_eq!(page_count(&out), 3);

        let pn = PageNumberOptions {
            format: "Page {n} of {total}".to_string(),
            ..Default::default()
        };
        let out = add_page_numbers(pdf, pn).expect("page numbers work");
        assert_eq!(page_count(&out), 3);
    }

    #[test]
    fn place_signature_keeps_pages() {
        let out = place_signature(
            three_pages(),
            make_signature_png(),
            SignatureOptions::default(),
        )
        .expect("signature works");
        assert_eq!(page_count(&out), 3);

        // Out-of-range target page is an error.
        let opts = SignatureOptions {
            page: 9,
            ..Default::default()
        };
        let err = place_signature(three_pages(), make_signature_png(), opts).unwrap_err();
        assert_eq!(err.code(), "PAGE_OUT_OF_RANGE");
    }

    #[test]
    fn records_default_like_core() {
        // The generated Kotlin/Swift constructors use these same
        // per-field defaults, so keep them aligned with the core.
        assert!(CommonOptions::default().compress);
        assert!(MergeOptions::default().preserve_metadata);
        assert_eq!(RotateOptions::default().angle, 90);
        assert_eq!(WatermarkOptions::default().text.as_deref(), Some("DRAFT"));
        assert_eq!(WatermarkOptions::default().opacity, 0.3);
        assert_eq!(WatermarkOptions::default().rotation, 45.0);
        assert_eq!(PageNumberOptions::default().format, "{n}");
        assert_eq!(PageNumberOptions::default().start_at, 1);
        assert_eq!(
            PageNumberOptions::default().position,
            Position::BottomCenter
        );
        assert_eq!(
            ImagesToPdfOptions::default().background,
            vec![1.0, 1.0, 1.0]
        );
        assert_eq!(SignatureOptions::default().page, 1);
        assert_eq!(SignatureOptions::default().x, 36.0);
    }
}
