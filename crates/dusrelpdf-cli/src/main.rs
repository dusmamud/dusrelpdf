//! `dusrelpdf` CLI — every core operation, plus `info` and `mock`.
//!
//! Exit code 0 on success, 1 on any error. Errors print the stable
//! error code (`INVALID_PDF`, `PAGE_OUT_OF_RANGE`, …) plus the English
//! and Hindi reasons; with `--json`, errors and `info` output are
//! machine-readable JSON instead.

use clap::{Parser, Subcommand};
use dusrelpdf::{
    CommonOptions, DusrelError, ImagesToPdfOptions, MergeOptions, PageNumberOptions,
    PageSetOptions, PageSize, Position, RotateOptions, SignatureOptions, WatermarkOptions,
    add_page_numbers, add_watermark, extract_pages_range, get_pdf_info, images_to_pdf, merge_pdfs,
    place_signature, remove_pages, reorder_pages, rotate_pages, split_each_page, split_pdf_ranges,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "dusrelpdf",
    version,
    about = "Offline PDF toolkit — merge, split, extract, reorder, rotate, watermark, page numbers, images-to-PDF and signature placement."
)]
struct Cli {
    /// Print errors and `info` output as JSON.
    #[arg(long, global = true)]
    json: bool,
    /// Password for encrypted PDFs.
    #[arg(long, global = true)]
    password: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print the dusrelpdf version.
    Version,
    /// Show a PDF's page count, version, encryption and title.
    Info {
        /// Input PDF file.
        pdf: PathBuf,
    },
    /// Merge multiple PDFs into one.
    Merge {
        /// Input PDF files, in order.
        inputs: Vec<PathBuf>,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Split one PDF into separate PDFs per range.
    Split {
        /// Input PDF file.
        pdf: PathBuf,
        /// Ranges like "1-3,4-5" — one output file per range.
        #[arg(long, conflicts_with = "each_page")]
        ranges: Option<String>,
        /// One output file per page.
        #[arg(long)]
        each_page: bool,
        /// Directory for the output files.
        #[arg(long, default_value = ".")]
        out_dir: PathBuf,
        /// Output filename prefix (default: input file stem).
        #[arg(long)]
        prefix: Option<String>,
    },
    /// Extract selected pages into a new PDF.
    Extract {
        /// Input PDF file.
        pdf: PathBuf,
        /// Pages like "1-3,5" (1-based).
        #[arg(long)]
        pages: String,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Delete pages from a PDF.
    Remove {
        /// Input PDF file.
        pdf: PathBuf,
        /// Pages to delete, like "2,4" (1-based).
        #[arg(long)]
        pages: String,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Change the page order (a full permutation, e.g. 3,1,2).
    Reorder {
        /// Input PDF file.
        pdf: PathBuf,
        /// New order, comma-separated 1-based page numbers.
        #[arg(long)]
        order: String,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Rotate pages (angle must be a multiple of 90).
    Rotate {
        /// Input PDF file.
        pdf: PathBuf,
        /// Rotation in degrees: 90, 180, 270 (or negative, e.g. -90).
        /// Both `--angle -90` and `--angle=-90` work.
        #[arg(long, default_value_t = 90, allow_negative_numbers = true)]
        angle: i32,
        /// Pages to rotate, like "1,3" (default: all pages).
        #[arg(long)]
        pages: Option<String>,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Make a PDF from images (one page per image).
    ImagesToPdf {
        /// Input image files (PNG/JPEG/WebP…), in order.
        images: Vec<PathBuf>,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
        /// Page size: auto (image size), a4, letter.
        #[arg(long, default_value = "a4")]
        page_size: String,
    },
    /// Stamp a text or image watermark over pages.
    Watermark {
        /// Input PDF file.
        pdf: PathBuf,
        /// Watermark text (ASCII only).
        #[arg(long, conflicts_with = "image")]
        text: Option<String>,
        /// Watermark image file (PNG/JPEG).
        #[arg(long)]
        image: Option<PathBuf>,
        /// Opacity 0.0–1.0 (default 0.3).
        #[arg(long, default_value_t = 0.3)]
        opacity: f32,
        /// Rotation in degrees, clockwise (default 45).
        #[arg(long, default_value_t = 45.0)]
        angle: f32,
        /// Font size in points (default 48).
        #[arg(long, default_value_t = 48.0)]
        font_size: f32,
        /// Pages to watermark, like "1-2" (default: all pages).
        #[arg(long)]
        pages: Option<String>,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Stamp page numbers on pages.
    Numbers {
        /// Input PDF file.
        pdf: PathBuf,
        /// Template with {n} and {total}, e.g. "Page {n} of {total}".
        #[arg(long, default_value = "{n}")]
        format: String,
        /// Position: top-left, top-center, top-right, bottom-left,
        /// bottom-center, bottom-right, center.
        #[arg(long, default_value = "bottom-center")]
        position: String,
        /// First printed number (default 1).
        #[arg(long, default_value_t = 1)]
        start_at: u32,
        /// Font size in points (default 10).
        #[arg(long, default_value_t = 10.0)]
        font_size: f32,
        /// Pages to number, like "1-3" (default: all pages).
        #[arg(long)]
        pages: Option<String>,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Place a handwritten signature IMAGE on a page (not a digital
    /// signature — no certificate, nothing verifiable).
    Sign {
        /// Input PDF file.
        pdf: PathBuf,
        /// Signature image file (transparent PNG recommended).
        #[arg(long)]
        image: PathBuf,
        /// Target page, 1-based (default 1).
        #[arg(long, default_value_t = 1)]
        page: u32,
        /// X position in points from the bottom-left (default 36).
        #[arg(long, default_value_t = 36.0)]
        x: f32,
        /// Y position in points from the bottom-left (default 36).
        #[arg(long, default_value_t = 36.0)]
        y: f32,
        /// Display width in points (default 144, height from aspect).
        #[arg(long)]
        width: Option<f32>,
        /// Display height in points.
        #[arg(long)]
        height: Option<f32>,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Generate a mock multi-page PDF for tests and demos.
    Mock {
        /// Number of pages (default 3).
        #[arg(long, default_value_t = 3)]
        pages: u32,
        /// Output PDF file.
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn common(cli: &Cli) -> CommonOptions {
    CommonOptions {
        password: cli.password.clone(),
        compress: true,
    }
}

fn page_set(cli: &Cli) -> PageSetOptions {
    PageSetOptions {
        common: common(cli),
    }
}

fn parse_csv_u32(s: &str) -> Result<Vec<u32>, DusrelError> {
    s.split(',')
        .map(|p| {
            p.trim().parse::<u32>().map_err(|_| {
                DusrelError::invalid_argument(
                    &format!("\"{s}\" is not a comma-separated list of page numbers."),
                    &format!("\"{s}\" page numbers ki comma-separated list nahi hai."),
                )
            })
        })
        .collect()
}

fn read(path: &Path) -> Result<Vec<u8>, DusrelError> {
    std::fs::read(path).map_err(|e| {
        DusrelError::invalid_argument(
            &format!("Cannot read {}: {e}", path.display()),
            &format!("{} padh nahi paye: {e}", path.display()),
        )
    })
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), DusrelError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            DusrelError::invalid_argument(
                &format!("Cannot create directory {}: {e}", parent.display()),
                &format!("{} directory bana nahi paye: {e}", parent.display()),
            )
        })?;
    }
    std::fs::write(path, bytes).map_err(|e| {
        DusrelError::invalid_argument(
            &format!("Cannot write {}: {e}", path.display()),
            &format!("{} likh nahi paye: {e}", path.display()),
        )
    })
}

fn parse_position(s: &str) -> Result<Position, DusrelError> {
    match s.to_ascii_lowercase().as_str() {
        "top-left" => Ok(Position::TopLeft),
        "top-center" | "top-centre" => Ok(Position::TopCenter),
        "top-right" => Ok(Position::TopRight),
        "bottom-left" => Ok(Position::BottomLeft),
        "bottom-center" | "bottom-centre" => Ok(Position::BottomCenter),
        "bottom-right" => Ok(Position::BottomRight),
        "center" | "centre" => Ok(Position::Center),
        _ => Err(DusrelError::invalid_argument(
            "Unknown position. Use top-left, top-center, top-right, bottom-left, bottom-center, bottom-right or center.",
            "Unknown position hai. top-left, top-center, top-right, bottom-left, bottom-center, bottom-right ya center use karein.",
        )),
    }
}

fn print_error(json: bool, err: &DusrelError) -> ExitCode {
    if json {
        eprintln!(
            "{}",
            serde_json::json!({
                "ok": false,
                "code": err.code_str(),
                "reason_en": err.reason_en,
                "reason_hi": err.reason_hi,
            })
        );
    } else {
        eprintln!("ERROR {}: {}", err.code_str(), err.reason_en);
        eprintln!("({})", err.reason_hi);
    }
    ExitCode::from(1)
}

fn run(cli: &Cli) -> Result<Option<String>, DusrelError> {
    match &cli.command {
        Commands::Version => Ok(Some(format!("dusrelpdf {}", dusrelpdf::VERSION))),
        Commands::Info { pdf } => {
            let bytes = read(pdf)?;
            let info = get_pdf_info(&bytes, &common(cli))?;
            if cli.json {
                Ok(Some(serde_json::to_string(&info).unwrap()))
            } else {
                let mut s = format!(
                    "{}: {} pages, PDF {}, encrypted: {}",
                    pdf.display(),
                    info.page_count,
                    info.version,
                    info.encrypted
                );
                if let Some(title) = &info.title {
                    s.push_str(&format!(", title: {title}"));
                }
                Ok(Some(s))
            }
        }
        Commands::Merge { inputs, output } => {
            if inputs.is_empty() {
                return Err(DusrelError::empty_input("merge inputs"));
            }
            let mut bufs = Vec::new();
            for p in inputs {
                bufs.push(read(p)?);
            }
            let out = merge_pdfs(
                &bufs,
                &MergeOptions {
                    common: common(cli),
                    ..Default::default()
                },
            )?;
            write(output, &out)?;
            Ok(Some(format!(
                "Merged {} PDFs into {} ({} pages)",
                inputs.len(),
                output.display(),
                get_pdf_info(&out, &common(cli))?.page_count
            )))
        }
        Commands::Split {
            pdf,
            ranges,
            each_page,
            out_dir,
            prefix,
        } => {
            let bytes = read(pdf)?;
            let stem = prefix.clone().unwrap_or_else(|| {
                pdf.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "split".to_string())
            });
            let outs = if *each_page {
                split_each_page(&bytes, &page_set(cli))?
            } else if let Some(r) = ranges {
                split_pdf_ranges(&bytes, r, &page_set(cli))?
            } else {
                return Err(DusrelError::invalid_argument(
                    "Split needs --ranges \"1-3,4-5\" or --each-page.",
                    "Split ke liye --ranges \"1-3,4-5\" ya --each-page chahiye.",
                ));
            };
            std::fs::create_dir_all(out_dir).map_err(|e| {
                DusrelError::invalid_argument(
                    &format!("Cannot create directory {}: {e}", out_dir.display()),
                    &format!("{} directory bana nahi paye: {e}", out_dir.display()),
                )
            })?;
            for (i, part) in outs.iter().enumerate() {
                let path = out_dir.join(format!("{stem}-{}.pdf", i + 1));
                write(&path, part)?;
            }
            Ok(Some(format!(
                "Split into {} files in {}",
                outs.len(),
                out_dir.display()
            )))
        }
        Commands::Extract { pdf, pages, output } => {
            let bytes = read(pdf)?;
            let out = extract_pages_range(&bytes, pages, &page_set(cli))?;
            write(output, &out)?;
            Ok(Some(format!(
                "Extracted pages {pages} to {}",
                output.display()
            )))
        }
        Commands::Remove { pdf, pages, output } => {
            let bytes = read(pdf)?;
            let page_vec = dusrelpdf::parse_page_selection(
                pages,
                get_pdf_info(&bytes, &common(cli))?.page_count,
            )?;
            let out = remove_pages(&bytes, &page_vec, &page_set(cli))?;
            write(output, &out)?;
            Ok(Some(format!(
                "Removed pages {pages}, wrote {}",
                output.display()
            )))
        }
        Commands::Reorder { pdf, order, output } => {
            let bytes = read(pdf)?;
            let order_vec = parse_csv_u32(order)?;
            let out = reorder_pages(&bytes, &order_vec, &page_set(cli))?;
            write(output, &out)?;
            Ok(Some(format!(
                "Reordered to {order}, wrote {}",
                output.display()
            )))
        }
        Commands::Rotate {
            pdf,
            angle,
            pages,
            output,
        } => {
            let bytes = read(pdf)?;
            let total = get_pdf_info(&bytes, &common(cli))?.page_count;
            let page_vec = match pages {
                Some(p) => Some(dusrelpdf::parse_page_selection(p, total)?),
                None => None,
            };
            let out = rotate_pages(
                &bytes,
                &RotateOptions {
                    common: common(cli),
                    angle: *angle,
                    pages: page_vec,
                },
            )?;
            write(output, &out)?;
            Ok(Some(format!(
                "Rotated by {angle}°, wrote {}",
                output.display()
            )))
        }
        Commands::ImagesToPdf {
            images,
            output,
            page_size,
        } => {
            if images.is_empty() {
                return Err(DusrelError::empty_input("images"));
            }
            let size = match page_size.to_ascii_lowercase().as_str() {
                "auto" => PageSize::Auto,
                "a4" => PageSize::A4,
                "letter" => PageSize::Letter,
                _ => {
                    return Err(DusrelError::invalid_argument(
                        "Page size must be auto, a4 or letter.",
                        "Page size auto, a4 ya letter honi chahiye.",
                    ));
                }
            };
            let mut bufs = Vec::new();
            for p in images {
                bufs.push(read(p)?);
            }
            let out = images_to_pdf(
                &bufs,
                &ImagesToPdfOptions {
                    page_size: size,
                    ..Default::default()
                },
            )?;
            write(output, &out)?;
            Ok(Some(format!(
                "Made {} from {} images",
                output.display(),
                images.len()
            )))
        }
        Commands::Watermark {
            pdf,
            text,
            image,
            opacity,
            angle,
            font_size,
            pages,
            output,
        } => {
            let bytes = read(pdf)?;
            let total = get_pdf_info(&bytes, &common(cli))?.page_count;
            let page_vec = match pages {
                Some(p) => Some(dusrelpdf::parse_page_selection(p, total)?),
                None => None,
            };
            let image_bytes = match image {
                Some(p) => Some(read(p)?),
                None => None,
            };
            let out = add_watermark(
                &bytes,
                &WatermarkOptions {
                    common: common(cli),
                    text: text.clone(),
                    image: image_bytes,
                    opacity: *opacity,
                    rotation: *angle,
                    font_size: *font_size,
                    position: Position::Center,
                    pages: page_vec,
                    ..Default::default()
                },
            )?;
            write(output, &out)?;
            Ok(Some(format!("Watermarked, wrote {}", output.display())))
        }
        Commands::Numbers {
            pdf,
            format,
            position,
            start_at,
            font_size,
            pages,
            output,
        } => {
            let bytes = read(pdf)?;
            let total = get_pdf_info(&bytes, &common(cli))?.page_count;
            let page_vec = match pages {
                Some(p) => Some(dusrelpdf::parse_page_selection(p, total)?),
                None => None,
            };
            let out = add_page_numbers(
                &bytes,
                &PageNumberOptions {
                    common: common(cli),
                    format: format.clone(),
                    position: parse_position(position)?,
                    start_at: *start_at,
                    font_size: *font_size,
                    pages: page_vec,
                    ..Default::default()
                },
            )?;
            write(output, &out)?;
            Ok(Some(format!("Numbered, wrote {}", output.display())))
        }
        Commands::Sign {
            pdf,
            image,
            page,
            x,
            y,
            width,
            height,
            output,
        } => {
            let bytes = read(pdf)?;
            let sig = read(image)?;
            let out = place_signature(
                &bytes,
                &sig,
                &SignatureOptions {
                    common: common(cli),
                    page: *page,
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                },
            )?;
            write(output, &out)?;
            Ok(Some(format!("Signed, wrote {}", output.display())))
        }
        Commands::Mock { pages, output } => {
            let bytes = dusrelpdf::fixtures::make_test_pdf(*pages, None);
            write(output, &bytes)?;
            Ok(Some(format!(
                "Mock PDF with {pages} pages wrote to {}",
                output.display()
            )))
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(Some(msg)) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Ok(None) => ExitCode::SUCCESS,
        Err(e) => print_error(cli.json, &e),
    }
}
