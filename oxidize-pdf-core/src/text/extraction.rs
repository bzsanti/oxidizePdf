//! Text extraction from PDF content streams
//!
//! This module provides functionality to extract text from PDF pages,
//! handling text positioning, transformations, and basic encodings.

use crate::graphics::Color;
use crate::parser::content::{ContentOperation, ContentParser, TextElement};
use crate::parser::document::PdfDocument;
use crate::parser::objects::PdfObject;
use crate::parser::page_tree::ParsedPage;
use crate::parser::ParseResult;
use crate::text::extraction_cmap::{CMapTextExtractor, FontInfo};
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Text extraction options
#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    /// Preserve the original layout (spacing and positioning)
    pub preserve_layout: bool,
    /// Minimum space width to insert space character (in text space units)
    pub space_threshold: f64,
    /// Minimum vertical distance to insert newline (in text space units)
    pub newline_threshold: f64,
    /// Sort text fragments by position (useful for multi-column layouts)
    pub sort_by_position: bool,
    /// Detect and handle columns
    pub detect_columns: bool,
    /// Column separation threshold (in page units)
    pub column_threshold: f64,
    /// Merge hyphenated words at line ends
    pub merge_hyphenated: bool,
    /// Track space insertion decisions in each TextFragment (default: false).
    /// When false: zero overhead. When true: populates `TextFragment::space_decisions`.
    pub track_space_decisions: bool,
    /// Reconstruct visual lines and paragraphs from the raw text fragments
    /// produced by PDF text-show operators. When `true`, the extractor groups
    /// fragments by baseline into single-line fragments, then groups
    /// consecutive lines with normal leading into paragraph-level fragments.
    /// This is what the partition pipeline needs to produce Element values at
    /// paragraph granularity rather than at per-`Tj` granularity (see
    /// [issue #261](https://github.com/bzsanti/oxidizePdf/issues/261)).
    ///
    /// Default `false` for backward compatibility with direct `extract_text`
    /// callers. The `PdfDocument::partition*` entry points force this to
    /// `true`.
    pub reconstruct_paragraphs: bool,
    /// Include content inside `/Artifact` marked-content scopes (page headers,
    /// footers, watermarks, decorative content). Default `false` — Artifact
    /// content is filtered out, as the PDF/UA conformance level recommends
    /// for accessibility tooling and as RAG callers consistently want
    /// (issue #269 Phase 1). Opt-in by setting `true` when extracting
    /// page furniture matters (e.g. forensic auditing, redaction tools).
    pub include_artifacts: bool,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            preserve_layout: false,
            space_threshold: 0.3,
            newline_threshold: 10.0,
            sort_by_position: true,
            detect_columns: false,
            column_threshold: 50.0,
            merge_hyphenated: true,
            track_space_decisions: false,
            reconstruct_paragraphs: false,
            include_artifacts: false,
        }
    }
}

/// Extracted text with position information
#[derive(Debug, Clone)]
pub struct ExtractedText {
    /// The extracted text content
    pub text: String,
    /// Text fragments with position information (if preserve_layout is true)
    pub fragments: Vec<TextFragment>,
}

/// Metadata about a space insertion decision during text extraction.
/// Only populated when [`ExtractionOptions::track_space_decisions`] is `true`.
#[derive(Debug, Clone)]
pub struct SpaceDecision {
    /// Character offset in the extracted text.
    pub offset: usize,
    /// Actual horizontal gap (dx) in text space units.
    pub dx: f64,
    /// The threshold used at this point.
    pub threshold: f64,
    /// Confidence: `|dx - threshold| / threshold`, clamped to [0.0, 1.0].
    pub confidence: f64,
    /// Whether a space was inserted.
    pub inserted: bool,
}

/// A fragment of text with position information
#[derive(Debug, Clone)]
pub struct TextFragment {
    /// Text content
    pub text: String,
    /// X position in page coordinates
    pub x: f64,
    /// Y position in page coordinates
    pub y: f64,
    /// Width of the text
    pub width: f64,
    /// Height of the text
    pub height: f64,
    /// Font size
    pub font_size: f64,
    /// Font name (if known) - used for kerning-aware text spacing
    pub font_name: Option<String>,
    /// Whether the font is bold (detected from font name)
    pub is_bold: bool,
    /// Whether the font is italic (detected from font name)
    pub is_italic: bool,
    /// Fill color of the text (from graphics state)
    pub color: Option<Color>,
    /// Space insertion decisions (empty unless `track_space_decisions` is true).
    pub space_decisions: Vec<SpaceDecision>,
    /// Marked-content identifier from the innermost ancestor BDC with `/MCID`
    /// (issue #269 Phase 1). `None` for non-tagged PDFs, which preserves the
    /// pre-Phase-1 grouping behavior (`None == None` collapses to legacy keys).
    pub mcid: Option<u32>,
    /// Structural tag of the owning BDC (e.g. `"P"`, `"H1"`, `"Figure"`,
    /// `"Artifact"`). Set on the same ancestor that supplied `mcid`. Phase 3
    /// will consume this for partitioner classification; Phase 1 only carries it.
    pub struct_tag: Option<String>,
}

/// One entry on the marked-content stack maintained by `TextState`.
///
/// PDF marked-content operators (BDC/BMC/EMC) form a balanced LIFO stack
/// per content stream. Each entry remembers the tag (`"P"`, `"H1"`,
/// `"Artifact"`, …), the optional `MCID` for fragment grouping, the
/// optional `/ActualText` substitution string, and a computed
/// `is_artifact` flag that inherits from any ancestor (so nested
/// `/P` inside `/Artifact` is still filtered out).
#[derive(Debug, Clone)]
struct MarkedContentEntry {
    /// The BDC/BMC tag (e.g. `"P"`, `"Figure"`, `"Artifact"`, `"Span"`).
    tag: String,
    /// MCID from `/MCID <int>` if present in the BDC props.
    mcid: Option<u32>,
    /// Decoded ActualText from `/ActualText (...)` if present. Decoded
    /// once at BDC time (UTF-16BE BOM detection in `decode_pdf_string`)
    /// rather than per-fragment.
    #[allow(dead_code)] // Task 9 reads this via pending_actualtext flush path
    actual_text: Option<String>,
    /// True if this entry's tag == `"Artifact"` OR any ancestor on the
    /// stack at push time had `is_artifact == true`. Inheritance lets the
    /// emitter check only the innermost entry to decide filtering.
    is_artifact: bool,
}

/// A pending ActualText run. Created when a BDC pushes an entry with
/// `actual_text == Some(_)`; drained and emitted as a single synthetic
/// `TextFragment` when the matching EMC pops the entry.
///
/// Spec §3a/§4 (collapse-on-EMC): per-`Tj` emission inside an ActualText
/// scope is suppressed; on scope close we emit one fragment whose `text`
/// is the substitution string, `x`/`y` is the first `Tj` origin, and
/// `width` is the sum of suppressed text widths.
#[derive(Debug, Clone)]
struct PendingActualText {
    /// Substitution text from the BDC's `/ActualText` (already decoded).
    text: String,
    /// Pen origin of the first suppressed `Tj` (page-space).
    first_x: f64,
    /// Same for Y.
    first_y: f64,
    /// Accumulated effective width of suppressed `Tj` runs.
    width: f64,
    /// Effective font size at the time the first `Tj` was suppressed.
    font_size: f64,
    /// Font name + style at first `Tj`. Set on first suppression.
    font_name: Option<String>,
    /// Bold/italic from the font name at first suppression.
    is_bold: bool,
    is_italic: bool,
    /// Fill color at first suppression.
    color: Option<Color>,
    /// Depth in `mc_stack` at which this run was opened. When the entry at
    /// this depth is popped, the pending run is flushed.
    stack_depth: usize,
    /// Whether a `Tj`/`TJ`/`'`/`"` has been observed yet inside the scope.
    /// Until the first one fires, the run has no origin to record.
    populated: bool,
}

/// Text extraction state
struct TextState {
    /// Current text matrix
    text_matrix: [f64; 6],
    /// Current text line matrix
    text_line_matrix: [f64; 6],
    /// Current transformation matrix (CTM)
    ctm: [f64; 6],
    /// Text leading (line spacing)
    leading: f64,
    /// Character spacing
    char_space: f64,
    /// Word spacing
    word_space: f64,
    /// Horizontal scaling
    horizontal_scale: f64,
    /// Text rise
    text_rise: f64,
    /// Current font size
    font_size: f64,
    /// Current font name
    font_name: Option<String>,
    /// Render mode (0 = fill, 1 = stroke, etc.)
    render_mode: u8,
    /// Fill color (for text rendering)
    fill_color: Option<Color>,
    /// Graphics state stack for `q`/`Q` operators. Each entry holds the CTM
    /// and other graphics state items that the text extractor needs to restore.
    /// Per PDF spec §8.4.4, `q` pushes the full graphics state and `Q` pops it;
    /// here we save only the fields that influence text extraction.
    saved_states: Vec<SavedGraphicsState>,
    /// Marked-content stack (issue #269 Phase 1). Pushed on BMC/BDC,
    /// popped on EMC. Empty on entry to each page.
    mc_stack: Vec<MarkedContentEntry>,
    /// Pending ActualText run if any BDC ancestor declared `/ActualText`.
    /// At most one active run at a time — nested ActualText replaces the
    /// outer (innermost wins, per spec §4).
    pending_actualtext: Option<PendingActualText>,
}

/// Subset of graphics state saved by `q` and restored by `Q` (issue #262).
#[derive(Clone)]
struct SavedGraphicsState {
    ctm: [f64; 6],
    fill_color: Option<Color>,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            text_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            text_line_matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            ctm: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            leading: 0.0,
            char_space: 0.0,
            word_space: 0.0,
            horizontal_scale: 100.0,
            text_rise: 0.0,
            font_size: 0.0,
            font_name: None,
            render_mode: 0,
            fill_color: None,
            saved_states: Vec::new(),
            mc_stack: Vec::new(),
            pending_actualtext: None,
        }
    }
}

/// Parse font style (bold/italic) from font name
///
/// Detects bold and italic styles from common font naming patterns.
/// Works with PostScript font names (e.g., "Helvetica-Bold", "Times-BoldItalic")
/// and TrueType names (e.g., "Arial Bold", "Courier Oblique").
///
/// # Examples
///
/// ```
/// use oxidize_pdf::text::extraction::parse_font_style;
///
/// assert_eq!(parse_font_style("Helvetica-Bold"), (true, false));
/// assert_eq!(parse_font_style("Times-BoldItalic"), (true, true));
/// assert_eq!(parse_font_style("Courier"), (false, false));
/// assert_eq!(parse_font_style("Arial-Italic"), (false, true));
/// ```
///
/// # Returns
///
/// Tuple of (is_bold, is_italic)
pub fn parse_font_style(font_name: &str) -> (bool, bool) {
    let name_lower = font_name.to_lowercase();

    // Detect bold from common patterns
    let is_bold = name_lower.contains("bold")
        || name_lower.contains("-b")
        || name_lower.contains(" b ")
        || name_lower.ends_with(" b");

    // Detect italic/oblique from common patterns
    let is_italic = name_lower.contains("italic")
        || name_lower.contains("oblique")
        || name_lower.contains("-i")
        || name_lower.contains(" i ")
        || name_lower.ends_with(" i");

    (is_bold, is_italic)
}

/// Text extractor for PDF pages with CMap support
pub struct TextExtractor {
    options: ExtractionOptions,
    /// Font cache for the current page (name-keyed, rebuilt per page since names are page-local)
    font_cache: HashMap<String, FontInfo>,
    /// Persistent font cache keyed by PDF object reference — avoids re-parsing the same font
    /// object across pages. Most multi-page PDFs reuse the same font objects.
    font_object_cache: HashMap<(u32, u16), FontInfo>,
}

impl TextExtractor {
    /// Create a new text extractor with default options
    pub fn new() -> Self {
        Self {
            options: ExtractionOptions::default(),
            font_cache: HashMap::new(),
            font_object_cache: HashMap::new(),
        }
    }

    /// Create a text extractor with custom options
    pub fn with_options(options: ExtractionOptions) -> Self {
        Self {
            options,
            font_cache: HashMap::new(),
            font_object_cache: HashMap::new(),
        }
    }

