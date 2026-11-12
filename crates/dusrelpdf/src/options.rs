//! Options for every operation — maximum developer control, the same
//! philosophy as indycheck's `Options`: every `*_with_options` function
//! takes one of these, and every plain function uses the default.

use serde::{Deserialize, Serialize};

/// Options shared by every operation that reads a PDF.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CommonOptions {
    /// Password for an encrypted PDF. Empty-password PDFs open without it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// Compress output streams (default true). Set false for faster saves.
    #[serde(default = "default_true")]
    pub compress: bool,
}

fn default_true() -> bool {
    true
}

impl CommonOptions {
    pub fn with_password(password: impl Into<String>) -> Self {
        Self {
            password: Some(password.into()),
            compress: true,
        }
    }
}

/// Options for [`crate::merge_pdfs`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeOptions {
    #[serde(flatten)]
    pub common: CommonOptions,
    /// Copy the first input's document info (title/author) into the output.
    /// Default true. Outlines are always rebuilt as flat first-page
    /// bookmarks — lopdf cannot preserve the original outline trees.
    #[serde(default = "default_true")]
    pub preserve_metadata: bool,
    /// Add a flat outline bookmark for the first page of each input.
    /// Default true.
    #[serde(default = "default_true")]
    pub add_bookmarks: bool,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions {
                password: None,
                compress: true,
            },
            preserve_metadata: true,
            add_bookmarks: true,
        }
    }
}

/// Options for split / extract / remove.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PageSetOptions {
    #[serde(flatten)]
    pub common: CommonOptions,
}

/// Where on the page a stamp (watermark / page number) goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

/// A base-14 (built-in) PDF font. No embedding needed, so these always
/// work; only WinAnsi (Latin) text is supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseFont {
    #[default]
    Helvetica,
    HelveticaBold,
    TimesRoman,
    Courier,
    CourierBold,
}

impl BaseFont {
    pub fn base_name(&self) -> &'static str {
        match self {
            BaseFont::Helvetica => "Helvetica",
            BaseFont::HelveticaBold => "Helvetica-Bold",
            BaseFont::TimesRoman => "Times-Roman",
            BaseFont::Courier => "Courier",
            BaseFont::CourierBold => "Courier-Bold",
        }
    }

    /// Width of one character in thousandths of an em, for centring.
    /// Courier is exact (monospace, 600). Helvetica / Times use the
    /// standard base-14 AFM widths; values are approximate for
    /// non-ASCII-capped text only — watermark/number text is ASCII.
    pub fn char_width(&self, ch: char) -> u32 {
        match self {
            BaseFont::Courier | BaseFont::CourierBold => 600,
            BaseFont::Helvetica | BaseFont::HelveticaBold => helvetica_width(ch),
            BaseFont::TimesRoman => times_width(ch),
        }
    }

    /// Width of `text` in points at the given font size.
    pub fn text_width_pt(&self, text: &str, font_size: f32) -> f32 {
        let units: u32 = text.chars().map(|c| self.char_width(c)).sum();
        units as f32 / 1000.0 * font_size
    }
}

fn helvetica_width(ch: char) -> u32 {
    // Standard Helvetica AFM widths for printable ASCII (32..=126).
    const W: [u32; 95] = [
        278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556,
        556, 556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722,
        722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722,
        667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556,
        556, 222, 222, 500, 222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500,
        500, 334, 260, 334, 584,
    ];
    let c = ch as usize;
    if (32..=126).contains(&c) {
        W[c - 32]
    } else {
        556
    }
}

fn times_width(ch: char) -> u32 {
    const W: [u32; 95] = [
        250, 333, 408, 500, 500, 833, 778, 180, 333, 333, 500, 564, 250, 333, 250, 278, 500, 500,
        500, 500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444, 921, 722, 667, 667,
        722, 611, 556, 722, 722, 333, 389, 722, 611, 889, 722, 722, 556, 722, 667, 556, 611, 722,
        722, 944, 722, 722, 611, 333, 278, 333, 469, 500, 333, 444, 500, 444, 500, 444, 333, 500,
        500, 278, 278, 500, 278, 778, 500, 500, 500, 500, 333, 389, 278, 500, 500, 722, 500, 500,
        444, 480, 200, 480, 541,
    ];
    let c = ch as usize;
    if (32..=126).contains(&c) {
        W[c - 32]
    } else {
        500
    }
}

