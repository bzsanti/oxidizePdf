# Issue #271 — Partitioner classifier Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:test-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate two partitioner bugs on tightly-spaced structured PDFs (NCSC CAF v4.0):
1. Body paragraphs in the top 5% of the page get reclassified as `Element::Header`.
2. `Element::Title` count is 0 because heading fragments don't exceed the font-size ratio threshold and `TextFragment.struct_tag` is ignored.

**Architecture:** Single-file change in `oxidize-pdf-core/src/pipeline/partition.rs`. Adds a `struct_tag` precedence step before the existing 1-6 pipeline, two new gates on Header/Footer detection (length cap + struct_tag check), and two new heuristic Title detectors (bold-short, numeric-prefix) running in parallel with the existing font-ratio check.

**Tech Stack:** Rust 2021 edition, MSRV 1.77, `cargo test`, pre-commit hook running fmt + clippy + build.

**Spec:** `docs/superpowers/specs/2026-05-28-issue-271-partitioner-classifier-design.md`
**Branch:** `fix/issue-271-partitioner-classifier`

---

## File Structure

- **Modify** `oxidize-pdf-core/src/pipeline/partition.rs`:
  - Add module-level constants (`MAX_HEADER_TEXT_LEN`, `MAX_BOLD_TITLE_LEN`, `MAX_NUMERIC_TITLE_LEN`).
  - Add private enum `StructTagClass`.
  - Add helpers: `classify_by_struct_tag`, `struct_tag_is_body`, `bold_short_title`, `numeric_prefix_title`, `matches_section_prefix`, `strip_section_prefix`, `ends_with_sentence_terminator`.
  - Insert Step 0 (struct_tag-driven classification) in `partition_fragments` before the existing Step 1.
  - Modify Step 1 (Header/Footer) to add length cap + struct_tag-body gates.
  - Modify Step 4 (Title detection) to OR the three signals.
  - Add unit tests in existing `#[cfg(test)] mod tests` (if absent, create) — about 12 helpers tests.
- **Create** `oxidize-pdf-core/tests/partition_classifier_test.rs` — synthetic-fragment unit tests on the public `Partitioner` API.
- **Create** `oxidize-pdf-core/tests/partition_struct_tag_test.rs` — integration tests building tagged PDFs via writer, extracting, partitioning.
- **Create** `oxidize-pdf-core/tests/partition_ncsc_classifier_test.rs` — real-corpus NCSC tests (conditional on `corpus_cache/e0e3ff11371c09c2.pdf`).
- **Create** `oxidize-pdf-core/tests/partition_classifier_regression_test.rs` — real-corpus Higgs/BSI/ENS regression tests (conditional on cached fixtures).

No public API change. No Cargo.toml change. No new crate dependencies.

---

## Task 1: TDD — `classify_by_struct_tag` + `struct_tag_is_body` helpers

**Files:**
- Modify: `oxidize-pdf-core/src/pipeline/partition.rs`

- [ ] **Step 1: Write the failing unit tests**

In `oxidize-pdf-core/src/pipeline/partition.rs`, append a `#[cfg(test)] mod tests` block at the bottom of the file (if one already exists, append the cases inside it):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_by_struct_tag_recognizes_heading_tags() {
        for tag in &["H", "H1", "H2", "H3", "H4", "H5", "H6", "Title"] {
            assert_eq!(
                classify_by_struct_tag(tag),
                Some(StructTagClass::Heading),
                "tag {} should classify as Heading",
                tag
            );
        }
    }

    #[test]
    fn classify_by_struct_tag_recognizes_list_tags() {
        assert_eq!(classify_by_struct_tag("L"), Some(StructTagClass::List));
        assert_eq!(classify_by_struct_tag("LI"), Some(StructTagClass::ListItem));
        assert_eq!(classify_by_struct_tag("Lbl"), Some(StructTagClass::ListItem));
        assert_eq!(classify_by_struct_tag("LBody"), Some(StructTagClass::ListItem));
    }

    #[test]
    fn classify_by_struct_tag_recognizes_artifact() {
        assert_eq!(classify_by_struct_tag("Artifact"), Some(StructTagClass::Artifact));
    }

    #[test]
    fn classify_by_struct_tag_returns_none_for_passthrough_tags() {
        for tag in &["P", "Span", "Figure", "Table", "Caption", "Form", "Note", "Random"] {
            assert_eq!(
                classify_by_struct_tag(tag),
                None,
                "tag {} should be None (fall through)",
                tag
            );
        }
    }

    #[test]
    fn struct_tag_is_body_recognizes_body_tags() {
        for tag in &["P", "Span", "L", "LI", "H1", "H2", "H6", "Title", "Lbl", "LBody"] {
            assert!(
                struct_tag_is_body(&Some(tag.to_string())),
                "tag {} should be body",
                tag
            );
        }
    }

    #[test]
    fn struct_tag_is_body_returns_false_for_artifact() {
        assert!(!struct_tag_is_body(&Some("Artifact".to_string())));
    }

    #[test]
    fn struct_tag_is_body_returns_false_for_none() {
        assert!(!struct_tag_is_body(&None));
    }
}
```

Run `cargo test -p oxidize-pdf --lib partition::tests::classify_by_struct_tag` — must fail with `cannot find function 'classify_by_struct_tag'` etc.

- [ ] **Step 2: Implement the helpers**

Add to `oxidize-pdf-core/src/pipeline/partition.rs` after the existing `compute_kv_confidence` function (around line 668):

```rust
/// Classification class implied by a PDF structural tag (`/H1`, `/L`, `/Artifact`, …).
///
/// Returned by [`classify_by_struct_tag`] when the tag carries an unambiguous
/// document-role signal. Untagged or pass-through tags return `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructTagClass {
    Heading,
    List,
    ListItem,
    Artifact,
}

