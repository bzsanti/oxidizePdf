//! Text extraction from PDF content streams
//!
//! This module provides functionality to extract text from PDF pages,
//! handling text positioning, transformations, and basic encodings.

use crate::graphics::Color;
use crate::parser::content::{ContentOperation, ContentParser, TextElement};
use crate::parser::document::PdfDocument;
use crate::parser::objects::{PdfDictionary, PdfObject};
use crate::parser::page_tree::ParsedPage;
use crate::parser::ParseResult;
use crate::text::extraction_cmap::{CMapTextExtractor, FontInfo};
use crate::text::flat_reading_order;
use crate::text::graphics_state_stack::GraphicsStateStack;
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Controls how carriage returns decoded from PDF text-showing strings are
/// represented in extracted plain text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarriageReturnHandling {
    /// Remove each standalone carriage return.
    Remove,
    /// Replace each standalone carriage return with a collapsible `U+0020` space.
    ReplaceWithSpace,
    /// Preserve standalone carriage returns and normalize CRLF to one line feed.
    NormalizeLineEnding,
}

impl Default for CarriageReturnHandling {
    fn default() -> Self {
        Self::Remove
    }
}

/// Text extraction options
#[derive(Debug, Clone)]
pub struct ExtractionOptions {
    /// Preserve the original layout (spacing and positioning)
    pub preserve_layout: bool,
    /// Minimum space width to insert space character (in text space units)
    pub space_threshold: f64,
    /// Threshold for synthesising an implicit `U+0020` from a `TJ` numeric
    /// kerning offset, expressed as a fraction of the current font size.
    /// A TJ kern advances the text matrix by `-adjustment/1000 * font_size`
    /// without rendering any glyph; many PDFs (academic publishers, LaTeX,
    /// kerned typography) encode inter-word gaps purely as wide negative
    /// kerns rather than literal space bytes. When the synthesised advance
    /// exceeds `tj_space_threshold * font_size`, the extractor inserts one
    /// `U+0020`. Default `0.2` (200 milli-em) sits well between typical
    /// intra-word kerning (10-50 milli-em) and the width of a `space`
    /// glyph in most fonts (250-300 milli-em). Lower values catch tighter
    /// spaces; higher values reduce false positives in fonts with unusually
    /// wide kerning. Separate from `space_threshold` (which governs the
    /// post-glyph gap between separate text-show operators) because the TJ
    /// numeric kern is measured without any glyph advance baseline and
    /// needs a more sensitive threshold (issue #272).
    pub tj_space_threshold: f64,
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
    /// Reorder flat-text output by column so per-column tokens stay adjacent in
    /// multi-column layouts (issue #389). Only affects the flat path
    /// (`preserve_layout = false`); in layout mode `detect_columns` already
    /// reorders. Default `false` → the flat path is byte-identical to before.
    /// When on, `.text` is produced by the fragment pipeline (its shape matches
    /// the layout path's reconstruction, not stream order); `.fragments` stays
    /// empty.
    ///
    /// Column reflow only triggers for column blocks whose rows are spaced at
    /// least one line height apart. Layouts pitched tighter than that are
    /// geometrically indistinguishable from tight-leading prose that merely
    /// contains a wide gap, so they are intentionally left in reading order
    /// rather than risk shredding prose (issue #417); text is never corrupted.
    ///
    /// Column blocks require gaps that align horizontally across rows: a set of
    /// unrelated wide gaps at different X (e.g. a label/value form with varying
    /// label lengths) is left in reading order, never reflowed (#422). A genuine
    /// table whose column corridor drifts more than ~10pt between rows may also
    /// be left un-reordered; text is never corrupted.
    pub reorder_columns: bool,
    /// Stop accumulating decoded text for a page once this many bytes have been
    /// collected, bounding the per-page peak memory of extraction. The limit is
    /// enforced *during* accumulation, not by truncating the finished string, so
    /// a single page with a huge or adversarially inflated content stream cannot
    /// materialise an unbounded `String` before the caller sees it (issue #382).
    ///
    /// Semantics are *undershoot*: extraction stops before the fragment that
    /// would push the accumulated bytes past the limit, so the returned
    /// `text.len() <= max_extracted_bytes` and a multi-byte UTF-8 character is
    /// never split. When the limit cuts extraction short,
    /// [`ExtractedText::truncated`] is set to `true`.
    ///
    /// `None` (default) means no limit — output is byte-identical to before.
    /// The `text.len() <= max_extracted_bytes` invariant holds on **every** path
    /// (flat, `reorder_columns`, `preserve_layout`): the layout paths rebuild
    /// `.text` from the already-bounded fragment set and are then clamped to the
    /// limit at a UTF-8 char boundary as a final safety net.
    ///
    /// Because whole decoded runs are the unit of truncation, a page whose text
    /// is a single run larger than the whole budget (e.g. one huge `Tj`, or an
    /// `/ActualText` override) comes back with `text == ""` and
    /// `truncated == true` rather than a partial run — the limit is never
    /// satisfied by splitting a run mid-character.
    pub max_extracted_bytes: Option<usize>,
}

impl Default for ExtractionOptions {
    fn default() -> Self {
        Self {
            preserve_layout: false,
            space_threshold: 0.3,
            tj_space_threshold: 0.2,
            newline_threshold: 10.0,
            sort_by_position: true,
            detect_columns: false,
            column_threshold: 50.0,
            merge_hyphenated: true,
            track_space_decisions: false,
            reconstruct_paragraphs: false,
            include_artifacts: false,
            reorder_columns: false,
            max_extracted_bytes: None,
        }
    }
}

/// Extracted text with position information.
///
/// Pipeline output: returned by the `extract_text*` entry points on
/// [`Page`](crate::page::Page) / [`PdfDocument`](crate::parser::PdfDocument).
/// `#[non_exhaustive]` so future fields (e.g. per-run diagnostics) can be added
/// without a breaking change — construct one outside the crate via
/// [`ExtractedText::new`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ExtractedText {
    /// The extracted text content
    pub text: String,
    /// Text fragments with position information (if preserve_layout is true)
    pub fragments: Vec<TextFragment>,
    /// `true` when extraction stopped early because
    /// [`ExtractionOptions::max_extracted_bytes`] was reached, so `text` is a
    /// bounded prefix of the page's full text rather than the whole page
    /// (issue #382). Always `false` when no limit is set.
    pub truncated: bool,
}

impl ExtractedText {
    /// Build an `ExtractedText` from its text and fragments, with `truncated`
    /// set to `false`. Provided because `ExtractedText` is `#[non_exhaustive]`,
    /// so external callers cannot use a struct literal. Set [`truncated`](Self::truncated)
    /// afterwards if you are synthesizing a bounded result.
    pub fn new(text: String, fragments: Vec<TextFragment>) -> Self {
        Self {
            text,
            fragments,
            truncated: false,
        }
    }
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
    ///
    /// Bounded: see [`GraphicsStateStack`] for the depth cap and for why the
    /// pushes it refuses have to be counted (issue #455).
    saved_states: GraphicsStateStack<SavedGraphicsState>,
    /// Marked-content stack (issue #269 Phase 1). Pushed on BMC/BDC,
    /// popped on EMC. Empty on entry to each page.
    mc_stack: Vec<MarkedContentEntry>,
    /// Pending ActualText run if any BDC ancestor declared `/ActualText`.
    /// At most one active run at a time — nested ActualText replaces the
    /// outer (innermost wins, per spec §4).
    pending_actualtext: Option<PendingActualText>,
}

impl TextState {
    /// `q` (§8.4.4): snapshot the graphics state.
    ///
    /// The snapshot is built lazily so that past the depth cap it is not built
    /// at all: a `q` flood must not pay for the font-name clone of an entry the
    /// stack is about to refuse (issue #455).
    ///
    /// That laziness is what forces the stack out of the state and back: the
    /// closure calls [`SavedGraphicsState::capture`], which borrows the WHOLE
    /// `TextState` — the ten fields of the snapshot are defined in one place on
    /// purpose, so the `q` path and the implicit save around `Do` cannot drift
    /// apart — and that borrow overlaps the mutable borrow of `saved_states`.
    /// Moving a four-word stack twice per `q` is the price of not duplicating
    /// the snapshot definition. The plain extractor reads its three fields
    /// inline instead, so its borrows are disjoint and it needs none of this.
    fn save_graphics_state(&mut self) {
        let mut stack = std::mem::take(&mut self.saved_states);
        stack.push_with(|| SavedGraphicsState::capture(self));
        self.saved_states = stack;
    }
}

/// Graphics state saved by `q` and restored by `Q` (issues #262, #452).
///
/// Holds the CTM, the fill colour, and the TEXT STATE parameters. The text
/// state is graphics state per ISO 32000-1 §9.3 and Table 52 — leading,
/// character and word spacing, horizontal scaling, font and size, text rise
/// and render mode all live there, so `Q` must put them back. Before #452 only
/// the CTM and the colour were restored, and a leading set inside a `q … Q`
/// block kept driving line breaks after the block closed.
///
/// `text_matrix` and `text_line_matrix` are deliberately NOT here: they are
/// text OBJECT state, established by `BT` and discarded by `ET` (§9.4.1), not
/// graphics state. Restoring them on `Q` would be a different bug.
///
/// Four of the text-state fields — `char_space`, `word_space`, `text_rise` and
/// `render_mode` — are currently written by their operators but never read by
/// the extractor, so restoring them changes no output today and no test can
/// guard them. They are here because they are graphics state: whoever wires
/// them into the pen advance, the y offset or invisible-text filtering should
/// not have to rediscover this bug.
struct SavedGraphicsState {
    ctm: [f64; 6],
    fill_color: Option<Color>,
    leading: f64,
    char_space: f64,
    word_space: f64,
    horizontal_scale: f64,
    text_rise: f64,
    font_size: f64,
    font_name: Option<String>,
    render_mode: u8,
}

impl SavedGraphicsState {
    /// Snapshot the graphics state, for `q` and for the implicit save around
    /// `Do` (§8.10.1). Both callers go through here so the two can never drift
    /// into disagreeing about what the graphics state contains.
    fn capture(state: &TextState) -> Self {
        Self {
            ctm: state.ctm,
            fill_color: state.fill_color,
            leading: state.leading,
            char_space: state.char_space,
            word_space: state.word_space,
            horizontal_scale: state.horizontal_scale,
            text_rise: state.text_rise,
            font_size: state.font_size,
            font_name: state.font_name.clone(),
            render_mode: state.render_mode,
        }
    }

    /// Put the snapshot back. Consumes it, so the `String` moves instead of
    /// being cloned.
    ///
    /// Note the fields it does NOT touch: the text matrices (text object state,
    /// §9.4.1), the marked-content stack (its nesting is independent of
    /// `q`/`Q`, §14.6) and the saved-state stack itself.
    fn restore_into(self, state: &mut TextState) {
        state.ctm = self.ctm;
        state.fill_color = self.fill_color;
        state.leading = self.leading;
        state.char_space = self.char_space;
        state.word_space = self.word_space;
        state.horizontal_scale = self.horizontal_scale;
        state.text_rise = self.text_rise;
        state.font_size = self.font_size;
        state.font_name = self.font_name;
        state.render_mode = self.render_mode;
    }
}

/// Mutable accumulator threaded through `process_operations` so the op loop
/// can be driven recursively (page content stream → Form XObjects) while
/// carrying text state, position, and accumulated output. Bundled into one
/// struct so the op match moves verbatim into the recursive method (#319).
struct OpRunState {
    state: TextState,
    in_text_object: bool,
    last_x: f64,
    last_y: f64,
    extracted_text: String,
    fragments: Vec<TextFragment>,
    /// Set once the per-page byte budget (`max_extracted_bytes`) has cut text
    /// accumulation short. Propagates through Form XObject recursion and into
    /// [`ExtractedText::truncated`] (issue #382).
    truncated: bool,
    /// Closed line groups for the reading-order option (issue #448). Empty and
    /// untouched unless `ExtractionOptions::reading_order` is on. Each group
    /// records the byte range of its text in `extracted_text` plus its page-space
    /// box, so the finalizer can permute groups without rebuilding their text —
    /// the identity permutation is byte-identical (design §5.2).
    line_groups: Vec<LineGroupGeom>,
    /// The group currently being accumulated (opens on the first glyph after a
    /// newline separator). Flushed into `line_groups` at page end.
    cur_group: Option<LineGroupGeom>,
}

