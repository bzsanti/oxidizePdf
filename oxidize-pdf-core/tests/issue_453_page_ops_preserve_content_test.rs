//! #453: the page operations (rotate, split, reorder, page extraction) must
//! preserve page content, not reconstruct it.
//!
//! Each operation used to walk the content stream with its own operator `match`
//! and re-emit text through the high-level page API — mapping every font to one
//! of the standard 14, decoding bytes with `from_utf8`, and dropping every
//! operator it did not recognize (images, XObjects, curves, and any positioning
//! operator beyond `Td`). The produced PDF was therefore wrong: text at the
//! wrong place or gone, images gone. The correct behavior is to copy the
//! original content streams and resources verbatim, exactly as `merge` does.
//!
//! The oracle is poppler (`pdftotext` / `pdfimages`), independent of this
//! crate's own extractor: it reads the *bytes we wrote*, so a passing test means
//! the written PDF is correct for any reader, not just ours.

use oxidize_pdf::operations::PageRange;
use oxidize_pdf::operations::{
    extract_page_to_file, reorder_pdf_pages, rotate_pdf_pages, split_into_pages, RotateOptions,
    RotationAngle,
};
use std::path::Path;
use std::process::Command;

const FIXTURE: &str = "tests/fixtures/Cold_Email_Hacks.pdf";

/// The fixture's title page (index 0) is image-only; page 16 (1-based) is the
/// first that carries both substantial prose and images, so it exercises text
/// and raster preservation together. Verified against poppler: 100 words, 2
/// images, `/Rotate` absent.
const RICH_PAGE_1BASED: u32 = 16;
const RICH_PAGE_0BASED: usize = 15;
const PAGE_COUNT: usize = 44;

