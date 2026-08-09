//! #465: verbatim page copy drops an image's nested `/SMask` (and other nested
//! XObject references), so transparency is lost through `merge` and the page
//! operations.
//!
//! Root cause: `Page::from_parsed_with_content` resolves only the *top-level*
//! XObject reference (`/XObject/<name> N 0 R` → stream); it does not walk into
//! the resolved image stream, so a `/SMask N 0 R` inside it stays a reference
//! into the *source* document's object table. The writer then emits that image
//! verbatim with a dangling `/SMask`, and the referenced soft-mask stream is
//! never written to the output.
//!
//! The oracle is poppler (`pdfimages -list`), independent of this crate's own
//! reader: a soft mask shows up as a row of `type` = `smask`. It reads the bytes
//! we wrote, so a passing test means the output is correct for any reader.
//!
//! Fixture: `Cold_Email_Hacks.pdf` page 16 (1-based) carries one image with a
//! soft mask (poppler: one `smask` row, object 31). The other pages are plain.

use oxidize_pdf::operations::{
    extract_page_to_file, merge_pdfs, split_into_pages, MergeInput, MergeOptions, PageRange,
};
use std::path::Path;
use std::process::Command;

const FIXTURE: &str = "tests/fixtures/Cold_Email_Hacks.pdf";

/// The 1-based page whose image carries a `/SMask`.
const SMASK_PAGE_1BASED: u32 = 16;
const SMASK_PAGE_0BASED: usize = 15;

fn poppler_available() -> bool {
    Command::new("pdfimages").arg("-v").output().is_ok()
}

/// Number of soft-mask rows (`type` = `smask`) poppler finds in a 1-based page
/// range. This is the transparency signal #465 is about.
fn poppler_smask_count(path: &Path, first: u32, last: u32) -> Option<usize> {
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
    // Header is two lines; the `type` column (index 2) is `smask` for a soft mask.
    Some(
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .skip(2)
            .filter(|l| l.split_whitespace().nth(2) == Some("smask"))
            .count(),
    )
}

#[test]
fn extract_page_preserves_smask() {
    if !poppler_available() {
        eprintln!("skipping: poppler not on PATH");
        return;
    }
    let src = Path::new(FIXTURE);
    let want =
        poppler_smask_count(src, SMASK_PAGE_1BASED, SMASK_PAGE_1BASED).expect("source smask");
    assert!(
        want >= 1,
        "fixture page {SMASK_PAGE_1BASED} must have a soft mask for this test to be meaningful (got {want})"
    );

    let out = std::env::temp_dir().join("issue465_extract.pdf");
    extract_page_to_file(src, SMASK_PAGE_0BASED, &out).expect("extract");

    let got = poppler_smask_count(&out, 1, 1).expect("output smask");
    assert_eq!(
        got, want,
        "extracted page lost the image soft mask (transparency): {got} vs {want}"
    );
}

#[test]
fn merge_preserves_smask() {
    if !poppler_available() {
        eprintln!("skipping: poppler not on PATH");
        return;
    }
    let src = Path::new(FIXTURE);
    let want =
        poppler_smask_count(src, SMASK_PAGE_1BASED, SMASK_PAGE_1BASED).expect("source smask");

    // Merge the soft-mask page after one plain page from the same file, so the
    // smask page lands at index 1 (1-based page 2) of the output.
    let inputs = vec![
        MergeInput::with_pages(src, PageRange::Single(0)),
        MergeInput::with_pages(src, PageRange::Single(SMASK_PAGE_0BASED)),
    ];
    let out = std::env::temp_dir().join("issue465_merge.pdf");
    merge_pdfs(inputs, &out, MergeOptions::default()).expect("merge");

    let got = poppler_smask_count(&out, 2, 2).expect("output smask");
    assert_eq!(
        got, want,
        "merged page lost the image soft mask (transparency): {got} vs {want}"
    );
}

#[test]
fn split_preserves_smask() {
    if !poppler_available() {
        eprintln!("skipping: poppler not on PATH");
        return;
    }
    let src = Path::new(FIXTURE);
    let want =
        poppler_smask_count(src, SMASK_PAGE_1BASED, SMASK_PAGE_1BASED).expect("source smask");

    let dir = std::env::temp_dir().join("issue465_split");
    std::fs::create_dir_all(&dir).unwrap();
    let pattern = dir.join("page_{}.pdf");
    let paths = split_into_pages(src, pattern.to_str().unwrap()).expect("split");
    let rich = &paths[SMASK_PAGE_0BASED];

    let got = poppler_smask_count(rich, 1, 1).expect("output smask");
    assert_eq!(
        got, want,
        "split page lost the image soft mask (transparency): {got} vs {want}"
    );
}
