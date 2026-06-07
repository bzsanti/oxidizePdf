# Ruling-based Table Detection in Partition — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the existing ruling-based `TableDetector` into the partition pipeline as the primary path for bordered tables (page-level, grid-first), with the spatial detector filling the remaining fragments.

**Architecture:** A new `prefer_ruling_tables` config (default true) makes `do_partition_pages` extract per-page vector graphics and pass them to a new non-breaking `Partitioner::partition_fragments_with_graphics`. When the page has table-grid line structure, `TableDetector` runs first, emits bordered tables, and claims their fragments; the existing spatial detector then runs on the unclaimed remainder.

**Tech Stack:** Rust (MSRV 1.88), `cargo test`. Tests are integration-level in `oxidize-pdf-core/tests/`, content-verifying per `CLAUDE.md` (no smoke tests). Spike-verified 2026-06-06: a writer-generated bordered table round-trips to `has_table_structure()==true` (H=18,V=18) and `TableDetector` reconstructs a 3×3 grid at confidence 1.00 with exact cell text, **when text is extracted with `preserve_layout: true`** (the partition pipeline already uses that).

---

## Verified interfaces (do not re-derive)

- `Partitioner::partition_fragments(&self, fragments: &[TextFragment], page: u32, page_height: f64) -> Vec<Element>` — `pipeline/partition.rs:122`. Table block at lines 246–310.
- `PartitionConfig` — `pipeline/partition.rs:21`; `Default` at line 44 (`detect_tables: true`, `min_table_confidence: 0.5`).
- `TableDetector::default()`, `TableDetector::detect(&self, graphics: &ExtractedGraphics, text_fragments: &[TextFragment]) -> Result<Vec<DetectedTable>, TableDetectionError>` — `text/table_detection.rs`.
- `DetectedTable { bbox: BoundingBox, cells: Vec<TableCell>, rows: usize, columns: usize, confidence: f64 }`; `TableCell { row: usize, column: usize, bbox, text: String, has_borders: bool }`; `BoundingBox { x, y, width, height }`.
- `ExtractedGraphics { lines: Vec<VectorLine>, horizontal_count, vertical_count }` + `has_table_structure(&self) -> bool` — `graphics/extraction.rs`. `GraphicsExtractor::default()`, `extract_from_page<R>(&mut self, &PdfDocument<R>, page_index: usize) -> Result<ExtractedGraphics, ExtractionError>`.
- `VectorLine::new(...)` — `graphics/extraction.rs:100` (used to hand-build graphics in a unit test; the implementer reads its exact signature there).
- `TableElementData { rows: Vec<Vec<String>>, metadata: ElementMetadata }` — `pipeline/element.rs:211`.
- `do_partition_pages(&self, options, config)` — `parser/document.rs:1565`; loop calls `partitioner.partition_fragments(&page_text.fragments, page_idx_u32, page_height)`. Public entries: `Document::partition()` (1532), `partition_with(options, config)` (1537).
- Writer table: `oxidize_pdf::text::Table::with_equal_columns(cols, total_width)`, `set_position(x, y)`, `add_row(Vec<String>) -> Result<...>`; `Page::add_table(&Table)`. Default borders on.

---

## File Structure

- `oxidize-pdf-core/src/pipeline/partition.rs` — add `prefer_ruling_tables` config field; split `partition_fragments` into a `_with_graphics` variant + delegator; add ruling-first block + `ruling_table_to_rows` helper.
- `oxidize-pdf-core/src/parser/document.rs` — plumb per-page graphics in `do_partition_pages`.
- `oxidize-pdf-core/tests/ruling_table_partition_test.rs` — new integration tests (synthetic round-trip, hand-built multi-line, full pipeline, flag-off, borderless fallback). Shared PDF-builder helper lives here.

---

## Task 1: `prefer_ruling_tables` config field

**Files:** Modify `oxidize-pdf-core/src/pipeline/partition.rs` (struct ~21, `Default` ~44).

- [ ] **Step 1: Write the failing test**

Append to a new file `oxidize-pdf-core/tests/ruling_table_partition_test.rs`:

```rust
use oxidize_pdf::pipeline::PartitionConfig;

#[test]
fn prefer_ruling_tables_defaults_on() {
    assert!(PartitionConfig::default().prefer_ruling_tables);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test prefer_ruling_tables_defaults_on 2>&1 | head -20`
