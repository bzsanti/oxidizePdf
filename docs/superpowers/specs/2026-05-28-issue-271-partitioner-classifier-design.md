# Partitioner classifier — struct_tag consumption + heuristic heading detector + Header recalibration (#271)

**Date:** 2026-05-28
**Branch:** `fix/issue-271-partitioner-classifier`
**Closes:** [#271](https://github.com/bzsanti/oxidizePdf/issues/271)
**Scope:** Partitioner (`oxidize-pdf-core/src/pipeline/partition.rs`) only. No changes to extractor, parser, chunker, or writer.

## Context

`pipeline/partition.rs` produces two failure modes on the NCSC Cyber Assessment Framework v4.0 corpus that the issue documents:

- **Bug A — body misclassified as `Element::Header`.** Lines 152-185 run header detection FIRST, before table detection, with only one signal: `f.y >= page_height * (1.0 - header_zone)` (default `header_zone = 0.05`). Any fragment that sits in the top 5% of the page is claimed as Header regardless of length, regardless of whether other fragments below it form a tightly-packed table region. NCSC's assessment tables ("Achieved / Partially / Not Achieved" cells of sections A1/A2/A3) begin within the top 5%, so the topmost row of each table — 150-210 char paragraph statements — gets reclassified as a running header and removed from the table detector's input. Measured on develop HEAD: 115 of 184 NCSC chunks tagged `element_types: ["header"]`; 75 of those have text length > 100 chars.

- **Bug B — 0 `Element::Title` (the "heading" tag in `element_types`).** Title detection at lines 334-346 requires `font_size >= body_font_size * 1.3 && > body_font_size`. NCSC section headings (`"Principle A2 Risk Management"`, `"A2.a Risk Management Process"`, `"A1.b Roles and Responsibilities"`) are emitted bold but at the same font size as body text. They never satisfy the ratio gate. Furthermore, the partitioner does not consume `TextFragment.struct_tag` at all — verified with `grep`. The field is populated by #269 Phase 1 but its use was deferred to a hypothetical Phase 3.

Joint effect: on a 60-page government / compliance PDF with clear hierarchical structure, the partitioner reports 0 headings and 115 body paragraphs reclassified as page furniture. Downstream, the chunker emits chunks without `parent_heading`, RAG retrieval loses the section context, and the document is effectively flat.

## Goals

- Close #271.
- Honor `TextFragment.struct_tag` ("H1".."H6", "Title", "P", "L", "LI", "Artifact", "Figure", "Table", "Span") as the primary classification signal when present. Tagged PDFs that ship structural metadata get reading-aware classification for free.
- Add a heuristic heading detector that activates when `struct_tag` is absent or ambiguous. Signals: bold-and-short, alphanumeric section prefix (`A2.a`, `1.1`, `IV.`), standalone non-terminated line.
- Recalibrate Header detection so that a 200-char paragraph in the top 5% of the page is not classified as a running header.
- Preserve existing behavior for the 4 corpora that already work (Higgs 205 headings, BSI 125, ENS 3, BOE-Sumario 0 — BOE is a separate font-encoding root cause, not a classifier issue).

## Non-Goals

- Parsing `StructTreeRoot` / `ParentTree` from the document catalog. The `struct_tag` already on `TextFragment` is the only structural input. (Phase 2 of the Tagged-PDF initiative.)
- BOE-Sumario heading detection — root cause is CFF font encoding (separate issue, not classifier).
- Multi-line title detection (a heading split across two fragments).
- Cross-page running-header detection (compare same text at same Y across consecutive pages). Out of scope; future work.
- Image / Figure / Code element classification beyond pass-through.
- Reading-order changes.

## Success Criteria

All of the following must hold, verified by content-checking tests (no smoke tests per `CLAUDE.md`).

### Functional

1. **Struct-tag Title.** A synthetic PDF with `BDC /H1 << /MCID 0 >> ... EMC` containing `"Section Title"` (no font ratio elevation) produces `Element::Title` with `text == "Section Title"`.
2. **Struct-tag H1..H6 all map to Title.** Same behavior for `H2`, `H3`, `H4`, `H5`, `H6`, `Title`. `H` (untagged-level heading per PDF 32000-1) maps to Title.
3. **Struct-tag Paragraph short-circuit.** `BDC /P ... EMC` with bold short text does NOT classify as Title via the heuristic fallback; struct_tag takes precedence.
4. **NCSC heading heuristic.** On NCSC CAF v4.0 (`corpus_cache/e0e3ff11371c09c2.pdf`), the partitioner produces **at least 30** `Element::Title` instances. Each must contain at least one of: a top-level "Principle Ax" label, an "Ax.y" sub-section label, an "Ax.z" sub-sub-section label.
5. **Bold-short heuristic.** A synthetic fragment with `is_bold=true`, length ≤ 80 chars, no terminal `.`/`!`/`?`, font_size = body_size produces `Element::Title`.
6. **Numeric prefix heuristic.** A non-bold fragment matching one of `^[A-Z]?\d+(\.\d+)*([a-z]\.?)?\s+\S` produces `Element::Title`. Examples that must match: `"A2.a Risk Management"`, `"1.1 Overview"`, `"3.2.1 Details"`, `"Section 4: Implementation"`. Examples that must NOT match: `"1.2 million"`, `"version 3.0.1"` (no trailing word), `"step 1. take action"` (lowercase verb after).
7. **NCSC body NOT classified as Header.** On NCSC, **zero** `Element::Header` whose `text.chars().count() > 100`. Header detection rejects long fragments regardless of Y position.
8. **Header length cap.** Synthetic fragment at `y = page_height * 0.97`, length = 250 chars: classified as `Paragraph`, not Header.
9. **Real headers still detected.** Synthetic fragment at `y = page_height * 0.97`, length = 18 chars (`"My Report — 2026"`): classified as `Header`.

### Regression (non-NCSC corpora)

10. **rag_realworld 5/5.** `examples/rag_realworld.rs` continues to process 5/5 PDFs without panic.
11. **Higgs Titles ≥ 100.** (Baseline 205 on develop. Floor enforced at 100 to leave room for incidental changes.)
12. **BSI Titles ≥ 80.** (Baseline 125.)
13. **ENS Titles ≥ 3.** (Baseline 3.)
14. **No new Header misclassifications.** On Higgs+BSI+ENS combined: ratio of `Element::Header` with `text > 100 chars` divided by total `Element::Header` count must stay below 5%. (Baseline is already low on tagged corpora; guard is for regression.)

### Suite

15. `cargo test --workspace --no-fail-fast` passes. No existing test deleted, weakened, or marked `#[ignore]`.
16. `cargo clippy --workspace --tests -- -D warnings` clean on touched files.

## Architecture

The partition pipeline order changes from:

```
1. Header/Footer (Y position only)
2. Tables
3. Key-Value
4. Title (font ratio)
5. List
6. Paragraph (default)
```

to:

```
0. struct_tag-driven classification  [NEW]
   - H1..H6 / H / Title  → Element::Title
   - L / LI              → Element::ListItem
   - Artifact            → flows into step 1 (Header/Footer)
   - P / Span / (none)   → flow into steps 1..6
1. Header/Footer (Y position AND length cap AND struct_tag check)
2. Tables                                    (unchanged)
3. Key-Value                                 (unchanged)
4. Title — font ratio OR bold-short OR numeric-prefix  [EXTENDED]
5. List                                      (unchanged)
6. Paragraph
```

### Step 0: struct_tag classification

```rust
fn classify_by_struct_tag(tag: &str) -> Option<StructTagClass> {
    match tag {
        "H" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6" | "Title" => Some(StructTagClass::Heading),
        "L" => Some(StructTagClass::List),       // outer list — children handled per-fragment
        "LI" | "Lbl" | "LBody" => Some(StructTagClass::ListItem),
        "Artifact" => Some(StructTagClass::Artifact),
        _ => None, // P, Span, Figure, Table, Caption, etc.: fall through to heuristics
    }
}
```

Fragments tagged `Heading` are emitted as `Element::Title` and removed from the unclaimed pool before steps 1-6. Confidence is set to `1.0` (structural ground truth) regardless of font size.

`Artifact` fragments are not removed pre-emptively. Step 1 (Header/Footer) gives them priority — they almost always represent running headers/footers in practice. If step 1 doesn't claim them (e.g. Y position is mid-page), they fall through to paragraph classification, which is acceptable because the extractor already filters `Artifact` by default unless `include_artifacts = true`.

### Step 1: Header/Footer recalibration

Two new gates added to the existing zone check:

```rust
if f.y >= header_threshold
    && f.text.chars().count() <= MAX_HEADER_TEXT_LEN  // NEW gate
    && !struct_tag_is_body(&f.struct_tag)             // NEW gate
{
    // classify as Header
}
```

Constants:

```rust
const MAX_HEADER_TEXT_LEN: usize = 100;  // 100 chars ≈ one typical wrapped header line
```

`struct_tag_is_body` returns true for `Some("P")`, `Some("Span")`, `Some(h)` where `h` matches H1..H6 (a heading is never a Header), `Some("L")`, `Some("LI")`. Returns false for `None` (no signal) or `Some("Artifact")`.

Footer detection gets the symmetric treatment.

### Step 4: Title detection — multi-signal

Replaces single `font_size >= title_threshold` test with an `or` chain:

```rust
let is_title = font_ratio_title(f, body_font_size, title_threshold)
    || bold_short_title(f)
    || numeric_prefix_title(f);

if is_title {
    // emit Element::Title with combined confidence
}
```

`bold_short_title`:

```rust
fn bold_short_title(f: &TextFragment) -> bool {
    if !f.is_bold { return false; }
    let trimmed = f.text.trim();
    let char_count = trimmed.chars().count();
    if char_count == 0 || char_count > MAX_BOLD_TITLE_LEN { return false; }
    if ends_with_sentence_terminator(trimmed) { return false; }
    true
}

const MAX_BOLD_TITLE_LEN: usize = 120;

fn ends_with_sentence_terminator(s: &str) -> bool {
    matches!(s.chars().last(), Some('.') | Some('!') | Some('?'))
}
```

`numeric_prefix_title`:

```rust
fn numeric_prefix_title(f: &TextFragment) -> bool {
    let trimmed = f.text.trim();
    let char_count = trimmed.chars().count();
    if char_count == 0 || char_count > MAX_NUMERIC_TITLE_LEN { return false; }
    if !matches_section_prefix(trimmed) { return false; }
    // Require a following word with capital letter (rules out "1.2 million", "version 3.0.1")
    let rest = strip_section_prefix(trimmed);
    let first_char = rest.trim_start().chars().next();
    matches!(first_char, Some(c) if c.is_uppercase())
}

const MAX_NUMERIC_TITLE_LEN: usize = 200;
```

Section prefix regex (compiled once with `OnceLock`):

```
^([A-Z]\d+(\.\d+)*([a-z]\.?)?|\d+(\.\d+)*\.?|Section\s+\d+:?|Chapter\s+\d+:?|[IVX]+\.)\s+
```

Matches:
- `A2.a Risk Management` ✓
- `1.1 Overview` ✓
- `3.2.1 Details` ✓
- `Section 4: Implementation` ✓
- `IV. Findings` ✓

Rejects:
- `1.2 million users` → `million` lowercase first char → rejected
- `version 3.0.1` → no leading prefix match
- `step 1. take action` → first char `t` lowercase → rejected

### Confidence

Struct-tag-driven Title: `1.0`.
Font-ratio Title: existing `compute_title_confidence`.
Bold-short Title: `0.7`.
Numeric-prefix Title: `0.8`.
Multi-signal Title (e.g. bold + numeric prefix): `clamp(0.7 + 0.2 + bonus, 0.5, 1.0)` capped at 1.0.

Header (zone + length cap satisfied): unchanged `compute_zone_confidence`.

## Data structures

No new public API surface. Internal helpers added to `partition.rs`:

```rust
enum StructTagClass {
    Heading,
    List,
    ListItem,
    Artifact,
}

fn classify_by_struct_tag(tag: &str) -> Option<StructTagClass>;
fn struct_tag_is_body(tag: &Option<String>) -> bool;
fn bold_short_title(f: &TextFragment) -> bool;
fn numeric_prefix_title(f: &TextFragment) -> bool;
fn matches_section_prefix(s: &str) -> bool;  // OnceLock-cached regex
fn strip_section_prefix(s: &str) -> &str;
fn ends_with_sentence_terminator(s: &str) -> bool;
```

`Element::Title` already carries `parent_heading` via the post-classification pass — no change needed.

## Test plan

Content-verifying tests only (per `CLAUDE.md`). All in `oxidize-pdf-core/tests/`.

### Unit tests (helpers) — `partition_classifier_test.rs` (NEW)

- `struct_tag_h1_h2_h3_map_to_title` — table-driven, all `H1..H6` + `H` + `Title`.
- `struct_tag_li_maps_to_list_item`.
- `struct_tag_p_falls_through_to_heuristic` — `/P` tagged bold text does not become Title via the bold-short heuristic if explicitly tagged P. (Test verifies struct_tag precedence.)
- `bold_short_classifies_as_title` — bold, 50 chars, no terminator → Title.
- `bold_long_does_not_classify_as_title` — bold, 200 chars → Paragraph.
- `bold_with_period_does_not_classify_as_title` — bold short ending in `.` → Paragraph.
- `numeric_prefix_classifies_as_title` — table: `A2.a Risk`, `1.1 Overview`, `Section 4: Imp`, `IV. Findings`.
- `numeric_prefix_rejects_money_amount` — `1.2 million dollars` → not Title.
- `numeric_prefix_rejects_version_string` — `version 3.0.1` → not Title.
- `header_zone_with_long_text_rejected` — `y` in top 5%, 200 chars → Paragraph.
- `header_zone_with_short_text_accepted` — `y` in top 5%, 18 chars → Header.
- `header_zone_with_p_struct_tag_rejected` — `y` in top 5%, 30 chars, struct_tag=`P` → Paragraph.

### Integration — `partition_struct_tag_test.rs` (NEW)

- `extract_then_partition_h1_tagged_pdf_yields_title` — build minimal tagged PDF via writer with `<H1>Section</H1>`, extract, partition, assert `Element::Title`.
- `extract_then_partition_artifact_tagged_excluded_from_titles` — `<Artifact>Page 1</Artifact>` does not become Title.

### Real-corpus — `partition_ncsc_classifier_test.rs` (NEW)

Cached fixture `corpus_cache/e0e3ff11371c09c2.pdf`. Skip with `eprintln!` (not fail) if not present, matching the `ncsc_no_alphabet_soup_test.rs` pattern.

- `ncsc_caf_v4_yields_at_least_30_titles` — open NCSC, partition all pages, count `Element::Title`, assert >= 30.
- `ncsc_caf_v4_no_body_text_in_headers` — count `Element::Header` with `chars().count() > 100`, assert 0.
- `ncsc_caf_v4_contains_principle_titles` — assert at least one Title text starts with `"Principle "`.
- `ncsc_caf_v4_contains_section_titles` — assert at least one Title text matches the `Ax.y` section prefix.

### Regression — `partition_classifier_regression_test.rs` (NEW)

Each test conditional on fixture in `corpus_cache/`.

- `higgs_yields_at_least_100_titles`.
- `bsi_yields_at_least_80_titles`.
- `ens_yields_at_least_3_titles`.
- `no_long_text_misclassified_as_header_higgs_bsi_ens` — combined ratio of `Header(text > 100)` / total `Header` < 5%.

### End-to-end — extend `rag_realworld_jsonl_test.rs` (if present) or add to integration

- `rag_realworld_ncsc_emits_at_least_30_heading_chunks` — run the example pipeline, count chunks with `element_types: ["heading"]` or `["title"]` (verify mapping in `RagChunk`), assert >= 30.

Total: 25 tests, all content-verifying.

## Edge cases and rejections

| Input | Expected | Reason |
|---|---|---|
| `font_size = 24, bold, 30 chars, struct_tag = "P"` | Paragraph | struct_tag takes precedence (caller said "this is body") |
| `font_size = 12, bold, 80 chars, struct_tag = None, no terminator` | Title (bold-short) | heuristic fallback |
| `font_size = 12, bold, 80 chars, struct_tag = "H2"` | Title | struct_tag confirms |
| `font_size = 12, not bold, 80 chars, prefix "A2.a Section Name"` | Title (numeric-prefix) | heuristic fallback |
| `font_size = 18, struct_tag = "H1"` | Title (confidence 1.0) | structural |
| `text = "1.2 million users in 2025"` | Paragraph | prefix matches but next word is lowercase |
| `y in top 5%, 200 chars, struct_tag = None` | Paragraph | header length cap |
| `y in top 5%, 50 chars, struct_tag = "P"` | Paragraph | struct_tag says body |
| `y in top 5%, 50 chars, struct_tag = "Artifact"` | Header | struct_tag confirms furniture |

## Rollback

Single file (`partition.rs`) + new test files. Revert by:

```
git revert <fix-commit-sha>
```

No data migration, no public API surface change, no Cargo.toml change.

## Open questions

None. All design decisions are local to the partitioner and tested.

## Out of scope (tracked elsewhere)

- BOE-Sumario heading detection (font encoding, separate issue).
- Multi-fragment heading (heading wraps across two `TextFragment`s).
- Cross-page running-header detection.
- StructTreeRoot parsing (Phase 2 of Tagged-PDF initiative).
