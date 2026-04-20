//! Size regression tests for font subsetting against real fixtures.
//!
//! These tests guard the combined effect of:
//! - CFF charstring desubroutinization (Task 5/6)
//! - SID→CID conversion that drops the OTF wrapper (Task 7)
//! - TTF stripping of cmap/OS/2/name (Task 8)
//!
//! Each test skips gracefully if its fixture is missing.

use oxidize_pdf::text::fonts::truetype_subsetter::subset_font;
use oxidize_pdf::{Document, Font, Page};
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

// =============================================================================
// Cycle G: end-to-end PDF size regression after TTF instruction stripping and
// FontFile2/FontFile3 FlateDecode compression.
//
// These tests measure the WHOLE PDF output (not just the subset bytes) and
// guard against regression of the full pipeline:
//   parsing → subsetting → instruction stripping → SFNT rebuild →
//   PDF embedding with FlateDecode.
// =============================================================================

/// Full-PDF TTF size guard. With instruction stripping + FlateDecode on the
/// FontFile2 stream, a Roboto PDF carrying ~45 Latin characters must fit under
/// 50 KB. Latin fonts benefit less than CJK from these fixes (little hinting,
/// small glyph count), so the threshold is set against a comfortable
/// regression ceiling rather than a tight best-case target.
#[cfg(feature = "compression")]
#[test]
fn test_roboto_ttf_pdf_end_to_end_under_50kb() {
    let font_data = match load_fixture(ROBOTO_PATH) {
        Some(d) => d,
        None => return,
    };
    let text = "The quick brown fox jumps over the lazy dog.";

    let mut doc = Document::new();
    doc.add_font_from_bytes("Roboto", font_data)
        .expect("add_font_from_bytes must succeed");
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("Roboto".to_string()), 12.0)
        .at(50.0, 500.0)
        .write(text)
        .expect("writing must succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation must succeed");
    assert!(
        pdf_bytes.len() < 50_000,
        "Roboto TTF PDF (~45 chars) must be under 50 KB, got {} bytes",
        pdf_bytes.len()
    );
}

/// Full-PDF CJK CFF size guard. SourceHanSansSC (~16 MB) subset to ~25 CJK
/// chars, wrapped in a PDF with FlateDecode-compressed FontFile3, must fit
/// in under 100 KB. v2.5.1 (CFF Local Subr subsetting) already reached
/// 141 KB uncompressed for a similar case; with compression we expect
/// ~70-90 KB. This is the direct follow-up to Issue #165.
#[cfg(feature = "compression")]
#[test]
fn test_cjk_cff_pdf_end_to_end_under_100kb() {
    let font_data = match load_fixture(SOURCE_HAN_PATH) {
        Some(d) => d,
        None => return,
    };
    let cjk_text = "你好世界人大中国文字学习工作生活时间地方事情问题方法发展政府";

    let mut doc = Document::new();
    doc.add_font_from_bytes("SourceHanSansSC", font_data)
        .expect("add_font_from_bytes must succeed");
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Custom("SourceHanSansSC".to_string()), 12.0)
        .at(50.0, 500.0)
        .write(cjk_text)
        .expect("writing CJK text must succeed");
    doc.add_page(page);

    let pdf_bytes = doc.to_bytes().expect("PDF generation must succeed");
    assert!(
        pdf_bytes.len() < 100_000,
        "CJK CFF PDF (~30 chars) must be under 100 KB, got {} bytes",
        pdf_bytes.len()
    );
}

// =============================================================================
// String INDEX elimination
//
// Before this fix the subsetted CFF copied the original font's full String
// INDEX verbatim (~22 KB for SourceSans3, ~5 KB for SourceHanSansSC) even
// though our rebuilt Top DICT only references standard SIDs (≤391) and our
// minimal FD dicts reference no strings at all. The String INDEX was entirely
// unreachable — 93% of the output for Latin CFF was unused metadata.
// =============================================================================

/// Parse CFF output and return the number of entries in its String INDEX.
fn string_index_entry_count(cff: &[u8]) -> usize {
    fn skip_index(cff: &[u8], start: usize) -> usize {
        if start + 2 > cff.len() {
            return start;
        }
        let count = u16::from_be_bytes([cff[start], cff[start + 1]]) as usize;
        if count == 0 {
            return start + 2;
        }
        let off_size = cff[start + 2] as usize;
        let offsets_start = start + 3;
        let offsets_end = offsets_start + (count + 1) * off_size;
        if offsets_end > cff.len() {
            return start;
        }
        let mut last = 0usize;
        let last_off_start = offsets_end - off_size;
        for i in 0..off_size {
            last = (last << 8) | cff[last_off_start + i] as usize;
        }
        offsets_end + last - 1
    }

    let header_size = cff[2] as usize;
    let name_end = skip_index(cff, header_size);
    let top_dict_end = skip_index(cff, name_end);
    if top_dict_end + 2 > cff.len() {
        return 0;
    }
    u16::from_be_bytes([cff[top_dict_end], cff[top_dict_end + 1]]) as usize
}

#[test]
fn test_non_cid_cff_subset_has_empty_string_index() {
    let font_data = match load_fixture(SOURCE_SANS_PATH) {
        Some(d) => d,
        None => return,
    };
    let used: HashSet<char> = "ABC".chars().collect();
    let result = subset_font(font_data, &used).expect("subsetting must succeed");

    let count = string_index_entry_count(&result.font_data);
    assert_eq!(
        count, 0,
        "SID→CID converted CFF must emit an empty String INDEX (count=0), got {}",
        count
    );
}

#[test]
fn test_cid_cff_subset_has_empty_string_index() {
    let font_data = match load_fixture(SOURCE_HAN_PATH) {
        Some(d) => d,
        None => return,
    };
    let used: HashSet<char> = "你好世界".chars().collect();
    let result = subset_font(font_data, &used).expect("subsetting must succeed");

    let count = string_index_entry_count(&result.font_data);
    assert_eq!(
        count, 0,
        "CID CFF subset must emit an empty String INDEX (count=0), got {}",
        count
    );
}

#[test]
fn test_non_cid_cff_subset_size_under_5kb() {
    // After the String INDEX fix, a 3-char Latin CFF subset must fit under
    // 5 KB. Before the fix it was ~23 KB (dominated by the unused original
    // String INDEX copied verbatim).
    let font_data = match load_fixture(SOURCE_SANS_PATH) {
        Some(d) => d,
        None => return,
    };
    let used: HashSet<char> = "ABC".chars().collect();
    let result = subset_font(font_data, &used).expect("subsetting must succeed");
    assert!(
        result.font_data.len() < 5_000,
        "SID→CID CFF subset (3 chars) must be under 5 KB, got {} bytes",
        result.font_data.len()
    );
}