Expected: compile error — no field `prefer_ruling_tables` on `PartitionConfig`.

- [ ] **Step 3: Add the field and default**

In the `PartitionConfig` struct (after `min_table_confidence`):

```rust
    /// Prefer the ruling-based (vector-grid) table detector for bordered tables,
    /// falling back to the spatial detector for the rest. When false, only the
    /// spatial detector runs and no page graphics are extracted. Default: true.
    pub prefer_ruling_tables: bool,
```

In `impl Default for PartitionConfig` (alongside `min_table_confidence: 0.5,`):

```rust
            prefer_ruling_tables: true,
```

If `PartitionConfig` is also constructed by struct literal in profile presets (search `PartitionConfig {` across `src/`), add `prefer_ruling_tables: true,` to each, or change them to `..Default::default()` if they already partially do. Verify with: `grep -rn "PartitionConfig {" oxidize-pdf-core/src/`.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test prefer_ruling_tables_defaults_on`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/pipeline/partition.rs oxidize-pdf-core/tests/ruling_table_partition_test.rs
git commit -m "feat(rag): add prefer_ruling_tables config (default true) (#292)"
```

---

## Task 2: Non-breaking `partition_fragments_with_graphics`

**Files:** Modify `oxidize-pdf-core/src/pipeline/partition.rs`.

- [ ] **Step 1: Write the failing test**

Append to `oxidize-pdf-core/tests/ruling_table_partition_test.rs`:

```rust
use oxidize_pdf::pipeline::Partitioner;
use oxidize_pdf::text::TextFragment;

#[test]
fn with_graphics_none_matches_legacy_partition() {
    // A trivial fragment set; both entry points must produce identical elements.
    let frags: Vec<TextFragment> = vec![];
    let p = Partitioner::new(PartitionConfig::default());
    let legacy = p.partition_fragments(&frags, 1, 800.0);
    let with_graphics = p.partition_fragments_with_graphics(&frags, None, 1, 800.0);
    assert_eq!(legacy.len(), with_graphics.len());
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test with_graphics_none_matches_legacy_partition 2>&1 | head -20`
Expected: compile error — no method `partition_fragments_with_graphics`.

- [ ] **Step 3: Refactor `partition_fragments` into a `_with_graphics` variant + delegator**

At the top of `partition.rs`, add the import:

```rust
use crate::graphics::extraction::ExtractedGraphics;
```

Rename the existing `pub fn partition_fragments(&self, fragments: &[TextFragment], page: u32, page_height: f64) -> Vec<Element>` to:

```rust
    pub fn partition_fragments_with_graphics(
        &self,
        fragments: &[TextFragment],
        graphics: Option<&ExtractedGraphics>,
        page: u32,
        page_height: f64,
    ) -> Vec<Element> {
```

(Keep the entire existing body unchanged for now; `graphics` is unused this task — prefix with `_` ONLY if clippy complains, but Task 3 uses it, so add `#[allow(unused_variables)]` is NOT needed; instead leave it and complete Task 3 in the same PR. If committing Task 2 alone triggers `-D warnings` on unused `graphics`, bind it: `let _ = graphics;` at the top of the body, removed in Task 3.)

Add the delegator immediately above it:

```rust
    /// Partition fragments without page graphics (spatial table detection only).
    pub fn partition_fragments(
        &self,
        fragments: &[TextFragment],
        page: u32,
        page_height: f64,
    ) -> Vec<Element> {
        self.partition_fragments_with_graphics(fragments, None, page, page_height)
    }
```

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test with_graphics_none_matches_legacy_partition`
Expected: PASS.
Run: `cargo build -p oxidize-pdf --lib` — Expected: Finished, warning-clean.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/pipeline/partition.rs oxidize-pdf-core/tests/ruling_table_partition_test.rs
git commit -m "refactor(rag): add partition_fragments_with_graphics delegator (#292)"
```

---

## Task 3: Ruling-first detection + mapping helper

**Files:** Modify `oxidize-pdf-core/src/pipeline/partition.rs`; add tests.

- [ ] **Step 1: Write the failing tests**

