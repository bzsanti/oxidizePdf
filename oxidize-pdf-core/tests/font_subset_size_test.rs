//! Size regression tests for font subsetting against real fixtures.
//!
//! These tests guard the combined effect of:
//! - CFF charstring desubroutinization (Task 5/6)
//! - SID→CID conversion that drops the OTF wrapper (Task 7)
//! - TTF stripping of cmap/OS/2/name (Task 8)
//!
//! Each test skips gracefully if its fixture is missing.

use oxidize_pdf::text::fonts::truetype_subsetter::subset_font;
use std::collections::HashSet;

const SOURCE_SANS_PATH: &str = "../test-pdfs/SourceSans3-Regular.otf";
const ROBOTO_PATH: &str = "../test-pdfs/Roboto-Regular.ttf";
const SOURCE_HAN_PATH: &str = "../test-pdfs/SourceHanSansSC-Regular.otf";

fn load_fixture(path: &str) -> Option<Vec<u8>> {
    match std::fs::read(path) {
        Ok(data) => Some(data),
        Err(_) => {
            eprintln!("SKIPPED: {} not found", path);
            None
        }
    }
}

/// Non-CID CFF (SID-keyed) subset should be a small fraction of the original
/// after SID→CID conversion + desubroutinization + raw-CFF output.
/// SourceSans3-Regular.otf is ~334 KB; subsetting to 3 ASCII chars should
/// leave well under 10% of that.
#[test]
fn test_non_cid_cff_subset_size_under_10_percent() {
    let font_data = match load_fixture(SOURCE_SANS_PATH) {
        Some(d) => d,
        None => return,
    };
    let original_size = font_data.len();
    let used: HashSet<char> = "ABC".chars().collect();
    let result = subset_font(font_data, &used).expect("subsetting must succeed");

    let ratio = result.font_data.len() as f64 / original_size as f64;
    assert!(result.is_raw_cff, "non-CID CFF must be emitted as raw CFF");
    assert!(
        ratio < 0.10,
        "CFF subset ({} bytes) should be <10% of original ({} bytes); ratio {:.4}",
        result.font_data.len(),
        original_size,
        ratio
    );
}

/// TTF subset after stripping cmap/OS/2/name should be a small fraction of
/// the original. Roboto-Regular.ttf is ~515 KB; 3 ASCII chars should leave
/// well under 10% of that.
#[test]
fn test_ttf_subset_size_under_10_percent() {
    let font_data = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };
    let original_size = font_data.len();
    let used: HashSet<char> = "ABC".chars().collect();
    let result = subset_font(font_data, &used).expect("subsetting must succeed");

    let ratio = result.font_data.len() as f64 / original_size as f64;
    assert!(
        !result.is_raw_cff,
        "TTF subset keeps the SFNT wrapper (is_raw_cff=false)"
    );
    assert!(
        ratio < 0.10,
        "TTF subset ({} bytes) should be <10% of original ({} bytes); ratio {:.4}",
        result.font_data.len(),
        original_size,
        ratio
    );
}

/// CID-keyed CFF with ~65K glyphs: subsetting to 4 CJK chars should produce
/// a sub-1% ratio. This is the headline case for Issue #165.
#[test]
fn test_cid_cff_subset_size_under_1_percent() {
    let font_data = match load_fixture(SOURCE_HAN_PATH) {
        Some(d) => d,
        None => return,
    };
    let original_size = font_data.len();
    let used: HashSet<char> = "你好世界".chars().collect();
    let result = subset_font(font_data, &used).expect("subsetting must succeed");

    let ratio = result.font_data.len() as f64 / original_size as f64;
    assert!(result.is_raw_cff, "CID-keyed CFF must be raw CFF");
    assert!(
        ratio < 0.01,
        "CID CFF subset ({} bytes) should be <1% of original ({} bytes); ratio {:.4}",
        result.font_data.len(),
        original_size,
        ratio
    );
}
