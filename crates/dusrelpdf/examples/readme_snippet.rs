// The Rust snippet from the repo-root README — kept runnable so it
// can never drift out of date (the indycheck v0.2.1 lesson).
use dusrelpdf::{
    CommonOptions, PageSetOptions, extract_pages_range, fixtures::make_test_pdf, merge_pdfs_default,
};

fn main() -> Result<(), dusrelpdf::DusrelError> {
    let pdf_a = make_test_pdf(2, None);
    let pdf_b = make_test_pdf(3, None);
    let merged = merge_pdfs_default(&[pdf_a, pdf_b])?;
    let pages = extract_pages_range(&merged, "1-3,5", &PageSetOptions::default())?;
    assert_eq!(
        dusrelpdf::get_pdf_info(&pages, &CommonOptions::default())?.page_count,
        4
    );
    Ok(())
}