Append to `oxidize-pdf-core/tests/ruling_table_partition_test.rs`. First a shared helper that builds a bordered table PDF and returns its graphics + fragments:

```rust
use oxidize_pdf::graphics::extraction::{ExtractedGraphics, GraphicsExtractor};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::pipeline::Element;
use oxidize_pdf::text::{ExtractionOptions, Table, TextExtractor};
use oxidize_pdf::{Document, Page};

/// Build a 3-column bordered table PDF with the given rows; return (graphics, fragments).
fn bordered_table_inputs(rows: &[[&str; 3]]) -> (ExtractedGraphics, Vec<oxidize_pdf::text::TextFragment>) {
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut table = Table::with_equal_columns(3, 400.0);
    table.set_position(50.0, 700.0);
    for r in rows {
        table.add_row(vec![r[0].to_string(), r[1].to_string(), r[2].to_string()]).unwrap();
    }
    page.add_table(&table).unwrap();
    doc.add_page(page);
    let path = std::env::temp_dir().join(format!("rt_{}.pdf", rows.len()));
    doc.save(&path).unwrap();

    let pdoc = PdfReader::open_document(&path).unwrap();
    let mut gx = GraphicsExtractor::default();
    let graphics = gx.extract_from_page(&pdoc, 0).unwrap();
    let opts = ExtractionOptions { preserve_layout: true, ..Default::default() };
    let mut tx = TextExtractor::with_options(opts);
    let frags = tx.extract_from_page(&pdoc, 0).unwrap().fragments;
    (graphics, frags)
}

fn table_rows(elements: &[Element]) -> Vec<Vec<String>> {
    elements
        .iter()
        .find_map(|e| match e {
            Element::Table(t) => Some(t.rows.clone()),
            _ => None,
        })
        .expect("a Table element")
}

#[test]
fn ruling_detects_exact_bordered_grid() {
    let (graphics, frags) =
        bordered_table_inputs(&[["H1", "H2", "H3"], ["a1", "a2", "a3"], ["b1", "b2", "b3"]]);
    assert!(graphics.has_table_structure(), "writer borders must yield grid lines");

    let p = Partitioner::new(PartitionConfig::default());
    let elements = p.partition_fragments_with_graphics(&frags, Some(&graphics), 0, 842.0);

    assert_eq!(
        table_rows(&elements),
        vec![
            vec!["H1".to_string(), "H2".to_string(), "H3".to_string()],
            vec!["a1".to_string(), "a2".to_string(), "a3".to_string()],
            vec!["b1".to_string(), "b2".to_string(), "b3".to_string()],
        ]
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test ruling_detects_exact_bordered_grid -- --nocapture 2>&1 | tail -25`
Expected: FAIL — without ruling wiring, the spatial path may emit different rows or no Table; the `assert_eq!` (or `expect("a Table element")`) fails.

- [ ] **Step 3: Add the mapping helper**

Near the other free functions in `partition.rs` (e.g. beside `segment_into_table_regions`), add:

```rust
/// Flatten a ruling-detected table into row-major `Vec<Vec<String>>`, filling
/// absent cells with empty strings.
fn ruling_table_to_rows(table: &crate::text::table_detection::DetectedTable) -> Vec<Vec<String>> {
    let mut grid = vec![vec![String::new(); table.columns]; table.rows];
    for cell in &table.cells {
        if cell.row < table.rows && cell.column < table.columns {
            grid[cell.row][cell.column] = cell.text.clone();
        }
    }
    grid
}
```

- [ ] **Step 4: Add the ruling-first block**

Inside `partition_fragments_with_graphics`, at the START of the `if self.config.detect_tables {` block (before `let unclaimed_frags` at line ~247), insert:

