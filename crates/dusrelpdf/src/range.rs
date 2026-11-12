//! Page-range parsing: `"1-3,5"` → `[1, 2, 3, 5]`.
//!
//! lopdf numbers pages from 1 and has no range parser, so this module is
//! the single place that turns user-facing range strings into validated
//! 1-based page numbers. Every parser validates against the document's
//! real page count and returns a proper error code instead of panicking.

use crate::error::{DusrelError, Result};

/// Parse a flat page selection such as `"1-3,5"` or `"2"` into page
/// numbers, in the order written. Duplicates are kept (the extractor
/// copies the page once per occurrence); reversed ranges (`"5-2"`) and
/// numbers outside `1..=total` are errors.
pub fn parse_page_selection(input: &str, total: u32) -> Result<Vec<u32>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(DusrelError::invalid_range(input));
    }
    let mut out = Vec::new();
    for part in trimmed.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(DusrelError::invalid_range(input));
        }
        if let Some((a, b)) = part.split_once('-') {
            let a: u32 = a
                .trim()
                .parse()
                .map_err(|_| DusrelError::invalid_range(input))?;
            let b: u32 = b
                .trim()
                .parse()
                .map_err(|_| DusrelError::invalid_range(input))?;
            if a == 0 || b == 0 || a > b || b > total {
                return Err(DusrelError::invalid_range(input));
            }
            out.extend(a..=b);
        } else {
            let n: u32 = part
                .parse()
                .map_err(|_| DusrelError::invalid_range(input))?;
            if n == 0 || n > total {
                return Err(DusrelError::invalid_range(input));
            }
            out.push(n);
        }
    }
    if out.is_empty() {
        return Err(DusrelError::invalid_range(input));
    }
    Ok(out)
}

/// Parse a split spec such as `"1-3,4-5"` into one page list per output
/// file. Each comma-separated item is one range; a bare number is a
/// one-page range. Identical in syntax to [`parse_page_selection`] but
/// grouped per chunk.
pub fn parse_page_ranges(input: &str, total: u32) -> Result<Vec<Vec<u32>>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(DusrelError::invalid_range(input));
    }
    let mut out = Vec::new();
    for part in trimmed.split(',') {
        let part = part.trim();
        if part.is_empty() {
            return Err(DusrelError::invalid_range(input));
        }
        out.push(parse_page_selection(part, total)?);
    }
    Ok(out)
}

/// Validate an explicit page-number list against a document's page
/// count. Returns [`ErrorCode::PageOutOfRange`](crate::error::ErrorCode::PageOutOfRange)
/// on the first bad number (including 0).
pub fn validate_pages(pages: &[u32], total: u32) -> Result<()> {
    for &page in pages {
        if page == 0 || page > total {
            return Err(DusrelError::page_out_of_range(page, total));
        }
    }
    Ok(())
}

/// Validate that `order` is a complete permutation of `1..=total`
/// (every page once, none missing).
pub fn validate_permutation(order: &[u32], total: u32) -> Result<()> {
    if order.len() as u32 != total {
        return Err(DusrelError::invalid_argument(
            &format!(
                "Reorder needs every page exactly once: got {} entries for a {total}-page PDF.",
                order.len()
            ),
            &format!(
                "Reorder me har page ek baar chahiye: {total} pages wale PDF ke liye {} entries mili.",
                order.len()
            ),
        ));
    }
    let mut seen = vec![false; total as usize];
    for &page in order {
        if page == 0 || page > total {
            return Err(DusrelError::page_out_of_range(page, total));
        }
        if seen[(page - 1) as usize] {
            return Err(DusrelError::invalid_argument(
                &format!("Reorder lists page {page} more than once."),
                &format!("Reorder me page {page} ek se zyada baar hai."),
            ));
        }
        seen[(page - 1) as usize] = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ranges() {
        assert_eq!(parse_page_selection("1-3,5", 10).unwrap(), vec![1, 2, 3, 5]);
        assert_eq!(parse_page_selection("2", 10).unwrap(), vec![2]);
        assert_eq!(
            parse_page_selection(" 1 - 2 , 4 ", 10).unwrap(),
            vec![1, 2, 4]
        );
    }

    #[test]
    fn rejects_bad_ranges() {
        assert!(parse_page_selection("", 10).is_err());
        assert!(parse_page_selection("0", 10).is_err());
        assert!(parse_page_selection("11", 10).is_err());
        assert!(parse_page_selection("5-2", 10).is_err());
        assert!(parse_page_selection("1-", 10).is_err());
        assert!(parse_page_selection("a-b", 10).is_err());
        assert!(parse_page_selection("1,,2", 10).is_err());
    }

    #[test]
    fn parses_split_ranges() {
        assert_eq!(
            parse_page_ranges("1-2,3,4-5", 5).unwrap(),
            vec![vec![1, 2], vec![3], vec![4, 5]]
        );
    }

    #[test]
    fn permutation_checks() {
        assert!(validate_permutation(&[3, 1, 2], 3).is_ok());
        assert!(validate_permutation(&[1, 2], 3).is_err());
        assert!(validate_permutation(&[1, 1, 2], 3).is_err());
        assert!(validate_permutation(&[0, 1, 2], 3).is_err());
        assert!(validate_permutation(&[1, 2, 4], 3).is_err());
    }
}