/// One flat-path line group for the reading-order option (issue #448): the byte
/// range of its text within `extracted_text`, and the page-space box the
/// ordering primitive sees. Byte offsets (not owned text) keep the identity
/// permutation provably byte-identical — the finalizer joins the recorded
/// slices with `'\n'`, which is exactly the separator the flat path put between
/// groups in the first place.
#[derive(Debug, Clone, Copy)]
struct LineGroupGeom {
    start: usize,
    end: usize,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    font_size: f64,
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
            saved_states: GraphicsStateStack::default(),
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

/// Relative font-size difference below which two lines still count as the same
/// typographic style. Absorbs the sub-point jitter a scaled text matrix
/// produces (11.96 vs 12.0) without absorbing a real size step: the smallest
/// step in common use is 12 → 13pt (8%).
const PARAGRAPH_STYLE_SIZE_TOLERANCE: f64 = 0.05;

/// Whether two consecutive lines share the typographic style that makes them
/// one paragraph.
///
/// A paragraph is a run of lines set in the same face; a change of size or
/// weight marks a new block. Vertical gap alone cannot tell a heading from its
/// body — a title set 40pt above 10pt body text falls inside the same 1.5×
/// median-line-height window as ordinary line spacing (issue #436).
///
/// The cost of the two errors is asymmetric, which is why this splits on a
/// signal as weak as a weight change. An over-split leaves two adjacent
/// fragments that downstream chunking can still group. An under-split is
/// irreversible: the merged fragment inherits the heading's size and weight,
/// so `partition` classifies the whole block as a `Title` and its text becomes
/// the `heading_path` breadcrumb of everything that follows.
fn same_paragraph_style(a: &TextFragment, b: &TextFragment) -> bool {
    if a.is_bold != b.is_bold {
        return false;
    }
    let scale = a.font_size.abs().max(b.font_size.abs());
    if scale <= 0.0 {
        return true; // no usable size on either line: gap decides
    }
    (a.font_size - b.font_size).abs() / scale <= PARAGRAPH_STYLE_SIZE_TOLERANCE
}

/// Whether `next` is plausibly the wrapped continuation of `prev` on a new
/// line, using the exact same Y-gap test `reconstruct_text_from_fragments`
/// already uses to decide "new line vs. same line" (`|Δy| > newline_threshold`).
///
/// Deliberately mirrors that threshold rather than inventing a stricter one:
/// this function's only job is to protect an already-correct merge decision
/// from being corrupted by an unrelated fragment sorting in between the two
/// halves (issue #482) — not to second-guess which pairs `merge_hyphenated`
/// would otherwise join. Used by `merge_hyphenated_line_wraps_in_emission_order`.
fn is_line_wrap_geometry(prev: &TextFragment, next: &TextFragment, newline_threshold: f64) -> bool {
    (prev.y - next.y).abs() > newline_threshold
}

/// Text extractor for PDF pages with CMap support
pub struct TextExtractor {
    options: ExtractionOptions,
    /// Reorder the flat `.text` line groups into reading order (issue #448).
    /// Off by default; set via [`TextExtractor::with_reading_order`]. Held here,
    /// not on the public [`ExtractionOptions`], so enabling it is a
    /// non-breaking method addition rather than a breaking struct-field addition.
    reading_order: bool,
    /// Policy for CR bytes decoded from text-showing strings. Held outside the
    /// public `ExtractionOptions` so adding it does not break exhaustive struct
    /// literals in downstream crates.
    carriage_return_handling: CarriageReturnHandling,
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
            reading_order: false,
            carriage_return_handling: CarriageReturnHandling::default(),
            font_cache: HashMap::new(),
            font_object_cache: HashMap::new(),
        }
    }

    /// Create a text extractor with custom options
    pub fn with_options(options: ExtractionOptions) -> Self {
        Self {
            options,
            reading_order: false,
            carriage_return_handling: CarriageReturnHandling::default(),
            font_cache: HashMap::new(),
            font_object_cache: HashMap::new(),
        }
    }

    /// Enable (or disable) flat-path reading-order reordering (issue #448).
    ///
    /// Off by default. When on, the flat `.text` path permutes its line groups
    /// into reading order (left column before right, top block before bottom)
    /// using the scale-relative XY-cut primitive; the text inside each group is
    /// untouched, and the result is byte-identical whenever the stream order is
    /// already the reading order. Only affects the flat path
    /// (`ExtractionOptions::preserve_layout = false`, no `reorder_columns`).
    ///
    /// Consuming builder, so it chains after the constructors:
    /// `TextExtractor::with_options(opts).with_reading_order(true)`.
    ///
    /// Known ceiling (issue #448 design §5.1): only reorders groups the newline
    /// heuristic already separated — two columns drawn row-interleaved fall into
    /// one group. `/Rotate ≠ 0` pages are ordered in unrotated page space.
    pub fn with_reading_order(mut self, enable: bool) -> Self {
        self.reading_order = enable;
        self
    }

    /// Select how standalone carriage returns decoded from PDF text strings
    /// are represented. CRLF is always normalized to one line feed.
    ///
    /// The default is [`CarriageReturnHandling::Remove`].
    pub fn with_carriage_return_handling(mut self, handling: CarriageReturnHandling) -> Self {
        self.carriage_return_handling = handling;
        self
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
    /// Fragments are grouped by `(row_id, Y_bucket, mcid)`, where `row_id`
    /// comes from `assign_row_ids` (increments on Y-up-jumps in emission
    /// order). Within a line the tie-break is emission index for tagged PDFs
    /// (any fragment carries an mcid — ISO 32000 mandates logical order) and
    /// X coordinate for non-tagged PDFs. A space is inserted between adjacent
    /// fragments when the X gap exceeds `space_threshold * font_size`.
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

        // Whether this page has at least one tagged (mcid-carrying) fragment.
        // `.any()` returns true if even one fragment has mcid=Some; the within-line
        // tie-break then uses emission index for the whole page rather than X.
        // See `docs/superpowers/specs/2026-05-23-issue-265-line-interleaving-design.md`.
        //
        // For tagged PDFs (PDF/UA, ISO 32000-2 tagged), the content stream delivers
        // text in logical reading order, so within a visual line we preserve emission
        // order rather than sorting by X. Out-of-left-to-right glyph placement
        // (common in typeset tagged PDFs where the PDF author lays out glyphs via
        // non-monotone Td/Tm operators) is correctly rendered by keeping emission order.
        //
        // For non-tagged PDFs (all mcid=None), we retain the X-sort fallback
        // because many generators emit glyphs in arbitrary (often right-to-left
        // or random) order and only the X coordinate gives reading order.
        let is_tagged = fragments.iter().any(|f| f.mcid.is_some());

        // Sort for line GROUPING only: row_id, then Y descending, then X.
        // row_id keeps fragments from different visual rows in separate
        // Y-bucket groups; Y descending puts higher-on-page lines first. The
        // X tie-break only makes same-line fragments adjacent for grouping —
        // the authoritative reading order WITHIN each line is decided per line
        // below (#302 symptom 1), so this grouping order is not the final order.
        let mut indexed: Vec<(u32, usize, &TextFragment)> = row_ids
            .iter()
            .copied()
            .zip(fragments.iter().enumerate())
            .map(|(rid, (idx, f))| (rid, idx, f))
            .collect();
        indexed.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(b.2.y.total_cmp(&a.2.y))
                .then(a.2.x.total_cmp(&b.2.x))
        });

        // Group into visual lines, carrying each fragment's emission index so
        // the per-line ordering decision below can restore emission order.
        let mut lines: Vec<Vec<(usize, &TextFragment)>> = Vec::new();
        let mut last_seen_row_id: Option<u32> = None;
        for (rid, idx, frag) in indexed {
            let same_batch = last_seen_row_id == Some(rid);
            let placed = same_batch
                && lines.last_mut().is_some_and(|line| {
                    let head = line[0].1;
                    let tol = (head.height.min(frag.height)) * 0.2;
                    (head.y - frag.y).abs() < tol && head.mcid == frag.mcid
                });
            if placed {
                lines.last_mut().unwrap().push((idx, frag));
            } else {
                lines.push(vec![(idx, frag)]);
                last_seen_row_id = Some(rid);
            }
        }

        // Decide reading order per visual line (#302 symptom 1).
        //
        // X-sort is wrong when one line mixes fonts whose glyph metrics differ
        // (e.g. an italic particle symbol set in roman body text): the producer
        // gives the font-switched run an x-origin that falls INSIDE the x-span
        // of its neighbours, so sorting by x interleaves it
        // ("to the Z boson" -> "tZboso theon"). The content stream still emits
        // these runs in correct reading order, so when a line's emission order
        // has no DISJOINT backward x-step (only span overlaps, or is already
        // x-monotone) we keep emission order. A disjoint backward step signals
        // a genuinely scrambled stream (right-to-left / random generators), for
        // which x-order stays authoritative. Deciding per line — not per
        // column — prevents one scrambled line from forcing x-sort on the rest.
        lines
            .into_iter()
            .map(|mut line| {
                if is_tagged || line_prefers_emission_order(&line) {
                    line.sort_by_key(|&(idx, _)| idx);
                } else {
                    line.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
                }
                let frags: Vec<&TextFragment> = line.into_iter().map(|(_, f)| f).collect();
                self.build_line_fragment(frags)
            })
            .collect()
    }

    /// Space-glyph advance for `font_name` in text space (point units at
    /// `font_size`), or `None` when unknown. Prefers the font's embedded
    /// `/Widths` entry for code 32; falls back to the Adobe Core-14 AFM space
    /// width for the standard base fonts (Times/Helvetica/Courier/Symbol/
    /// ZapfDingbats), which ship no `/Widths` array (#302 symptom 2).
    fn font_space_advance(&self, font_name: Option<&str>, font_size: f64) -> Option<f64> {
        let info = self.font_cache.get(font_name?)?;
        if let Some(ref widths) = info.metrics.widths {
            let first = info.metrics.first_char.unwrap_or(0);
            if first <= 32 {
                if let Some(&w) = widths.get((32 - first) as usize) {
                    if w > 0.0 {
                        return Some(w / 1000.0 * font_size);
                    }
                }
            }
        }
        standard_14_space_width(&info.name).map(|em| em / 1000.0 * font_size)
    }

    /// Minimum inter-fragment x-gap that counts as a word space for `frag`.
    /// Anchored to the font's real space-glyph advance when known — word gaps
    /// scale with the font's space metric, not with a fixed fraction of font
    /// size — falling back to `space_threshold * font_size` otherwise. Tightly
    /// set justified text (e.g. Standard-14 Times body) has word gaps near
    /// 0.2em, far below the legacy 0.3*font_size, which dropped spaces
    /// ("thequadrupletis"); a font with a 250-unit space then gets a 0.125em
    /// threshold instead (#302 symptom 2).
    fn space_gap_threshold(&self, frag: &TextFragment) -> f64 {
        match self.font_space_advance(frag.font_name.as_deref(), frag.font_size) {
            Some(adv) if adv > 0.0 => 0.5 * adv,
            _ => self.options.space_threshold * frag.font_size,
        }
    }

    /// Assemble one visual line's fragments into a single line `TextFragment`,
    /// inserting a space between consecutive fragments whose x-gap exceeds the
    /// font-anchored [`space_gap_threshold`](Self::space_gap_threshold).
    fn build_line_fragment(&self, line: Vec<&TextFragment>) -> TextFragment {
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
                if gap > self.space_gap_threshold(frag) {
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

    /// Group consecutive lines into paragraphs based on vertical gap and
    /// typographic style.
    ///
    /// Two consecutive lines are part of the same paragraph when the vertical
    /// gap between them is less than 1.5× the median line height in the input
    /// **and** they share the same style — see [`same_paragraph_style`].
    /// Hyphenated line breaks (previous line ends with `-` and
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

            if gap < 0.0
                || gap > max_paragraph_gap
                || current.mcid != line.mcid
                || !same_paragraph_style(&current, line)
            {
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

        let extracted_text = String::new();
        let fragments = Vec::new();
        let state = TextState::default();
        let in_text_object = false;
        let last_x = 0.0;
        let last_y = 0.0;

        // Page resources (owned) for XObject + /Properties lookup during
        // recursive Form XObject extraction (issue #319).
        let page_resources: Option<crate::parser::objects::PdfDictionary> =
            if let Some(rr) = page.dict.get("Resources").and_then(|o| o.as_reference()) {
                document
                    .get_object(rr.0, rr.1)
                    .ok()
                    .and_then(|o| o.as_dict().cloned())
            } else {
                page.get_resources().cloned()
            };

        let mut run = OpRunState {
            state,
            in_text_object,
            last_x,
            last_y,
            extracted_text,
            fragments,
            truncated: false,
            line_groups: Vec::new(),
            cur_group: None,
        };

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

            run = self.process_operations(
                operations,
                document,
                page_resources.as_ref(),
                run,
                page_index,
                0,
            )?;

            // Per-page byte budget reached (issue #382): don't decode the
            // remaining content streams — the text is already at the limit.
            if run.truncated {
                break;
            }
        }

        let OpRunState {
            mut extracted_text,
            mut fragments,
            mut truncated,
            mut line_groups,
            cur_group,
            ..
        } = run;
        {
            let _span = tracing::info_span!("layout_finalize").entered();

            // Fuse hyphen-wrapped tokens while fragments are still in emission
            // order (issue #482), *before* any Y-sort below can interleave an
            // unrelated fragment between a wrapped line's two halves. Fragments
            // only exist here for the `preserve_layout`/`reorder_columns` paths
            // (see the `emit_text_fragment` call sites), both of which feed
            // `reconstruct_text_from_fragments` further down, so this always
            // runs ahead of the merge it's protecting.
            //
            // `merge_close_fragments` must run first: a word's trailing hyphen
            // is frequently its own separate glyph-run fragment (e.g. a style
            // or kerning boundary right at "3016" | "-"), so the hyphen check
            // below would otherwise fire against that lone "-" fragment (whose
            // own predecessor is the stranded "6") instead of against the real
            // "...3016-" line. Coalescing same-line adjacent runs first makes
            // the trailing-hyphen text end up on one fragment, as the check
            // assumes. `merge_close_fragments` is a local, order-preserving
            // pass over adjacent pairs (see its own doc comment on being used
            // this way for `reconstruct_paragraphs`), so it is safe to run here
            // on unsorted emission order.
            if !fragments.is_empty() {
                fragments = self.merge_close_fragments(&fragments);
                fragments = self.merge_hyphenated_line_wraps_in_emission_order(fragments);
            }

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

            // Flat path with column reordering (issue #389): fragments were
            // collected only to reorder. `sort_and_merge_fragments` already ran
            // at the top of this block (sort_by_position defaults true) and now
            // applies column clustering via the gate above; call it here too so
            // the behaviour is independent of `sort_by_position`, then rebuild
            // the flat text from the reordered fragments and drop them (the
            // `.fragments` contract only exposes fragments under preserve_layout).
            if self.options.reorder_columns
                && !self.options.preserve_layout
                && !fragments.is_empty()
            {
                self.sort_and_merge_fragments(&mut fragments);
                extracted_text = self.reconstruct_text_from_fragments(&fragments);
                fragments.clear();
            }

            // Flat-path reading order (issue #448): permute the line groups into
            // reading order with the scale-relative XY-cut primitive. Only the
            // pure flat path — `preserve_layout` and `reorder_columns` rebuild
            // `.text` from fragments and own their ordering. Rejoining the
            // recorded group slices with `'\n'` (the flat path's own inter-group
            // separator) makes an identity permutation byte-identical (§5.2).
            if self.reading_order && !self.options.preserve_layout && !self.options.reorder_columns
            {
                if let Some(g) = cur_group {
                    line_groups.push(g);
                }
                if line_groups.len() > 1 {
                    let boxes: Vec<flat_reading_order::OrderBox> = line_groups
                        .iter()
                        .map(|g| flat_reading_order::OrderBox {
                            min_x: g.min_x,
                            max_x: g.max_x,
                            min_y: g.min_y,
                            max_y: g.max_y,
                            font_size: g.font_size,
                        })
                        .collect();
                    let order = flat_reading_order::reading_order(&boxes, &READING_ORDER_CFG);
                    let mut rebuilt = String::with_capacity(extracted_text.len());
                    for (n, &i) in order.iter().enumerate() {
                        if n > 0 {
                            rebuilt.push('\n');
                        }
                        rebuilt.push_str(&extracted_text[line_groups[i].start..line_groups[i].end]);
                    }
                    extracted_text = rebuilt;
                }
            }

            // Final safety net (issue #382): the layout/reorder reconstruction
            // above rebuilds `.text` with its own separators, so guarantee the
            // `text.len() <= max_extracted_bytes` invariant for every path here.
            // No-op for the flat path (already bounded) and when no limit is set.
            clamp_to_budget(
                &mut extracted_text,
                self.options.max_extracted_bytes,
                &mut truncated,
            );
        }

        Ok(ExtractedText {
            text: extracted_text,
            fragments,
            truncated,
        })
    }

    /// Run a content-stream operation list, recursing into Form XObjects so
    /// text drawn inside a `Do`-painted Form XObject is extracted (issue #319).
    #[allow(clippy::too_many_arguments)]
    fn process_operations<R: Read + Seek>(
        &mut self,
        operations: Vec<ContentOperation>,
        document: &PdfDocument<R>,
        resources: Option<&crate::parser::objects::PdfDictionary>,
        run: OpRunState,
        page_index: u32,
        depth: u8,
    ) -> ParseResult<OpRunState> {
        let OpRunState {
            mut state,
            mut in_text_object,
            mut last_x,
            mut last_y,
            mut extracted_text,
            mut fragments,
            mut truncated,
            mut line_groups,
            mut cur_group,
        } = run;

        let page_properties: Option<&crate::parser::objects::PdfDictionary> =
            resources.and_then(|res| match res.get("Properties") {
                Some(crate::parser::objects::PdfObject::Dictionary(d)) => Some(d),
                _ => None,
            });

        let _ops_span = tracing::info_span!("text_ops_loop").entered();
        for op in operations {
            // Per-page byte budget reached (issue #382): stop processing further
            // operators. Show-text arms also `break` mid-run, but a state-only
            // op between two show ops would otherwise keep the loop alive.
            if truncated {
                break;
            }
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

                // `tx ty TD` (ISO 32000-1 §9.4.2) is defined as `-ty TL`
                // followed by `tx ty Td`: it moves to the next line AND sets
                // the leading. The operator was parsed but never handled, so
                // the line break did not exist for the extractor (`dx = dy =
                // 0` at the boundary) and every later `T*` inherited a stale
                // leading (issue #451).
                ContentOperation::MoveTextSetLeading(tx, ty) => {
                    state.leading = -(ty as f64);
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

                        // Mirror the gate inside `emit_text_fragment` so that
                        // `.text` and `.fragments` stay consistent for pages
                        // wrapped in an `/Artifact` marked-content scope —
                        // issue #330.
                        let skip_text = skip_artifact_text(&state, self.options.include_artifacts);

                        // Add spacing based on position change
                        // Separator of the run that was actually appended, for the
                        // reading-order line grouping (issue #448); `None` when the
                        // run was skipped.
                        let mut emitted_sep: Option<Option<char>> = None;
                        if !skip_text {
                            let separator = if !extracted_text.is_empty() {
                                // Baseline-frame deltas (issue #443): identical
                                // to raw Δx/Δy for axis-aligned matrices,
                                // rotation-normalized otherwise.
                                let (dx, dy_signed) = pen_delta(&state, (last_x, last_y), (x, y));
                                let dy = dy_signed.abs();

                                // A large backward jump in x is a line wrap: the
                                // pen returns to the left margin on a new line.
                                // When the line height is below `newline_threshold`
                                // the dy check alone misses it, so treat a backward
                                // dx beyond one line-height (2× the threshold,
                                // conservative) as a newline even when dy is small
                                // (issue #390). With a nonzero leading that gate is
                                // enough; but at dy == 0 the jump is ambiguous with
                                // a same-line reposition (issue #441). Resolve it by
                                // magnitude: a reposition is local, a same-Y wrap
                                // returns across the whole column, so a jump beyond
                                // `SAME_Y_WRAP_EM` font sizes is a wrap even at dy == 0
                                // (issue #447). dx/dy are baseline-relative (issue
                                // #443), so this holds under rotation; the epsilon
                                // absorbs projection rounding noise.
                                let same_y_wrap = dx < -(state.font_size.abs() * SAME_Y_WRAP_EM);
                                let line_wrap = dx < -(self.options.newline_threshold * 2.0)
                                    && (dy > SAME_LINE_EPS || same_y_wrap);
                                if dy > self.options.newline_threshold || line_wrap {
                                    Some('\n')
                                } else if dx > self.options.space_threshold * state.font_size {
                                    Some(' ')
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            // Per-page byte budget (issue #382): stop before the
                            // run that would overshoot; the outer loop guard ends
                            // extraction on the next iteration. Hyphen-wrap fusion
                            // (issue #486) may replace the requested `\n` with no
                            // separator at all — `emitted_sep` must reflect what
                            // was actually applied, not what was requested, so
                            // reading-order line grouping below sees the run as a
                            // continuation rather than a new line.
                            let outcome = append_bounded(
                                &mut extracted_text,
                                separator,
                                &decoded,
                                self.options.max_extracted_bytes,
                                &mut truncated,
                                self.options.merge_hyphenated,
                            );
                            if !outcome.appended {
                                break;
                            }
                            emitted_sep = Some(outcome.applied_separator);
                        }

                        // Get font info for accurate width calculation.
                        // Width comes from the char codes (`text_bytes`), not
                        // the decoded Unicode: the Widths array is code-indexed
                        // (issue #302).
                        let text_width = {
                            let font_info = state
                                .font_name
                                .as_ref()
                                .and_then(|name| self.font_cache.get(name));
                            calculate_text_width_from_codes(
                                text_bytes,
                                &decoded,
                                state.font_size,
                                font_info,
                                state.char_space,
                                state.word_space,
                            )
                        };

                        if self.options.preserve_layout || self.options.reorder_columns {
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

                        // Record the run into the reading-order line groups
                        // (issue #448) once its width is known.
                        if self.reading_order {
                            if let Some(sep) = emitted_sep {
                                record_line_group(
                                    &mut line_groups,
                                    &mut cur_group,
                                    extracted_text.len(),
                                    decoded.len(),
                                    sep,
                                    x,
                                    y,
                                    text_width,
                                    &state,
                                );
                            }
                        }

                        // Advance the text matrix and track the true post-advance
                        // pen point (folds in Tz and CTM scale, issue #386; a
                        // full point so rotated baselines advance y too, #443).
                        (last_x, last_y) = advance_pen(&mut state, text_width);
                    }
                }

                ContentOperation::ShowTextArray(array) => {
                    if in_text_object {
                        // True until this `TJ` array draws its first glyph. Only
                        // on that first text element can a forward pen jump come
                        // from the operator boundary (a `Tm`, or the previous
                        // operator's advance); once a glyph is drawn, a later
                        // forward jump is the array's own kerning, which
                        // `TextElement::Spacing` already turns into a space. A
                        // leading kern does NOT clear this (see the Spacing arm).
                        // See the boundary gate below.
                        let mut at_array_start = true;
                        for item in array {
                            match item {
                                TextElement::Text(text_bytes) => {
                                    let decoded = self.decode_text(&text_bytes, &state)?;
                                    // Mirror the gate inside `emit_text_fragment`
                                    // so `.text` and `.fragments` stay consistent
                                    // for Artifact scopes (issue #330).
                                    let skip_text =
                                        skip_artifact_text(&state, self.options.include_artifacts);

                                    // Pen origin in user space = (CTM × text_matrix)(0, 0).
                                    let (x, y) = text_origin(&state);

                                    // Insert a newline when this TJ piece starts on a
                                    // different visual line than the previously shown
                                    // text (issue #381), or when the pen jumps far back
                                    // to the left — a line wrap whose line height is
                                    // below `newline_threshold` (issue #390). Only the
                                    // newline case is handled here: horizontal word
                                    // spacing within a line is governed by the
                                    // `TextElement::Spacing` kern logic below, and a
                                    // forward dx-based space would wrongly split a single
                                    // word that a TJ array draws as several positioned
                                    // pieces. A *backward* dx beyond one line-height
                                    // (2× the threshold, conservative) is a wrap, not a
                                    // kern, so it is safe to break there — but only when
                                    // the pen also moved vertically: with a nonzero
                                    // leading that gate identifies the wrap. At dy == 0
                                    // the backward jump is ambiguous with a same-line
                                    // reposition (issue #441); resolve it by magnitude,
                                    // treating a jump beyond `SAME_Y_WRAP_EM` font sizes
                                    // as a same-Y wrap (issue #447). Deltas are
                                    // baseline-relative (issue #443), so both gates hold
                                    // under rotation; the epsilon absorbs projection
                                    // rounding noise.
                                    let (dx, dy_signed) =
                                        pen_delta(&state, (last_x, last_y), (x, y));
                                    let dy = dy_signed.abs();
                                    let same_y_wrap =
                                        dx < -(state.font_size.abs() * SAME_Y_WRAP_EM);
                                    let line_wrap = dx < -(self.options.newline_threshold * 2.0)
                                        && (dy > SAME_LINE_EPS || same_y_wrap);
                                    // Separator of the run actually appended, for the
                                    // reading-order line grouping (issue #448).
                                    let mut emitted_sep: Option<Option<char>> = None;
                                    if !skip_text {
                                        // Word spacing at the operator boundary.
                                        // The `Tj` arm turns a forward jump wider
                                        // than `space_threshold` into a space;
                                        // this arm used to decide newlines only,
                                        // so two `TJ` operators drawn side by side
                                        // on the same line came out glued: a
                                        // multi-column table cell read as
                                        // `CellOneCellTwo` (issue #458), and a list
                                        // bullet 0.75 em from its item text read as
                                        // `vlarge` (found on preserve_027613.pdf,
                                        // an IBM manual whose every bullet is a
                                        // separate `TJ`).
                                        //
                                        // Restricted to the array's first element
                                        // because that is the only jump the
                                        // boundary owns. Inside the array the jump
                                        // IS the kern, and `TextElement::Spacing`
                                        // below already synthesises its space —
                                        // firing here too would double it. A short
                                        // forward gap (below the threshold) is left
                                        // alone: the pen advance is only as
                                        // accurate as the font widths, so a
                                        // producer that draws one word as several
                                        // positioned runs must not be split.
                                        let boundary_space = at_array_start
                                            && dx > TJ_BOUNDARY_SPACE_EM * state.font_size
                                            && !extracted_text.ends_with(' ');
                                        let separator = if extracted_text.is_empty() {
                                            None
                                        } else if dy > self.options.newline_threshold || line_wrap {
                                            Some('\n')
                                        } else if boundary_space {
                                            Some(' ')
                                        } else {
                                            None
                                        };

                                        // Per-page byte budget (issue #382).
                                        // Hyphen-wrap fusion (issue #486) may
                                        // replace the requested `\n` with no
                                        // separator; `emitted_sep` reflects what
                                        // was actually applied (see the `Tj` arm
                                        // above for the full rationale).
                                        let outcome = append_bounded(
                                            &mut extracted_text,
                                            separator,
                                            &decoded,
                                            self.options.max_extracted_bytes,
                                            &mut truncated,
                                            self.options.merge_hyphenated,
                                        );
                                        if !outcome.appended {
                                            break;
                                        }
                                        emitted_sep = Some(outcome.applied_separator);
                                    }

                                    let text_width = {
                                        let font_info = state
                                            .font_name
                                            .as_ref()
                                            .and_then(|name| self.font_cache.get(name));
                                        calculate_text_width_from_codes(
                                            &text_bytes,
                                            &decoded,
                                            state.font_size,
                                            font_info,
                                            state.char_space,
                                            state.word_space,
                                        )
                                    };

                                    if self.options.preserve_layout || self.options.reorder_columns
                                    {
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

                                    // Record the run into the reading-order line
                                    // groups (issue #448) once its width is known.
                                    if self.reading_order {
                                        if let Some(sep) = emitted_sep {
                                            record_line_group(
                                                &mut line_groups,
                                                &mut cur_group,
                                                extracted_text.len(),
                                                decoded.len(),
                                                sep,
                                                x,
                                                y,
                                                text_width,
                                                &state,
                                            );
                                        }
                                    }

                                    // Keep the pen position in sync so a following
                                    // `Tj`/`TJ` measures its gap from the right origin
                                    // (issue #381: a stale `last_y` dropped newlines;
                                    // issue #386: the pen must fold in Tz/CTM scale).
                                    (last_x, last_y) = advance_pen(&mut state, text_width);
                                    at_array_start = false;
                                }
                                TextElement::Spacing(adjustment) => {
                                    // `at_array_start` is deliberately NOT cleared
                                    // here. It marks "no glyph drawn yet", not "no
                                    // item seen yet": a `TJ` array may open with a
                                    // small intra-word kern while the real
                                    // operator-boundary jump (a `Tm` to a new
                                    // column) still lands on the first *text*
                                    // element. Clearing the flag on the leading
                                    // kern would suppress the boundary space and
                                    // re-glue separate columns (issue #458). The
                                    // kern's own space synthesis below works off
                                    // `tx` (the kern delta), not `dx` (the full pen
                                    // jump), so the two checks measure different
                                    // quantities; the `!ends_with(' ')` guard on
                                    // each keeps a leading kern that IS wide from
                                    // producing a double space.
                                    // Text position adjustment (negative = move left,
                                    // i.e. shifts the pen forward). When the synthesised
                                    // forward advance exceeds `tj_space_threshold * font_size`
                                    // we treat the kern as an implicit `U+0020` (issue #272):
                                    // many PDFs encode word breaks purely as wide negative
                                    // kerns and never emit a literal space byte.
                                    let tx = -(adjustment as f64) / 1000.0 * state.font_size;

                                    let skip_tj_space =
                                        skip_artifact_text(&state, self.options.include_artifacts);
                                    if !skip_tj_space
                                        && tx > self.options.tj_space_threshold * state.font_size
                                        && !extracted_text.is_empty()
                                        && !extracted_text.ends_with(' ')
                                    {
                                        // Per-page byte budget (issue #382): even
                                        // the synthesised space counts, so the
                                        // `text.len() <= limit` invariant holds.
                                        // Always a space, never `\n` — hyphen-wrap
                                        // fusion (issue #486) does not apply here.
                                        if !append_bounded(
                                            &mut extracted_text,
                                            Some(' '),
                                            "",
                                            self.options.max_extracted_bytes,
                                            &mut truncated,
                                            self.options.merge_hyphenated,
                                        )
                                        .appended
                                        {
                                            break;
                                        }
                                        // The synthesised space is intra-group
                                        // (issue #448): keep it inside the current
                                        // group's byte range, no new group.
                                        if self.reading_order {
                                            if let Some(g) = cur_group.as_mut() {
                                                g.end = extracted_text.len();
                                            }
                                        }

                                        // Skip the fragment-level emission while an
                                        // ActualText scope is pending: the synthesised
                                        // space is a heuristic, not real content, and
                                        // emitting it would call `emit_text_fragment`
                                        // whose ActualText short-circuit would inflate
                                        // `pending.width` and set `pending.populated`
                                        // even though no real `Tj` has fired yet. The
                                        // EMC flush will supply the canonical fragment
                                        // text from the override (Phase 1 #269 contract).
                                        if (self.options.preserve_layout
                                            || self.options.reorder_columns)
                                            && state.pending_actualtext.is_none()
                                        {
                                            // Emit a synthetic single-space fragment at the
                                            // current pen origin so downstream layout merges
                                            // (e.g. `merge_close_fragments`) see the gap as
                                            // explicit content rather than as a sub-threshold
                                            // x-jump. Width = the kern advance so the next
                                            // text fragment begins flush against it.
                                            let (sx, sy) = text_origin(&state);
                                            emit_text_fragment(
                                                &mut fragments,
                                                " ",
                                                tx,
                                                sx,
                                                sy,
                                                &mut state,
                                                self.options.include_artifacts,
                                            );
                                        }
                                    }

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

                        // Mirror the artifact gate (issue #330).
                        let skip_text = skip_artifact_text(&state, self.options.include_artifacts);
                        let mut emitted_sep: Option<Option<char>> = None;
                        if !skip_text {
                            let separator = if extracted_text.is_empty() {
                                None
                            } else {
                                Some('\n')
                            };
                            // Per-page byte budget (issue #382). Hyphen-wrap
                            // fusion (issue #486) may replace the requested `\n`
                            // with no separator; `emitted_sep` reflects what was
                            // actually applied (see the `Tj` arm for the full
                            // rationale). `'` (this operator) always requests a
                            // new line by definition (ISO 32000-1 §9.4.3's
                            // `T* Tj`), so this is where a hyphen at the end of
                            // one `'`-delimited line meets the start of the next.
                            let outcome = append_bounded(
                                &mut extracted_text,
                                separator,
                                &decoded,
                                self.options.max_extracted_bytes,
                                &mut truncated,
                                self.options.merge_hyphenated,
                            );
                            if !outcome.appended {
                                break;
                            }
                            emitted_sep = Some(outcome.applied_separator);
                        }

                        let text_width = {
                            let font_info = state
                                .font_name
                                .as_ref()
                                .and_then(|name| self.font_cache.get(name));
                            calculate_text_width_from_codes(
                                &text,
                                &decoded,
                                state.font_size,
                                font_info,
                                state.char_space,
                                state.word_space,
                            )
                        };

                        if self.options.preserve_layout || self.options.reorder_columns {
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

                        // Record into the reading-order line groups (issue #448).
                        if self.reading_order {
                            if let Some(sep) = emitted_sep {
                                record_line_group(
                                    &mut line_groups,
                                    &mut cur_group,
                                    extracted_text.len(),
                                    decoded.len(),
                                    sep,
                                    x,
                                    y,
                                    text_width,
                                    &state,
                                );
                            }
                        }

                        (last_x, last_y) = advance_pen(&mut state, text_width);
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

                        // Mirror the artifact gate (issue #330).
                        let skip_text = skip_artifact_text(&state, self.options.include_artifacts);
                        let mut emitted_sep: Option<Option<char>> = None;
                        if !skip_text {
                            let separator = if extracted_text.is_empty() {
                                None
                            } else {
                                Some('\n')
                            };
                            // Per-page byte budget (issue #382). Hyphen-wrap
                            // fusion (issue #486) may replace the requested `\n`
                            // with no separator; `emitted_sep` reflects what was
                            // actually applied (see the `Tj` arm for the full
                            // rationale). `"` (this operator) always requests a
                            // new line by definition, same as `'` above.
                            let outcome = append_bounded(
                                &mut extracted_text,
                                separator,
                                &decoded,
                                self.options.max_extracted_bytes,
                                &mut truncated,
                                self.options.merge_hyphenated,
                            );
                            if !outcome.appended {
                                break;
                            }
                            emitted_sep = Some(outcome.applied_separator);
                        }

                        let text_width = {
                            let font_info = state
                                .font_name
                                .as_ref()
                                .and_then(|name| self.font_cache.get(name));
                            calculate_text_width_from_codes(
                                &text,
                                &decoded,
                                state.font_size,
                                font_info,
                                state.char_space,
                                state.word_space,
                            )
                        };

                        if self.options.preserve_layout || self.options.reorder_columns {
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

                        // Record into the reading-order line groups (issue #448).
                        if self.reading_order {
                            if let Some(sep) = emitted_sep {
                                record_line_group(
                                    &mut line_groups,
                                    &mut cur_group,
                                    extracted_text.len(),
                                    decoded.len(),
                                    sep,
                                    x,
                                    y,
                                    text_width,
                                    &state,
                                );
                            }
                        }

                        (last_x, last_y) = advance_pen(&mut state, text_width);
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
                    state.save_graphics_state();
                }
                ContentOperation::RestoreGraphicsState => {
                    // Text state is graphics state (§9.3, Table 52): a leading,
                    // font or scale set inside the block dies with it (issue
                    // #452). Unbalanced Q (pop on empty stack) is silently
                    // ignored to keep extraction robust to malformed PDFs.
                    if let Some(saved) = state.saved_states.pop() {
                        saved.restore_into(&mut state);
                    }
                }

                // Color operations (Phase 4: Color extraction)
                ContentOperation::SetNonStrokingGray(gray) => {
                    state.fill_color = Some(Color::gray(gray as f64));
                }

                ContentOperation::SetNonStrokingRGB(r, g, b) => {
                    state.fill_color = Some(Color::rgb(r as f64, g as f64, b as f64));
                }

                ContentOperation::SetNonStrokingCMYK(c, m, y, k) => {
                    state.fill_color = Some(Color::cmyk(c as f64, m as f64, y as f64, k as f64));
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
                            if run.populated
                                && (self.options.preserve_layout || self.options.reorder_columns)
                            {
                                let (mcid, struct_tag) = innermost_mc_tag(&state.mc_stack);
                                let in_artifact = state.mc_stack.iter().any(|e| e.is_artifact);
                                if !in_artifact || self.options.include_artifacts {
                                    // Per-page byte budget (issue #382): the
                                    // `/ActualText` override is this scope's
                                    // canonical text and can be arbitrarily
                                    // large. It bypasses the per-`Tj`
                                    // `append_bounded` gate, so account it here
                                    // against the same ledger (`extracted_text`,
                                    // which these paths rebuild from `fragments`).
                                    // If it would overshoot, drop the fragment and
                                    // stop — a huge override must not escape the
                                    // cap while reporting `truncated = false`.
                                    // Always `None` separator, never `\n` —
                                    // hyphen-wrap fusion (issue #486) does not
                                    // apply here. This site is also only reached
                                    // under `preserve_layout`/`reorder_columns`
                                    // (see the gate above), not the flat path.
                                    if !append_bounded(
                                        &mut extracted_text,
                                        None,
                                        &run.text,
                                        self.options.max_extracted_bytes,
                                        &mut truncated,
                                        self.options.merge_hyphenated,
                                    )
                                    .appended
                                    {
                                        break;
                                    }
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

                ContentOperation::PaintXObject(name) => {
                    // Issue #319: recurse into Form XObjects. `Do` paints a
                    // Form XObject in an implicit q/Q, with the XObject's
                    // /Matrix composed onto the CTM and its own /Resources
                    // fonts in scope. Without this, text drawn inside the
                    // XObject (the page body, for RML2PDF "inclPDF" output)
                    // is never extracted.
                    const MAX_XOBJECT_DEPTH: u8 = 12;
                    if depth < MAX_XOBJECT_DEPTH {
                        if let Some((xobj_ops, xobj_res, matrix)) =
                            self.load_form_xobject(resources, &name, document)
                        {
                            // `Do` paints inside an IMPLICIT q/Q (§8.10.1),
                            // so the whole graphics state — text state included
                            // (issue #452) — comes back afterwards. Same
                            // snapshot the `q` arm takes, so the two cannot
                            // disagree about what that state is.
                            let outer = SavedGraphicsState::capture(&state);
                            let saved_fonts = self.font_cache.clone();
                            // The form gets its own save-state stack: a stray
                            // `Q` inside it must not pop the page's snapshots.
                            // Truncating afterwards could not undo that — a
                            // popped entry is gone — and with the text state
                            // now in each snapshot, a mispaired restore
                            // corrupts font decoding, not just the CTM.
                            //
                            // The count of pushes the depth cap refused is part
                            // of the stack, so it changes hands here too: a
                            // form that inherited the page's count would let its
                            // own `Q` consume it, and the page would come back
                            // short (issue #455).
                            let outer_stack = std::mem::take(&mut state.saved_states);

                            if let Some(m) = matrix {
                                let [a0, b0, c0, d0, e0, f0] = state.ctm;
                                let [a, b, c, d, e, f] = m;
                                state.ctm = [
                                    a * a0 + b * c0,
                                    a * b0 + b * d0,
                                    c * a0 + d * c0,
                                    c * b0 + d * d0,
                                    e * a0 + f * c0 + e0,
                                    e * b0 + f * d0 + f0,
                                ];
                            }
                            if let Some(ref xr) = xobj_res {
                                self.cache_fonts_from_resources::<R>(xr, document);
                            }

                            let sub = OpRunState {
                                state,
                                in_text_object: false,
                                last_x,
                                last_y,
                                extracted_text,
                                fragments,
                                truncated,
                                line_groups,
                                cur_group,
                            };
                            let mut out = self.process_operations(
                                xobj_ops,
                                document,
                                xobj_res.as_ref(),
                                sub,
                                page_index,
                                depth + 1,
                            )?;

                            outer.restore_into(&mut out.state);
                            out.state.saved_states = outer_stack;
                            self.font_cache = saved_fonts;

                            state = out.state;
                            last_x = out.last_x;
                            last_y = out.last_y;
                            extracted_text = out.extracted_text;
                            fragments = out.fragments;
                            truncated = out.truncated;
                            line_groups = out.line_groups;
                            cur_group = out.cur_group;
                        }
                    }
                }
                _ => {
                    // Other operations don't affect text extraction
                }
            }
        }

        Ok(OpRunState {
            state,
            in_text_object,
            last_x,
            last_y,
            extracted_text,
            fragments,
            truncated,
            line_groups,
            cur_group,
        })
    }

    /// Load a Form XObject by name: parsed operations, resolved /Resources,
    /// and optional /Matrix. None for image XObjects or anything unparseable.
    fn load_form_xobject<R: Read + Seek>(
        &self,
        resources: Option<&crate::parser::objects::PdfDictionary>,
        name: &str,
        document: &PdfDocument<R>,
    ) -> Option<(
        Vec<ContentOperation>,
        Option<crate::parser::objects::PdfDictionary>,
        Option<[f64; 6]>,
    )> {
        use crate::parser::objects::PdfObject;
        let res = resources?;
        let xobjects = match res.get("XObject")? {
            PdfObject::Dictionary(d) => d.clone(),
            PdfObject::Reference(n, g) => match document.get_object(*n, *g).ok()? {
                PdfObject::Dictionary(d) => d,
                _ => return None,
            },
            _ => return None,
        };
        let (n, g) = xobjects.get(name)?.as_reference()?;
        let obj = document.get_object(n, g).ok()?;
        let stream = obj.as_stream()?;
        if stream
            .dict
            .get("Subtype")
            .and_then(|o| o.as_name())
            .map(|nm| nm.0.as_str())
            != Some("Form")
        {
            return None;
        }
        let data = stream.decode(&Default::default()).ok()?;
        let ops = ContentParser::parse_content(&data).ok()?;
        let xobj_res = match stream.dict.get("Resources") {
            Some(PdfObject::Dictionary(d)) => Some(d.clone()),
            Some(PdfObject::Reference(rn, rg)) => document
                .get_object(*rn, *rg)
                .ok()
                .and_then(|o| o.as_dict().cloned()),
            _ => None,
        };
        let matrix = stream
            .dict
            .get("Matrix")
            .and_then(|o| o.as_array())
            .and_then(|a| {
                if a.0.len() == 6 {
                    let mut m = [0.0f64; 6];
                    for (i, slot) in m.iter_mut().enumerate() {
                        *slot = a.0[i]
                            .as_real()
                            .or_else(|| a.0[i].as_integer().map(|x| x as f64))?;
                    }
                    Some(m)
                } else {
                    None
                }
            });
        Some((ops, xobj_res, matrix))
    }

    /// Fuse a hyphen-ended fragment with its line-wrap continuation while
    /// fragments are still in emission (content-stream) order, before any
    /// Y-coordinate sort runs.
    ///
    /// `sort_and_merge_fragments`'s global Y-sort has no concept of separate
    /// content regions (issue #482): an unrelated fragment (an annotation's
    /// appearance stream, a watermark, …) whose Y-coordinate happens to fall
    /// between two wrapped lines of unrelated body text gets sorted in
    /// between them, and the hyphen-merge check in `reconstruct_text_from_fragments`
    /// — which only ever looks at the *immediately preceding* fragment in
    /// the already-sorted list — then joins the hyphen to the wrong
    /// fragment, corrupting both regions at once.
    ///
    /// Emission order does not have this problem: two fragments that are
    /// genuinely adjacent lines of the same wrapped text are (barring a
    /// pathological content stream) emitted consecutively, regardless of
    /// where an unrelated annotation's text happens to sit on the Y axis.
    /// Fusing the pair here, before the sort, makes the wrapped token a
    /// single atomic fragment that nothing can be spliced into afterward.
    ///
    /// Uses `is_line_wrap_geometry` (the same Y-gap test
    /// `reconstruct_text_from_fragments` already applies) to confirm the
    /// pair actually looks like consecutive lines before merging, so an
    /// unrelated same-line hyphen (e.g. "well-known" on one line) is not
    /// fused with whatever fragment happens to follow it in emission order.
    fn merge_hyphenated_line_wraps_in_emission_order(
        &self,
        fragments: Vec<TextFragment>,
    ) -> Vec<TextFragment> {
        if !self.options.merge_hyphenated || fragments.len() < 2 {
            return fragments;
        }

        let mut result: Vec<TextFragment> = Vec::with_capacity(fragments.len());
        for fragment in fragments {
            let should_merge = result
                .last()
                .map(|prev: &TextFragment| {
                    prev.text.ends_with('-')
                        && is_line_wrap_geometry(prev, &fragment, self.options.newline_threshold)
                })
                .unwrap_or(false);

            if should_merge {
                // Safe: just checked `result.last()` is `Some` above.
                let prev = result.last_mut().expect("checked non-empty above");
                prev.text.pop(); // drop the trailing hyphen
                prev.text.push_str(&fragment.text);
                // Extend the fused fragment's box to cover both lines so
                // downstream geometry (space/newline decisions keyed on
                // `x + width`, `y`) still reasons about real coverage
                // rather than only the first line's box.
                let x_min = prev.x.min(fragment.x);
                let x_max = (prev.x + prev.width).max(fragment.x + fragment.width);
                let y_min = prev.y.min(fragment.y);
                let y_max = (prev.y + prev.height).max(fragment.y + fragment.height);
                prev.x = x_min;
                prev.width = x_max - x_min;
                prev.y = y_min;
                prev.height = y_max - y_min;
            } else {
                result.push(fragment);
            }
        }
        result
    }

    /// Sort text fragments by position and merge them appropriately
    fn sort_and_merge_fragments(&self, fragments: &mut [TextFragment]) {
        // Establish reading order (top-to-bottom, left-to-right) without ever
        // collapsing two distinct visual lines into one.
        //
        // A single `sort_by` with a threshold-based "same line" comparator is not
        // transitive (A≈B, B≈C ⇏ A≈C), which Rust's sort requires. The previous
        // implementation restored transitivity by quantizing Y into fixed bands of
        // `newline_threshold` width — but fixed bands collide two lines that
        // straddle a band boundary while sitting closer than the band width. With
        // 8pt leading under the 10pt default, y=684 → band −68 and y=676 → band
        // −68 land in the same band; the secondary X sort then interleaved the two
        // lines glyph-by-glyph, shredding any token that straddled the corruption
        // (issue #408).
        //
        // Instead, sort in two transitive phases over an index permutation. First
        // by exact Y (top-to-bottom — a real total order). Then group consecutive
        // fragments into visual lines with a jitter tolerance anchored to the
        // line's head, matching `merge_into_lines` (`height * 0.2`, which tracks
        // font size, not the paragraph-break `newline_threshold`), and order each
        // line left-to-right by X. Ties broken by original index keep it stable.
        let n = fragments.len();
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&i, &j| fragments[j].y.total_cmp(&fragments[i].y).then(i.cmp(&j)));

        let mut line_start = 0usize;
        while line_start < n {
            let head_y = fragments[order[line_start]].y;
            let head_h = fragments[order[line_start]].height;
            let mut line_end = line_start + 1;
            while line_end < n {
                let frag = &fragments[order[line_end]];
                let tol = head_h.min(frag.height) * 0.2;
                // Negated `< tol` (not `>= tol`) so a non-finite Y from a
                // degenerate text matrix forces a line break instead of a
                // NaN comparison silently swallowing every remaining fragment.
                if !((head_y - frag.y).abs() < tol) {
                    break;
                }
                line_end += 1;
            }
            order[line_start..line_end].sort_by(|&i, &j| fragments[i].x.total_cmp(&fragments[j].x));
            line_start = line_end;
        }

        // Apply the permutation in place (one clone per fragment, then move back).
        let reordered: Vec<TextFragment> = order.iter().map(|&i| fragments[i].clone()).collect();
        for (slot, frag) in fragments.iter_mut().zip(reordered) {
            *slot = frag;
        }

        // Detect columns if requested. `reorder_columns` forces column detection
        // only on the flat path (`!preserve_layout`); in layout mode `detect_columns`
        // is the intended control, keeping the `reorder_columns` field flat-only as
        // documented (issue #389).
        if self.options.detect_columns
            || (self.options.reorder_columns && !self.options.preserve_layout)
        {
            self.detect_and_sort_columns(fragments);
        }
    }

    /// Detect columns and re-sort fragments accordingly
    fn detect_and_sort_columns(&self, fragments: &mut [TextFragment]) {
        // `fragments` arrives pre-sorted by `sort_and_merge_fragments` in reading
        // order: top-to-bottom by Y band, left-to-right by X within a band.
        //
        // Column boundaries are scoped to the row-span of the block that produced
        // them (issue #403). A page that mixes a small table with unrelated
        // full-width prose must not apply the table's column gaps to the
        // paragraph: doing so bucketed the paragraph's per-glyph fragments into
        // different "columns" by x-position and shredded any token that straddled
        // a boundary. We therefore only reorder fragments inside a *columnar
        // block* — a maximal run of consecutive lines that each exhibit an
        // internal gap wider than `column_threshold` — and leave full-width
        // "flow" lines in their natural reading order.

        // Group fragment indices into lines. Indices (not `&mut`) so we can later
        // reorder the slice by a computed permutation.
        //
        // The tolerance is anchored to the *line head* with a font-relative jitter
        // (`min(head, frag).height * 0.2`), matching `sort_and_merge_fragments` /
        // `merge_into_lines` (issue #408). A fixed `newline_threshold` band keyed
        // to the *previous* fragment accumulated drift on tight (sub-threshold)
        // leading and merged nearly a whole page into one pseudo-line, which the
        // block reorder below then reshuffled by X, shredding tokens (issue #417).
        let mut lines: Vec<Vec<usize>> = Vec::new();
        let mut current_line: Vec<usize> = Vec::new();
        let mut head_y = f64::INFINITY;
        let mut head_h = 0.0_f64;
        for (i, fragment) in fragments.iter().enumerate() {
            if !current_line.is_empty() {
                let tol = head_h.min(fragment.height) * 0.2;
                // Negated `< tol` (not `>= tol`) so a non-finite Y from a
                // degenerate text matrix forces a line break rather than swallowing
                // the whole page into one line.
                if !((head_y - fragment.y).abs() < tol) {
                    lines.push(std::mem::take(&mut current_line));
                }
            }
            if current_line.is_empty() {
                head_y = fragment.y;
                head_h = fragment.height;
            }
            current_line.push(i);
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // A line is "columnar" when it has at least one internal gap wider than
        // `column_threshold`.
        let line_is_columnar = |line: &[usize]| -> bool {
            line.windows(2).any(|w| {
                let (a, b) = (&fragments[w[0]], &fragments[w[1]]);
                b.x - (a.x + a.width) > self.options.column_threshold
            })
        };

        // Two column boundaries within this many points are the same corridor.
        // Shared by the alignment gate (#422) and the boundary-dedup step below (#403).
        const COLUMN_ALIGN_TOL: f64 = 10.0;

        // Wide-gap boundary X positions of a line: the midpoint of each internal gap
        // wider than `column_threshold`. Non-empty iff `line_is_columnar(line)`.
        let line_boundaries = |line: &[usize]| -> Vec<f64> {
            let mut bs = Vec::new();
            for w in line.windows(2) {
                let (a, b) = (&fragments[w[0]], &fragments[w[1]]);
                let gap = b.x - (a.x + a.width);
                if gap > self.options.column_threshold {
                    bs.push(a.x + a.width + gap / 2.0);
                }
            }
            bs
        };

        // Segment lines into blocks: consecutive columnar lines share one segment
        // id (a multi-line column block); every other line is its own segment.
        // Segment ids are monotonic top-to-bottom, so a later stable sort keyed on
        // (segment, column) keeps regions in their original vertical order.
        let n = fragments.len();
        let mut segment_of = vec![0usize; n];
        let mut column_of = vec![0usize; n];

        let mut has_columnar_block = false;
        let mut seg_id = 0usize;
        let mut prev_columnar = false;
        let mut prev_y = f64::INFINITY;
        let mut prev_h = 0.0_f64;
        // Anchor corridors of the CURRENT block: the wide-gap boundaries that
        // have recurred (within COLUMN_ALIGN_TOL) on *every* line of the block
        // so far, not just the immediately preceding line.
        let mut block_boundaries: Vec<f64> = Vec::new();

        for (li, line) in lines.iter().enumerate() {
            let boundaries = line_boundaries(line);
            let columnar = !boundaries.is_empty();
            let head = &fragments[line[0]];
            // Two consecutive columnar lines share a multi-line column block only
            // when they are spaced like real table rows — at least a line height
            // apart. Tight-leading wrapped prose whose lines each happen to hold a
            // wide gap forms a common whitespace corridor and is geometrically
            // indistinguishable from a 2-column layout; merging it and reordering
            // column-major shredded the prose (#417).
            let row_spaced = (prev_y - head.y).abs() >= head.height.max(prev_h);
            // ...and only when a wide gap ALIGNS horizontally with the block's
            // running anchor. A real column is a whitespace corridor shared
            // across every row; several unrelated wide gaps at different X (a
            // label/value form with varying label lengths) are not a table.
            //
            // Alignment is checked against the whole block, not just the
            // previous line: a pairwise-only check let unrelated gaps chain
            // through accumulated drift (line N aligns with N-1, N-1 with N-2,
            // yet N shares no corridor with the anchor) into one giant block,
            // scattering a token embedded in that span across the page (#425).
            // Anchoring to the block — the way sort_and_merge_fragments anchors
            // line tolerance to the line head (#408) — removes the drift. The
            // pairwise `prev_boundaries` check that this replaces first landed
            // for #422; the anchor set subsumes it.
            let shared: Vec<f64> = block_boundaries
                .iter()
                .copied()
                .filter(|&p| boundaries.iter().any(|&c| (p - c).abs() < COLUMN_ALIGN_TOL))
                .collect();
            if li > 0 && columnar && prev_columnar && row_spaced && !shared.is_empty() {
                // Line joins the current block; tighten the anchor to the
                // corridors that persist, so a boundary must recur consistently
                // across the whole block to survive.
                block_boundaries = shared;
            } else {
                // Break the block: new segment, anchored to this line's own gaps.
                if li > 0 {
                    seg_id += 1;
                }
                block_boundaries = boundaries;
            }
            for &i in line {
                segment_of[i] = seg_id;
            }
            prev_columnar = columnar;
            prev_y = head.y;
            prev_h = head.height;
        }

        // For each columnar block (a segment whose lines are columnar), derive
        // boundaries from that block's lines only and assign each fragment its
        // column. Flow segments keep column 0, so the stable sort preserves their
        // left-to-right reading order untouched.
        let mut block_start = 0usize;
        while block_start < lines.len() {
            if !line_is_columnar(&lines[block_start]) {
                block_start += 1;
                continue;
            }
            let seg = segment_of[lines[block_start][0]];
            let mut block_end = block_start;
            while block_end < lines.len() && segment_of[lines[block_end][0]] == seg {
                block_end += 1;
            }

            // A real column boundary is a whitespace corridor that RECURS across
            // rows. Collect each line's wide-gap midpoints, then keep only
            // corridors seen on at least two distinct lines (within
            // COLUMN_ALIGN_TOL). A one-off gap — a single wide space inside
            // otherwise-flowing text, e.g. the space before a mid-page token —
            // is not a column; pooling it as a boundary bucketed the token into
            // a phantom column and relocated its pieces across the block (#425).
            let mut corridors: Vec<(f64, usize)> = Vec::new(); // (position, line count)
            for line in &lines[block_start..block_end] {
                // This line's wide-gap corridors, deduped within tolerance so a
                // line credits each corridor at most once.
                let mut line_bs: Vec<f64> = Vec::new();
                for w in line.windows(2) {
                    let (a, b) = (&fragments[w[0]], &fragments[w[1]]);
                    let gap = b.x - (a.x + a.width);
                    if gap > self.options.column_threshold {
                        let bpos = a.x + a.width + gap / 2.0;
                        if !line_bs.iter().any(|&c| (c - bpos).abs() < COLUMN_ALIGN_TOL) {
                            line_bs.push(bpos);
                        }
                    }
                }
                for bpos in line_bs {
                    if let Some(entry) = corridors
                        .iter_mut()
                        .find(|(c, _)| (*c - bpos).abs() < COLUMN_ALIGN_TOL)
                    {
                        entry.1 += 1;
                    } else {
                        corridors.push((bpos, 1));
                    }
                }
            }
            let mut boundaries = vec![0.0];
            for (pos, count) in &corridors {
                if *count >= 2 {
                    boundaries.push(*pos);
                }
            }
            boundaries.sort_by(|a, b| a.total_cmp(b));

            if boundaries.len() > 1 {
                has_columnar_block = true;
                for line in &lines[block_start..block_end] {
                    for &i in line {
                        // Column = index of the last boundary not exceeding x.
                        // `boundaries[0]` is 0.0; a fragment drawn off-page-left
                        // (x < 0) saturates to column 0 rather than underflowing.
                        let col = boundaries
                            .iter()
                            .position(|&boundary| fragments[i].x < boundary)
                            .map_or(boundaries.len() - 1, |p| p.saturating_sub(1));
                        column_of[i] = col;
                    }
                }
            }
            block_start = block_end;
        }

        // No columnar block → nothing to reorder; the reading-order sort stands.
        if !has_columnar_block {
            return;
        }

        // Stable permutation by (segment, column), tie-broken by original index so
        // reading order is preserved within each (segment, column) — top-to-bottom
        // then left-to-right, i.e. column-major within a block.
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&i, &j| {
            segment_of[i]
                .cmp(&segment_of[j])
                .then(column_of[i].cmp(&column_of[j]))
                .then(i.cmp(&j))
        });

        // Materialize the permuted order once (one clone per fragment), then move
        // each element back into place — avoids a second full-slice clone.
        let reordered: Vec<TextFragment> = order.iter().map(|&i| fragments[i].clone()).collect();
        for (slot, frag) in fragments.iter_mut().zip(reordered) {
            *slot = frag;
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

            // Y-tolerance for same-line merging.
            //
            // Legacy path (`reconstruct_paragraphs=false`): fragments arrive
            // after `sort_and_merge_fragments` which quantizes Y into 10pt bands.
            // All same-band fragments share nearly identical Y, so 1.0pt is enough.
            //
            // Reconstruct-paragraphs path (`reconstruct_paragraphs=true`): fragments
            // arrive in emission order. Inline superscripts (e.g. citation numbers
            // raised via `Td` operators) have Y deltas of 3-4pt for 10pt body text.
            // Without a wider tolerance, each superscript becomes its own fragment
            // → line proliferation (issue #265 follow-up). Use 0.5 * font_size,
            // which captures typical superscript/subscript offsets (typically
            // 0.33-0.4 * font_size from baseline) and stays below the row_id
            // threshold (also 0.5 * font_size) so adjacent rows are not collapsed.
            let y_tol = if self.options.reconstruct_paragraphs {
                // Defend against malformed PDFs that emit text before any `Tf` font
                // operator (font_size=0 in TextState initial). 0.5 * 0 = 0 would
                // prevent any merge, even at identical Y. Fall back to the legacy
                // 1.0pt threshold in that case so the path is at least as forgiving
                // as the non-reconstruct path.
                let base = 0.5 * current.font_size.min(fragment.font_size);
                if base > 0.0 {
                    base
                } else {
                    1.0
                }
            } else {
                1.0
            };

            let should_merge = y_diff < y_tol
                && x_gap >= 0.0  // Fragment is to the right
                && x_gap < fragment.font_size * 0.5 // Gap less than 50% of font size
                && current.mcid == fragment.mcid;

            if should_merge {
                // Merge this fragment into current, preserving word boundaries
                // when the gap exceeds the font-anchored space threshold.
                if x_gap > self.space_gap_threshold(fragment) {
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
                self.cache_fonts_from_resources::<R>(&resources, document);
            }
        } else if let Some(resources) = page.get_resources() {
            // Fallback to get_resources() if Resources is not a reference
            self.cache_fonts_from_resources::<R>(resources, document);
        }

        Ok(())
    }

    /// Cache every font declared in a page's `/Resources` `/Font` dictionary.
    ///
    /// `/Font` itself may be either an inline dictionary or an indirect
    /// reference (`/Font 191 0 R`); both are common in real PDFs (e.g. the
    /// ATLAS Higgs paper references it). Resolving the reference is required —
    /// otherwise the font cache stays empty, decoding loses ToUnicode, and
    /// glyph widths fall back to a flat estimate that scrambles multi-column
    /// layout (issue #302).
    fn cache_fonts_from_resources<R: Read + Seek>(
        &mut self,
        resources: &PdfDictionary,
        document: &PdfDocument<R>,
    ) {
        for (font_name, entry) in
            crate::text::extraction_cmap::resolve_font_entries(resources, document)
        {
            match entry {
                crate::text::extraction_cmap::FontEntry::Indirect(num, gen) => {
                    self.cache_font_by_ref::<R>(&font_name, (num, gen), document);
                }
                crate::text::extraction_cmap::FontEntry::Inline(font_dict) => {
                    self.cache_inline_font::<R>(&font_name, &font_dict, document);
                }
            }
        }
    }

    /// Cache a font written directly into the page's resources.
    ///
    /// Unlike [`Self::cache_font_by_ref`] this cannot touch the persistent
    /// cache: an inline dictionary has no object id to key on, and two pages
    /// may write different fonts under the same name. It is parsed per page.
    fn cache_inline_font<R: Read + Seek>(
        &mut self,
        font_name: &str,
        font_dict: &PdfDictionary,
        document: &PdfDocument<R>,
    ) {
        let mut cmap_extractor: CMapTextExtractor<R> = CMapTextExtractor::new();
        if let Ok(font_info) = cmap_extractor.extract_font_info(font_dict, document) {
            tracing::debug!(
                "Parsed inline font {} (ToUnicode: {})",
                font_name,
                font_info.to_unicode.is_some()
            );
            self.font_cache.insert(font_name.to_string(), font_info);
        }
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
                    // Only accept if we got meaningful text (not all null bytes
                    // or garbage). Whitespace counts as meaningful: a decode
                    // that is exactly a space is a space, not a failed decode
                    // (#438). See `decode_is_usable`.
                    let sanitized = sanitize_extracted_text_with_policy(
                        &decoded,
                        self.carriage_return_handling,
                    );
                    if crate::text::extraction_cmap::decode_is_usable(&sanitized) {
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
        let sanitized =
            sanitize_extracted_text_with_policy(&fallback_result, self.carriage_return_handling);
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
/// Whether the current marked-content stack should suppress text emission.
///
/// Mirrors the gate inside [`emit_text_fragment`]: when an ancestor in the
/// stack is `/Artifact` and the caller has not opted into artifact content
/// via `include_artifacts`, neither `.text` nor `.fragments` should receive
/// the run. Used by the four show-text operator arms to keep `extracted_text`
/// and `fragments` symmetric — a page whose entire content is an
/// `/Artifact BMC … EMC` scope (the common pattern for screen-reader-skipped
/// disclaimers / footers / decorative tagged-PDF content) used to surface
/// text in `.text` while leaving `.fragments` empty, silently dropping the
/// page from `partition_with(...)` / `rag_chunks(...)` (issue #330).
fn skip_artifact_text(state: &TextState, include_artifacts: bool) -> bool {
    !include_artifacts && state.mc_stack.iter().any(|e| e.is_artifact)
}

/// Page-space scale factors `(x_scale, y_scale)` of the current text/CTM
/// combination (issue #262). Converts a text-space width/size into page space,
/// mirroring the scaling [`emit_text_fragment`] applies, so the reading-order
/// boxes and the median-font unit that judges their gaps share the page-space
/// scale of the `x`/`y` origins.
fn combined_text_scale(state: &TextState) -> (f64, f64) {
    let combined = multiply_matrix(&state.text_matrix, &state.ctm);
    let x_scale = (combined[0] * combined[0] + combined[1] * combined[1]).sqrt();
    let y_scale = (combined[2] * combined[2] + combined[3] * combined[3]).sqrt();
    (x_scale, y_scale)
}

/// Record a just-emitted glyph run into the flat-path line groups (issue #448).
///
/// Called only when `ExtractionOptions::reading_order` is on, right after a
/// successful [`append_bounded`], with `text_len` = `extracted_text.len()` after
/// the append. `width` and `font_size` must already be page-space (scaled via
/// [`combined_text_scale`]) so the box matches the page-space `x`/`y` origin. A
/// run whose separator is a newline opens a new group; anything else extends the
/// current one. The group's byte range excludes the leading newline (a group's
/// text starts at `text_len - decoded_len`, which is past the separator), so
/// rejoining slices with `'\n'` reproduces the original exactly.
#[allow(clippy::too_many_arguments)]
fn record_line_group(
    line_groups: &mut Vec<LineGroupGeom>,
    cur_group: &mut Option<LineGroupGeom>,
    text_len: usize,
    decoded_len: usize,
    separator: Option<char>,
    x: f64,
    y: f64,
    text_width: f64,
    state: &TextState,
) {
    // Convert the text-space advance and font size to page space so the box
    // matches the page-space `x`/`y` and the median-font unit is on the same
    // scale as the gaps it judges (issue #262).
    let (x_scale, y_scale) = combined_text_scale(state);
    let width = text_width * x_scale;
    let font_size = state.font_size * y_scale;
    let run_start = text_len.saturating_sub(decoded_len);
    let (rx0, rx1) = (x.min(x + width), x.max(x + width));
    let (ry0, ry1) = (y.min(y + font_size), y.max(y + font_size));
    let opens = matches!(separator, Some('\n')) || cur_group.is_none();
    if opens {
        if let Some(g) = cur_group.take() {
            line_groups.push(g);
        }
        *cur_group = Some(LineGroupGeom {
            start: run_start,
            end: text_len,
            min_x: rx0,
            max_x: rx1,
            min_y: ry0,
            max_y: ry1,
            font_size,
        });
    } else if let Some(g) = cur_group.as_mut() {
        g.end = text_len;
        g.min_x = g.min_x.min(rx0);
        g.max_x = g.max_x.max(rx1);
        g.min_y = g.min_y.min(ry0);
        g.max_y = g.max_y.max(ry1);
        g.font_size = g.font_size.max(font_size);
    }
}

/// Outcome of [`append_bounded`]: whether the run was appended, and — when it
/// was — the separator actually applied. The applied separator can differ
/// from the one the caller requested when hyphen-wrap fusion (issue #486)
/// consumes a trailing `-` instead of inserting the requested `\n`; callers
/// that feed the separator into reading-order line grouping (`record_line_group`)
/// must use `applied_separator`, not the separator they originally computed,
/// so a fused run correctly extends its line group instead of opening a new one.
struct AppendOutcome {
    appended: bool,
    applied_separator: Option<char>,
}

/// Append an optional `separator` plus `decoded` to `acc`, honouring the
/// per-page byte budget `limit` (issue #382), with optional hyphen-wrap
/// fusion (issue #486).
///
/// Returns [`AppendOutcome`] with `appended: true` when the run was appended.
/// Returns `appended: false` — appending nothing and setting `*truncated` —
/// when the combined bytes would exceed `limit`. The separator is counted
/// against the budget so the invariant `acc.len() <= limit` holds *exactly*,
/// and because whole runs are the unit of truncation a multi-byte UTF-8
/// character is never split (undershoot semantics). A `None` limit always
/// appends and never truncates, keeping the no-limit path byte-identical to
/// before. Once `*truncated` is set the helper is a no-op, so a caller that
/// keeps calling it after the budget is reached simply accumulates nothing
/// further.
///
/// When `merge_hyphenated` is set and the caller requests a `'\n'` separator
/// (a genuine line wrap) while `acc` already ends with `-`, the hyphen is
/// producer noise from a hyphenated word/number wrapping across two lines,
/// not a real word boundary (issue #486: `merge_hyphenated` had no effect on
/// this flat/default extraction path, unlike `preserve_layout`'s
/// `reconstruct_text_from_fragments` and `reconstruct_paragraphs`'s
/// `merge_into_paragraphs`, both of which already apply this same rule). The
/// trailing hyphen is popped and `decoded` is appended directly with no
/// separator, fusing the wrapped token into one word instead of splitting it
/// on a newline — e.g. `"...3016-"` + `"0900"` becomes `"...30160900"`
/// instead of `"...3016-\n0900"`. `separator` is only ever `'\n'` here when
/// `acc` is already non-empty (every call site gates on that), so the pop is
/// always into at least one existing byte.
fn append_bounded(
    acc: &mut String,
    separator: Option<char>,
    decoded: &str,
    limit: Option<usize>,
    truncated: &mut bool,
    merge_hyphenated: bool,
) -> AppendOutcome {
    if *truncated {
        return AppendOutcome {
            appended: false,
            applied_separator: None,
        };
    }

    let hyphen_fusion = merge_hyphenated && separator == Some('\n') && acc.ends_with('-');
    let separator = if hyphen_fusion { None } else { separator };

    if let Some(max) = limit {
        // Popping the hyphen frees one byte before the new run is added, so
        // account against the post-pop length — otherwise a run that fits
        // once the hyphen is dropped could be wrongly rejected as
        // over-budget by one byte.
        let base_len = if hyphen_fusion {
            acc.len() - 1
        } else {
            acc.len()
        };
        let add = separator.map_or(0, char::len_utf8) + decoded.len();
        if base_len + add > max {
            *truncated = true;
            return AppendOutcome {
                appended: false,
                applied_separator: None,
            };
        }
    }

    if hyphen_fusion {
        acc.pop();
    }
    if let Some(sep) = separator {
        acc.push(sep);
    }
    acc.push_str(decoded);
    AppendOutcome {
        appended: true,
        applied_separator: separator,
    }
}

/// Defensive final clamp of a page's text to the byte budget (issue #382).
///
/// The `preserve_layout` / `reorder_columns` paths rebuild `.text` from the
/// already-bounded fragment set via `reconstruct_text_from_fragments`, which
/// reorders fragments and inserts its own separators — so the reconstructed
/// length is not provably `<= limit` from the accumulation-time accounting
/// alone. This clamps the result to `limit` at a UTF-8 char boundary (never
/// splitting a character) and sets `*truncated` if it had to cut, making the
/// `text.len() <= max_extracted_bytes` invariant hold for *every* path. A no-op
/// when there is no limit or the text already fits.
fn clamp_to_budget(text: &mut String, limit: Option<usize>, truncated: &mut bool) {
    if let Some(max) = limit {
        if text.len() > max {
            let mut cut = max;
            while cut > 0 && !text.is_char_boundary(cut) {
                cut -= 1;
            }
            text.truncate(cut);
            *truncated = true;
        }
    }
}

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
    // Text rise (`Ts`) shifts the glyph origin up the text-space y-axis before
    // the text/CTM transform (ISO 32000-1 §9.4.4). For an axis-aligned matrix
    // this moves the user-space y by exactly `Ts`.
    transform_point(0.0, state.text_rise, &combined)
}

/// Advance the text matrix by one shown glyph run of unscaled width
/// `text_width` and return the pen's new x in user space.
///
/// The advance applied to the text matrix is `text_width * Tz/100`
/// (`state.horizontal_scale`), and the resulting user-space displacement also
/// folds in the CTM's x-scale. The caller's `last_x` (used for `dx`-based
/// space decisions) must therefore come from the post-advance pen origin, not
/// from `origin_x + text_width`, which ignores both factors and trails the
/// real pen whenever `Tz != 100` or the CTM scales x (issue #386).
fn advance_pen(state: &mut TextState, text_width: f64) -> (f64, f64) {
    let tx = text_width * state.horizontal_scale / 100.0;
    state.text_matrix = multiply_matrix(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &state.text_matrix);
    text_origin(state)
}

/// Projection-noise floor for the perpendicular pen delta. Same-baseline
/// glyph runs produce a `dy` that is exactly 0 in real arithmetic but can
/// carry ~1e-13 of float rounding after the baseline projection; anything
/// below this epsilon is "the same baseline". The smallest meaningful
/// leading in real documents is orders of magnitude above it.
const SAME_LINE_EPS: f64 = 1e-6;

/// Scale-relative cut thresholds for the flat-path reading-order option
/// (issue #448), in multiples of a region's median glyph size: a horizontal gap
/// is a column gutter past `horizontal_k`, a vertical gap a section break past
/// `vertical_k`.
///
/// Validated against the opt-in differential order gate on the full `t3-stress`
/// corpus (`t3-stress-reading-order` baseline): with the option on, the
/// misplaced-word rate drops 0.2486 → 0.2255 (−9.3%) versus the default flat
/// path, at identical alignment coverage — the gain is reordering, not dropped
/// text. That clears the design probe's ceiling estimate (~0.2375), so these
/// values are kept rather than sweeping for a marginal further gain. (An
/// earlier, geometrically wrong build that fed the cut un-CTM-scaled boxes
/// scored a hair better here, 0.2226, purely because the corpus is
/// identity-CTM-dominated; the correct page-space geometry is kept.)
const READING_ORDER_CFG: flat_reading_order::CutConfig = flat_reading_order::CutConfig {
    horizontal_k: 1.0,
    vertical_k: 1.5,
};

/// Minimum forward pen jump, in em, that reads as a word break at the boundary
/// between two show-text operators — the first element of a `TJ` array whose
/// pen jumped forward from the previous operator (a `Tm` reposition, or the
/// prior operator's advance). Without it, two `TJ` operators drawn side by side
/// on the same line come out glued: a multi-column table cell reads as
/// `CellOneCellTwo` (issue #458), and a list bullet 0.75 em from its item text
/// reads as `vlarge` (found on preserve_027613.pdf, an IBM manual whose every
/// bullet is a separate `TJ`).
///
/// Calibrated on the full `t3-stress` corpus against poppler, with the
/// reading-order (misplaced) rate as the objective and alignment coverage as
/// the guard. Recalibrated on the Tc/Tw-corrected pen advance (#456), which
/// feeds the `dx` this threshold judges:
///
/// | em | 0.0 | 0.3 | 0.7 | 1.0 | 2.0 | 3.0 | 6.0 | off |
/// |---|---|---|---|---|---|---|---|---|
/// | misplaced rate | .2806 | .2485 | **.2486** | .2486 | .2486 | .2512 | .2702 | .2766 |
///
/// Below ~0.3 em the rule splits words a producer draws as several positioned
/// runs — the pen advance is only as accurate as the font widths, so a short
/// run inflates the apparent gap, and em=0.0 lands *worse* than not firing at
/// all. From 0.3 to 2.0 the corpus cannot discriminate (a flat plateau within
/// 1e-4 of the minimum); above 3 em the rule stops firing on genuine column
/// gaps and converges back on the un-fixed number.
///
/// 0.7 sits inside that plateau with margin on both sides: comfortably above
/// the word-splitting floor, and below 0.748 em — the narrowest real word gap
/// verified by hand (the bullet above), which the threshold must stay under to
/// keep separating. On the corrected advance the fix moves the rate .2766 →
/// .2486 (vs .2874 → .2714 before #456: an accurate advance lets the boundary
/// fire more cleanly).
const TJ_BOUNDARY_SPACE_EM: f64 = 0.7;

/// Backward-jump magnitude, in multiples of the font size, above which a
/// same-baseline (`dy == 0`) backward pen jump is a line wrap rather than a
/// glyph reposition (issue #447).
///
/// At `dy == 0` a backward jump is ambiguous: a same-line reposition
/// (justification, kerned overlay, out-of-order emission — issue #441) and a
/// real wrap whose two lines happen to land on the same content-stream Y
/// (issue #447) both produce it. They separate by MAGNITUDE: a reposition is
/// local (a word/phrase — a few em), while a wrap returns across the whole
/// text column (many em). This bound sits in that gap, scaled to font size
/// because the reposition scale is the glyph/word scale, not the fixed
/// paragraph-break `newline_threshold`. Scaled to `font_size.abs()`: `Tf`
/// accepts negative sizes (mirrored text), and the sign must not flip the
/// threshold's sense — otherwise a negative size makes every backward jump a
/// "wrap" and resurrects the #441 defect.
///
/// Accepted, documented limitation (the #417/#422 trade-off): a same-line
/// reposition that jumps back more than this many em is misread as a wrap, and
/// a same-Y wrap of a line shorter than this is glued. Both are rare and
/// neither loses a glyph — only the separator is wrong. A wrap with any
/// nonzero leading (the common case, issue #390) is unaffected: it breaks on
/// the `dy`-aware gate regardless of magnitude.
const SAME_Y_WRAP_EM: f64 = 10.0;

/// Pen movement from the previous post-advance pen point `last` to the
/// current glyph origin `cur` (both user space), measured in the frame of the
/// current text baseline (issue #443): `dx` along the baseline direction,
/// `dy` perpendicular to it (signed; callers take `.abs()` for line
/// detection).
///
/// The baseline direction is the image of the text-space x-axis under the
/// text rendering matrix `Tm × CTM`. For an axis-aligned matrix
/// (identity/translation/positive scale — the overwhelming majority of
/// content) the baseline IS the user-space x-axis and this returns exactly
/// `(Δx, Δy)`, the pre-#443 behavior. Under a rotated CTM (and any
/// similarity transform) the projection recovers the text's own line
/// geometry exactly, which raw user-space deltas conflate: a plain forward
/// advance along a rotated baseline changes the user-space y, which the
/// separator heuristics misread as a line change. Axis-aligned shear
/// (`b == 0`, `c != 0`) also projects exactly (the perpendicular reduces to
/// the y-axis); a shear COMBINED with a rotated baseline is approximated —
/// the perpendicular is built by rotating the baseline 90°, not from the
/// true image of the text-space y-axis.
///
/// A mirrored baseline (negative x-scale) measures `dx` along the text's own
/// advance direction, so a forward advance is positive `dx` — the spacing
/// and wrap gates apply as for unmirrored text (pre-#443 they saw a raw
/// negative `dx` and misfired the wrap gate on plain advances).
///
/// A degenerate baseline (zero-length or non-finite) falls back to the raw
/// user-space deltas, preserving pre-#443 behavior for malformed matrices.
fn pen_delta(state: &TextState, last: (f64, f64), cur: (f64, f64)) -> (f64, f64) {
    let dxu = cur.0 - last.0;
    let dyu = cur.1 - last.1;
    let m = multiply_matrix(&state.text_matrix, &state.ctm);
    let (bx, by) = (m[0], m[1]);
    let norm = (bx * bx + by * by).sqrt();
    if !norm.is_finite() || norm <= f64::EPSILON {
        return (dxu, dyu);
    }
    let (ux, uy) = (bx / norm, by / norm);
    (dxu * ux + dyu * uy, -dxu * uy + dyu * ux)
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
/// A string inside marked-content properties (notably `/ActualText`) is a PDF
/// text string like any other, so this is
/// [`PdfString::to_text`](crate::parser::objects::PdfString::to_text): UTF-16BE
/// when a byte order mark is present — the canonical encoding for non-ASCII
/// `/ActualText`, e.g. an `fi` ligature or a Greek symbol — and the WinAnsi
/// reading of PDFDocEncoding otherwise. Before that helper existed this mapped
/// non-BOM bytes to `char` one by one, which is Latin-1 and wrong for the
/// typographic punctuation WinAnsi puts in `0x80..=0x9F`.
fn decode_pdf_string(bytes: &[u8]) -> String {
    crate::parser::objects::decode_text_string(bytes)
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

/// Compute advance width from the original character **codes**, not the decoded
/// Unicode text.
///
/// A simple font's `Widths` array is indexed by character code (`first_char..=
/// last_char`), i.e. the byte value in the content stream — not by the Unicode
/// codepoint the code decodes to. [`calculate_text_width`] indexes by the decoded
/// codepoint (`ch as u32`), which is correct only when code == codepoint (ASCII /
/// WinAnsi fonts). For custom-encoded fonts (Type1 with `Differences`, embedded
/// Computer Modern in LaTeX PDFs, ToUnicode remaps) the codepoint diverges from
/// the code, so the wrong slot — or `missing_width` — is read, desyncing glyph
/// advance and scrambling word order once fragments are sorted by position
/// (issue #302).
///
/// `decoded` is the already-decoded text for this run; it is only consulted for
/// composite (Type0) fonts, whose multi-byte codes cannot be indexed byte-wise
/// and whose width path is unchanged here to avoid regressing CJK extraction.
/// Unscaled text-space advance of a run (before `Th`), including the text-state
/// spacing parameters (ISO 32000-1 §9.4.4): the glyph displacement is
/// `w0/1000 * Tfs + Tc + Tw`, so `char_space` (`Tc`) is added once per glyph and
/// `word_space` (`Tw`) once per *single-byte* space (code 32, §9.3.3). Both are
/// unscaled text-space units, added directly (not multiplied by the font size);
/// the caller's `advance_pen` applies `Th`.
fn calculate_text_width_from_codes(
    codes: &[u8],
    decoded: &str,
    font_size: f64,
    font_info: Option<&FontInfo>,
    char_space: f64,
    word_space: f64,
) -> f64 {
    // Composite (Type0) fonts use multi-byte codes; a single byte is not a code,
    // so byte-indexed width lookup is invalid. Preserve the existing decoded-based
    // behavior for them, adding `Tc` per glyph. `Tw` applies only to the
    // single-byte code 32 (§9.3.3), which a multi-byte code can never be, so it
    // does not apply here.
    let is_composite =
        font_info.is_some_and(|f| f.font_type == "Type0" || f.descendant_font.is_some());
    if is_composite {
        let glyphs = decoded.chars().count() as f64;
        return calculate_text_width(decoded, font_size, font_info) + char_space * glyphs;
    }

    // `Tc` on every byte-code, `Tw` on every space byte. Shared by the metric
    // and no-metric branches below.
    let spacing = |codes: &[u8]| -> f64 {
        char_space * codes.len() as f64
            + word_space * codes.iter().filter(|&&b| b == b' ').count() as f64
    };

    if let Some(font) = font_info {
        if let Some(ref widths) = font.metrics.widths {
            let first_char = font.metrics.first_char.unwrap_or(0);
            let last_char = font.metrics.last_char.unwrap_or(255);
            let missing_width = font.metrics.missing_width.unwrap_or(500.0);

            let mut total_width = 0.0;
            let mut iter = codes.iter().peekable();
            while let Some(&byte) = iter.next() {
                let code = byte as u32;
                let width = if code >= first_char && code <= last_char {
                    widths
                        .get((code - first_char) as usize)
                        .copied()
                        .unwrap_or(missing_width)
                } else {
                    missing_width
                };
                total_width += width / 1000.0 * font_size;

                // Kerning is keyed by code pair, consistent with code-based widths.
                if let Some(ref kerning) = font.metrics.kerning {
                    if let Some(&next_byte) = iter.peek() {
                        if let Some(&kern_value) = kerning.get(&(code, *next_byte as u32)) {
                            total_width += kern_value / 1000.0 * font_size;
                        }
                    }
                }
            }

            return total_width + spacing(codes);
        }
    }

    // No metrics: one fallback width per code (byte), the simple-font glyph count.
    codes.len() as f64 * font_size * 0.5 + spacing(codes)
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
/// - Normalizes `\r` and `\r\n` to `\n`
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
    sanitize_extracted_text_with_policy(text, CarriageReturnHandling::default())
}

/// Sanitize extracted text using an explicit carriage-return policy.
pub fn sanitize_extracted_text_with_policy(
    text: &str,
    carriage_return_handling: CarriageReturnHandling,
) -> String {
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

            '\r' => {
                // CRLF is unambiguously one line ending under every policy.
                // Ignore controls that sanitization would remove between the
                // pair, otherwise a first pass could create CRLF and a second
                // pass would change it again (for example `"\r\u{1}\n"`).
                let removed_controls_before_lf = chars
                    .clone()
                    .take_while(|next| {
                        next.is_ascii_control() && !matches!(next, '\0' | '\t' | '\n' | '\r')
                    })
                    .count();
                let followed_by_lf = chars.clone().nth(removed_controls_before_lf) == Some('\n');

                if followed_by_lf {
                    for _ in 0..=removed_controls_before_lf {
                        chars.next();
                    }
                    result.push('\n');
                    last_was_space = false;
                } else {
                    match carriage_return_handling {
                        CarriageReturnHandling::Remove => {}
                        CarriageReturnHandling::ReplaceWithSpace => {
                            if !last_was_space {
                                result.push(' ');
                                last_was_space = true;
                            }
                        }
                        CarriageReturnHandling::NormalizeLineEnding => {
                            // A standalone CR is valid input and is not
                            // equivalent to LF. Only the CRLF sequence above
                            // is normalized as a line ending.
                            result.push('\r');
                            last_was_space = false;
                        }
                    }
                }
            }

            // Preserve allowed whitespace
            '\t' | '\n' => {
                result.push(ch);
                // Reset space tracking on newlines but not tabs.
                last_was_space = ch == '\t';
            }

            // Regular space - collapse multiples
            ' ' => {
                if !last_was_space {
                    result.push(' ');
                    last_was_space = true;
                }
            }

            // Other control characters (0x01-0x1F except tab/newline) - remove
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
///
/// # Invariants
/// Returns a `Vec<u32>` with exactly `fragments.len()` elements — one
/// row id per input fragment, in input order. Callers may safely `.zip(fragments)`.
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
    debug_assert_eq!(
        result.len(),
        fragments.len(),
        "assign_row_ids: output length must equal input length"
    );
    result
}

/// Decide whether a single visual line should be read in emission order.
///
/// `line` holds `(emission_index, fragment)` pairs for one visual line in any
/// order. Returns `true` when, walked in emission order, the line has no
/// DISJOINT backward x-step — i.e. no fragment lands entirely to the LEFT of
/// everything emitted so far on the line. Such a left jump is the signature of
/// a genuinely scrambled stream (right-to-left / random generators), for which
/// x-order is authoritative.
///
/// The comparison is against the line's running left edge, not the immediately
/// preceding fragment: dense bodies are split into sub-word glyph runs, so a
/// run that legitimately backfills the line (a font-switched math symbol, or a
/// word whose run starts left of the previous short run — #302 symptom 1 /
/// #305) overlaps the *covered span* even when it does not overlap the single
/// fragment right before it. As long as it does not jump past the line's left
/// edge, emission order is preserved. Lines that are already x-monotone in
/// emission satisfy this trivially and decode identically under either policy.
fn line_prefers_emission_order(line: &[(usize, &TextFragment)]) -> bool {
    if line.len() < 2 {
        return true;
    }
    let mut em: Vec<&(usize, &TextFragment)> = line.iter().collect();
    em.sort_by_key(|&&(idx, _)| idx);
    let mut min_start = em[0].1.x;
    for &&(_, f) in &em[1..] {
        let end = f.x + f.width;
        // A fragment whose right edge is at or left of the leftmost glyph seen
        // so far is a true backward jump — emission order is not reading order.
        if end <= min_start {
            return false;
        }
        min_start = min_start.min(f.x);
    }
    true
}

/// Space-glyph advance width (1000-em units) for the Adobe Core-14 base fonts,
/// keyed by `/BaseFont`. Subset prefixes (`ABCDEF+`) are stripped; common
/// substitute names (Arial→Helvetica, TimesNewRoman→Times, CourierNew→Courier)
/// map to their metric-compatible base. Returns `None` for unknown fonts, which
/// leaves the caller on its fixed-fraction fallback. These fonts legitimately
/// ship no `/Widths` array, so their space metric is only available here.
fn standard_14_space_width(base_font: &str) -> Option<f64> {
    let name = base_font.rsplit('+').next().unwrap_or(base_font);
    let lower = name.to_ascii_lowercase();
    if lower.contains("courier") {
        Some(600.0)
    } else if lower.contains("helvetica") || lower.contains("arial") {
        Some(278.0)
    } else if lower.contains("times") {
        Some(250.0)
    } else if lower == "symbol" {
        Some(250.0)
    } else if lower.contains("zapfdingbats") || lower.contains("dingbats") {
        Some(278.0)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── issue #443: baseline-frame pen deltas ────────────────────────────────

    fn state_with_ctm(ctm: [f64; 6]) -> TextState {
        TextState {
            ctm,
            ..Default::default()
        }
    }

    #[test]
    fn pen_delta_identity_matrix_returns_raw_deltas() {
        let state = state_with_ctm([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let (dx, dy) = pen_delta(&state, (10.0, 20.0), (14.5, 17.0));
        assert_eq!((dx, dy), (4.5, -3.0), "axis-aligned = raw Δx/Δy exactly");
    }

    #[test]
    fn pen_delta_rotation_recovers_text_space_advance() {
        // 30° rotation; the pen advances 5 units along the rotated baseline.
        let (s30, c30) = 30f64.to_radians().sin_cos();
        let state = state_with_ctm([c30, s30, -s30, c30, 0.0, 0.0]);
        let (dx, dy) = pen_delta(&state, (0.0, 0.0), (5.0 * c30, 5.0 * s30));
        assert!((dx - 5.0).abs() < 1e-12, "advance recovered: {dx}");
        assert!(dy.abs() < 1e-12, "same baseline → dy ≈ 0: {dy}");
        assert!(
            dy.abs() < SAME_LINE_EPS,
            "noise below the same-line epsilon"
        );
    }

    #[test]
    fn pen_delta_mirrored_baseline_measures_advance_direction() {
        // Horizontal mirror: a forward text-space advance moves the pen LEFT
        // in user space. dx must still be positive (the text's own advance
        // direction), so the wrap gate does not misfire on plain advances.
        let state = state_with_ctm([-1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        let (dx, dy) = pen_delta(&state, (100.0, 50.0), (95.0, 50.0));
        assert_eq!(dx, 5.0, "forward advance is positive along the baseline");
        assert_eq!(dy.abs(), 0.0, "same baseline");
    }

    #[test]
    fn pen_delta_degenerate_matrix_falls_back_to_raw_deltas() {
        // Zero baseline (a=b=0): projection impossible → raw user-space
        // deltas, the pre-#443 behavior.
        let state = state_with_ctm([0.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(pen_delta(&state, (1.0, 2.0), (4.0, 6.0)), (3.0, 4.0));
        // Non-finite baseline: same fallback.
        let nan_state = state_with_ctm([f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0]);
        assert_eq!(pen_delta(&nan_state, (1.0, 2.0), (4.0, 6.0)), (3.0, 4.0));
    }

    // ── issue #382: per-page byte-budget helper ──────────────────────────────

    #[test]
    fn test_append_bounded_no_limit_always_appends() {
        let mut s = String::new();
        let mut trunc = false;
        assert!(append_bounded(&mut s, None, "hello", None, &mut trunc, true).appended);
        assert!(append_bounded(&mut s, Some(' '), "world", None, &mut trunc, true).appended);
        assert_eq!(s, "hello world");
        assert!(!trunc, "no limit never truncates");
    }

    #[test]
    fn test_append_bounded_undershoot_counts_separator() {
        // "abcd" (4) is at budget 5; a Some('\n') + "x" would need 2 more → over.
        let mut s = String::from("abcd");
        let mut trunc = false;
        assert!(!append_bounded(&mut s, Some('\n'), "x", Some(5), &mut trunc, true).appended);
        assert_eq!(s, "abcd", "nothing appended when it would overshoot");
        assert!(trunc, "budget hit sets truncated");
        // Exactly-fits case: "e" alone (1 byte, no separator) reaches 5.
        let mut s2 = String::from("abcd");
        let mut t2 = false;
        assert!(append_bounded(&mut s2, None, "e", Some(5), &mut t2, true).appended);
        assert_eq!(s2, "abcde");
        assert!(!t2);
        assert!(s2.len() <= 5, "invariant: len <= limit exactly");
    }

    #[test]
    fn test_append_bounded_zero_limit_truncates_immediately() {
        let mut s = String::new();
        let mut trunc = false;
        assert!(!append_bounded(&mut s, None, "a", Some(0), &mut trunc, true).appended);
        assert!(s.is_empty());
        assert!(trunc);
    }

    #[test]
    fn test_append_bounded_is_noop_once_truncated() {
        let mut s = String::from("kept");
        let mut trunc = true; // already truncated
        assert!(!append_bounded(&mut s, None, "more", Some(1_000), &mut trunc, true).appended);
        assert_eq!(s, "kept", "no further accumulation after truncation");
    }

    // ── issue #486: flat-path hyphen-wrap fusion ─────────────────────────────

    #[test]
    fn test_append_bounded_fuses_hyphen_wrap_when_enabled() {
        // Real-world shape: a hyphen-wrapped phone number split across two
        // lines, e.g. "...3016-" / "0900" must reconstruct as "...30160900".
        let mut s = String::from("+55 11 3016-");
        let mut trunc = false;
        let outcome = append_bounded(&mut s, Some('\n'), "0900", None, &mut trunc, true);
        assert!(outcome.appended);
        assert_eq!(
            outcome.applied_separator, None,
            "hyphen fusion applies no separator, not the requested '\\n'"
        );
        assert_eq!(s, "+55 11 30160900", "hyphen popped, halves fused");
    }

    #[test]
    fn test_append_bounded_no_fusion_without_a_trailing_hyphen() {
        let mut s = String::from("hello world");
        let mut trunc = false;
        let outcome = append_bounded(&mut s, Some('\n'), "next line", None, &mut trunc, true);
        assert!(outcome.appended);
        assert_eq!(
            outcome.applied_separator,
            Some('\n'),
            "no trailing hyphen: requested separator applies unchanged"
        );
        assert_eq!(s, "hello world\nnext line");
    }

    #[test]
    fn test_append_bounded_does_not_fuse_when_merge_hyphenated_disabled() {
        let mut s = String::from("rating-");
        let mut trunc = false;
        let outcome = append_bounded(&mut s, Some('\n'), "aa-exp-sf", None, &mut trunc, false);
        assert!(outcome.appended);
        assert_eq!(outcome.applied_separator, Some('\n'));
        assert_eq!(s, "rating-\naa-exp-sf", "no fusion: split as requested");
    }

    #[test]
    fn test_append_bounded_does_not_fuse_a_space_separator() {
        // Only a requested '\n' is a wrap candidate; a same-line space must
        // never trigger hyphen fusion even if the accumulator ends in '-'.
        let mut s = String::from("well-");
        let mut trunc = false;
        let outcome = append_bounded(&mut s, Some(' '), "known", None, &mut trunc, true);
        assert!(outcome.appended);
        assert_eq!(outcome.applied_separator, Some(' '));
        assert_eq!(s, "well- known");
    }

    #[test]
    fn test_append_bounded_hyphen_fusion_respects_budget() {
        // "rating-" (7 bytes, trailing hyphen) minus the popped hyphen (6)
        // plus fused "aa-exp" (6 bytes, no separator) = 12.
        // Budget 12 must fit; budget 11 must not (would need to drop the
        // hyphen-adjusted run, not silently truncate mid-word).
        let mut s = String::from("rating-");
        let mut trunc = false;
        let outcome = append_bounded(&mut s, Some('\n'), "aa-exp", Some(12), &mut trunc, true);
        assert!(outcome.appended);
        assert_eq!(s, "ratingaa-exp");
        assert!(!trunc);

        let mut s2 = String::from("rating-");
        let mut trunc2 = false;
        let outcome2 = append_bounded(&mut s2, Some('\n'), "aa-exp", Some(11), &mut trunc2, true);
        assert!(!outcome2.appended);
        assert_eq!(s2, "rating-", "nothing appended when over budget");
        assert!(trunc2);
    }

    #[test]
    fn test_clamp_to_budget_no_limit_or_fits_is_noop() {
        let mut a = String::from("hello");
        let mut t = false;
        clamp_to_budget(&mut a, None, &mut t);
        assert_eq!(a, "hello");
        assert!(!t, "no limit never truncates");

        let mut b = String::from("hi");
        clamp_to_budget(&mut b, Some(10), &mut t);
        assert_eq!(b, "hi", "already fits");
        assert!(!t);
    }

    #[test]
    fn test_clamp_to_budget_cuts_and_flags() {
        let mut s = String::from("abcdefgh");
        let mut t = false;
        clamp_to_budget(&mut s, Some(3), &mut t);
        assert_eq!(s, "abc");
        assert!(t, "clamp that cut must set truncated");
    }

    #[test]
    fn test_clamp_to_budget_never_splits_utf8() {
        // "é" is 2 bytes (0xC3 0xA9). A 3-byte budget on "éé" (4 bytes) must cut
        // back to the char boundary at 2, keeping one whole "é".
        let mut s = String::from("éé");
        let mut t = false;
        clamp_to_budget(&mut s, Some(3), &mut t);
        assert_eq!(s, "é", "must retreat to a char boundary, not split 'é'");
        assert!(s.len() <= 3);
        assert!(t);
    }

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
        assert_eq!(
            CarriageReturnHandling::default(),
            CarriageReturnHandling::Remove
        );
    }

    #[test]
    fn test_extraction_options_custom() {
        let options = ExtractionOptions {
            preserve_layout: true,
            space_threshold: 0.5,
            tj_space_threshold: 0.15,
            newline_threshold: 15.0,
            sort_by_position: false,
            detect_columns: true,
            column_threshold: 75.0,
            merge_hyphenated: false,
            track_space_decisions: false,
            reconstruct_paragraphs: false,
            include_artifacts: false,
            reorder_columns: false,
            max_extracted_bytes: None,
        };
        assert!(options.preserve_layout);
        assert_eq!(options.space_threshold, 0.5);
        assert_eq!(options.tj_space_threshold, 0.15);
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
            truncated: false,
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
            tj_space_threshold: 0.2,
            newline_threshold: 12.0,
            sort_by_position: false,
            detect_columns: true,
            column_threshold: 60.0,
            merge_hyphenated: false,
            track_space_decisions: false,
            reconstruct_paragraphs: false,
            include_artifacts: false,
            reorder_columns: false,
            max_extracted_bytes: None,
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: None,
                last_char: None,
                widths: None,
                missing_width: Some(500.0),
                kerning: None,
            },
            cid_encoding: None,
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(widths),
                missing_width: Some(500.0),
                kerning: None,
            },
            cid_encoding: None,
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
    fn width_from_codes_uses_char_code_not_decoded_unicode() {
        use crate::text::extraction_cmap::{FontInfo, FontMetrics};

        // Simple Type1 font with a code-indexed Widths array: code 1 -> 1000,
        // code 2 -> 100. A custom encoding decodes code 1 -> 'm' (U+006D) and
        // code 2 -> 'i' (U+0069), so the decoded Unicode codepoints (109, 105)
        // are far from the codes (1, 2). The advance width MUST come from the
        // codes; indexing the Widths array by the decoded Unicode codepoint
        // reads out-of-range -> missing_width, desyncing glyph advance on
        // custom-encoded fonts (issue #302, Higgs/Computer-Modern scramble).
        let font_info = FontInfo {
            name: "F1".to_string(),
            font_type: "Type1".to_string(),
            encoding: None,
            to_unicode: None,
            differences: None,
            descendant_font: None,
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(1),
                last_char: Some(2),
                widths: Some(vec![1000.0, 100.0]),
                missing_width: Some(500.0),
                kerning: None,
            },
            cid_encoding: None,
        };

        let codes = [1u8, 2u8];
        let decoded = "mi"; // what decode_text produced for these codes
        let width =
            calculate_text_width_from_codes(&codes, decoded, 10.0, Some(&font_info), 0.0, 0.0);
        let expected = (1000.0 + 100.0) / 1000.0 * 10.0; // 11.0
        assert!(
            (width - expected).abs() < 1e-6,
            "width must come from char codes: expected {expected}, got {width}"
        );

        // The decoded-Unicode-indexed path is the bug: 109 and 105 are outside
        // [1,2] so both fall back to missing_width -> (500+500)/1000*10 = 10.0.
        let buggy = calculate_text_width(decoded, 10.0, Some(&font_info));
        assert_eq!(buggy, 10.0);
        assert_ne!(
            width, buggy,
            "code-based width must differ from the Unicode-indexed bug"
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(65), // 'A'
                last_char: Some(90),  // 'Z'
                widths: Some(widths),
                missing_width: Some(500.0),
                kerning: None,
            },
            cid_encoding: None,
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(widths),
                missing_width: Some(600.0),
                kerning: None,
            },
            cid_encoding: None,
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(vec![500.0; 95]),
                missing_width: Some(500.0),
                kerning: None,
            },
            cid_encoding: None,
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(vec![500.0; 95]),
                missing_width: Some(600.0),
                kerning: None,
            },
            cid_encoding: None,
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(65), // 'A'
                last_char: Some(65),  // 'A'
                widths: Some(vec![722.0]),
                missing_width: Some(500.0),
                kerning: None,
            },
            cid_encoding: None,
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(105), // 'i'
                last_char: Some(107),  // covers i, j, k
                widths: Some(proportional_widths),
                missing_width: Some(500.0),
                kerning: None,
            },
            cid_encoding: None,
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(105),
                last_char: Some(107),
                widths: Some(monospace_widths),
                missing_width: Some(600.0),
                kerning: None,
            },
            cid_encoding: None,
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
            cid_ordering: None,
            metrics: FontMetrics {
                first_char: Some(32),
                last_char: Some(126),
                widths: Some(widths),
                missing_width: Some(500.0),
                kerning: Some(kerning),
            },
            cid_encoding: None,
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
        use crate::text::extraction_cmap::parse_truetype_kern_table;

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

        let result = parse_truetype_kern_table(&ttf_data);
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
    fn standard_14_space_width_maps_base_fonts_and_substitutes() {
        // Adobe Core-14 AFM space advances, with subset prefixes stripped and
        // metric-compatible substitutes folded in (#302 symptom 2).
        assert_eq!(super::standard_14_space_width("Times-Roman"), Some(250.0));
        assert_eq!(
            super::standard_14_space_width("Times-BoldItalic"),
            Some(250.0)
        );
        assert_eq!(super::standard_14_space_width("Helvetica"), Some(278.0));
        assert_eq!(super::standard_14_space_width("Courier-Bold"), Some(600.0));
        assert_eq!(super::standard_14_space_width("Symbol"), Some(250.0));
        assert_eq!(super::standard_14_space_width("ZapfDingbats"), Some(278.0));
        // subset prefix stripped
        assert_eq!(
            super::standard_14_space_width("ABCDEF+Times-Roman"),
            Some(250.0)
        );
        // metric-compatible substitutes
        assert_eq!(super::standard_14_space_width("Arial-BoldMT"), Some(278.0));
        assert_eq!(
            super::standard_14_space_width("TimesNewRomanPSMT"),
            Some(250.0)
        );
        assert_eq!(
            super::standard_14_space_width("CourierNewPSMT"),
            Some(600.0)
        );
        // unknown / embedded fonts fall through to the caller's fallback
        assert_eq!(super::standard_14_space_width("Poppins-Regular"), None);
        assert_eq!(super::standard_14_space_width("VUNXGH+Calibri"), None);
    }

    #[test]
    fn merge_into_lines_keeps_emission_order_for_font_switch_overlap() {
        // #302 symptom 1: a font-switched glyph (e.g. the italic particle
        // symbol "Z" in "to the Z boson") is positioned by the producer with
        // an x-origin that falls INSIDE the x-span of the preceding roman run
        // ("to the"). The content stream still delivers it in correct reading
        // order. Sorting a row purely by x-origin interleaves the overlapping
        // fragment, yielding "Zto the" instead of "to theZ". When a row's only
        // backward emission steps are span overlaps (not disjoint jumps),
        // emission order is the authoritative reading order.
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        // emission order = reading order; "Z" overlaps "to t" + "he" in x.
        let row = vec![
            tf("to t", 455.5, 400.0, 12.0, 10.0), // 455.5 .. 467.5
            tf("he", 467.5, 400.0, 10.0, 10.0),   // 467.5 .. 477.5
            tf("Z", 455.3, 400.0, 23.0, 10.0),    // 455.3 .. 478.3 (overlaps both)
        ];
        let lines = extractor.merge_into_lines(&row);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0].text, "to theZ",
            "overlapping font-switch fragment must keep emission (reading) order"
        );
    }

    #[test]
    fn merge_into_lines_keeps_emission_when_run_backfills_covered_span() {
        // #305: dense justified body text is split into sub-word fragments by
        // the font's arbitrary glyph runs. A later word ("described", x 492..537)
        // is emitted with a backward x-origin that lands INSIDE the span already
        // covered by the line ("...selections", 479..521), but does NOT overlap
        // the short immediately-preceding fragment ("s", 517..521). Emission is
        // still the reading order, so the line must keep it — the overlap test
        // has to consider the line's running extent, not just the previous
        // fragment. (Real case: Higgs p5 "kinematic selections described in".)
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        let row = vec![
            tf("selection", 479.0, 400.0, 38.0, 8.0), // 479..517
            tf("s", 517.0, 400.0, 4.0, 8.0),          // 517..521  short predecessor
            tf("d", 492.0, 400.0, 4.0, 8.0),          // 492..496  backfill, no overlap with "s"
            tf("escribed", 496.0, 400.0, 41.0, 8.0),  // 496..537
        ];
        let lines = extractor.merge_into_lines(&row);
        assert_eq!(
            lines[0].text, "selectionsdescribed",
            "a run that backfills the line's covered span must keep emission order"
        );
    }

    #[test]
    fn merge_into_lines_uses_x_order_for_disjoint_backward_jump() {
        // Guard: a genuinely scrambled non-tagged stream (fragments emitted
        // out of x-order at DISJOINT positions, e.g. right-to-left or random
        // generators) must still be reordered by x. Here "the" is emitted
        // after "boson" with no span overlap, so x-order is authoritative.
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        let row = vec![
            tf("boson", 100.0, 400.0, 28.0, 10.0), // 100 .. 128
            tf("the", 80.0, 400.0, 15.0, 10.0),    // 80 .. 95 (disjoint, left of boson)
        ];
        let lines = extractor.merge_into_lines(&row);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0].text, "the boson",
            "disjoint backward emission jump must be reordered by x"
        );
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
    fn merge_close_fragments_superscript_merges_when_reconstruct_paragraphs() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        // Citation superscript: body text at y=400, raised digit at y=403.5
        // (3.5pt above baseline for 10pt font). y_tol = 0.5 * 10 = 5.0 > 3.5
        // and x_gap = 4pt < 10*0.5 = 5pt, so the superscript must merge into
        // the body fragment.
        let frags = vec![
            tf("body-text", 50.0, 400.0, 25.0, 10.0),
            tf("1", 79.0, 403.5, 4.0, 10.0),
        ];
        let merged = extractor.merge_close_fragments(&frags);
        assert_eq!(
            merged.len(),
            1,
            "superscript within 5pt of baseline must merge in reconstruct path"
        );
        assert!(merged[0].text.contains("body-text"));
        assert!(merged[0].text.contains("1"));
    }

    #[test]
    fn merge_close_fragments_superscript_does_not_merge_in_legacy_path() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: false,
            ..Default::default()
        });
        // Legacy path: y_tol=1.0 fixed. A 3.5pt delta must NOT merge.
        let frags = vec![
            tf("body-text", 50.0, 400.0, 25.0, 10.0),
            tf("1", 79.0, 403.5, 4.0, 10.0),
        ];
        let merged = extractor.merge_close_fragments(&frags);
        assert_eq!(
            merged.len(),
            2,
            "3.5pt Y delta exceeds legacy 1.0pt threshold; superscript stays separate"
        );
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

    /// A heading is a different block from the body that follows it, even when
    /// the vertical gap is small enough to look like line spacing. Merging them
    /// destroys the two signals `partition` uses to classify a `Title`
    /// (font-size ratio and bold-short), so the heading text is never
    /// recoverable downstream (issue #436).
    #[test]
    fn merge_into_paragraphs_splits_on_font_size_change() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        // 20pt title at y=760, 10pt body line 40pt below: gap = 30pt, which is
        // exactly the 1.5 * median(20, 10) = 30pt vertical threshold, so only
        // the style change can separate them.
        let lines = vec![
            tf("Section Heading", 72.0, 760.0, 120.0, 20.0),
            tf("Body text of this section.", 72.0, 720.0, 150.0, 10.0),
        ];
        let paragraphs = extractor.merge_into_paragraphs(&lines);
        assert_eq!(
            paragraphs.len(),
            2,
            "font-size change must end the paragraph"
        );
        assert_eq!(paragraphs[0].text, "Section Heading");
        assert_eq!(paragraphs[0].font_size, 20.0);
        assert_eq!(paragraphs[1].text, "Body text of this section.");
    }

    /// Same size, different weight: the classic run-in bold heading. `partition`
    /// classifies it through `bold_short_title`, which needs the heading to
    /// survive extraction as its own fragment (issue #436).
    #[test]
    fn merge_into_paragraphs_splits_on_weight_change() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        let mut heading = tf("Overview", 72.0, 400.0, 60.0, 12.0);
        heading.is_bold = true;
        let lines = vec![heading, tf("Body line.", 72.0, 386.0, 60.0, 12.0)];
        let paragraphs = extractor.merge_into_paragraphs(&lines);
        assert_eq!(paragraphs.len(), 2, "weight change must end the paragraph");
        assert_eq!(paragraphs[0].text, "Overview");
        assert!(paragraphs[0].is_bold);
        assert_eq!(paragraphs[1].text, "Body line.");
    }

    /// Sub-point rounding (11.96pt vs 12pt from a scaled text matrix) is not a
    /// style change: the paragraph must stay whole.
    #[test]
    fn merge_into_paragraphs_tolerates_subpoint_font_size_jitter() {
        let extractor = TextExtractor::with_options(ExtractionOptions {
            reconstruct_paragraphs: true,
            ..Default::default()
        });
        let lines = vec![
            tf("Line one.", 50.0, 400.0, 60.0, 12.0),
            tf("Line two.", 50.0, 386.0, 60.0, 11.96),
        ];
        let paragraphs = extractor.merge_into_paragraphs(&lines);
        assert_eq!(
            paragraphs.len(),
            1,
            "0.3% size jitter is not a style change"
        );
        assert_eq!(paragraphs[0].text, "Line one.\nLine two.");
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

    #[test]
    fn sort_and_merge_fragments_nan_y_does_not_swallow_other_lines() {
        // A fragment with a non-finite Y (reachable from a degenerate text
        // matrix in a malformed PDF) must not chain every remaining fragment
        // into one pseudo-line. The tolerance filter compares with `< tol`; a
        // `>= tol` phrasing would let a NaN anchor never terminate the line,
        // collapsing the whole page into a single X-sorted "line".
        let extractor = TextExtractor::with_options(ExtractionOptions::default());

        // Four well-separated lines whose X order is the reverse of their Y
        // (reading) order: if the NaN anchor swallows the rest, they get
        // re-sorted purely by X into D,C,B,A instead of the reading order.
        let mut fragments = vec![
            tf("A", 400.0, f64::NAN, 10.0, 12.0),
            tf("B", 300.0, 500.0, 10.0, 12.0),
            tf("C", 200.0, 300.0, 10.0, 12.0),
            tf("D", 100.0, 100.0, 10.0, 12.0),
        ];
        extractor.sort_and_merge_fragments(&mut fragments);

        let order: Vec<&str> = fragments.iter().map(|f| f.text.as_str()).collect();
        assert_eq!(
            order,
            vec!["A", "B", "C", "D"],
            "NaN-Y fragment must stay its own line; the finite lines keep \
             top-to-bottom reading order instead of collapsing to X order"
        );
    }
}