```rust
            // Ruling-first: when the page has a drawn table grid, detect bordered
            // tables from vector lines and claim their fragments so the spatial
            // pass below only sees the remainder. region_looks_like_list is NOT
            // applied here — drawn borders are strong table evidence.
            if self.config.prefer_ruling_tables {
                if let Some(graphics) = graphics {
                    if graphics.has_table_structure() {
                        let detector = crate::text::table_detection::TableDetector::default();
                        if let Ok(tables) = detector.detect(graphics, fragments) {
                            for table in &tables {
                                if table.confidence < self.config.min_table_confidence {
                                    continue;
                                }
                                let rows = ruling_table_to_rows(table);
                                let bbox = ElementBBox::new(
                                    table.bbox.x,
                                    table.bbox.y,
                                    table.bbox.width,
                                    table.bbox.height,
                                );
                                elements.push(Element::Table(TableElementData {
                                    rows,
                                    metadata: ElementMetadata {
                                        page,
                                        bbox,
                                        confidence: table.confidence,
                                        ..Default::default()
                                    },
                                }));
                                let (rx, ry) = (table.bbox.x, table.bbox.y);
                                let (rr, rt) = (table.bbox.x + table.bbox.width, table.bbox.y + table.bbox.height);
                                for (i, f) in fragments.iter().enumerate() {
                                    if !claimed[i]
                                        && f.x >= rx - 1.0
                                        && f.x <= rr + 1.0
                                        && f.y >= ry - 1.0
                                        && f.y <= rt + 1.0
                                    {
                                        claimed[i] = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
```

If you added `let _ = graphics;` in Task 2, remove it now.

- [ ] **Step 5: Run to verify it passes**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test ruling_detects_exact_bordered_grid -- --nocapture 2>&1 | tail -15`
Expected: PASS.

- [ ] **Step 6: Add the multi-line cell fidelity test (hand-built graphics)**

This proves the ruling path keeps a wrapped cell as one cell. Build a 2-column × 1-row grid by hand and place two text fragments stacked inside the single left cell. Append:

`VectorLine::new` is `(x1, y1, x2, y2, stroke_width, is_stroked, color)` — orientation
is computed internally (no orientation arg). `TextFragment` does NOT derive
`Default` (it derives only `Debug, Clone`); build it via an explicit helper with
all 15 fields. Both verified in `src/text/extraction.rs` / `src/graphics/extraction.rs`
on 2026-06-06.

```rust
use oxidize_pdf::graphics::extraction::VectorLine;
use oxidize_pdf::text::TextFragment;

fn h_line(x1: f64, x2: f64, y: f64) -> VectorLine {
    VectorLine::new(x1, y, x2, y, 1.0, true, None)
}
fn v_line(x: f64, y1: f64, y2: f64) -> VectorLine {
    VectorLine::new(x, y1, x, y2, 1.0, true, None)
}
fn frag(text: &str, x: f64, y: f64) -> TextFragment {
    TextFragment {
        text: text.to_string(),
        x,
        y,
        width: 10.0,
        height: 8.0,
        font_size: 8.0,
        font_name: None,
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: vec![],
        mcid: None,
        struct_tag: None,
    }
}

#[test]
fn ruling_keeps_wrapped_cell_as_single_cell() {
    // A 2-col x 2-row grid: x in {100,200,300}, y in {100,150,200}.
    // Cells: top row y 150..200, bottom row y 100..150; left col x 100..200.
    let mut graphics = ExtractedGraphics::new();
    for y in [100.0, 150.0, 200.0] {
        graphics.add_line(h_line(100.0, 300.0, y));
    }
    for x in [100.0, 200.0, 300.0] {
        graphics.add_line(v_line(x, 100.0, 200.0));
    }
    assert!(graphics.has_table_structure());

    // Two stacked fragments in the SAME top-left cell (a wrapped line) + one
    // fragment in each of the other three cells. Centers fall inside their cell.
    let frags = vec![
        frag("Wrapped", 120.0, 180.0), // top-left, upper line
        frag("Line", 120.0, 160.0),    // top-left, lower line
        frag("B", 220.0, 170.0),       // top-right
        frag("c", 120.0, 120.0),       // bottom-left
        frag("d", 220.0, 120.0),       // bottom-right
    ];

    let p = Partitioner::new(PartitionConfig::default());
    let elements = p.partition_fragments_with_graphics(&frags, Some(&graphics), 0, 842.0);
    let rows = table_rows(&elements);
    // The wrapped top-left cell must be ONE cell holding both words, not two rows.
    assert_eq!(rows.len(), 2, "grid has exactly two rows, got {:?}", rows);
    assert!(
        rows[0][0].contains("Wrapped") && rows[0][0].contains("Line"),
        "wrapped lines stay in one cell, got {:?}",
        rows[0][0]
    );
}
```

Note: `TableCell.text` joins fragments whose center falls in the cell bbox; the
exact join separator (space vs newline) is whatever `table_detection.rs` uses —
the assertion checks `contains`, so it is robust to the separator. If the two
words land in different rows (assertion fails), the cell-assignment y-banding in
`table_detection.rs` is the thing to inspect, not the test.

- [ ] **Step 7: Run both Task-3 tests**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test ruling_ -- --nocapture 2>&1 | tail -15`
Expected: both PASS.

