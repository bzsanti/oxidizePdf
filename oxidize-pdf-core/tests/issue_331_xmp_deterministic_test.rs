//! Acceptance tests for issue #331: `XmpMetadata::to_xmp_packet()` must be
//! deterministic — same inputs must produce byte-identical XMP output across
//! calls (and across processes), so PDF consumers that content-address bytes,
//! diff fixtures, or pre-hash for signing get stable results.
//!
//! Root cause being fixed: `metadata/xmp.rs:353` builds a `HashMap<String,
//! String>` of `(prefix, uri)` and iterates it to emit `xmlns:*` declarations.
//! `HashMap` iteration order is randomized per instance, so each call produces
//! a different attribute ordering. `custom_namespaces` (line 179) is the same
//! type at storage time, compounding the issue.

use oxidize_pdf::metadata::xmp::{XmpMetadata, XmpNamespace};

/// Build a metadata packet with enough distinct namespaces that any single
/// permutation has a high prior probability of being non-canonical.
fn build_packet() -> XmpMetadata {
    let mut xmp = XmpMetadata::new();
    // Six standard prefixes (xmp, pdf, dc, xmpMM, xmpRights, photoshop).
    xmp.set_text(XmpNamespace::DublinCore, "title", "Issue 331 fixture");
    xmp.set_text(XmpNamespace::DublinCore, "creator", "test");
    xmp.set_text(XmpNamespace::XmpBasic, "CreateDate", "2026-06-16T00:00:00Z");
    xmp.set_text(XmpNamespace::Pdf, "Producer", "oxidize-pdf");
    xmp.set_text(XmpNamespace::XmpRights, "Marked", "false");
    xmp.set_text(
        XmpNamespace::XmpMediaManagement,
        "DocumentID",
        "uuid:fixture-331",
    );
    xmp.set_text(XmpNamespace::Photoshop, "AuthorsPosition", "test");
    // Three custom namespaces — different prefix lexical positions so any
    // sort-order regression is visible.
    xmp.register_namespace(
        "zzcustom".to_string(),
        "https://example.invalid/zzcustom/1.0/".to_string(),
    );
    xmp.register_namespace(
        "aalpha".to_string(),
        "https://example.invalid/aalpha/1.0/".to_string(),
    );
    xmp.register_namespace(
        "mmid".to_string(),
        "https://example.invalid/mmid/1.0/".to_string(),
    );
    xmp
}

/// Extract the lines that declare `xmlns:` attributes (the order-sensitive part).
fn xmlns_lines(packet: &str) -> Vec<&str> {
    packet
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.starts_with("xmlns:"))
        .collect()
}

/// Primary RED contract: invoking `to_xmp_packet()` repeatedly on the same
/// `XmpMetadata` must produce byte-identical output. `HashMap::new()` picks a
/// fresh random seed each call, so two invocations land different iteration
/// orders with very high probability across 10 attempts at this fan-in.
#[test]
fn xmp_packet_is_byte_stable_across_repeated_calls() {
    let xmp = build_packet();
    let baseline = xmp.to_xmp_packet();
    let mut diverged_at: Option<usize> = None;
    for i in 1..10 {
        let again = xmp.to_xmp_packet();
        if again != baseline {
            diverged_at = Some(i);
            // Surface both packets in the failure message so the diff is
            // readable in CI without re-running.
            assert_eq!(
                again,
                baseline,
                "to_xmp_packet() diverged on call #{i} — non-deterministic output. \
                 Baseline xmlns lines:\n  {:#?}\nDivergent xmlns lines:\n  {:#?}",
                xmlns_lines(&baseline),
                xmlns_lines(&again)
            );
        }
    }
    assert!(
        diverged_at.is_none(),
        "to_xmp_packet() must be deterministic across all 10 attempts"
    );
}

/// Anchor the deterministic contract to its actual semantics: namespace
/// declarations must be emitted in lexicographic prefix order. Guards against
/// future refactors that "restore determinism" via a different non-canonical
/// scheme (e.g. insertion order from a `Vec` that happens to be stable today
/// but reorders on cheap permutations). Sorted-by-prefix is what `BTreeMap`
/// provides by type contract.
#[test]
fn xmp_namespace_declarations_are_sorted_by_prefix() {
    let xmp = build_packet();
    let packet = xmp.to_xmp_packet();

    let prefixes: Vec<String> = xmlns_lines(&packet)
        .iter()
        .filter_map(|line| {
            // line shape: xmlns:<prefix>="<uri>"
            let after = line.strip_prefix("xmlns:")?;
            let eq = after.find('=')?;
            Some(after[..eq].to_string())
        })
        .collect();

    assert!(
        prefixes.len() >= 6,
        "fixture must declare >= 6 namespaces — got {} ({:?})",
        prefixes.len(),
        prefixes
    );

    let mut sorted = prefixes.clone();
    sorted.sort();
    assert_eq!(
        prefixes, sorted,
        "xmlns:* declarations must be emitted in lexicographic prefix order — \
         got {:?}, expected {:?}",
        prefixes, sorted
    );
}