/// Map a PDF structural tag string to a partitioner classification class.
///
/// Recognizes the heading family (`H`, `H1..H6`, `Title`), list family
/// (`L`, `LI`, `Lbl`, `LBody`), and `Artifact`. Returns `None` for
/// pass-through tags (`P`, `Span`, `Figure`, …) so they flow into the
/// heuristic classifier.
fn classify_by_struct_tag(tag: &str) -> Option<StructTagClass> {
    match tag {
        "H" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6" | "Title" => Some(StructTagClass::Heading),
        "L" => Some(StructTagClass::List),
        "LI" | "Lbl" | "LBody" => Some(StructTagClass::ListItem),
        "Artifact" => Some(StructTagClass::Artifact),
        _ => None,
    }
}

/// `true` when the struct tag indicates the fragment is body content (paragraph,
/// span, heading, list item). Used by Header/Footer detection to skip claiming
/// fragments whose author already declared them as body.
///
/// `None` (no tag) returns `false` because absence of evidence does not imply
/// body. `Artifact` returns `false` because artifacts ARE page furniture.
fn struct_tag_is_body(tag: &Option<String>) -> bool {
    let Some(t) = tag.as_deref() else { return false };
    matches!(
        t,
        "P" | "Span"
            | "H"
            | "H1"
            | "H2"
            | "H3"
            | "H4"
            | "H5"
            | "H6"
            | "Title"
            | "L"
            | "LI"
            | "Lbl"
            | "LBody"
    )
}
```

Run `cargo test -p oxidize-pdf --lib partition::tests::classify_by_struct_tag` — all 7 tests pass.

- [ ] **Step 3: `cargo clippy --workspace --tests -p oxidize-pdf -- -D warnings`** clean.

- [ ] **Step 4: commit**
  - Message: `feat(partition): struct_tag classification helpers (addresses #271)`
  - Files: `oxidize-pdf-core/src/pipeline/partition.rs`

---

## Task 2: TDD — `ends_with_sentence_terminator` + `bold_short_title` + `numeric_prefix_title` heuristics

**Files:**
- Modify: `oxidize-pdf-core/src/pipeline/partition.rs`

- [ ] **Step 1: Add failing unit tests**

In the same `#[cfg(test)] mod tests` block, append:

```rust
fn frag(text: &str, bold: bool, font_size: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 12.0,
        font_size,
        font_name: None,
        is_bold: bold,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
        mcid: None,
        struct_tag: None,
    }
}

#[test]
fn ends_with_sentence_terminator_table() {
    assert!(ends_with_sentence_terminator("This is a paragraph."));
    assert!(ends_with_sentence_terminator("Really?"));
    assert!(ends_with_sentence_terminator("Stop!"));
    assert!(!ends_with_sentence_terminator("Section heading"));
    assert!(!ends_with_sentence_terminator("A2.a Risk Management"));
    assert!(!ends_with_sentence_terminator(""));
}

#[test]
fn bold_short_title_accepts_bold_short_no_terminator() {
    assert!(bold_short_title(&frag("Section Heading", true, 12.0)));
    assert!(bold_short_title(&frag("Principle A2", true, 11.0)));
}

#[test]
fn bold_short_title_rejects_non_bold() {
    assert!(!bold_short_title(&frag("Section Heading", false, 12.0)));
}

#[test]
fn bold_short_title_rejects_long_text() {
    let long = "x".repeat(150);
    assert!(!bold_short_title(&frag(&long, true, 12.0)));
}

#[test]
fn bold_short_title_rejects_sentence_with_period() {
    assert!(!bold_short_title(&frag("This is a complete sentence.", true, 12.0)));
}

#[test]
fn bold_short_title_rejects_empty() {
    assert!(!bold_short_title(&frag("   ", true, 12.0)));
}

#[test]
fn numeric_prefix_title_accepts_known_patterns() {
    let cases = &[
        "A2.a Risk Management Process",
        "A1.b Roles and Responsibilities",
        "1.1 Overview",
        "3.2.1 Detailed Requirements",
        "Section 4: Implementation",
        "Chapter 7 Conclusion",
        "IV. Findings",
    ];
    for c in cases {
        assert!(
            numeric_prefix_title(&frag(c, false, 12.0)),
            "should match: {}",
            c
        );
    }
}

#[test]
fn numeric_prefix_title_rejects_money_amount() {
    assert!(!numeric_prefix_title(&frag("1.2 million users were affected", false, 12.0)));
}

#[test]
fn numeric_prefix_title_rejects_version_string() {
    assert!(!numeric_prefix_title(&frag("version 3.0.1 release notes", false, 12.0)));
}

#[test]
fn numeric_prefix_title_rejects_lowercase_continuation() {
    assert!(!numeric_prefix_title(&frag("1. take action now", false, 12.0)));
}

#[test]
fn numeric_prefix_title_rejects_text_without_prefix() {
    assert!(!numeric_prefix_title(&frag("Overview of the system", false, 12.0)));
}

#[test]
fn numeric_prefix_title_rejects_too_long() {
    let mut s = String::from("A2.a ");
    s.push_str(&"X".repeat(220));
    assert!(!numeric_prefix_title(&frag(&s, false, 12.0)));
}
```

Run `cargo test -p oxidize-pdf --lib partition::tests::numeric_prefix` — fails (helpers don't exist).

- [ ] **Step 2: Implement helpers**

Add after `struct_tag_is_body`:

```rust
const MAX_HEADER_TEXT_LEN: usize = 100;
const MAX_BOLD_TITLE_LEN: usize = 120;
const MAX_NUMERIC_TITLE_LEN: usize = 200;

fn ends_with_sentence_terminator(s: &str) -> bool {
    matches!(s.chars().last(), Some('.') | Some('!') | Some('?'))
}

fn bold_short_title(f: &TextFragment) -> bool {
    if !f.is_bold {
        return false;
    }
    let trimmed = f.text.trim();
    let char_count = trimmed.chars().count();
    if char_count == 0 || char_count > MAX_BOLD_TITLE_LEN {
        return false;
    }
    !ends_with_sentence_terminator(trimmed)
}

/// Section-prefix regex matching:
/// - `A2.a Foo`, `A1.b Bar` (capital letter + digits + optional `.digit*` + optional lowercase letter)
/// - `1.1 Foo`, `3.2.1 Bar` (digits + optional `.digit*`)
/// - `Section 4: Foo`, `Chapter 7 Foo`
/// - `IV. Findings` (uppercase Roman numerals followed by `.`)
fn section_prefix_regex() -> &'static regex::Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"^([A-Z]\d+(\.\d+)*([a-z]\.?)?|\d+(\.\d+)*\.?|Section\s+\d+:?|Chapter\s+\d+:?|[IVX]+\.)\s+",
        )
        .expect("section_prefix_regex must compile")
    })
}

fn matches_section_prefix(s: &str) -> bool {
    section_prefix_regex().is_match(s)
}

fn strip_section_prefix(s: &str) -> &str {
    if let Some(m) = section_prefix_regex().find(s) {
        &s[m.end()..]
    } else {
        s
    }
}

fn numeric_prefix_title(f: &TextFragment) -> bool {
    let trimmed = f.text.trim();
    let char_count = trimmed.chars().count();
    if char_count == 0 || char_count > MAX_NUMERIC_TITLE_LEN {
        return false;
    }
    if !matches_section_prefix(trimmed) {
        return false;
    }
    let rest = strip_section_prefix(trimmed).trim_start();
    matches!(rest.chars().next(), Some(c) if c.is_uppercase())
}
```

**Dependency check:** verify `regex` is already in `oxidize-pdf-core/Cargo.toml`. If absent, add as a dev-and-runtime dep. (Run `grep regex oxidize-pdf-core/Cargo.toml` in Task setup before this step.)

Run `cargo test -p oxidize-pdf --lib partition::tests` — all 17 tests pass.

- [ ] **Step 3: clippy + commit**
  - Message: `feat(partition): heading detection heuristics (addresses #271)`

---

## Task 3: TDD — Header/Footer length cap + struct_tag gates

**Files:**
- Modify: `oxidize-pdf-core/src/pipeline/partition.rs`

- [ ] **Step 1: Failing unit tests** (in `mod tests`):

```rust
fn frag_at(text: &str, x: f64, y: f64, font_size: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width: 100.0,
        height: font_size,
        font_size,
        font_name: None,
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
        mcid: None,
        struct_tag: None,
    }
}

#[test]
fn header_zone_rejects_long_text() {
    // page_height=800, header_zone=0.05 → threshold y >= 760
    // Fragment at y=780 (in zone) but text = 200 chars → not Header.
    let long = "X".repeat(200);
    let frags = vec![frag_at(&long, 50.0, 780.0, 12.0)];
    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    let header_count = elements
        .iter()
        .filter(|e| matches!(e, Element::Header(_)))
        .count();
    assert_eq!(header_count, 0, "long text in header zone must not classify as Header");
}

#[test]
fn header_zone_accepts_short_text() {
    let frags = vec![frag_at("My Report 2026", 50.0, 780.0, 12.0)];
    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    assert!(
        elements.iter().any(|e| matches!(e, Element::Header(_))),
        "short text in header zone must classify as Header"
    );
}

#[test]
fn header_zone_rejects_p_struct_tag() {
    let mut f = frag_at("Short body text", 50.0, 780.0, 12.0);
    f.struct_tag = Some("P".to_string());
    let frags = vec![f];
    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    let header_count = elements
        .iter()
        .filter(|e| matches!(e, Element::Header(_)))
        .count();
    assert_eq!(header_count, 0, "struct_tag=P in header zone must not classify as Header");
}

#[test]
fn footer_zone_rejects_long_text() {
    let long = "X".repeat(200);
    let frags = vec![frag_at(&long, 50.0, 10.0, 12.0)];
    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    let footer_count = elements
        .iter()
        .filter(|e| matches!(e, Element::Footer(_)))
        .count();
    assert_eq!(footer_count, 0);
}
```

Run `cargo test header_zone` — failing (Header still claims long text).

- [ ] **Step 2: Modify Step 1 of `partition_fragments`**

Replace the body of the `if f.y >= header_threshold` branch (around lines 161-171):

```rust
if f.y >= header_threshold
    && f.text.chars().count() <= MAX_HEADER_TEXT_LEN
    && !struct_tag_is_body(&f.struct_tag)
{
    let zone_size = page_height * self.config.header_zone;
    let distance = f.y - header_threshold;
    let header_confidence = compute_zone_confidence(distance, zone_size);
    let mut meta = meta_from_fragment(f, page);
    meta.confidence = header_confidence;
    elements.push(Element::Header(ElementData {
        text: f.text.clone(),
        metadata: meta,
    }));
    claimed[i] = true;
}
```

And symmetrically for the footer branch:

```rust
} else if f.y + f.height <= footer_threshold
    && f.text.chars().count() <= MAX_HEADER_TEXT_LEN
    && !struct_tag_is_body(&f.struct_tag)
{
    let zone_size = page_height * self.config.footer_zone;
    let distance = footer_threshold - (f.y + f.height);
    let footer_confidence = compute_zone_confidence(distance, zone_size);
    let mut meta = meta_from_fragment(f, page);
    meta.confidence = footer_confidence;
    elements.push(Element::Footer(ElementData {
        text: f.text.clone(),
        metadata: meta,
    }));
    claimed[i] = true;
}
```

Note: keep the existing `else if` chain structure. The two `if`s should remain mutually exclusive within the same loop iteration.

Run `cargo test header_zone footer_zone` — all 4 tests pass.

- [ ] **Step 3: Run full library tests + clippy**
  - `cargo test -p oxidize-pdf --lib`
  - Watch for regressions in existing partition tests.

- [ ] **Step 4: commit**
  - Message: `fix(partition): length cap and struct_tag gate on Header/Footer (addresses #271)`

---

## Task 4: TDD — Step 0 struct_tag-driven classification + extended Title detection

**Files:**
- Modify: `oxidize-pdf-core/src/pipeline/partition.rs`

- [ ] **Step 1: Failing integration tests** added to `mod tests`:

```rust
#[test]
fn struct_tag_h1_yields_title_no_font_ratio_needed() {
    let mut f = frag_at("Section One", 50.0, 400.0, 12.0); // same as body font
    f.struct_tag = Some("H1".to_string());
    let frags = vec![f];
    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    assert_eq!(
        elements.iter().filter(|e| matches!(e, Element::Title(_))).count(),
        1,
        "H1-tagged fragment must classify as Title regardless of font size"
    );
}

#[test]
fn struct_tag_p_overrides_bold_short_heuristic() {
    // Bold + short would normally fire bold-short heuristic. struct_tag=P forces paragraph.
    let mut f = frag_at("Bold Short Text", 50.0, 400.0, 12.0);
    f.is_bold = true;
    f.struct_tag = Some("P".to_string());
    let frags = vec![f];
    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    assert!(elements.iter().any(|e| matches!(e, Element::Paragraph(_))));
    assert_eq!(
        elements.iter().filter(|e| matches!(e, Element::Title(_))).count(),
        0
    );
}

#[test]
fn bold_short_title_fires_without_struct_tag() {
    let mut f = frag_at("Risk Management", 50.0, 400.0, 12.0);
    f.is_bold = true;
    // no struct_tag, no font size elevation, no numeric prefix
    let frags = vec![f];
    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    assert_eq!(
        elements.iter().filter(|e| matches!(e, Element::Title(_))).count(),
        1,
        "bold-short heuristic must fire when no other signals present"
    );
}

#[test]
fn numeric_prefix_title_fires_without_bold() {
    let f = frag_at("A2.a Risk Management Process", 50.0, 400.0, 12.0);
    // no bold, no font elevation, no struct_tag
    let frags = vec![f];
    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    assert_eq!(
        elements.iter().filter(|e| matches!(e, Element::Title(_))).count(),
        1,
        "numeric-prefix heuristic must fire on NCSC-style sections"
    );
}

#[test]
fn struct_tag_li_yields_list_item() {
    let mut f = frag_at("Bullet content", 50.0, 400.0, 12.0);
    f.struct_tag = Some("LI".to_string());
    let frags = vec![f];
    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    assert_eq!(
        elements.iter().filter(|e| matches!(e, Element::ListItem(_))).count(),
        1
    );
}

#[test]
fn font_ratio_title_still_works() {
    // Verify the existing font-ratio path is not broken by adding new signals.
    // Body fragments at 12pt + one fragment at 20pt → 20pt one is Title.
    let mut frags = vec![];
    for i in 0..5 {
        frags.push(frag_at(&format!("body line {}", i), 50.0, 400.0 - (i as f64) * 15.0, 12.0));
    }
    frags.push(frag_at("Big Heading", 50.0, 500.0, 20.0));

    let partitioner = Partitioner::new(PartitionConfig::default());
    let elements = partitioner.partition_fragments(&frags, 0, 800.0);

    assert_eq!(
        elements.iter().filter(|e| matches!(e, Element::Title(_))).count(),
        1,
        "font-ratio Title path must still fire"
    );
}
```

Run these tests — must fail (struct_tag not consumed, heuristics not wired).

- [ ] **Step 2: Wire Step 0 — struct_tag-driven classification**

In `partition_fragments` immediately AFTER `let mut elements = Vec::new();` and BEFORE the Step 1 (Header/Footer) block, insert:

```rust
// 0. Struct-tag-driven classification (consumes TextFragment.struct_tag populated by #269 Phase 1).
//    Heading/List/ListItem tags are classified here with confidence 1.0 (structural ground truth);
//    other tags (P, Span, Figure, Table, …) fall through to the heuristic pipeline.
for (i, f) in fragments.iter().enumerate() {
    if claimed[i] {
        continue;
    }
    let Some(tag) = f.struct_tag.as_deref() else {
        continue;
    };
    match classify_by_struct_tag(tag) {
        Some(StructTagClass::Heading) => {
            let mut meta = meta_from_fragment(f, page);
            meta.confidence = 1.0;
            elements.push(Element::Title(ElementData {
                text: f.text.trim().to_string(),
                metadata: meta,
            }));
            claimed[i] = true;
        }
        Some(StructTagClass::ListItem) => {
            let mut meta = meta_from_fragment(f, page);
            meta.confidence = 1.0;
            elements.push(Element::ListItem(ElementData {
                text: f.text.trim().to_string(),
                metadata: meta,
            }));
            claimed[i] = true;
        }
        Some(StructTagClass::List) | Some(StructTagClass::Artifact) | None => {
            // `L` is an outer list container — children are LI; nothing to do here.
            // `Artifact` flows through to Header/Footer detection.
            // None: pass through.
        }
    }
}
```

- [ ] **Step 3: Extend Step 4 Title detection**

Replace the existing block at lines 334-346 with:

```rust
// 4. Title detection — three signals OR'd together; struct_tag=P/Span block heuristic fallbacks.
let struct_tag_blocks_heuristic = matches!(
    f.struct_tag.as_deref(),
    Some("P") | Some("Span")
);

let mut is_title = false;
let mut title_confidence = 0.0_f64;

// 4a. Font-ratio (existing logic)
if f.font_size >= title_threshold && f.font_size > body_font_size {
    is_title = true;
    let ratio = f.font_size / body_font_size;
    title_confidence = title_confidence
        .max(compute_title_confidence(ratio, self.config.title_min_font_ratio));
}

// 4b. Bold-short heuristic (only if struct_tag does not explicitly mark body)
if !struct_tag_blocks_heuristic && bold_short_title(f) {
    is_title = true;
    title_confidence = title_confidence.max(0.7);
}

// 4c. Numeric-prefix heuristic (only if struct_tag does not explicitly mark body)
if !struct_tag_blocks_heuristic && numeric_prefix_title(f) {
    is_title = true;
    title_confidence = title_confidence.max(0.8);
}

if is_title {
    let mut meta = meta;
    meta.confidence = title_confidence.clamp(0.5, 1.0);
    elements.push(Element::Title(ElementData {
        text: text.to_string(),
        metadata: meta,
    }));
    continue;
}
```

- [ ] **Step 4: Run unit tests**
  - `cargo test -p oxidize-pdf --lib partition` — all green.
  - Watch for regression in any existing partition test.

- [ ] **Step 5: clippy + commit**
  - Message: `feat(partition): consume struct_tag and add bold-short / numeric-prefix heading detectors (addresses #271)`

---

## Task 5: Integration test — synthetic tagged-PDF roundtrip

**Files:**
- Create: `oxidize-pdf-core/tests/partition_struct_tag_test.rs`

- [ ] **Step 1: Inspect writer API for marked-content emission**

Run `grep -n "begin_marked_content\|BDC\|H1" oxidize-pdf-core/src/writer/`. Identify the public path for writing `BDC /H1 ... EMC` blocks. (Phase 1 of #269 added this on the read side; verify writer-side availability before authoring the test.) If absent, fall back to a fixture-based approach: hand-craft a minimal tagged PDF byte string and ship it as `tests/fixtures/issue_271_h1_tagged.pdf`.

- [ ] **Step 2: Write the test**

The test exercises:
1. Open a PDF with a single page containing `BDC /H1 << /MCID 0 >> (Section One) Tj EMC` and `BDC /P << /MCID 1 >> (Body paragraph text here.) Tj EMC`.
2. Extract with default options (Artifact filter on; mcid+struct_tag carried through).
3. Pass to `Partitioner::new(PartitionConfig::default()).partition_fragments(...)`.
4. Assert: 1 `Element::Title` with `text == "Section One"`, 1 `Element::Paragraph` with `text` containing `"Body paragraph"`. Zero `Element::Header`.

If writer API not exposed: ship the PDF as a fixture and load it. Use a minimal PDF (no compression) so it's reviewable.

- [ ] **Step 3: Run + commit**
  - Message: `test(partition): struct_tag → Title integration test (addresses #271)`

---

## Task 6: NCSC real-corpus tests

**Files:**
- Create: `oxidize-pdf-core/tests/partition_ncsc_classifier_test.rs`

- [ ] **Step 1: Mirror the pattern of `ncsc_no_alphabet_soup_test.rs`** for skip-if-missing behavior:

```rust
//! Issue #271 — NCSC CAF v4.0 partitioner classifier verification.
//!
//! Fixture: `corpus_cache/e0e3ff11371c09c2.pdf`. If missing, the tests
//! `eprintln!` a skip notice and return — they do NOT fail (matches the
//! pattern of `ncsc_no_alphabet_soup_test.rs`).

use std::path::PathBuf;

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::{Element, PartitionConfig, Partitioner};
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};

const NCSC_FIXTURE: &str = "corpus_cache/e0e3ff11371c09c2.pdf";

fn ncsc_path() -> Option<PathBuf> {
    let p = PathBuf::from(NCSC_FIXTURE);
    if p.exists() { Some(p) } else { None }
}

fn extract_and_partition(path: &PathBuf) -> Vec<Element> {
    let reader = PdfReader::open(path).expect("open NCSC corpus");
    let document = PdfDocument::new(reader);
    let mut all_elements = Vec::new();
    let extractor = TextExtractor::with_options(ExtractionOptions::default());
    let page_count = document.page_count().expect("page_count");
    let partitioner = Partitioner::new(PartitionConfig::default());

    for page_idx in 0..page_count {
        let page = document.get_page(page_idx).expect("get_page");
        let height = page.height();
        let extracted = extractor.extract_from_page(&document, page_idx).expect("extract");
        let elements = partitioner.partition_fragments(&extracted.fragments, page_idx as u32, height);
        all_elements.extend(elements);
    }
    all_elements
}

#[test]
fn ncsc_caf_v4_yields_at_least_30_titles() {
    let Some(path) = ncsc_path() else {
        eprintln!("ncsc_caf classifier test: corpus missing, skipping");
        return;
    };
    let elements = extract_and_partition(&path);
    let title_count = elements
        .iter()
        .filter(|e| matches!(e, Element::Title(_)))
        .count();
    assert!(
        title_count >= 30,
        "NCSC CAF v4.0 must yield ≥30 Titles (got {})",
        title_count
    );
}

#[test]
fn ncsc_caf_v4_no_body_text_in_headers() {
    let Some(path) = ncsc_path() else {
        eprintln!("ncsc_caf classifier test: corpus missing, skipping");
        return;
    };
    let elements = extract_and_partition(&path);
    let long_headers: Vec<&Element> = elements
        .iter()
        .filter(|e| matches!(e, Element::Header(_)))
        .filter(|e| e.text().chars().count() > 100)
        .collect();
    assert!(
        long_headers.is_empty(),
        "NCSC CAF v4.0 must not classify long body text as Header (found {} offenders, e.g. \"{}\")",
        long_headers.len(),
        long_headers.first().map(|e| e.text()).unwrap_or("")
    );
}

#[test]
fn ncsc_caf_v4_contains_principle_titles() {
    let Some(path) = ncsc_path() else {
        eprintln!("ncsc_caf classifier test: corpus missing, skipping");
        return;
    };
    let elements = extract_and_partition(&path);
    let has_principle = elements.iter().any(|e| match e {
        Element::Title(d) => d.text.starts_with("Principle "),
        _ => false,
    });
    assert!(has_principle, "NCSC CAF v4.0 must contain at least one 'Principle Ax' Title");
}

#[test]
fn ncsc_caf_v4_contains_section_titles() {
    let Some(path) = ncsc_path() else {
        eprintln!("ncsc_caf classifier test: corpus missing, skipping");
        return;
    };
    let elements = extract_and_partition(&path);
    let has_section = elements.iter().any(|e| match e {
        Element::Title(d) => {
            // Section pattern: "A2.a", "A1.b", "A3.c", etc.
            let trimmed = d.text.trim();
            trimmed.len() >= 4 && {
                let bytes = trimmed.as_bytes();
                bytes[0].is_ascii_uppercase()
                    && bytes[1].is_ascii_digit()
                    && bytes[2] == b'.'
                    && bytes[3].is_ascii_lowercase()
            }
        }
        _ => false,
    });
    assert!(has_section, "NCSC CAF v4.0 must contain at least one 'Ax.y' section Title");
}
```

**Note on imports:** confirm exact `TextExtractor` API. If `extract_from_page` doesn't exist (Phase 1 may have a different name), adapt to the actual signature.

- [ ] **Step 2: Run**
  - `cargo test -p oxidize-pdf --test partition_ncsc_classifier_test -- --nocapture`

- [ ] **Step 3: commit**
  - Message: `test(partition): NCSC CAF v4.0 classifier real-corpus tests (#271)`

---

## Task 7: Regression tests (Higgs, BSI, ENS)

**Files:**
- Create: `oxidize-pdf-core/tests/partition_classifier_regression_test.rs`

- [ ] **Step 1: Implement parameterized regression tests**

Use the same `extract_and_partition` helper pattern (extract to a shared `tests/common/` module if helpful, or inline). For each cached fixture:

| Slug | Hash | Min Title Count |
|---|---|---|
| higgs | `b9cf1a025b683adf` | 100 |
| bsi-tr-02102 | `6a001a0684cd51ca` | 80 |
| ens | `6320a941c903a04f` | 3 |

(Note: hashes need verification — they're the SHA1 of the URL, computable from `examples/rag_realworld.rs`. Run `grep "ens\|higgs\|bsi\|ncsc" examples/rag_realworld.rs` and `xxd corpus_cache/*.pdf | head -1` if needed to confirm mapping. The NCSC hash `e0e3ff11371c09c2` is already known.)

Tests:
- `higgs_yields_at_least_100_titles`
- `bsi_yields_at_least_80_titles`
- `ens_yields_at_least_3_titles`
- `no_long_headers_on_tagged_corpora` — sum `Header(text > 100)` across all three; assert `<5%` of total `Header` count, or `== 0` if total Header count is 0.

- [ ] **Step 2: Run + commit**
  - Message: `test(partition): regression guards for Higgs/BSI/ENS Title counts (#271)`

---

## Task 8: End-to-end — rag_realworld + JSONL inspection

**Files:**
- Modify (optional): `oxidize-pdf-core/tests/rag_realworld_jsonl_test.rs` if it exists.

- [ ] **Step 1: Run `cargo run --release --example rag_realworld`** and capture output.

- [ ] **Step 2: Verify summary line for NCSC** shows `>= 30 headings` (previously `0 headings`).

- [ ] **Step 3: Verify summary lines for other corpora** show counts in the bands established by Task 7.

- [ ] **Step 4: Inspect `out/ncsc-caf.jsonl`** with `jq`:
  - Heading chunks count >= 30.
  - Long-text chunks (text length > 100) tagged `header` count == 0.

- [ ] **Step 5: Save log to `.private/` (not committed)** for the PR body:
  - `.private/rag_realworld_after_271_fix.log`

---

## Task 9: Full test suite + clippy + corpus tiers

- [ ] **Step 1: `nice -n 10 cargo test -p oxidize-pdf --lib --no-fail-fast`** — clean.
- [ ] **Step 2: `nice -n 10 cargo test -p oxidize-pdf --tests --no-fail-fast`** — all integration tests green. Do NOT rely on the summary alone; scan for `FAILED`, `panicked`, `test result: ok`.
- [ ] **Step 3: `cargo test -p oxidize-pdf --test t1_spec`** (if present + corpus available) — pass rate ≥99% within timeouts.
- [ ] **Step 4: `cargo test -p oxidize-pdf --test t3_stress`** (if present + corpus available) — 0 panics, 0 timeouts.
- [ ] **Step 5: `cargo clippy --workspace --tests -- -D warnings`** — clean.

If any tier regresses, do NOT proceed to push/PR. Stop and diagnose.

---

## Task 10: Verification + PR draft

- [ ] **Step 1: `git log --oneline develop..HEAD`** — review commit sequence:
  - feat(partition): struct_tag classification helpers
  - feat(partition): heading detection heuristics
  - fix(partition): length cap and struct_tag gate on Header/Footer
  - feat(partition): consume struct_tag and add heading heuristics in classifier
  - test(partition): struct_tag → Title integration test
  - test(partition): NCSC CAF v4.0 classifier real-corpus tests
  - test(partition): regression guards for Higgs/BSI/ENS Title counts

- [ ] **Step 2: Draft PR body** in `.private/pr_271_body.md` (NOT committed):
  - Summary: closes #271 (no auto-close keyword; use "addresses #271").
  - Before/after table on NCSC (115 → 0 long headers, 0 → ≥30 titles).
  - Regression numbers for Higgs/BSI/ENS.
  - Detailed changes per file.
  - Test plan executed.

- [ ] **Step 3: STOP and request authorization** before:
  - `git push -u origin fix/issue-271-partitioner-classifier`
  - `gh pr create`
  - Any GitHub issue comment.

Per `feedback_no_auto_issue_comments.md`: never auto-close. Use "addresses #271", not "closes/fixes/resolves".

---

## Done criteria

All of:

- 10 tasks above marked complete.
- `cargo test --workspace --no-fail-fast` clean (lib + tests).
- `cargo clippy --workspace --tests -- -D warnings` clean.
- NCSC: `≥30 Titles, 0 long-text Headers`.
- Higgs/BSI/ENS: Title counts within bands; no regression on Header misclassification ratio.
- `rag_realworld 5/5` succeeds.
- Spec + plan committed.
- PR body drafted in `.private/`; PR NOT opened without user authorization.