    /// Run the full fragment-merge chain used by the partition pipeline:
    /// kerning fix → line reconstruction → paragraph reconstruction.
    ///
    /// Honors `ExtractionOptions::reconstruct_paragraphs`: when `false`, only
    /// `merge_close_fragments` (the kerning fix) runs and the input is
    /// returned at fragment granularity.
    ///
    /// This method is `pub` so the integration test in
    /// `tests/paragraph_reconstruction_test.rs` can exercise it without going
    /// through a PDF file. Production callers should prefer
    /// `PdfDocument::partition()` and friends, which use this internally.
    pub fn merge_fragments_for_partition(&self, fragments: &[TextFragment]) -> Vec<TextFragment> {
        let kerning_fixed = self.merge_close_fragments(fragments);
        if !self.options.reconstruct_paragraphs {
            return kerning_fixed;
        }
        let lines = self.merge_into_lines(&kerning_fixed);
        self.merge_into_paragraphs(&lines)
    }

    /// Group fragments by baseline into single-line fragments.
    ///
    /// Two fragments are on the same line when their Y centers differ by less
    /// than `0.2 * min(head.height, frag.height)`. The 0.2 ratio absorbs
    /// sub-point baseline jitter from text-matrix arithmetic while keeping
    /// tightly-spaced visual rows (e.g. table cells whose baselines are
    /// separated by ~2-3pt at 9pt font) on distinct logical lines — see
    /// issue #265.
    ///
    /// Within a line, fragments are processed in their globally-sorted order
    /// (Y desc, X asc); a space is inserted between adjacent fragments when
    /// the X gap exceeds `space_threshold * font_size`.
    ///
    /// The output bounding box for each line is the axis-aligned union of the
    /// input fragments' bounding boxes; `font_size` and `font_name` are
    /// inherited from the line's first fragment.
    fn merge_into_lines(&self, fragments: &[TextFragment]) -> Vec<TextFragment> {
        if fragments.is_empty() {
            return Vec::new();
        }

        // Pre-pass: assign row_id from Y-up-jumps in emission order. This
        // disambiguates columns in multi-column layouts where a single outer
        // BDC makes mcid uniform across visually distinct columns. See
        // `docs/superpowers/specs/2026-05-23-issue-265-line-interleaving-design.md`.
        let row_ids = assign_row_ids(fragments);

        // Whether the page has any tagged (mcid-carrying) fragment. For tagged
        // PDFs (PDF/UA, ISO 32000-2 tagged), the content stream delivers text
        // in logical reading order, so within a visual line we preserve
        // emission order rather than sorting by X. Out-of-left-to-right glyph
        // placement (common in typeset tagged PDFs where the PDF author lays
        // out glyphs via non-monotone Td/Tm operators) is correctly rendered
        // by keeping emission order.
        //
        // For non-tagged PDFs (all mcid=None), we retain the X-sort fallback
        // because many generators emit glyphs in arbitrary (often right-to-left
        // or random) order and only the X coordinate gives reading order.
        let is_tagged = fragments.iter().any(|f| f.mcid.is_some());

        // Sort by (row_id asc, y desc, emission_idx/x asc).
        // row_id primary keeps fragments from different visual rows in
        // separate Y-bucket groups. Within a row, we sort by Y descending
        // (so higher-on-page fragments come first) and then by emission index
        // (tagged) or X coordinate (non-tagged).
        let mut indexed: Vec<(u32, usize, &TextFragment)> = row_ids
            .iter()
            .copied()
            .zip(fragments.iter().enumerate().map(|(i, f)| (i, f)))
            .map(|(rid, (idx, f))| (rid, idx, f))
            .collect();
        indexed.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(b.2.y.total_cmp(&a.2.y))
                .then(if is_tagged {
                    a.1.cmp(&b.1)
                } else {
                    a.2.x
                        .partial_cmp(&b.2.x)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        let mut lines: Vec<Vec<&TextFragment>> = Vec::new();
        let mut current_row_id: Option<u32> = None;
        for (rid, _idx, frag) in indexed {
            let same_row_id = current_row_id == Some(rid);
            let placed = same_row_id
                && lines.last_mut().is_some_and(|line| {
                    let head = line[0];
                    let tol = (head.height.min(frag.height)) * 0.2;
                    (head.y - frag.y).abs() < tol && head.mcid == frag.mcid
                });
            if placed {
                lines.last_mut().unwrap().push(frag);
            } else {
                lines.push(vec![frag]);
                current_row_id = Some(rid);
            }
        }

        lines
            .into_iter()
            .map(|line| build_line_fragment(line, self.options.space_threshold))
            .collect()
    }

    /// Group consecutive lines into paragraphs based on vertical gap.
    ///
    /// Two consecutive lines are part of the same paragraph when the vertical
    /// gap between them is less than 1.5× the median line height in the
    /// input. Hyphenated line breaks (previous line ends with `-` and
    /// `merge_hyphenated` is set) join without a separator and drop the
    /// hyphen; otherwise lines join with `'\n'`.
    fn merge_into_paragraphs(&self, lines: &[TextFragment]) -> Vec<TextFragment> {
        if lines.is_empty() {
            return Vec::new();
        }

        // Median line height — robust to outliers
        let mut heights: Vec<f64> = lines.iter().map(|l| l.height).collect();
        heights.sort_by(f64::total_cmp);
        let median_h = heights[heights.len() / 2];
        let max_paragraph_gap = median_h * 1.5;

        let mut paragraphs: Vec<TextFragment> = Vec::new();
        let mut current = lines[0].clone();

        for line in &lines[1..] {
            let prev_bottom = current.y;
            let line_top = line.y + line.height;
            let gap = prev_bottom - line_top;

            if gap < 0.0 || gap > max_paragraph_gap || current.mcid != line.mcid {
                paragraphs.push(current);
                current = line.clone();
                continue;
            }

            // Same paragraph — join
            let joined_text = if self.options.merge_hyphenated && current.text.ends_with('-') {
                let mut s = current.text.clone();
                s.pop(); // drop trailing hyphen
                s.push_str(&line.text);
                s
            } else {
                format!("{}\n{}", current.text, line.text)
            };

            let x_min = current.x.min(line.x);
            let x_max = (current.x + current.width).max(line.x + line.width);
            let y_min = current.y.min(line.y);
            let y_max = (current.y + current.height).max(line.y + line.height);

            current = TextFragment {
                text: joined_text,
                x: x_min,
                y: y_min,
                width: x_max - x_min,
                height: y_max - y_min,
                font_size: current.font_size,
                font_name: current.font_name.clone(),
                is_bold: current.is_bold,
                is_italic: current.is_italic,
                color: current.color,
                space_decisions: Vec::new(),
                mcid: current.mcid,
                struct_tag: current.struct_tag.clone(),
            };
        }
        paragraphs.push(current);

        paragraphs
    }

    /// Extract text from a PDF document
    pub fn extract_from_document<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
    ) -> ParseResult<Vec<ExtractedText>> {
        let page_count = document.page_count()?;
        let mut results = Vec::new();

        for i in 0..page_count {
            let text = self.extract_from_page(document, i)?;
            results.push(text);
        }

        Ok(results)
    }

    /// Extract text from a specific page
    pub fn extract_from_page<R: Read + Seek>(
        &mut self,
        document: &PdfDocument<R>,
        page_index: u32,
    ) -> ParseResult<ExtractedText> {
        // Get the page
        let page = document.get_page(page_index)?;

        // Extract font resources first
        {
            let _span = tracing::info_span!("font_resources").entered();
            self.extract_font_resources(&page, document)?;
        }

        // Get content streams
        let streams = {
            let _span = tracing::info_span!("stream_decompress").entered();
            page.content_streams_with_document(document)?
        };

        let mut extracted_text = String::new();
        let mut fragments = Vec::new();
        let mut state = TextState::default();
        let mut in_text_object = false;
        let mut last_x = 0.0;
        let mut last_y = 0.0;

        // Issue #269 Phase 1: resolve /Properties resource dictionary for BDC
        // resource-ref operands (e.g. `/Span /PropsName BDC`). Optional — most
        // PDFs use inline dicts.
        let page_properties: Option<&crate::parser::objects::PdfDictionary> = page
            .get_resources()
            .and_then(|res| match res.get("Properties") {
                Some(crate::parser::objects::PdfObject::Dictionary(d)) => Some(d),
                _ => None,
            });

        // Process each content stream
        for (stream_idx, stream_data) in streams.iter().enumerate() {
            let operations = match {
                let _span = tracing::info_span!("content_parse").entered();
                ContentParser::parse_content(stream_data)
            } {
                Ok(ops) => ops,
                Err(e) => {
                    // Enhanced diagnostic logging for content stream parsing failures
                    tracing::debug!(
                        "Warning: Failed to parse content stream on page {}, stream {}/{}",
                        page_index + 1,
                        stream_idx + 1,
                        streams.len()
                    );
                    tracing::debug!("         Error: {}", e);
                    tracing::debug!("         Stream size: {} bytes", stream_data.len());

                    // Show first 100 bytes for diagnosis (or less if stream is smaller)
                    let preview_len = stream_data.len().min(100);
                    let preview = String::from_utf8_lossy(&stream_data[..preview_len]);
                    tracing::debug!(
                        "         Stream preview (first {} bytes): {:?}",
                        preview_len,
                        preview.chars().take(80).collect::<String>()
                    );

                    // Continue processing other streams
                    continue;
                }
            };

            let _ops_span = tracing::info_span!("text_ops_loop").entered();
            for op in operations {
                match op {
                    ContentOperation::BeginText => {
                        in_text_object = true;
                        // Reset text matrix to identity
                        state.text_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                        state.text_line_matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
                    }

                    ContentOperation::EndText => {
                        in_text_object = false;
                    }

                    ContentOperation::SetTextMatrix(a, b, c, d, e, f) => {
                        state.text_matrix =
                            [a as f64, b as f64, c as f64, d as f64, e as f64, f as f64];
                        state.text_line_matrix =
                            [a as f64, b as f64, c as f64, d as f64, e as f64, f as f64];
                    }

                    ContentOperation::MoveText(tx, ty) => {
                        // Update text matrix by translation
                        let new_matrix = multiply_matrix(
                            &[1.0, 0.0, 0.0, 1.0, tx as f64, ty as f64],
                            &state.text_line_matrix,
                        );
                        state.text_matrix = new_matrix;
                        state.text_line_matrix = new_matrix;
                    }

                    ContentOperation::NextLine => {
                        // Move to next line using current leading
                        let new_matrix = multiply_matrix(
                            &[1.0, 0.0, 0.0, 1.0, 0.0, -state.leading],
                            &state.text_line_matrix,
                        );
                        state.text_matrix = new_matrix;
                        state.text_line_matrix = new_matrix;
                    }

                    ContentOperation::ShowText(text) => {
                        if in_text_object {
                            let text_bytes = &text;
                            let decoded = self.decode_text(text_bytes, &state)?;

                            // Pen origin in user space = (CTM × text_matrix)(0, 0).
                            let (x, y) = text_origin(&state);

                            // Add spacing based on position change
                            if !extracted_text.is_empty() {
                                let dx = x - last_x;
                                let dy = (y - last_y).abs();

                                if dy > self.options.newline_threshold {
                                    extracted_text.push('\n');
                                } else if dx > self.options.space_threshold * state.font_size {
                                    extracted_text.push(' ');
                                }
                            }

                            extracted_text.push_str(&decoded);

                            // Get font info for accurate width calculation
                            let text_width = {
                                let font_info = state
                                    .font_name
                                    .as_ref()
                                    .and_then(|name| self.font_cache.get(name));
                                calculate_text_width(&decoded, state.font_size, font_info)
                            };

                            if self.options.preserve_layout {
                                emit_text_fragment(
                                    &mut fragments,
                                    &decoded,
                                    text_width,
                                    x,
                                    y,
                                    &mut state,
                                    self.options.include_artifacts,
                                );
                            }

                            // Update position for next text
                            last_x = x + text_width;
                            last_y = y;

                            // Update text matrix for next show operation
                            let tx = text_width * state.horizontal_scale / 100.0;
                            state.text_matrix =
                                multiply_matrix(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &state.text_matrix);
                        }
                    }

                    ContentOperation::ShowTextArray(array) => {
                        if in_text_object {
                            for item in array {
                                match item {
                                    TextElement::Text(text_bytes) => {
                                        let decoded = self.decode_text(&text_bytes, &state)?;
                                        extracted_text.push_str(&decoded);

                                        let text_width = {
                                            let font_info = state
                                                .font_name
                                                .as_ref()
                                                .and_then(|name| self.font_cache.get(name));
                                            calculate_text_width(
                                                &decoded,
                                                state.font_size,
                                                font_info,
                                            )
                                        };

                                        if self.options.preserve_layout {
                                            let (x, y) = text_origin(&state);
                                            emit_text_fragment(
                                                &mut fragments,
                                                &decoded,
                                                text_width,
                                                x,
                                                y,
                                                &mut state,
                                                self.options.include_artifacts,
                                            );
                                        }

                                        let tx = text_width * state.horizontal_scale / 100.0;
                                        state.text_matrix = multiply_matrix(
                                            &[1.0, 0.0, 0.0, 1.0, tx, 0.0],
                                            &state.text_matrix,
                                        );
                                    }
                                    TextElement::Spacing(adjustment) => {
                                        // Text position adjustment (negative = move left)
                                        let tx = -(adjustment as f64) / 1000.0 * state.font_size;
                                        state.text_matrix = multiply_matrix(
                                            &[1.0, 0.0, 0.0, 1.0, tx, 0.0],
                                            &state.text_matrix,
                                        );
                                    }
                                }
                            }
                        }
                    }

                    ContentOperation::NextLineShowText(text) => {
                        if in_text_object {
                            // ' = T* then Tj string. Advance line matrix by -leading.
                            let new_matrix = multiply_matrix(
                                &[1.0, 0.0, 0.0, 1.0, 0.0, -state.leading],
                                &state.text_line_matrix,
                            );
                            state.text_matrix = new_matrix;
                            state.text_line_matrix = new_matrix;

                            let decoded = self.decode_text(&text, &state)?;
                            let (x, y) = text_origin(&state);

                            if !extracted_text.is_empty() {
                                extracted_text.push('\n');
                            }
                            extracted_text.push_str(&decoded);

                            let text_width = {
                                let font_info = state
                                    .font_name
                                    .as_ref()
                                    .and_then(|name| self.font_cache.get(name));
                                calculate_text_width(&decoded, state.font_size, font_info)
                            };

                            if self.options.preserve_layout {
                                emit_text_fragment(
                                    &mut fragments,
                                    &decoded,
                                    text_width,
                                    x,
                                    y,
                                    &mut state,
                                    self.options.include_artifacts,
                                );
                            }

                            last_x = x + text_width;
                            last_y = y;

                            let tx = text_width * state.horizontal_scale / 100.0;
                            state.text_matrix =
                                multiply_matrix(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &state.text_matrix);
                        }
                    }

                    ContentOperation::SetSpacingNextLineShowText(word_space, char_space, text) => {
                        if in_text_object {
                            // " = aw Tw, ac Tc, then ' string. ISO 32000-1 §9.4.3.
                            // The variant fields mirror the spec field names:
                            // (word_spacing, char_spacing, text).
                            state.word_space = word_space as f64;
                            state.char_space = char_space as f64;

                            let new_matrix = multiply_matrix(
                                &[1.0, 0.0, 0.0, 1.0, 0.0, -state.leading],
                                &state.text_line_matrix,
                            );
                            state.text_matrix = new_matrix;
                            state.text_line_matrix = new_matrix;

                            let decoded = self.decode_text(&text, &state)?;
                            let (x, y) = text_origin(&state);

                            if !extracted_text.is_empty() {
                                extracted_text.push('\n');
                            }
                            extracted_text.push_str(&decoded);

                            let text_width = {
                                let font_info = state
                                    .font_name
                                    .as_ref()
                                    .and_then(|name| self.font_cache.get(name));
                                calculate_text_width(&decoded, state.font_size, font_info)
                            };

                            if self.options.preserve_layout {
                                emit_text_fragment(
                                    &mut fragments,
                                    &decoded,
                                    text_width,
                                    x,
                                    y,
                                    &mut state,
                                    self.options.include_artifacts,
                                );
                            }

                            last_x = x + text_width;
                            last_y = y;

                            let tx = text_width * state.horizontal_scale / 100.0;
                            state.text_matrix =
                                multiply_matrix(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &state.text_matrix);
                        }
                    }

                    ContentOperation::SetFont(name, size) => {
                        state.font_name = Some(name);
                        state.font_size = size as f64;
                    }

                    ContentOperation::SetLeading(leading) => {
                        state.leading = leading as f64;
                    }

                    ContentOperation::SetCharSpacing(spacing) => {
                        state.char_space = spacing as f64;
                    }

                    ContentOperation::SetWordSpacing(spacing) => {
                        state.word_space = spacing as f64;
                    }

                    ContentOperation::SetHorizontalScaling(scale) => {
                        state.horizontal_scale = scale as f64;
                    }

                    ContentOperation::SetTextRise(rise) => {
                        state.text_rise = rise as f64;
                    }

                    ContentOperation::SetTextRenderMode(mode) => {
                        state.render_mode = mode as u8;
                    }

                    ContentOperation::SetTransformMatrix(a, b, c, d, e, f) => {
                        // Update CTM: new_ctm = concat_matrix * current_ctm
                        let [a0, b0, c0, d0, e0, f0] = state.ctm;
                        let a = a as f64;
                        let b = b as f64;
                        let c = c as f64;
                        let d = d as f64;
                        let e = e as f64;
                        let f = f as f64;
                        state.ctm = [
                            a * a0 + b * c0,
                            a * b0 + b * d0,
                            c * a0 + d * c0,
                            c * b0 + d * d0,
                            e * a0 + f * c0 + e0,
                            e * b0 + f * d0 + f0,
                        ];
                    }

                    // Graphics state stack (issue #262). `q` snapshots the
                    // current CTM and fill_color; `Q` restores the most recent
                    // snapshot. Without these, every `cm` accumulates onto the
                    // CTM forever, producing absurd page-space coordinates and
                    // wrong font_size scaling on PDFs that nest graphics state.
                    ContentOperation::SaveGraphicsState => {
                        state.saved_states.push(SavedGraphicsState {
                            ctm: state.ctm,
                            fill_color: state.fill_color,
                        });
                    }
                    ContentOperation::RestoreGraphicsState => {
                        if let Some(saved) = state.saved_states.pop() {
                            state.ctm = saved.ctm;
                            state.fill_color = saved.fill_color;
                        }
                        // Unbalanced Q (pop on empty stack) is silently ignored
                        // to keep extraction robust to malformed PDFs.
                    }

                    // Color operations (Phase 4: Color extraction)
                    ContentOperation::SetNonStrokingGray(gray) => {
                        state.fill_color = Some(Color::gray(gray as f64));
                    }

                    ContentOperation::SetNonStrokingRGB(r, g, b) => {
                        state.fill_color = Some(Color::rgb(r as f64, g as f64, b as f64));
                    }

                    ContentOperation::SetNonStrokingCMYK(c, m, y, k) => {
                        state.fill_color =
                            Some(Color::cmyk(c as f64, m as f64, y as f64, k as f64));
                    }

                    // Issue #269 Phase 1: marked-content operators
                    ContentOperation::BeginMarkedContent(tag) => {
                        let parent_artifact = state.mc_stack.last().is_some_and(|e| e.is_artifact);
                        state.mc_stack.push(MarkedContentEntry {
                            is_artifact: tag == "Artifact" || parent_artifact,
                            tag,
                            mcid: None,
                            actual_text: None,
                        });
                    }

                    ContentOperation::BeginMarkedContentWithProps(tag, props) => {
                        let parent_artifact = state.mc_stack.last().is_some_and(|e| e.is_artifact);
                        let (mcid, actual_text) = resolve_props(&props, page_properties);

                        // If this scope declares ActualText, open a pending run that will be
                        // flushed on the matching EMC. Suppresses per-Tj emission inside the
                        // scope (innermost-ActualText-wins per spec §4).
                        if let Some(ref text) = actual_text {
                            state.pending_actualtext = Some(PendingActualText {
                                text: text.clone(),
                                first_x: 0.0,
                                first_y: 0.0,
                                width: 0.0,
                                font_size: state.font_size,
                                font_name: state.font_name.clone(),
                                is_bold: false, // overwritten on first Tj
                                is_italic: false,
                                color: state.fill_color,
                                stack_depth: state.mc_stack.len(), // BEFORE the push below
                                populated: false,
                            });
                        }

                        state.mc_stack.push(MarkedContentEntry {
                            is_artifact: tag == "Artifact" || parent_artifact,
                            tag,
                            mcid,
                            actual_text,
                        });
                    }

                    ContentOperation::EndMarkedContent => {
                        let popped_depth = state.mc_stack.len();
                        if state.mc_stack.pop().is_none() {
                            // Unbalanced EMC — log and ignore. Real PDFs occasionally emit
                            // dangling EMC (e.g. from incremental updates). We must not panic.
                            tracing::debug!(
                                "extraction: EMC with empty marked-content stack on page {}",
                                page_index + 1
                            );
                        } else if let Some(pending) = state.pending_actualtext.as_ref() {
                            // If we just closed the scope that opened the pending run, flush it.
                            if pending.stack_depth + 1 == popped_depth {
                                let run = state.pending_actualtext.take().unwrap();
                                if run.populated && self.options.preserve_layout {
                                    let (mcid, struct_tag) = innermost_mc_tag(&state.mc_stack);
                                    let in_artifact = state.mc_stack.iter().any(|e| e.is_artifact);
                                    if !in_artifact || self.options.include_artifacts {
                                        fragments.push(TextFragment {
                                            text: run.text,
                                            x: run.first_x,
                                            y: run.first_y,
                                            width: run.width,
                                            height: run.font_size,
                                            font_size: run.font_size,
                                            font_name: run.font_name,
                                            is_bold: run.is_bold,
                                            is_italic: run.is_italic,
                                            color: run.color,
                                            space_decisions: Vec::new(),
                                            mcid,
                                            struct_tag,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    _ => {
                        // Other operations don't affect text extraction
                    }
                }
            }
        }

        {
            let _span = tracing::info_span!("layout_finalize").entered();

            // Sort and process fragments if requested — but ONLY when we're not
            // going to run merge_into_lines later. merge_into_lines does its
            // own (row_id, y, x) sort that needs pre-sort emission order to
            // detect Y-up-jumps for column splitting (issue #265). For the
            // legacy path with reconstruct_paragraphs=false, the early sort is
            // still required because nothing downstream reorders fragments.
            if self.options.sort_by_position
                && !self.options.reconstruct_paragraphs
                && !fragments.is_empty()
            {
                self.sort_and_merge_fragments(&mut fragments);
            }

            // Merge close fragments to eliminate spacing artifacts (kerning fix)
            if self.options.preserve_layout && !fragments.is_empty() {
                fragments = self.merge_close_fragments(&fragments);
            }

            // Reconstruct visual lines and paragraphs from raw fragments.
            // Required for the partition pipeline to produce Element values at
            // paragraph granularity (issue #261).
            if self.options.reconstruct_paragraphs && !fragments.is_empty() {
                let lines = self.merge_into_lines(&fragments);
                fragments = self.merge_into_paragraphs(&lines);
            }

            // Reconstruct text from sorted fragments if layout is preserved
            if self.options.preserve_layout && !fragments.is_empty() {
                extracted_text = self.reconstruct_text_from_fragments(&fragments);
            }
        }

        Ok(ExtractedText {
            text: extracted_text,
            fragments,
        })
    }

    /// Sort text fragments by position and merge them appropriately
    fn sort_and_merge_fragments(&self, fragments: &mut [TextFragment]) {
        // Sort fragments by Y position (top to bottom) then X position (left to right).
        //
        // We quantize Y into bands of `newline_threshold` width so that fragments
        // on the "same line" get identical Y keys. This ensures the comparator is
        // a strict total order (transitive), which Rust's sort algorithm requires.
        // Without quantization, threshold-based "same line" detection breaks
        // transitivity: A≈B and B≈C does NOT imply A≈C.
        let threshold = self.options.newline_threshold;
        fragments.sort_by(|a, b| {
            // Quantize Y to nearest band (PDF Y increases upward, so negate first)
            let band_a = if threshold > 0.0 {
                (-a.y / threshold).round()
            } else {
                -a.y
            };
            let band_b = if threshold > 0.0 {
                (-b.y / threshold).round()
            } else {
                -b.y
            };

            // Compare by Y band (top to bottom), then by X within same band
            band_a.total_cmp(&band_b).then_with(|| a.x.total_cmp(&b.x))
        });

        // Detect columns if requested
        if self.options.detect_columns {
            self.detect_and_sort_columns(fragments);
        }
    }

    /// Detect columns and re-sort fragments accordingly
    fn detect_and_sort_columns(&self, fragments: &mut [TextFragment]) {
        // Group fragments by approximate Y position
        let mut lines: Vec<Vec<&mut TextFragment>> = Vec::new();
        let mut current_line: Vec<&mut TextFragment> = Vec::new();
        let mut last_y = f64::INFINITY;

        for fragment in fragments.iter_mut() {
            let fragment_y = fragment.y;
            if (last_y - fragment_y).abs() > self.options.newline_threshold
                && !current_line.is_empty()
            {
                lines.push(current_line);
                current_line = Vec::new();
            }
            current_line.push(fragment);
            last_y = fragment_y;
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Detect column boundaries
        let mut column_boundaries = vec![0.0];
        for line in &lines {
            if line.len() > 1 {
                for i in 0..line.len() - 1 {
                    let gap = line[i + 1].x - (line[i].x + line[i].width);
                    if gap > self.options.column_threshold {
                        let boundary = line[i].x + line[i].width + gap / 2.0;
                        if !column_boundaries
                            .iter()
                            .any(|&b| (b - boundary).abs() < 10.0)
                        {
                            column_boundaries.push(boundary);
                        }
                    }
                }
            }
        }
        column_boundaries.sort_by(|a, b| a.total_cmp(b));

        // Re-sort fragments by column then Y position
        if column_boundaries.len() > 1 {
            fragments.sort_by(|a, b| {
                // Determine column for each fragment
                let col_a = column_boundaries
                    .iter()
                    .position(|&boundary| a.x < boundary)
                    .unwrap_or(column_boundaries.len())
                    - 1;
                let col_b = column_boundaries
                    .iter()
                    .position(|&boundary| b.x < boundary)
                    .unwrap_or(column_boundaries.len())
                    - 1;

                if col_a != col_b {
                    col_a.cmp(&col_b)
                } else {
                    // Same column, sort by Y position
                    b.y.total_cmp(&a.y)
                }
            });
        }
    }

    /// Reconstruct text from sorted fragments
    fn reconstruct_text_from_fragments(&self, fragments: &[TextFragment]) -> String {
        // First, merge consecutive fragments that are very close together
        let merged_fragments = self.merge_close_fragments(fragments);

        let mut result = String::new();
        let mut last_y = f64::INFINITY;
        let mut last_x = 0.0;
        let mut last_line_ended_with_hyphen = false;

        for fragment in &merged_fragments {
            // Check if we need a newline
            let y_diff = (last_y - fragment.y).abs();
            if !result.is_empty() && y_diff > self.options.newline_threshold {
                // Handle hyphenation
                if self.options.merge_hyphenated && last_line_ended_with_hyphen {
                    // Remove the hyphen and don't add newline
                    if result.ends_with('-') {
                        result.pop();
                    }
                } else {
                    result.push('\n');
                }
            } else if !result.is_empty() {
                // Check if we need a space
                let x_gap = fragment.x - last_x;
                if x_gap > self.options.space_threshold * fragment.font_size {
                    result.push(' ');
                }
            }

            result.push_str(&fragment.text);
            last_line_ended_with_hyphen = fragment.text.ends_with('-');
            last_y = fragment.y;
            last_x = fragment.x + fragment.width;
        }

        result
    }

    /// Merge fragments that are very close together on the same line
    /// This fixes artifacts like "IN VO ICE" -> "INVOICE"
    fn merge_close_fragments(&self, fragments: &[TextFragment]) -> Vec<TextFragment> {
        if fragments.is_empty() {
            return Vec::new();
        }

        let mut merged = Vec::new();
        let mut current = fragments[0].clone();

        for fragment in &fragments[1..] {
            // Check if this fragment is on the same line and very close
            let y_diff = (current.y - fragment.y).abs();
            let x_gap = fragment.x - (current.x + current.width);

            // Merge if on same line and gap is less than a character width
            // Use 0.5 * font_size as threshold - this catches most artificial spacing
            let should_merge = y_diff < 1.0  // Same line (very tight tolerance)
                && x_gap >= 0.0  // Fragment is to the right
                && x_gap < fragment.font_size * 0.5 // Gap less than 50% of font size
                && current.mcid == fragment.mcid;

            if should_merge {
                // Merge this fragment into current, preserving word boundaries
                // when the gap exceeds the space threshold.
                if x_gap > self.options.space_threshold * fragment.font_size {
                    current.text.push(' ');
                }
                current.text.push_str(&fragment.text);
                current.width = (fragment.x + fragment.width) - current.x;
            } else {
                // Start a new fragment
                merged.push(current);
                current = fragment.clone();
            }
        }

        merged.push(current);
        merged
    }

    /// Extract font resources from page
    ///
    /// Clears the per-page name cache (font names are page-local in PDF), but
    /// reuses previously parsed font objects via `font_object_cache` to avoid
    /// re-parsing the same font object across multiple pages.
    fn extract_font_resources<R: Read + Seek>(
        &mut self,
        page: &ParsedPage,
        document: &PdfDocument<R>,
    ) -> ParseResult<()> {
        // Clear per-page name mapping (font names like /F1 are page-local)
        self.font_cache.clear();

        // Try to get resources manually from page dictionary first
        // This is necessary because ParsedPage.get_resources() may not always work
        if let Some(res_ref) = page.dict.get("Resources").and_then(|o| o.as_reference()) {
            if let Ok(PdfObject::Dictionary(resources)) = document.get_object(res_ref.0, res_ref.1)
            {
                if let Some(PdfObject::Dictionary(font_dict)) = resources.get("Font") {
                    for (font_name, font_obj) in font_dict.0.iter() {
                        if let Some(font_ref) = font_obj.as_reference() {
                            self.cache_font_by_ref::<R>(&font_name.0, font_ref, document);
                        }
                    }
                }
            }
        } else if let Some(resources) = page.get_resources() {
            // Fallback to get_resources() if Resources is not a reference
            if let Some(PdfObject::Dictionary(font_dict)) = resources.get("Font") {
                for (font_name, font_obj) in font_dict.0.iter() {
                    if let Some(font_ref) = font_obj.as_reference() {
                        self.cache_font_by_ref::<R>(&font_name.0, font_ref, document);
                    }
                }
            }
        }

        Ok(())
    }

    /// Cache a font, reusing the persistent object cache when possible.
    fn cache_font_by_ref<R: Read + Seek>(
        &mut self,
        font_name: &str,
        font_ref: (u32, u16),
        document: &PdfDocument<R>,
    ) {
        // Check persistent object cache first — avoids re-parsing across pages
        if let Some(cached) = self.font_object_cache.get(&font_ref) {
            self.font_cache
                .insert(font_name.to_string(), cached.clone());
            tracing::debug!(
                "Reused cached font object ({}, {}): {} (ToUnicode: {})",
                font_ref.0,
                font_ref.1,
                font_name,
                cached.to_unicode.is_some()
            );
            return;
        }

        // Parse font object
        if let Ok(PdfObject::Dictionary(font_dict)) = document.get_object(font_ref.0, font_ref.1) {
            let mut cmap_extractor: CMapTextExtractor<R> = CMapTextExtractor::new();
            if let Ok(font_info) = cmap_extractor.extract_font_info(&font_dict, document) {
                let has_to_unicode = font_info.to_unicode.is_some();
                // Store in persistent cache
                self.font_object_cache.insert(font_ref, font_info.clone());
                // Store in per-page name cache
                self.font_cache.insert(font_name.to_string(), font_info);
                tracing::debug!(
                    "Parsed and cached font ({}, {}): {} (ToUnicode: {})",
                    font_ref.0,
                    font_ref.1,
                    font_name,
                    has_to_unicode
                );
            }
        }
    }

    /// Decode text using the current font encoding and ToUnicode mapping
    fn decode_text(&self, text: &[u8], state: &TextState) -> ParseResult<String> {
        use crate::text::encoding::TextEncoding;

        // First, try to use cached font information with ToUnicode CMap
        if let Some(ref font_name) = state.font_name {
            if let Some(font_info) = self.font_cache.get(font_name) {
                // Try CMap-based decoding first (free function — no allocation)
                if let Ok(decoded) =
                    crate::text::extraction_cmap::decode_text_with_font(text, font_info)
                {
                    // Only accept if we got meaningful text (not all null bytes or garbage)
                    if !decoded.trim().is_empty()
                        && !decoded.chars().all(|c| c == '\0' || c.is_ascii_control())
                    {
                        // Apply sanitization to remove control characters (Issue #116)
                        let sanitized = sanitize_extracted_text(&decoded);
                        tracing::debug!(
                            "Successfully decoded text using CMap for font {}: {:?} -> \"{}\"",
                            font_name,
                            text,
                            sanitized
                        );
                        return Ok(sanitized);
                    }
                }

                tracing::debug!(
                    "CMap decoding failed or produced garbage for font {}, falling back to encoding",
                    font_name
                );
            }
        }

        // Fall back to encoding-based decoding
        let encoding = if let Some(ref font_name) = state.font_name {
            match font_name.to_lowercase().as_str() {
                name if name.contains("macroman") => TextEncoding::MacRomanEncoding,
                name if name.contains("winansi") => TextEncoding::WinAnsiEncoding,
                name if name.contains("standard") => TextEncoding::StandardEncoding,
                name if name.contains("pdfdoc") => TextEncoding::PdfDocEncoding,
                _ => {
                    // Default based on common patterns
                    if font_name.starts_with("Times")
                        || font_name.starts_with("Helvetica")
                        || font_name.starts_with("Courier")
                    {
                        TextEncoding::WinAnsiEncoding // Most common for standard fonts
                    } else {
                        TextEncoding::PdfDocEncoding // Safe default
                    }
                }
            }
        } else {
            TextEncoding::WinAnsiEncoding // Default for most PDFs
        };

        let fallback_result = encoding.decode(text);
        // Apply sanitization to remove control characters (Issue #116)
        let sanitized = sanitize_extracted_text(&fallback_result);
        tracing::debug!(
            "Fallback encoding decoding: {:?} -> \"{}\"",
            text,
            sanitized
        );
        Ok(sanitized)
    }
}

impl Default for TextExtractor {
    fn default() -> Self {
        Self::new()
    }
}

/// Emit a `TextFragment` for one decoded text-show event under `preserve_layout`.
///
/// Encapsulates the style-derivation + push sequence shared by every
/// text-show operator handler in `extract_from_page` (`Tj`, `TJ`, `'`,
/// `"`). The caller supplies the pen origin `(x, y)` already mapped to
/// user space (typically via `text_origin(&state)`); doing so avoids the
/// double `multiply_matrix + transform_point` that prior versions did
/// (handler computed it for `last_x`/`last_y`, then this fn recomputed
/// it on the same `state`).
///
/// Skips emission when an ancestor in the marked-content stack is `/Artifact`
/// and `include_artifacts` is false. When a pending ActualText run is
/// active in the current scope, accumulates the text-width contribution and
/// records the first origin instead of pushing a fragment (the run is flushed
/// once on EMC, see Task 8's EndMarkedContent handler).
///
/// `mcid` and `struct_tag` come from the innermost ancestor on the stack that
/// declared `/MCID`; non-tagged content leaves both as `None`.
fn emit_text_fragment(
    fragments: &mut Vec<TextFragment>,
    decoded: &str,
    text_width: f64,
    x: f64,
    y: f64,
    state: &mut TextState,
    include_artifacts: bool,
) {
    if decoded.is_empty() {
        return;
    }

    // Artifact filter (default: skip emission for Artifact subtrees).
    if !include_artifacts && state.mc_stack.iter().any(|e| e.is_artifact) {
        return;
    }

    let (is_bold, is_italic) = state
        .font_name
        .as_ref()
        .map(|name| parse_font_style(name))
        .unwrap_or((false, false));

    // Issue #262: font_size, height, and width must be in page space so that
    // downstream heuristics (line/paragraph reconstruction, header/footer zone
    // detection, table detection) reason about real geometry. `x` and `y` are
    // already page-space (caller transforms via `text_origin`); we still need
    // to scale the size/width fields by the combined `text_matrix × CTM`.
    let combined = multiply_matrix(&state.text_matrix, &state.ctm);
    let x_scale = (combined[0] * combined[0] + combined[1] * combined[1]).sqrt();
    let y_scale = (combined[2] * combined[2] + combined[3] * combined[3]).sqrt();
    let effective_width = text_width * x_scale;
    let effective_size = state.font_size * y_scale;

    // If a pending ActualText run is active in the current scope, accumulate
    // into it instead of emitting a fragment now. The run is flushed on the
    // matching EMC by the EndMarkedContent arm (Task 8).
    // Hoist font_name/fill_color reads before taking &mut on pending_actualtext
    // to avoid borrow-checker conflicts with the disjoint fields.
    let local_font_name = state.font_name.clone();
    let local_fill_color = state.fill_color;
    if let Some(pending) = state.pending_actualtext.as_mut() {
        if !pending.populated {
            pending.first_x = x;
            pending.first_y = y;
            pending.font_size = effective_size;
            pending.font_name = local_font_name;
            pending.is_bold = is_bold;
            pending.is_italic = is_italic;
            pending.color = local_fill_color;
            pending.populated = true;
        }
        pending.width += effective_width;
        return;
    }

    let (mcid, struct_tag) = innermost_mc_tag(&state.mc_stack);

    fragments.push(TextFragment {
        text: decoded.to_owned(),
        x,
        y,
        width: effective_width,
        height: effective_size,
        font_size: effective_size,
        font_name: state.font_name.clone(),
        is_bold,
        is_italic,
        color: state.fill_color,
        space_decisions: Vec::new(),
        mcid,
        struct_tag,
    });
}

/// Pen origin (user-space coordinates) of the next glyph in the current
/// text state.
///
/// Per ISO 32000-1 §8.3.4, the text rendering matrix is `Tm × CTM` (row-vector
/// convention). `multiply_matrix(a, b)` returns the matrix that applies `a`
/// first and then `b`, so the correct composition is
/// `multiply_matrix(text_matrix, ctm)`. Prior to issue #262 this used the
/// reverse order which gave correct results only when the CTM was an identity
/// or pure-translation matrix; non-uniform CTM scaling produced wrong origins.
fn text_origin(state: &TextState) -> (f64, f64) {
    let combined = multiply_matrix(&state.text_matrix, &state.ctm);
    transform_point(0.0, 0.0, &combined)
}

/// Multiply two transformation matrices
fn multiply_matrix(a: &[f64; 6], b: &[f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

/// Decode a PDF string operand into Rust `String`.
///
/// PDF strings inside marked-content properties (notably `/ActualText`)
/// may be encoded as:
///
/// - **UTF-16BE with BOM**: leading `0xFE 0xFF`, then big-endian 16-bit
///   code units. This is the canonical encoding for non-ASCII ActualText
///   (e.g. `fi` ligature, Greek/math symbols). Decoded via `String::from_utf16_lossy`
///   so invalid surrogate pairs become `U+FFFD` rather than panicking.
/// - **PDFDocEncoding** (the catch-all for non-BOM bytes). For the ASCII
///   subset (0x20-0x7E) PDFDocEncoding is identical to Latin-1. We
///   conservatively map byte-by-byte to `char`. A future revision can
///   plug in the full PDFDocEncoding table if a real PDF emerges with
///   high-bit characters in ActualText *without* a UTF-16BE BOM (rare;
///   most producers emit the BOM when going outside ASCII).
fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let mut code_units: Vec<u16> = Vec::with_capacity((bytes.len() - 2) / 2);
        let mut i = 2;
        while i + 1 < bytes.len() {
            code_units.push(u16::from_be_bytes([bytes[i], bytes[i + 1]]));
            i += 2;
        }
        String::from_utf16_lossy(&code_units)
    } else {
        bytes.iter().map(|&b| b as char).collect()
    }
}

/// Resolve a `MarkedContentProps` to `(mcid, actual_text)`.
///
/// For `Inline` props, walk the map: `/MCID` (Integer, must fit in `u32`)
/// becomes `mcid`; `/ActualText` (String) is decoded via `decode_pdf_string`.
///
/// For `ResourceRef(name)`, look up `properties.get(name)`. If found and
/// it's a Dictionary, extract `/MCID` and `/ActualText` from there. If
/// not found (or the named entry is not a dict), return `(None, None)`
/// — a malformed reference must not abort extraction.
fn resolve_props(
    props: &crate::parser::content::MarkedContentProps,
    properties: Option<&crate::parser::objects::PdfDictionary>,
) -> (Option<u32>, Option<String>) {
    use crate::parser::content::{MarkedContentProps, MarkedContentValue};

    let map_mcid_actual =
        |map: &std::collections::HashMap<String, MarkedContentValue>| -> (Option<u32>, Option<String>) {
            let mcid = match map.get("MCID") {
                Some(MarkedContentValue::Integer(n)) if *n >= 0 && *n <= u32::MAX as i64 => {
                    Some(*n as u32)
                }
                _ => None,
            };
            let actual = match map.get("ActualText") {
                Some(MarkedContentValue::String(bytes)) => Some(decode_pdf_string(bytes)),
                _ => None,
            };
            (mcid, actual)
        };

    match props {
        MarkedContentProps::Inline(map) => map_mcid_actual(map),
        MarkedContentProps::ResourceRef(name) => {
            let Some(properties) = properties else {
                return (None, None);
            };
            let Some(entry) = properties.get(name) else {
                return (None, None);
            };
            let crate::parser::objects::PdfObject::Dictionary(dict) = entry else {
                return (None, None);
            };
            let mcid = dict.get("MCID").and_then(|o| match o {
                crate::parser::objects::PdfObject::Integer(n)
                    if *n >= 0 && *n <= u32::MAX as i64 =>
                {
                    Some(*n as u32)
                }
                _ => None,
            });
            let actual_text = dict.get("ActualText").and_then(|o| match o {
                crate::parser::objects::PdfObject::String(s) => {
                    Some(decode_pdf_string(s.as_bytes()))
                }
                _ => None,
            });
            (mcid, actual_text)
        }
    }
}

/// Walk the marked-content stack from innermost (top) outward, returning the
/// first entry's `(mcid, tag)` pair whose `mcid` is `Some`. Returns
/// `(None, None)` when no ancestor declared an MCID — typical of non-tagged
/// PDFs, in which case the `None == None` grouping-key invariant preserves
/// legacy behaviour.
fn innermost_mc_tag(stack: &[MarkedContentEntry]) -> (Option<u32>, Option<String>) {
    stack
        .iter()
        .rev()
        .find(|e| e.mcid.is_some())
        .map_or((None, None), |e| (e.mcid, Some(e.tag.clone())))
}

/// Transform a point using a transformation matrix
fn transform_point(x: f64, y: f64, matrix: &[f64; 6]) -> (f64, f64) {
    let tx = matrix[0] * x + matrix[2] * y + matrix[4];
    let ty = matrix[1] * x + matrix[3] * y + matrix[5];
    (tx, ty)
}

/// Calculate text width using actual font metrics (including kerning)
fn calculate_text_width(text: &str, font_size: f64, font_info: Option<&FontInfo>) -> f64 {
    // If we have font metrics, use them for accurate width calculation
    if let Some(font) = font_info {
        if let Some(ref widths) = font.metrics.widths {
            let first_char = font.metrics.first_char.unwrap_or(0);
            let last_char = font.metrics.last_char.unwrap_or(255);
            let missing_width = font.metrics.missing_width.unwrap_or(500.0);

            let mut total_width = 0.0;
            let mut chars = text.chars().peekable();

            while let Some(ch) = chars.next() {
                let char_code = ch as u32;

                // Get width from Widths array or use missing_width
                let width = if char_code >= first_char && char_code <= last_char {
                    let index = (char_code - first_char) as usize;
                    widths.get(index).copied().unwrap_or(missing_width)
                } else {
                    missing_width
                };

                // Convert from glyph space (1/1000 units) to user space
                total_width += width / 1000.0 * font_size;

                // Apply kerning if available (for character pairs)
                if let Some(ref kerning) = font.metrics.kerning {
                    if let Some(&next_ch) = chars.peek() {
                        let next_char = next_ch as u32;
                        if let Some(&kern_value) = kerning.get(&(char_code, next_char)) {
                            // Kerning is in FUnits (1/1000), convert to user space
                            total_width += kern_value / 1000.0 * font_size;
                        }
                    }
                }
            }

            return total_width;
        }
    }

    // Fallback to simplified calculation if no metrics available
    text.len() as f64 * font_size * 0.5
}

/// Sanitize extracted text by removing or replacing control characters.
///
/// This function addresses Issue #116 where extracted text contains NUL bytes (`\0`)
/// and ETX characters (`\u{3}`) where spaces should appear.
///
/// # Behavior
///
/// - Replaces `\0\u{3}` sequences with a single space (common word separator pattern)
/// - Replaces standalone `\0` (NUL) with space
/// - Removes other ASCII control characters (0x01-0x1F) except:
///   - `\t` (0x09) - Tab
///   - `\n` (0x0A) - Line feed
///   - `\r` (0x0D) - Carriage return
/// - Collapses multiple consecutive spaces into a single space
///
/// # Examples
///
/// ```
/// use oxidize_pdf::text::extraction::sanitize_extracted_text;
///
/// // Issue #116 pattern: NUL+ETX as word separator
/// let dirty = "a\0\u{3}sergeant\0\u{3}and";
/// assert_eq!(sanitize_extracted_text(dirty), "a sergeant and");
///
/// // Standalone NUL becomes space
/// let with_nul = "word\0another";
/// assert_eq!(sanitize_extracted_text(with_nul), "word another");
///
/// // Clean text passes through unchanged
/// let clean = "Normal text";
/// assert_eq!(sanitize_extracted_text(clean), "Normal text");
/// ```
pub fn sanitize_extracted_text(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Pre-allocate with same capacity (result will be <= input length)
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut last_was_space = false;

    while let Some(ch) = chars.next() {
        match ch {
            // NUL byte - check if followed by ETX for the \0\u{3} pattern
            '\0' => {
                // Peek at next char to detect \0\u{3} sequence
                if chars.peek() == Some(&'\u{3}') {
                    chars.next(); // consume the ETX
                }
                // In both cases (standalone NUL or NUL+ETX), emit space
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }

            // ETX alone (not preceded by NUL) - remove it
            '\u{3}' => {
                // Don't emit anything, just skip
            }

            // Preserve allowed whitespace
            '\t' | '\n' | '\r' => {
                result.push(ch);
                // Reset space tracking on newlines but not tabs
                last_was_space = ch == '\t';
            }

            // Regular space - collapse multiples
            ' ' => {
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }

            // Other control characters (0x01-0x1F except tab/newline/CR) - remove
            c if c.is_ascii_control() => {
                // Skip control characters
            }

            // Normal characters - keep them
            _ => {
                result.push(ch);
                last_was_space = false;
            }
        }
    }

    result
}

/// Assign a logical row identifier to each fragment based on Y-up-jumps in
/// emission order. Used by `merge_into_lines` to distinguish columns in
/// multi-column layouts where a single outer BDC scope makes mcid uniform.
///
/// Increments `row_id` whenever the next fragment's Y exceeds the previous
/// by more than `max(font_size * 0.5, 2.0)`. Superscripts (small positive
/// deltas) and normal line descents (negative deltas) leave `row_id`
/// unchanged. See `docs/superpowers/specs/2026-05-23-issue-265-line-interleaving-design.md`.
fn assign_row_ids(fragments: &[TextFragment]) -> Vec<u32> {
    let mut result = Vec::with_capacity(fragments.len());
    let mut row_id: u32 = 0;
    let mut prev_y: Option<f64> = None;
    for frag in fragments {
        if let Some(py) = prev_y {
            let delta = frag.y - py;
            // Threshold anchored to the arriving fragment's font_size; for the
            // symmetric same-font case (body→body, same font) this is equivalent
            // to anchoring to the previous fragment.
            let threshold = (frag.font_size * 0.5).max(2.0);
            if delta > threshold {
                row_id += 1;
            }
        }
        result.push(row_id);
        prev_y = Some(frag.y);
    }
    result
}

fn build_line_fragment(line: Vec<&TextFragment>, space_threshold: f64) -> TextFragment {
    let head = line[0];
    let mut text = String::new();
    let mut x_min = head.x;
    let mut x_max = head.x + head.width;
    let mut y_min = head.y;
    let mut y_max = head.y + head.height;

    for (i, frag) in line.iter().enumerate() {
        if i > 0 {
            let prev = line[i - 1];
            let gap = frag.x - (prev.x + prev.width);
            if gap > space_threshold * frag.font_size {
                text.push(' ');
            }
        }
        text.push_str(&frag.text);
        x_min = x_min.min(frag.x);
        x_max = x_max.max(frag.x + frag.width);
        y_min = y_min.min(frag.y);
        y_max = y_max.max(frag.y + frag.height);
    }

    TextFragment {
        text,
        x: x_min,
        y: y_min,
        width: x_max - x_min,
        height: y_max - y_min,
        font_size: head.font_size,
        font_name: head.font_name.clone(),
        is_bold: head.is_bold,
        is_italic: head.is_italic,
        color: head.color,
        space_decisions: Vec::new(),
        mcid: head.mcid,
        struct_tag: head.struct_tag.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matrix_multiplication() {
        let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let translation = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];

        let result = multiply_matrix(&identity, &translation);
        assert_eq!(result, translation);

        let result2 = multiply_matrix(&translation, &identity);
        assert_eq!(result2, translation);
    }

    #[test]
    fn test_transform_point() {
        let translation = [1.0, 0.0, 0.0, 1.0, 10.0, 20.0];
        let (x, y) = transform_point(5.0, 5.0, &translation);
        assert_eq!(x, 15.0);
        assert_eq!(y, 25.0);
    }

    #[test]
    fn test_extraction_options_default() {
        let options = ExtractionOptions::default();
        assert!(!options.preserve_layout);
        assert_eq!(options.space_threshold, 0.3);
        assert_eq!(options.newline_threshold, 10.0);
        assert!(options.sort_by_position);
        assert!(!options.detect_columns);
        assert_eq!(options.column_threshold, 50.0);
        assert!(options.merge_hyphenated);
    }

    #[test]
    fn test_extraction_options_custom() {
        let options = ExtractionOptions {
            preserve_layout: true,
            space_threshold: 0.5,
            newline_threshold: 15.0,
            sort_by_position: false,
            detect_columns: true,
            column_threshold: 75.0,
            merge_hyphenated: false,
            track_space_decisions: false,
            reconstruct_paragraphs: false,
            include_artifacts: false,
        };
        assert!(options.preserve_layout);
        assert_eq!(options.space_threshold, 0.5);
        assert_eq!(options.newline_threshold, 15.0);
        assert!(!options.sort_by_position);
        assert!(options.detect_columns);
        assert_eq!(options.column_threshold, 75.0);
        assert!(!options.merge_hyphenated);
    }

    #[test]
    fn test_parse_font_style_bold() {
        // PostScript style
        assert_eq!(parse_font_style("Helvetica-Bold"), (true, false));
        assert_eq!(parse_font_style("TimesNewRoman-Bold"), (true, false));

        // TrueType style
        assert_eq!(parse_font_style("Arial Bold"), (true, false));
        assert_eq!(parse_font_style("Calibri Bold"), (true, false));

        // Short form
        assert_eq!(parse_font_style("Helvetica-B"), (true, false));
    }

    #[test]
    fn test_parse_font_style_italic() {
        // PostScript style
        assert_eq!(parse_font_style("Helvetica-Italic"), (false, true));
        assert_eq!(parse_font_style("Times-Oblique"), (false, true));

        // TrueType style
        assert_eq!(parse_font_style("Arial Italic"), (false, true));
        assert_eq!(parse_font_style("Courier Oblique"), (false, true));

        // Short form
        assert_eq!(parse_font_style("Helvetica-I"), (false, true));
    }

    #[test]
    fn test_parse_font_style_bold_italic() {
        assert_eq!(parse_font_style("Helvetica-BoldItalic"), (true, true));
        assert_eq!(parse_font_style("Times-BoldOblique"), (true, true));
        assert_eq!(parse_font_style("Arial Bold Italic"), (true, true));
    }

    #[test]
    fn test_parse_font_style_regular() {
        assert_eq!(parse_font_style("Helvetica"), (false, false));
        assert_eq!(parse_font_style("Times-Roman"), (false, false));
        assert_eq!(parse_font_style("Courier"), (false, false));
        assert_eq!(parse_font_style("Arial"), (false, false));
    }

    #[test]
    fn test_parse_font_style_edge_cases() {
        // Empty and unusual cases
        assert_eq!(parse_font_style(""), (false, false));
        assert_eq!(parse_font_style("UnknownFont"), (false, false));

        // Case insensitive
        assert_eq!(parse_font_style("HELVETICA-BOLD"), (true, false));
        assert_eq!(parse_font_style("times-ITALIC"), (false, true));
    }

    #[test]
    fn test_text_fragment() {
        let fragment = TextFragment {
            text: "Hello".to_string(),
            x: 100.0,
            y: 200.0,
            width: 50.0,
            height: 12.0,
            font_size: 10.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
            space_decisions: Vec::new(),
            mcid: None,
            struct_tag: None,
        };
        assert_eq!(fragment.text, "Hello");
        assert_eq!(fragment.x, 100.0);
        assert_eq!(fragment.y, 200.0);
        assert_eq!(fragment.width, 50.0);
        assert_eq!(fragment.height, 12.0);
        assert_eq!(fragment.font_size, 10.0);
    }

    #[test]
    fn test_extracted_text() {
        let fragments = vec![
            TextFragment {
                text: "Hello".to_string(),
                x: 100.0,
                y: 200.0,
                width: 50.0,
                height: 12.0,
                font_size: 10.0,
                font_name: None,
                is_bold: false,
                is_italic: false,
                color: None,
                space_decisions: Vec::new(),
                mcid: None,
                struct_tag: None,
            },
            TextFragment {
                text: "World".to_string(),
                x: 160.0,
                y: 200.0,
                width: 50.0,
                height: 12.0,
                font_size: 10.0,
                font_name: None,
                is_bold: false,
                is_italic: false,
                color: None,
                space_decisions: Vec::new(),
                mcid: None,
                struct_tag: None,
            },
        ];

        let extracted = ExtractedText {
            text: "Hello World".to_string(),
            fragments: fragments,
        };

        assert_eq!(extracted.text, "Hello World");
        assert_eq!(extracted.fragments.len(), 2);
        assert_eq!(extracted.fragments[0].text, "Hello");
        assert_eq!(extracted.fragments[1].text, "World");
    }

    #[test]
    fn test_text_state_default() {
        let state = TextState::default();
        assert_eq!(state.text_matrix, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(state.text_line_matrix, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(state.ctm, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(state.leading, 0.0);
        assert_eq!(state.char_space, 0.0);
        assert_eq!(state.word_space, 0.0);
        assert_eq!(state.horizontal_scale, 100.0);
        assert_eq!(state.text_rise, 0.0);
        assert_eq!(state.font_size, 0.0);
        assert!(state.font_name.is_none());
        assert_eq!(state.render_mode, 0);
    }

    #[test]
    fn test_matrix_operations() {
        // Test rotation matrix
        let rotation = [0.0, 1.0, -1.0, 0.0, 0.0, 0.0]; // 90 degree rotation
        let (x, y) = transform_point(1.0, 0.0, &rotation);
        assert_eq!(x, 0.0);
        assert_eq!(y, 1.0);

        // Test scaling matrix
        let scale = [2.0, 0.0, 0.0, 3.0, 0.0, 0.0];
        let (x, y) = transform_point(5.0, 5.0, &scale);
        assert_eq!(x, 10.0);
        assert_eq!(y, 15.0);

        // Test complex transformation
        let complex = [2.0, 1.0, 1.0, 2.0, 10.0, 20.0];
        let (x, y) = transform_point(1.0, 1.0, &complex);
        assert_eq!(x, 13.0); // 2*1 + 1*1 + 10
        assert_eq!(y, 23.0); // 1*1 + 2*1 + 20
    }

    #[test]
    fn test_text_extractor_new() {
        let extractor = TextExtractor::new();
        let options = extractor.options;
        assert!(!options.preserve_layout);
        assert_eq!(options.space_threshold, 0.3);
        assert_eq!(options.newline_threshold, 10.0);
        assert!(options.sort_by_position);
        assert!(!options.detect_columns);
        assert_eq!(options.column_threshold, 50.0);
        assert!(options.merge_hyphenated);
    }

    #[test]
    fn test_text_extractor_with_options() {
        let options = ExtractionOptions {
            preserve_layout: true,
            space_threshold: 0.3,
            newline_threshold: 12.0,
            sort_by_position: false,
            detect_columns: true,
            column_threshold: 60.0,
            merge_hyphenated: false,
            track_space_decisions: false,
            reconstruct_paragraphs: false,
            include_artifacts: false,
        };
        let extractor = TextExtractor::with_options(options.clone());
        assert_eq!(extractor.options.preserve_layout, options.preserve_layout);
        assert_eq!(extractor.options.space_threshold, options.space_threshold);
        assert_eq!(
            extractor.options.newline_threshold,
            options.newline_threshold
        );
        assert_eq!(extractor.options.sort_by_position, options.sort_by_position);
        assert_eq!(extractor.options.detect_columns, options.detect_columns);
        assert_eq!(extractor.options.column_threshold, options.column_threshold);
        assert_eq!(extractor.options.merge_hyphenated, options.merge_hyphenated);
    }

    // =========================================================================
    // RIGOROUS TESTS FOR FONT METRICS TEXT WIDTH CALCULATION
    // =========================================================================

    #[test]
    fn test_calculate_text_width_with_no_font_info() {
        // Test fallback: should use simplified calculation
        let width = calculate_text_width("Hello", 12.0, None);

        // Expected: 5 chars * 12.0 * 0.5 = 30.0
        assert_eq!(
            width, 30.0,
            "Without font info, should use simplified calculation: len * font_size * 0.5"
        );
    }

    #[test]
    fn test_calculate_text_width_with_empty_metrics() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with no widths array
        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: None,
                last_char: None,
                widths: None,
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        let width = calculate_text_width("Hello", 12.0, Some(&font_info));

        // Should fall back to simplified calculation
        assert_eq!(
            width, 30.0,
            "Without widths array, should fall back to simplified calculation"
        );
    }

    #[test]
    fn test_calculate_text_width_with_complete_metrics() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with complete metrics for ASCII range 32-126
        // Simulate typical Helvetica widths (in 1/1000 units)
        let mut widths = vec![0.0; 95]; // 95 chars from 32 to 126

        // Set specific widths for "Hello" (H=722, e=556, l=278, o=611)
        widths[72 - 32] = 722.0; // 'H' is ASCII 72
        widths[101 - 32] = 556.0; // 'e' is ASCII 101
        widths[108 - 32] = 278.0; // 'l' is ASCII 108
        widths[111 - 32] = 611.0; // 'o' is ASCII 111

        let font_info = FontInfo {
            name: "Helvetica".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(widths),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        let width = calculate_text_width("Hello", 12.0, Some(&font_info));

        // Expected calculation (widths in glyph space / 1000 * font_size):
        // H: 722/1000 * 12 = 8.664
        // e: 556/1000 * 12 = 6.672
        // l: 278/1000 * 12 = 3.336
        // l: 278/1000 * 12 = 3.336
        // o: 611/1000 * 12 = 7.332
        // Total: 29.34
        let expected = (722.0 + 556.0 + 278.0 + 278.0 + 611.0) / 1000.0 * 12.0;
        let tolerance = 0.0001; // Floating point tolerance
        assert!(
            (width - expected).abs() < tolerance,
            "Should calculate width using actual character metrics: expected {}, got {}, diff {}",
            expected,
            width,
            (width - expected).abs()
        );

        // Verify it's different from simplified calculation
        let simplified = 5.0 * 12.0 * 0.5; // 30.0
        assert_ne!(
            width, simplified,
            "Metrics-based calculation should differ from simplified (30.0)"
        );
    }

    #[test]
    fn test_calculate_text_width_character_outside_range() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with narrow range (only covers 'A'-'Z')
        let widths = vec![722.0; 26]; // All uppercase letters same width

        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(65), // 'A'
                last_char: Some(90),  // 'Z'
                widths: Some(widths),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        // Test with character outside range
        let width = calculate_text_width("A1", 10.0, Some(&font_info));

        // Expected:
        // 'A' (65) is in range: 722/1000 * 10 = 7.22
        // '1' (49) is outside range: missing_width 500/1000 * 10 = 5.0
        // Total: 12.22
        let expected = (722.0 / 1000.0 * 10.0) + (500.0 / 1000.0 * 10.0);
        assert_eq!(
            width, expected,
            "Should use missing_width for characters outside range"
        );
    }

    #[test]
    fn test_calculate_text_width_missing_width_in_array() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with incomplete widths array (some characters have 0.0)
        let mut widths = vec![500.0; 95]; // Default width
        widths[10] = 0.0; // Character at index 10 has no width defined

        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(widths),
                missing_width: Some(600.0),
                kerning: None,
            },
        };

        // Character 42 (index 10 from first_char 32)
        let char_code = 42u8 as char; // '*'
        let text = char_code.to_string();
        let width = calculate_text_width(&text, 10.0, Some(&font_info));

        // Character is in range but width is 0.0, should NOT fall back to missing_width
        // (0.0 is a valid width for zero-width characters)
        assert_eq!(
            width, 0.0,
            "Should use 0.0 width from array, not missing_width"
        );
    }

    #[test]
    fn test_calculate_text_width_empty_string() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(vec![500.0; 95]),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        let width = calculate_text_width("", 12.0, Some(&font_info));
        assert_eq!(width, 0.0, "Empty string should have zero width");

        // Also test without font info
        let width_no_font = calculate_text_width("", 12.0, None);
        assert_eq!(
            width_no_font, 0.0,
            "Empty string should have zero width (no font)"
        );
    }

    #[test]
    fn test_calculate_text_width_unicode_characters() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Font with limited ASCII range
        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(vec![500.0; 95]),
                missing_width: Some(600.0),
                kerning: None,
            },
        };

        // Test with Unicode characters outside ASCII range
        let width = calculate_text_width("Ñ", 10.0, Some(&font_info));

        // 'Ñ' (U+00D1, code 209) is outside range, should use missing_width
        // Expected: 600/1000 * 10 = 6.0
        assert_eq!(
            width, 6.0,
            "Unicode character outside range should use missing_width"
        );
    }

    #[test]
    fn test_calculate_text_width_different_font_sizes() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        let font_info = FontInfo {
            name: "TestFont".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(65), // 'A'
                last_char: Some(65),  // 'A'
                widths: Some(vec![722.0]),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        // Test same character with different font sizes
        let width_10 = calculate_text_width("A", 10.0, Some(&font_info));
        let width_20 = calculate_text_width("A", 20.0, Some(&font_info));

        // Widths should scale linearly with font size
        assert_eq!(width_10, 722.0 / 1000.0 * 10.0);
        assert_eq!(width_20, 722.0 / 1000.0 * 20.0);
        assert_eq!(
            width_20,
            width_10 * 2.0,
            "Width should scale linearly with font size"
        );
    }

    #[test]
    fn test_calculate_text_width_proportional_vs_monospace() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Simulate proportional font (different widths)
        let proportional_widths = vec![278.0, 556.0, 722.0]; // i, m, W
        let proportional_font = FontInfo {
            name: "Helvetica".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(105), // 'i'
                last_char: Some(107),  // covers i, j, k
                widths: Some(proportional_widths),
                missing_width: Some(500.0),
                kerning: None,
            },
        };

        // Simulate monospace font (same width)
        let monospace_widths = vec![600.0, 600.0, 600.0];
        let monospace_font = FontInfo {
            name: "Courier".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(105),
                last_char: Some(107),
                widths: Some(monospace_widths),
                missing_width: Some(600.0),
                kerning: None,
            },
        };

        let prop_width = calculate_text_width("i", 12.0, Some(&proportional_font));
        let mono_width = calculate_text_width("i", 12.0, Some(&monospace_font));

        // Proportional 'i' should be narrower than monospace 'i'
        assert!(
            prop_width < mono_width,
            "Proportional 'i' ({}) should be narrower than monospace 'i' ({})",
            prop_width,
            mono_width
        );
    }

