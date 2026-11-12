//! Stable error codes, in English and Hindi — the same contract as
//! indycheck's `ValidationResult` codes, adapted for binary PDF work.
//!
//! Every public function in this crate returns `Result<_, DusrelError>`
//! and never panics on bad input.

use std::fmt;

/// Stable, machine-readable error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    /// Input is not a PDF, is truncated, or cannot be parsed.
    InvalidPdf,
    /// The PDF is encrypted and no (or a wrong) password was supplied.
    EncryptedPdf,
    /// A page number is outside `1..=page_count`.
    PageOutOfRange,
    /// An input list / byte buffer is empty where at least one item is required.
    EmptyInput,
    /// A page-range string such as `"1-3,5"` is malformed or out of range.
    InvalidRange,
    /// A rotation angle is not a multiple of 90 degrees.
    InvalidAngle,
    /// An image is missing, corrupt, or in an unsupported format.
    InvalidImage,
    /// An argument is wrong (bad permutation, duplicate pages, bad opacity…).
    InvalidArgument,
    /// A non-string / non-bytes value reached a typed API (bindings layer).
    InvalidType,
    /// The underlying PDF operation failed for another reason.
    OperationFailed,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::InvalidPdf => "INVALID_PDF",
            ErrorCode::EncryptedPdf => "ENCRYPTED_PDF",
            ErrorCode::PageOutOfRange => "PAGE_OUT_OF_RANGE",
            ErrorCode::EmptyInput => "EMPTY_INPUT",
            ErrorCode::InvalidRange => "INVALID_RANGE",
            ErrorCode::InvalidAngle => "INVALID_ANGLE",
            ErrorCode::InvalidImage => "INVALID_IMAGE",
            ErrorCode::InvalidArgument => "INVALID_ARGUMENT",
            ErrorCode::InvalidType => "INVALID_TYPE",
            ErrorCode::OperationFailed => "OPERATION_FAILED",
        }
    }
}

/// An error carrying a stable code plus human-readable reasons in
/// English and Hindi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DusrelError {
    pub code: ErrorCode,
    pub reason_en: String,
    pub reason_hi: String,
}

impl DusrelError {
    pub fn new(
        code: ErrorCode,
        reason_en: impl Into<String>,
        reason_hi: impl Into<String>,
    ) -> Self {
        Self {
            code,
            reason_en: reason_en.into(),
            reason_hi: reason_hi.into(),
        }
    }

    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    pub fn invalid_pdf(detail_en: &str) -> Self {
        Self::new(
            ErrorCode::InvalidPdf,
            format!("Invalid PDF: {detail_en}"),
            format!("Galat PDF hai: {detail_en}"),
        )
    }

    pub fn encrypted_pdf() -> Self {
        Self::new(
            ErrorCode::EncryptedPdf,
            "This PDF is encrypted. Supply the password with Options.password.",
            "Ye PDF encrypted hai. Options.password ke saath password dein.",
        )
    }

    pub fn page_out_of_range(page: u32, total: u32) -> Self {
        Self::new(
            ErrorCode::PageOutOfRange,
            format!(
                "Page {page} is out of range (this PDF has {total} pages). Pages are numbered from 1."
            ),
            format!(
                "Page {page} range se bahar hai (is PDF me {total} pages hain). Pages 1 se number hote hain."
            ),
        )
    }

    pub fn empty_input(what: &str) -> Self {
        Self::new(
            ErrorCode::EmptyInput,
            format!("{what} is empty."),
            format!("{what} khaali hai."),
        )
    }

    pub fn invalid_range(range: &str) -> Self {
        Self::new(
            ErrorCode::InvalidRange,
            format!(
                "Invalid page range \"{range}\". Use forms like \"1-3,5\" with pages numbered from 1."
            ),
            format!(
                "Galat page range \"{range}\" hai. \"1-3,5\" jaise form use karein, pages 1 se number hote hain."
            ),
        )
    }

    pub fn invalid_angle(angle: i32) -> Self {
        Self::new(
            ErrorCode::InvalidAngle,
            format!(
                "Rotation angle {angle} is not a multiple of 90 degrees. Use 0, 90, 180 or 270."
            ),
            format!(
                "Rotation angle {angle} 90 ka multiple nahi hai. 0, 90, 180 ya 270 use karein."
            ),
        )
    }

    pub fn invalid_image(detail_en: &str) -> Self {
        Self::new(
            ErrorCode::InvalidImage,
            format!("Invalid image: {detail_en}"),
            format!("Galat image hai: {detail_en}"),
        )
    }

    pub fn invalid_argument(reason_en: &str, reason_hi: &str) -> Self {
        Self::new(ErrorCode::InvalidArgument, reason_en, reason_hi)
    }
}

impl fmt::Display for DusrelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.reason_en)
    }
}

impl std::error::Error for DusrelError {}

impl From<lopdf::Error> for DusrelError {
    fn from(err: lopdf::Error) -> Self {
        let msg = err.to_string();
        let lower = msg.to_lowercase();
        if lower.contains("decrypt") || lower.contains("password") || lower.contains("encrypt") {
            DusrelError::encrypted_pdf()
        } else {
            DusrelError::invalid_pdf(&msg)
        }
    }
}

impl From<std::io::Error> for DusrelError {
    fn from(err: std::io::Error) -> Self {
        DusrelError::new(
            ErrorCode::OperationFailed,
            format!("I/O error: {err}"),
            format!("I/O error hai: {err}"),
        )
    }
}

/// Convenience result type for the whole crate.
pub type Result<T> = std::result::Result<T, DusrelError>;
