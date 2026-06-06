# Design — wire ruling-based table detector into partition (#292)

**Date:** 2026-06-06
**Issue:** #292 (RAG: wire ruling-based table detector into partition pipeline)
**Branch:** `feature/issue-292-ruling-table-partition` (from `develop`)

## Motivation

Read-side table extraction in `pipeline/partition.rs` currently uses only the
**spatial** detector (`text/structured/table.rs`, X/Y clustering of text
positions). The **ruling-based** detector (`text/table_detection.rs`, which
reconstructs a grid from the PDF's vector H/V lines) exists but is standalone,
never wired into partition. Bordered tables — common in gov/financial corpora,
typical RAG input — are therefore processed by text clustering, which is
inferior when the grid is already drawn, and degrades on merged/multi-line cells
(independent X/Y clustering produces a cartesian product). Wiring the ruling
detector raises table fidelity in RAG chunks using PDF-native graphics a generic
multi-format pipeline cannot exploit.

## Verified current state (interfaces)

- `Partitioner::partition_fragments(&self, fragments: &[TextFragment], page: u32, page_height: f64) -> Vec<Element>` (`partition.rs:122`). Receives **text only**. Table block at lines 246–309: `StructuredDataDetector::new(...)`, `segment_into_table_regions`, `region_looks_like_list` anti-FP, `min_table_confidence` filter, builds `TableElementData`.
- Ruling detector: `TableDetector::detect(&self, graphics: &ExtractedGraphics, text_fragments: &[TextFragment]) -> Result<Vec<DetectedTable>, TableDetectionError>` (`text/table_detection.rs:268`). Requires **both** graphics and text. Gate: `graphics.has_table_structure()` (≥2 H + ≥2 V lines).
- `DetectedTable { bbox: BoundingBox, cells: Vec<TableCell>, rows: usize, columns: usize, confidence: f64 }`. `TableCell { row: usize, column: usize, bbox, text: String, has_borders: bool }`.
- `ExtractedGraphics { lines, horizontal_count, vertical_count }` produced by `GraphicsExtractor::extract_from_page(&PdfDocument<R>, page_index) -> Result<ExtractedGraphics, ExtractionError>` (`graphics/extraction.rs:258`).
- Partition input flows from `document.rs::do_partition_pages` (~line 1565), which holds `&self: PdfDocument<R>` and the per-page index — the correct place to extract graphics. `ExtractedText { text, fragments }` carries **no** graphics.
- `TableElementData { rows: Vec<Vec<String>>, metadata: ElementMetadata }` (`pipeline/element.rs:211`). Flat cell text, no per-cell bbox.

## Design

### Selection policy — page-level ruling-first, spatial fills the rest

Per page, in the table-detection block:

1. If `config.prefer_ruling_tables` AND graphics are present AND
   `graphics.has_table_structure()`: run `TableDetector::detect(graphics, fragments)`.
   For each `DetectedTable` whose `confidence >= config.min_table_confidence`,
   emit a `TableElementData` and claim the fragments inside its `bbox`
   (same ±1pt overlap claiming the spatial path already uses).
   `region_looks_like_list` is **not** applied to ruling tables — drawn borders
   are strong table evidence; that anti-FP heuristic governs only the spatial path.
2. Run the existing **spatial** detector on the **remaining unclaimed**
   fragments (catches borderless tables). Unchanged behavior.
3. If graphics absent / no line structure / flag off: spatial only — exactly
   today's behavior.

Ruling-first then spatial-on-the-rest avoids double-detecting the same region.

### Graphics plumbing

In `do_partition_pages`, only when `config.detect_tables && config.prefer_ruling_tables`,
extract graphics per page: `GraphicsExtractor::extract_from_page(self, idx).ok()`
→ `Option<ExtractedGraphics>`. Extraction failure → `None` → spatial fallback;
never fails partition. When the flag is off, graphics are not extracted at all
(zero added cost for users who opt out).

### Partition API (non-breaking)

Add `Partitioner::partition_fragments_with_graphics(&self, fragments: &[TextFragment], graphics: Option<&ExtractedGraphics>, page: u32, page_height: f64) -> Vec<Element>`.
The existing `partition_fragments(fragments, page, page_height)` delegates with
`graphics: None`, preserving the public API. `do_partition_pages` calls the new
method with the per-page graphics.

### Config

Add `prefer_ruling_tables: bool` to `PartitionConfig`, **default `true`**.
`Default` and any preset profiles set it `true`. Documented as: prefer the
ruling-based detector for bordered tables; set `false` to force the legacy
spatial-only path (and skip graphics extraction entirely).

### Mapping `DetectedTable` → `TableElementData`

`rows: Vec<Vec<String>>` built by grouping `cells` by `row` (0..rows), within
each row ordering by `column` (0..columns), taking `cell.text` (empty string for
absent cells). `metadata.bbox` from `DetectedTable.bbox`; `metadata.confidence`
from `DetectedTable.confidence`; `metadata.page = page`. `TableElementData` stays
flat — per-cell bbox / `has_borders` are not surfaced (YAGNI; markdown table
export consumes flat rows).

## Error handling

- Graphics extraction error → `None` → spatial path. No new error variants;
  partition signatures still return `Vec<Element>` / existing `Result`.
- `TableDetector::detect` error → log and fall back to spatial for that page.
- Empty / too-few line sets → `has_table_structure()` false → spatial.

## Testing (content-verifying, no smoke tests)

1. **Synthetic bordered table (exact cells).** Generate a PDF containing a known
   bordered table via the oxidize-pdf writer (the `advanced_tables_example`
   pattern draws ruled tables). Partition it with defaults; assert the emitted
   `TableElementData.rows` equals the exact known cell matrix.
   **Feasibility spike required first:** confirm the writer emits the borders as
   stroked vector lines that `GraphicsExtractor` recovers as H/V lines and
   `has_table_structure()` returns true. If the writer's borders are not
   recovered, fall back to a committed real bordered-table fixture for the
   primary assertion.
2. **Multi-line / merged cell.** A bordered table with a cell whose text wraps to
   two lines: assert the ruling path keeps it as one cell, and document that the
   spatial path splits it (the concrete fidelity gain). Drives the value claim.
3. **Real corpus.** A gov/financial PDF with a bordered table (fixture identified
   during implementation); assert specific known cell values.
4. **Borderless fallback (regression).** A table with no ruling lines → spatial
   path still produces the expected table; ruling path is skipped.
5. **Flag off.** `prefer_ruling_tables = false` → output matches the legacy
   spatial-only result and no graphics extraction occurs.

## Out of scope (YAGNI)

- Enriching `TableElementData` with per-cell bboxes / span info.
- Implementing the stubbed `detect_borderless` path in `table_detection.rs`.
- Caching graphics extraction across pipeline stages.
- Region-level (vs page-level) ruling/spatial selection.