    // =========================================================================
    // CRITICAL KERNING TESTS (Issue #87 - Quality Agent Required)
    // =========================================================================

    #[test]
    fn test_calculate_text_width_with_kerning() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};
        use std::collections::HashMap;

        // Create a font with kerning pairs
        let mut widths = vec![500.0; 95]; // ASCII 32-126
        widths[65 - 32] = 722.0; // 'A'
        widths[86 - 32] = 722.0; // 'V'
        widths[87 - 32] = 944.0; // 'W'

        let mut kerning = HashMap::new();
        // Typical kerning pairs (in FUnits, 1/1000)
        kerning.insert((65, 86), -50.0); // 'A' + 'V' → tighten by 50 FUnits
        kerning.insert((65, 87), -40.0); // 'A' + 'W' → tighten by 40 FUnits

        let font_info = FontInfo {
            name: "Helvetica".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_to_gid_map: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(widths),
                missing_width: Some(500.0),
                kerning: Some(kerning),
            },
        };

        // Test "AV" with kerning
        let width_av = calculate_text_width("AV", 12.0, Some(&font_info));
        // Expected: (722 + 722)/1000 * 12 + (-50/1000 * 12)
        //         = 17.328 - 0.6 = 16.728
        let expected_av = (722.0 + 722.0) / 1000.0 * 12.0 + (-50.0 / 1000.0 * 12.0);
        let tolerance = 0.0001;
        assert!(
            (width_av - expected_av).abs() < tolerance,
            "AV with kerning: expected {}, got {}, diff {}",
            expected_av,
            width_av,
            (width_av - expected_av).abs()
        );

        // Test "AW" with different kerning value
        let width_aw = calculate_text_width("AW", 12.0, Some(&font_info));
        // Expected: (722 + 944)/1000 * 12 + (-40/1000 * 12)
        //         = 19.992 - 0.48 = 19.512
        let expected_aw = (722.0 + 944.0) / 1000.0 * 12.0 + (-40.0 / 1000.0 * 12.0);
        assert!(
            (width_aw - expected_aw).abs() < tolerance,
            "AW with kerning: expected {}, got {}, diff {}",
            expected_aw,
            width_aw,
            (width_aw - expected_aw).abs()
        );

        // Test "VA" with NO kerning (pair not in HashMap)
        let width_va = calculate_text_width("VA", 12.0, Some(&font_info));
        // Expected: (722 + 722)/1000 * 12 = 17.328 (no kerning adjustment)
        let expected_va = (722.0 + 722.0) / 1000.0 * 12.0;
        assert!(
            (width_va - expected_va).abs() < tolerance,
            "VA without kerning: expected {}, got {}, diff {}",
            expected_va,
            width_va,
            (width_va - expected_va).abs()
        );

        // Verify kerning makes a measurable difference
        assert!(
            width_av < width_va,
            "AV with kerning ({}) should be narrower than VA without kerning ({})",
            width_av,
            width_va
        );
    }

    #[test]
    fn test_parse_truetype_kern_table_minimal() {
        use crate::text::extraction_cmap::parse_truetype_kern_table;

        // Complete TrueType font with kern table (Format 0, 2 kerning pairs)
        // Structure:
        // 1. Offset table (12 bytes)
        // 2. Table directory (2 tables: 'head' and 'kern', each 16 bytes = 32 total)
        // 3. 'head' table data (54 bytes)
        // 4. 'kern' table data (30 bytes)
        // Total: 128 bytes
        let mut ttf_data = vec![
            // Offset table
            0x00, 0x01, 0x00, 0x00, // scaler type: TrueType
            0x00, 0x02, // numTables: 2
            0x00, 0x20, // searchRange: 32
            0x00, 0x01, // entrySelector: 1
            0x00, 0x00, // rangeShift: 0
        ];

        // Table directory entry 1: 'head' table
        ttf_data.extend_from_slice(b"head"); // tag
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x2C]); // offset: 44 (12 + 32)
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x36]); // length: 54

        // Table directory entry 2: 'kern' table
        ttf_data.extend_from_slice(b"kern"); // tag
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // checksum
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x62]); // offset: 98 (44 + 54)
        ttf_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x1E]); // length: 30 (actual kern table size)

        // 'head' table data (54 bytes of zeros - minimal valid head table)
        ttf_data.extend_from_slice(&[0u8; 54]);

        // 'kern' table data (34 bytes)
        ttf_data.extend_from_slice(&[
            // Kern table header
            0x00, 0x00, // version: 0
            0x00, 0x01, // nTables: 1
            // Subtable header
            0x00, 0x00, // version: 0
            0x00, 0x1A, // length: 26 bytes (header 6 + nPairs data 8 + pairs 2*6=12)
            0x00, 0x00, // coverage: 0x0000 (Format 0 in lower byte, horizontal)
            0x00, 0x02, // nPairs: 2
            0x00, 0x08, // searchRange: 8
            0x00, 0x00, // entrySelector: 0
            0x00, 0x04, // rangeShift: 4
            // Kerning pair 1: A + V → -50
            0x00, 0x41, // left glyph: 65 ('A')
            0x00, 0x56, // right glyph: 86 ('V')
            0xFF, 0xCE, // value: -50 (signed 16-bit big-endian)
            // Kerning pair 2: A + W → -40
            0x00, 0x41, // left glyph: 65 ('A')
            0x00, 0x57, // right glyph: 87 ('W')
            0xFF, 0xD8, // value: -40 (signed 16-bit big-endian)
        ]);

        let result = parse_truetype_kern_table(&ttf_data);
        assert!(
            result.is_ok(),
            "Should parse minimal kern table successfully: {:?}",
            result.err()
        );

        let kerning_map = result.unwrap();
        assert_eq!(kerning_map.len(), 2, "Should extract 2 kerning pairs");

        // Verify pair 1: A + V → -50
        assert_eq!(
            kerning_map.get(&(65, 86)),
            Some(&-50.0),
            "Should have A+V kerning pair with value -50"
        );

        // Verify pair 2: A + W → -40
        assert_eq!(
            kerning_map.get(&(65, 87)),
            Some(&-40.0),
            "Should have A+W kerning pair with value -40"
        );
    }

    #[test]
    fn test_parse_kern_table_no_kern_table() {
        use crate::text::extraction_cmap::extract_truetype_kerning;

        // TrueType font data WITHOUT a 'kern' table
        // Structure:
        // - Offset table: scaler type + numTables + searchRange + entrySelector + rangeShift
        // - Table directory: 1 entry for 'head' table (not 'kern')
        let ttf_data = vec![
            // Offset table
            0x00, 0x01, 0x00, 0x00, // scaler type: TrueType
            0x00, 0x01, // numTables: 1
            0x00, 0x10, // searchRange: 16
            0x00, 0x00, // entrySelector: 0
            0x00, 0x00, // rangeShift: 0
            // Table directory entry: 'head' table (not 'kern')
            b'h', b'e', b'a', b'd', // tag: 'head'
            0x00, 0x00, 0x00, 0x00, // checksum
            0x00, 0x00, 0x00, 0x1C, // offset: 28
            0x00, 0x00, 0x00, 0x36, // length: 54
            // Mock 'head' table data (54 bytes of zeros)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let result = extract_truetype_kerning(&ttf_data);
        assert!(
            result.is_ok(),
            "Should gracefully handle missing kern table"
        );

        let kerning_map = result.unwrap();
        assert!(
            kerning_map.is_empty(),
            "Should return empty HashMap when no kern table exists"
        );
    }

    // Helper for paragraph-reconstruction unit tests. TextFragment has 11
    // fields so a helper keeps the test bodies focused on geometry.
    fn tf(text: &str, x: f64, y: f64, width: f64, font_size: f64) -> TextFragment {
        TextFragment {
            text: text.to_string(),
            x,
            y,
            width,
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
    fn merge_into_lines_groups_same_baseline_fragments() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        let input = vec![
            tf("Hello", 50.0, 400.0, 30.0, 12.0),
            tf("world", 90.0, 400.0, 30.0, 12.0),
            tf("now.", 130.0, 400.0, 25.0, 12.0),
            tf("Next", 50.0, 386.0, 30.0, 12.0),
            tf("line.", 90.0, 386.0, 25.0, 12.0),
        ];
        let lines = extractor.merge_into_lines(&input);
        assert_eq!(
            lines.len(),
            2,
            "two distinct baselines must produce two line fragments"
        );
        assert_eq!(
            lines[0].text, "Hello world now.",
            "first line concatenated with spaces"
        );
        assert_eq!(lines[1].text, "Next line.", "second line concatenated");
    }

    #[test]
    fn merge_into_lines_inserts_space_only_when_gap_exceeds_threshold() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            space_threshold: 0.3,
            ..Default::default()
        });
        // Gap of 4pt at font_size 12 = 0.33x — above threshold 0.3
        let with_gap = vec![
            tf("AB", 50.0, 400.0, 10.0, 12.0),
            tf("CD", 64.0, 400.0, 10.0, 12.0),
        ];
        let lines = extractor.merge_into_lines(&with_gap);
        assert_eq!(
            lines[0].text, "AB CD",
            "gap above threshold must insert space"
        );

        // Gap of 1pt = 0.083x — below threshold
        let tight = vec![
            tf("AB", 50.0, 400.0, 10.0, 12.0),
            tf("CD", 61.0, 400.0, 10.0, 12.0),
        ];
        let lines = extractor.merge_into_lines(&tight);
        assert_eq!(lines[0].text, "ABCD", "tight gap must NOT insert space");
    }

    #[test]
    fn merge_into_lines_unioned_bounding_box() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        let input = vec![
            tf("A", 50.0, 400.0, 10.0, 12.0),
            tf("B", 100.0, 400.0, 10.0, 12.0),
        ];
        let lines = extractor.merge_into_lines(&input);
        assert_eq!(lines.len(), 1);
        assert!((lines[0].x - 50.0).abs() < 0.01);
        assert!(
            (lines[0].width - 60.0).abs() < 0.01,
            "width must span 50->110"
        );
    }

    #[test]
    fn assign_row_ids_monotone_y_descending_keeps_zero() {
        let frags = vec![
            tf("A", 50.0, 400.0, 10.0, 9.0),
            tf("B", 50.0, 395.0, 10.0, 9.0),
            tf("C", 50.0, 390.0, 10.0, 9.0),
        ];
        let row_ids = super::assign_row_ids(&frags);
        assert_eq!(row_ids, vec![0u32, 0, 0]);
    }

    #[test]
    fn assign_row_ids_increments_on_y_up_jump_above_threshold() {
        // font_size=9 → threshold = max(4.5, 2.0) = 4.5
        // deltas: 395-400=-5, 420-395=+25 (>4.5)
        let frags = vec![
            tf("A", 50.0, 400.0, 10.0, 9.0),
            tf("B", 50.0, 395.0, 10.0, 9.0),
            tf("C", 50.0, 420.0, 10.0, 9.0),
        ];
        let row_ids = super::assign_row_ids(&frags);
        assert_eq!(row_ids, vec![0u32, 0, 1]);
    }

    #[test]
    fn assign_row_ids_ignores_superscript_within_threshold() {
        // font_size=9 → threshold 4.5. delta 2.5 must NOT trigger.
        let frags = vec![
            tf("A", 50.0, 400.0, 10.0, 9.0),
            tf("^2", 60.0, 402.5, 5.0, 9.0),
            tf("B", 65.0, 395.0, 10.0, 9.0),
        ];
        let row_ids = super::assign_row_ids(&frags);
        assert_eq!(row_ids, vec![0u32, 0, 0]);
    }

    #[test]
    fn assign_row_ids_floor_2pt_for_small_fonts() {
        // font_size=3 → font_size*0.5 = 1.5; floor lifts threshold to 2.0
        // delta = +2.5 > 2.0 must trigger.
        let frags = vec![
            tf("A", 50.0, 100.0, 10.0, 3.0),
            tf("B", 50.0, 102.5, 10.0, 3.0),
        ];
        let row_ids = super::assign_row_ids(&frags);
        assert_eq!(row_ids, vec![0u32, 1]);
    }

    #[test]
    fn assign_row_ids_empty_slice_returns_empty() {
        let frags: Vec<TextFragment> = vec![];
        let row_ids = super::assign_row_ids(&frags);
        assert!(row_ids.is_empty(), "empty input must yield empty output");
    }

    #[test]
    fn merge_into_lines_splits_two_columns_emitted_sequentially() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        // Emission order: col1.l1, col1.l2 (Y monotone down), then col2.l1
        // (Y jumps UP by 10 > threshold 5 for font 10pt), col2.l2.
        let input = vec![
            tf("col1-top", 50.0, 400.0, 80.0, 10.0),
            tf("col1-bot", 50.0, 395.0, 80.0, 10.0),
            tf("col2-top", 200.0, 405.0, 80.0, 10.0),
            tf("col2-bot", 200.0, 400.0, 80.0, 10.0),
        ];
        let lines = extractor.merge_into_lines(&input);
        assert_eq!(
            lines.len(),
            4,
            "two columns at near-identical Y must split into 4 lines"
        );
        // row_id=0 batch first (col1), then row_id=1 (col2). Within each batch, Y desc.
        assert_eq!(lines[0].text, "col1-top");
        assert_eq!(lines[0].y, 400.0);
        assert_eq!(lines[1].text, "col1-bot");
        assert_eq!(lines[1].y, 395.0);
        assert_eq!(lines[2].text, "col2-top");
        assert_eq!(lines[2].y, 405.0);
        assert_eq!(lines[3].text, "col2-bot");
        assert_eq!(lines[3].y, 400.0);
    }

    #[test]
    fn merge_into_lines_preserves_single_column_continuation() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        // Single column: same Y continuation (X grows), then next line down.
        let input = vec![
            tf("Hello", 50.0, 400.0, 30.0, 10.0),
            tf("world", 90.0, 400.0, 30.0, 10.0),
            tf("next-line", 50.0, 395.0, 70.0, 10.0),
        ];
        let lines = extractor.merge_into_lines(&input);
        assert_eq!(
            lines.len(),
            2,
            "single column continuation must collapse to 2 lines"
        );
        assert!(lines[0].text.contains("Hello"));
        assert!(lines[0].text.contains("world"));
        assert_eq!(lines[1].text, "next-line");
    }

    #[test]
    fn merge_into_lines_splits_columns_with_uniform_mcid() {
        // Regression guard for #265 root cause: NCSC page 12 has a single
        // outer BDC, so every fragment has mcid=Some(0). Column separation
        // must come from row_id alone, not from mcid.
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        let mut frags = vec![
            tf("col1-top", 50.0, 400.0, 80.0, 10.0),
            tf("col1-bot", 50.0, 395.0, 80.0, 10.0),
            tf("col2-top", 200.0, 405.0, 80.0, 10.0),
            tf("col2-bot", 200.0, 400.0, 80.0, 10.0),
        ];
        for f in &mut frags {
            f.mcid = Some(0);
        }
        let lines = extractor.merge_into_lines(&frags);
        assert_eq!(
            lines.len(),
            4,
            "uniform mcid must not prevent row_id-based column split (NCSC root cause)"
        );
        assert_eq!(lines[0].text, "col1-top");
        assert_eq!(lines[1].text, "col1-bot");
        assert_eq!(lines[2].text, "col2-top");
        assert_eq!(lines[3].text, "col2-bot");
    }

    #[test]
    fn merge_into_paragraphs_groups_consecutive_lines() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        // Three lines, 14pt leading (line height 12pt, gap 2pt)
        let lines = vec![
            tf("Line one.", 50.0, 400.0, 60.0, 12.0),
            tf("Line two.", 50.0, 386.0, 60.0, 12.0),
            tf("Line three.", 50.0, 372.0, 70.0, 12.0),
        ];
        let paragraphs = extractor.merge_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(paragraphs[0].text, "Line one.\nLine two.\nLine three.");
    }

    #[test]
    fn merge_into_paragraphs_splits_on_large_vertical_gap() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        let lines = vec![
            tf("P1L1.", 50.0, 400.0, 40.0, 12.0),
            tf("P1L2.", 50.0, 386.0, 40.0, 12.0),
            tf("P2L1.", 50.0, 300.0, 40.0, 12.0),
        ];
        let paragraphs = extractor.merge_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 2);
        assert_eq!(paragraphs[0].text, "P1L1.\nP1L2.");
        assert_eq!(paragraphs[1].text, "P2L1.");
    }

    #[test]
    fn merge_into_paragraphs_drops_hyphen_when_merge_hyphenated() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            merge_hyphenated: true,
            ..Default::default()
        });
        let lines = vec![
            tf("Kryp-", 50.0, 400.0, 30.0, 12.0),
            tf("tographie", 50.0, 386.0, 60.0, 12.0),
        ];
        let paragraphs = extractor.merge_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 1);
        assert_eq!(
            paragraphs[0].text, "Kryptographie",
            "hyphen elided, no newline inserted"
        );
    }

    #[test]
    fn decode_pdf_string_utf16be_bom_decodes_fi_ligature() {
        let bytes = [0xFE, 0xFF, 0x00, 0x66, 0x00, 0x69];
        assert_eq!(super::decode_pdf_string(&bytes), "fi");
    }

    #[test]
    fn decode_pdf_string_ascii_pdfdocencoding_passthrough() {
        let bytes = b"page 12";
        assert_eq!(super::decode_pdf_string(bytes), "page 12");
    }

    #[test]
    fn decode_pdf_string_empty_input_returns_empty() {
        assert_eq!(super::decode_pdf_string(&[]), "");
    }

    #[test]
    fn decode_pdf_string_lone_bom_returns_empty() {
        // BOM only, no code units after.
        assert_eq!(super::decode_pdf_string(&[0xFE, 0xFF]), "");
    }

    #[test]
    fn resolve_props_extracts_integer_mcid() {
        use crate::parser::content::{MarkedContentProps, MarkedContentValue};
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert("MCID".to_string(), MarkedContentValue::Integer(7));
        let props = MarkedContentProps::Inline(map);

        let (mcid, actual) = super::resolve_props(&props, None);
        assert_eq!(mcid, Some(7));
        assert_eq!(actual, None);
    }

    #[test]
    fn resolve_props_decodes_utf16be_actualtext() {
        use crate::parser::content::{MarkedContentProps, MarkedContentValue};
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(
            "ActualText".to_string(),
            MarkedContentValue::String(vec![0xFE, 0xFF, 0x00, 0x66, 0x00, 0x69]),
        );
        let props = MarkedContentProps::Inline(map);

        let (mcid, actual) = super::resolve_props(&props, None);
        assert_eq!(mcid, None);
        assert_eq!(actual.as_deref(), Some("fi"));
    }

    #[test]
    fn resolve_props_returns_none_for_unresolvable_resource_ref() {
        use crate::parser::content::MarkedContentProps;
        let props = MarkedContentProps::ResourceRef("PropsName".to_string());
        let (mcid, actual) = super::resolve_props(&props, None);
        assert_eq!((mcid, actual), (None, None));
    }

    #[test]
    fn resolve_props_negative_mcid_rejected() {
        use crate::parser::content::{MarkedContentProps, MarkedContentValue};
        use std::collections::HashMap;
        // MCID is unsigned per ISO 32000-1; negative integer is malformed.
        let mut map = HashMap::new();
        map.insert("MCID".to_string(), MarkedContentValue::Integer(-1));
        let props = MarkedContentProps::Inline(map);

        let (mcid, _) = super::resolve_props(&props, None);
        assert_eq!(mcid, None);
    }

    #[test]
    fn resolve_props_resource_ref_overflow_mcid_rejected() {
        // ISO 32000-1 §14.7.4: MCID is an unsigned 32-bit integer. A
        // PdfObject::Integer holds an i64, so a malformed PDF can carry an
        // out-of-range MCID. The ResourceRef path must reject those rather
        // than wrap silently via `as u32`. Mirrors the Inline-path guard
        // already covered by `resolve_props_negative_mcid_rejected`.
        use crate::parser::content::MarkedContentProps;
        use crate::parser::objects::{PdfDictionary, PdfObject};

        let mut inner = PdfDictionary::new();
        inner.insert("MCID".to_string(), PdfObject::Integer(i64::MAX));

        let mut properties = PdfDictionary::new();
        properties.insert("PropsName".to_string(), PdfObject::Dictionary(inner));

        let props = MarkedContentProps::ResourceRef("PropsName".to_string());
        let (mcid, _) = super::resolve_props(&props, Some(&properties));
        assert_eq!(mcid, None);
    }
}