- [ ] **Step 8: Commit**

```bash
git add oxidize-pdf-core/src/pipeline/partition.rs oxidize-pdf-core/tests/ruling_table_partition_test.rs
git commit -m "feat(rag): ruling-first bordered-table detection in partition (#292)"
```

---

## Task 4: Plumb per-page graphics in `do_partition_pages`

**Files:** Modify `oxidize-pdf-core/src/parser/document.rs` (~1565); add a full-pipeline test.

- [ ] **Step 1: Write the failing test**

Append to `oxidize-pdf-core/tests/ruling_table_partition_test.rs`:

```rust
#[test]
fn full_pipeline_partition_emits_bordered_table() {
    // Build the same bordered table, save, reopen, run the public partition entry.
    let mut doc = Document::new();
    let mut page = Page::a4();
    let mut table = Table::with_equal_columns(3, 400.0);
    table.set_position(50.0, 700.0);
    for r in [["N", "Qty", "Price"], ["Apple", "3", "1.20"], ["Pear", "5", "0.90"]] {
        table.add_row(vec![r[0].into(), r[1].into(), r[2].into()]).unwrap();
    }
    page.add_table(&table).unwrap();
    doc.add_page(page);
    let path = std::env::temp_dir().join("rt_fullpipe.pdf");
    doc.save(&path).unwrap();

    let pdoc = PdfReader::open_document(&path).unwrap();
    let elements = pdoc.partition().unwrap();
    let rows = table_rows(&elements);
    assert_eq!(rows[0], vec!["N".to_string(), "Qty".to_string(), "Price".to_string()]);
    assert_eq!(rows[2], vec!["Pear".to_string(), "5".to_string(), "0.90".to_string()]);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test full_pipeline_partition_emits_bordered_table -- --nocapture 2>&1 | tail -20`
Expected: FAIL — graphics not yet plumbed, so the pipeline still uses spatial-only and the exact bordered rows aren't produced (or no Table element).

- [ ] **Step 3: Plumb graphics in `do_partition_pages`**

In `parser/document.rs`, in `do_partition_pages`, before the page loop add the gated extractor, and inside the loop compute per-page graphics and call the new method. Replace the existing call
`partitioner.partition_fragments(&page_text.fragments, page_idx_u32, page_height)` with the graphics-aware path:

```rust
        let extract_graphics = config.detect_tables && config.prefer_ruling_tables;
        let mut graphics_extractor = crate::graphics::extraction::GraphicsExtractor::default();
        // ... inside the `for (page_idx, page_text) in pages.iter().enumerate()` loop,
        //     after page_height is computed:
        let page_graphics = if extract_graphics {
            graphics_extractor.extract_from_page(self, page_idx).ok()
        } else {
            None
        };
        let page_elements = partitioner.partition_fragments_with_graphics(
            &page_text.fragments,
            page_graphics.as_ref(),
            page_idx_u32,
            page_height,
        );
```

Keep whatever the existing code does with the returned elements (it currently collects them — preserve that; only the call changes). Read the surrounding loop to wire `page_elements` exactly as the old return value was used.

- [ ] **Step 4: Run to verify it passes**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test full_pipeline_partition_emits_bordered_table -- --nocapture 2>&1 | tail -15`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add oxidize-pdf-core/src/parser/document.rs
git commit -m "feat(rag): extract per-page graphics for ruling tables in partition pipeline (#292)"
```

---

## Task 5: Fallback/flag-off regression tests, verification, CHANGELOG

**Files:** add tests; `CHANGELOG.md`.

- [ ] **Step 1: Add flag-off and borderless-fallback tests**

Append to `oxidize-pdf-core/tests/ruling_table_partition_test.rs`:

```rust
#[test]
fn flag_off_uses_spatial_only() {
    // With the same bordered table inputs but prefer_ruling_tables = false,
    // the ruling path is skipped; passing graphics must not change the result
    // versus passing None (graphics are ignored when the flag is off).
    let (graphics, frags) =
        bordered_table_inputs(&[["H1", "H2", "H3"], ["a1", "a2", "a3"], ["b1", "b2", "b3"]]);
    let cfg = PartitionConfig { prefer_ruling_tables: false, ..Default::default() };
    let p = Partitioner::new(cfg);
    let with_g = p.partition_fragments_with_graphics(&frags, Some(&graphics), 0, 842.0);
    let without_g = p.partition_fragments_with_graphics(&frags, None, 0, 842.0);
    assert_eq!(with_g.len(), without_g.len(), "graphics ignored when flag off");
}

#[test]
fn no_grid_falls_back_to_spatial() {
    // Graphics with no table structure -> ruling path skipped, spatial still runs.
    let empty = ExtractedGraphics::new();
    assert!(!empty.has_table_structure());
    let (_g, frags) =
        bordered_table_inputs(&[["x1", "x2", "x3"], ["y1", "y2", "y3"]]);
    let p = Partitioner::new(PartitionConfig::default());
    // Passing empty graphics must not panic and must still classify via spatial.
    let elements = p.partition_fragments_with_graphics(&frags, Some(&empty), 0, 842.0);
    // Spatial may or may not find a table from these fragments; the contract here
    // is only that the ruling path is skipped and partition returns without panic
    // and produces the same element count as the None path.
    let none_path = p.partition_fragments_with_graphics(&frags, None, 0, 842.0);
    assert_eq!(elements.len(), none_path.len());
}
```

- [ ] **Step 2: Run the full test file**

Run: `cargo test -p oxidize-pdf --test ruling_table_partition_test 2>&1 | tail -15`
Expected: all tests PASS.

- [ ] **Step 3: Full verification**

Run each and confirm:
- `cargo test -p oxidize-pdf --lib 2>&1 | tail -3` → no regressions (~6500+ pass).
- `cargo clippy --all -- -D warnings 2>&1 | tail -2` → exit 0.
- `cargo +1.88 build --lib --all-features --locked 2>&1 | tail -2` → Finished, 0 warnings.
- `cargo fmt --all -- --check` → clean.

- [ ] **Step 4: Update CHANGELOG**

In `CHANGELOG.md`, under `## [Unreleased]` → `### Added` (create if absent, above `### Changed`):

```markdown
- Ruling-based (vector-grid) table detection wired into the partition pipeline:
  bordered tables are now reconstructed from the PDF's drawn grid (primary path),
  with the spatial detector handling the rest. Controlled by
  `PartitionConfig::prefer_ruling_tables` (default true) (#292).
```

Commit:

```bash
git add CHANGELOG.md oxidize-pdf-core/tests/ruling_table_partition_test.rs
git commit -m "test(rag): ruling table fallback/flag-off coverage + changelog (#292)"
```

---

## Self-Review Notes

- **Spec coverage:** selection policy + claiming (Task 3), graphics plumbing + gating (Task 4), config (Task 1), non-breaking API (Task 2), mapping helper (Task 3), tests for synthetic-exact / multi-line fidelity / full-pipeline / flag-off / borderless-fallback (Tasks 3–5). `region_looks_like_list` skipped for ruling path (Task 3 block comment).
- **Type consistency:** `partition_fragments_with_graphics(&[TextFragment], Option<&ExtractedGraphics>, u32, f64)`, `ruling_table_to_rows(&DetectedTable) -> Vec<Vec<String>>`, `prefer_ruling_tables: bool`, `TableDetector::default()/detect`, `ExtractedGraphics::new()/add_line/has_table_structure` — consistent across tasks.
- **Known implementer follow-ups (not placeholders, explicit):** (a) confirm/adjust other `PartitionConfig {` literals in `src/` (Task 1 Step 3); (b) wire `page_elements` exactly as the prior return value was consumed in `do_partition_pages` (Task 4 Step 3). `VectorLine::new` (7 args, no orientation) and `TextFragment` (15 fields, no `Default`) are already verified and given as concrete helpers in Task 3 Step 6.