/// Words of 4+ letters that poppler extracts from a 1-based page range of a file.
fn poppler_words(path: &Path, first: u32, last: u32) -> Option<Vec<String>> {
    let out = Command::new("pdftotext")
        .args([
            "-f",
            &first.to_string(),
            "-l",
            &last.to_string(),
            path.to_str()?,
            "-",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Some(
        text.split_whitespace()
            .filter(|w| w.chars().filter(|c| c.is_alphabetic()).count() >= 4)
            .map(|w| w.to_string())
            .collect(),
    )
}

/// Number of raster images (type `image`) poppler finds in a 1-based page
/// range. Soft masks (type `smask`) are counted separately here because they
/// were a distinct resource-embedding gap in the verbatim-copy path — the
/// operator-dispatch defect #453 fixes is orthogonal to it. That gap is now
/// closed by #465; its own regression coverage lives in
/// `issue_465_smask_preservation_test.rs`.
fn poppler_image_count(path: &Path, first: u32, last: u32) -> Option<usize> {
    let out = Command::new("pdfimages")
        .args([
            "-list",
            "-f",
            &first.to_string(),
            "-l",
            &last.to_string(),
            path.to_str()?,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    // Header is two lines; the `type` column (index 2) is `image` for a raster.
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .skip(2)
            .filter(|l| l.split_whitespace().nth(2) == Some("image"))
            .count(),
    )
}

/// `/Rotate` value poppler reports for a 1-based page (0 when absent).
fn poppler_rotation(path: &Path, page: u32) -> Option<i32> {
    let out = Command::new("pdfinfo")
        .args([
            "-f",
            &page.to_string(),
            "-l",
            &page.to_string(),
            path.to_str()?,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    // `Page N rot: 90`
    for line in text.lines() {
        if let Some(rest) = line.split("rot:").nth(1) {
            return rest.trim().parse().ok();
        }
    }
    Some(0)
}

fn poppler_available() -> bool {
    Command::new("pdftotext").arg("-v").output().is_ok()
        && Command::new("pdfimages").arg("-v").output().is_ok()
        && Command::new("pdfinfo").arg("-v").output().is_ok()
}

/// Fraction of `want` present in `got`.
fn retention(want: &[String], got: &[String]) -> f64 {
    if want.is_empty() {
        return 1.0;
    }
    let set: std::collections::HashSet<&String> = got.iter().collect();
    want.iter().filter(|w| set.contains(w)).count() as f64 / want.len() as f64
}

#[test]
fn extract_page_preserves_text_and_images() {
    if !poppler_available() {
        eprintln!("skipping: poppler not on PATH");
        return;
    }
    let src = Path::new(FIXTURE);
    let want_words = poppler_words(src, RICH_PAGE_1BASED, RICH_PAGE_1BASED).expect("source words");
    let want_images =
        poppler_image_count(src, RICH_PAGE_1BASED, RICH_PAGE_1BASED).expect("source images");
    assert!(
        want_words.len() > 20 && want_images >= 1,
        "fixture rich page must have prose and an image to make this test meaningful: \
         {} words, {} images",
        want_words.len(),
        want_images
    );

    let out = std::env::temp_dir().join("issue453_extract.pdf");
    extract_page_to_file(src, RICH_PAGE_0BASED, &out).expect("extract");

    let got_words = poppler_words(&out, 1, 1).expect("output words");
    let got_images = poppler_image_count(&out, 1, 1).expect("output images");

    let r = retention(&want_words, &got_words);
    assert!(
        r > 0.99,
        "extracted page lost text: retention {r:.4} ({}/{} words)",
        (r * want_words.len() as f64) as usize,
        want_words.len()
    );
    assert_eq!(
        got_images, want_images,
        "extracted page lost the image(s): {got_images} vs {want_images}"
    );
}

#[test]
fn split_preserves_text_and_images() {
    if !poppler_available() {
        eprintln!("skipping: poppler not on PATH");
        return;
    }
    let src = Path::new(FIXTURE);
    let want_words = poppler_words(src, RICH_PAGE_1BASED, RICH_PAGE_1BASED).expect("source words");
    let want_images =
        poppler_image_count(src, RICH_PAGE_1BASED, RICH_PAGE_1BASED).expect("source images");

    let dir = std::env::temp_dir().join("issue453_split");
    std::fs::create_dir_all(&dir).unwrap();
    let pattern = dir.join("page_{}.pdf");
    let paths = split_into_pages(src, pattern.to_str().unwrap()).expect("split");
    let rich = &paths[RICH_PAGE_0BASED];

    let got_words = poppler_words(rich, 1, 1).expect("output words");
    let got_images = poppler_image_count(rich, 1, 1).expect("output images");

    assert!(
        retention(&want_words, &got_words) > 0.99,
        "split rich page lost text: retention {:.4}",
        retention(&want_words, &got_words)
    );
    assert_eq!(
        got_images, want_images,
        "split rich page lost the image(s): {got_images} vs {want_images}"
    );
}

#[test]
fn reorder_preserves_text_and_images() {
    if !poppler_available() {
        eprintln!("skipping: poppler not on PATH");
        return;
    }
    let src = Path::new(FIXTURE);
    let want_words = poppler_words(src, RICH_PAGE_1BASED, RICH_PAGE_1BASED).expect("source words");

    let out = std::env::temp_dir().join("issue453_reorder.pdf");
    // Reorder needs a full permutation; reverse the whole document.
    let order: Vec<usize> = (0..PAGE_COUNT).rev().collect();
    reorder_pdf_pages(src, &out, order).expect("reorder");

    // After reversal the rich source page (0-based RICH_PAGE_0BASED) lands at
    // output 0-based PAGE_COUNT-1-RICH_PAGE_0BASED.
    let dest_1based = (PAGE_COUNT - 1 - RICH_PAGE_0BASED + 1) as u32;
    let got_words = poppler_words(&out, dest_1based, dest_1based).expect("output words");
    assert!(
        retention(&want_words, &got_words) > 0.99,
        "reorder mislaid the rich page's text: retention {:.4}",
        retention(&want_words, &got_words)
    );
}

#[test]
fn rotate_sets_native_rotate_and_preserves_content() {
    if !poppler_available() {
        eprintln!("skipping: poppler not on PATH");
        return;
    }
    let src = Path::new(FIXTURE);
    let want_words = poppler_words(src, RICH_PAGE_1BASED, RICH_PAGE_1BASED).expect("source words");
    let want_images =
        poppler_image_count(src, RICH_PAGE_1BASED, RICH_PAGE_1BASED).expect("source images");
    assert_eq!(
        poppler_rotation(src, RICH_PAGE_1BASED),
        Some(0),
        "fixture rich page is expected unrotated"
    );

    let out = std::env::temp_dir().join("issue453_rotate.pdf");
    rotate_pdf_pages(
        src,
        &out,
        RotateOptions {
            angle: RotationAngle::Clockwise90,
            pages: PageRange::All,
            preserve_page_size: false,
        },
    )
    .expect("rotate");

    // Native /Rotate, not a baked-in content transform.
    assert_eq!(
        poppler_rotation(&out, RICH_PAGE_1BASED),
        Some(90),
        "rotate did not set the native /Rotate entry"
    );
    // Content is untouched by a /Rotate rotation: same text, same image.
    let got_words = poppler_words(&out, RICH_PAGE_1BASED, RICH_PAGE_1BASED).expect("output words");
    let got_images =
        poppler_image_count(&out, RICH_PAGE_1BASED, RICH_PAGE_1BASED).expect("output images");
    assert!(
        retention(&want_words, &got_words) > 0.99,
        "rotate lost text: retention {:.4}",
        retention(&want_words, &got_words)
    );
    assert_eq!(
        got_images, want_images,
        "rotate lost the image(s): {got_images} vs {want_images}"
    );
}