/// Options for [`crate::add_watermark`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatermarkOptions {
    #[serde(flatten)]
    pub common: CommonOptions,
    /// Watermark text. Mutually exclusive with `image` — set one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Watermark image bytes (PNG/JPEG). Mutually exclusive with `text`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<Vec<u8>>,
    /// Opacity 0.0–1.0 (default 0.3). Pass an explicit value for
    /// image watermarks — the default is the same for text and images.
    pub opacity: f32,
    /// Rotation in degrees, clockwise (default 45). Pass 0 explicitly
    /// for image watermarks that should stay upright.
    pub rotation: f32,
    /// Font size in points (text only, default 48).
    pub font_size: f32,
    /// Font (text only, default Helvetica).
    #[serde(default)]
    pub font: BaseFont,
    /// Where to place it (default Center).
    #[serde(default)]
    pub position: Position,
    /// Custom position in points from the bottom-left corner.
    /// Used only when `position` is not enough — set `x`/`y` directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f32>,
    /// Pages to watermark, 1-based. `None` = every page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<u32>>,
    /// Image width/height in points (image watermarks only).
    /// `None` = natural size at 96 dpi.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_height: Option<f32>,
}

impl Default for WatermarkOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions {
                password: None,
                compress: true,
            },
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

/// Options for [`crate::add_page_numbers`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageNumberOptions {
    #[serde(flatten)]
    pub common: CommonOptions,
    /// Template with `{n}` for the page number and `{total}` for the
    /// total, e.g. `"{n}"`, `"Page {n} of {total}"`, `"{n} / {total}"`.
    /// Default `"{n}"`.
    pub format: String,
    /// Where to place the number (default BottomCenter).
    #[serde(default)]
    pub position: Position,
    /// First printed number (default 1). Use 0 to start from 0.
    pub start_at: u32,
    /// Font size in points (default 10).
    pub font_size: f32,
    /// Font (default Helvetica).
    #[serde(default)]
    pub font: BaseFont,
    /// Margin from the page edge in points (default 36).
    pub margin: f32,
    /// Pages to number, 1-based. `None` = every page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<u32>>,
}

impl Default for PageNumberOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions {
                password: None,
                compress: true,
            },
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

/// Options for [`crate::rotate_pages`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotateOptions {
    #[serde(flatten)]
    pub common: CommonOptions,
    /// Rotation in degrees; must be a multiple of 90 (default 90).
    /// Negative angles are normalised (e.g. -90 becomes 270).
    pub angle: i32,
    /// Pages to rotate, 1-based. `None` = every page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pages: Option<Vec<u32>>,
}

impl Default for RotateOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions {
                password: None,
                compress: true,
            },
            angle: 90,
            pages: None,
        }
    }
}

/// Page size for [`crate::images_to_pdf`].
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

/// How an image is fitted into its page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageFit {
    /// Scale to fit inside the page, preserving aspect ratio (default).
    #[default]
    Contain,
    /// Stretch to fill the whole page (aspect ratio not preserved).
    Fill,
}

/// Options for [`crate::images_to_pdf`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImagesToPdfOptions {
    /// Page size (default A4 with Contain fit).
    #[serde(default)]
    pub page_size: PageSize,
    /// Fit mode (default Contain).
    #[serde(default)]
    pub fit: ImageFit,
    /// Background colour as (r, g, b) 0.0–1.0 for letterboxed areas.
    /// Default white.
    pub background: [f32; 3],
}

impl Default for ImagesToPdfOptions {
    fn default() -> Self {
        Self {
            page_size: PageSize::A4,
            fit: ImageFit::Contain,
            background: [1.0, 1.0, 1.0],
        }
    }
}

/// Options for [`crate::place_signature`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignatureOptions {
    #[serde(flatten)]
    pub common: CommonOptions,
    /// Target page, 1-based (default 1).
    pub page: u32,
    /// Position in points from the bottom-left corner (default 36, 36).
    pub x: f32,
    pub y: f32,
    /// Display size in points. `None` width = 144 pt (2 inches) with
    /// height from the image aspect ratio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
}

impl Default for SignatureOptions {
    fn default() -> Self {
        Self {
            common: CommonOptions {
                password: None,
                compress: true,
            },
            page: 1,
            x: 36.0,
            y: 36.0,
            width: None,
            height: None,
        }
    }
}
