use crate::graphics::extraction::ExtractedGraphics;
use crate::pipeline::reading_order::{ReadingOrder, SimpleReadingOrder, XYCutReadingOrder};
use crate::pipeline::{
    Element, ElementBBox, ElementData, ElementMetadata, KeyValueElementData, TableElementData,
};
use crate::text::extraction::TextFragment;

/// Strategy for ordering text fragments before classification.
#[derive(Debug, Clone, Default)]
pub enum ReadingOrderStrategy {
    /// Simple top-to-bottom, left-to-right (default, line-threshold 5.0).
    #[default]
    Simple,
    /// XY-Cut recursive algorithm. Handles multi-column layouts correctly.
    XYCut { min_gap: f64 },
    /// No reordering — preserve input order as-is.
    None,
}

/// Configuration for the document partitioner.
#[derive(Debug, Clone)]
pub struct PartitionConfig {
    /// Whether to detect table structures.
    pub detect_tables: bool,
    /// Whether to detect headers and footers by position.
    pub detect_headers_footers: bool,
    /// Minimum font size ratio vs median to classify as title.
    /// A fragment with font_size >= median * ratio is considered a title.
    pub title_min_font_ratio: f64,
    /// Y-position threshold for header detection (fraction of page height from top).
    /// Fragments above `page_height * (1 - header_zone)` are header candidates.
    pub header_zone: f64,
    /// Y-position threshold for footer detection (fraction of page height from bottom).
    /// Fragments below `page_height * footer_zone` are footer candidates.
    pub footer_zone: f64,
    /// Reading order strategy applied to fragments before classification.
    pub reading_order: ReadingOrderStrategy,
    /// Minimum confidence score required for a detected table to be accepted.
    /// Tables whose confidence is below this value are discarded and their
    /// fragments fall through to the prose classification steps.
    /// Range: `[0.0, 1.0]`. Default: `0.5`.
    pub min_table_confidence: f64,
    /// Prefer the ruling-based (vector-grid) table detector for bordered tables,
    /// falling back to the spatial detector for the rest. When false, only the
    /// spatial detector runs and no page graphics are extracted. Default: true.
    pub prefer_ruling_tables: bool,
}

impl Default for PartitionConfig {
    fn default() -> Self {
        Self {
            detect_tables: true,
            detect_headers_footers: true,
            title_min_font_ratio: 1.3,
            header_zone: 0.05,
            footer_zone: 0.05,
            reading_order: ReadingOrderStrategy::Simple,
            min_table_confidence: 0.5,
            prefer_ruling_tables: true,
        }
    }
}

impl PartitionConfig {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the minimum font size ratio for title detection.
    pub fn with_title_min_font_ratio(mut self, ratio: f64) -> Self {
        self.title_min_font_ratio = ratio;
        self
    }

    /// Disable table detection.
    pub fn without_tables(mut self) -> Self {
        self.detect_tables = false;
        self
    }

    /// Disable header/footer detection.
    pub fn without_headers_footers(mut self) -> Self {
        self.detect_headers_footers = false;
        self
    }

    /// Set the reading order strategy applied before fragment classification.
    pub fn with_reading_order(mut self, strategy: ReadingOrderStrategy) -> Self {
        self.reading_order = strategy;
        self
    }

    /// Set the minimum confidence threshold for table detection.
    ///
    /// Tables whose `confidence` score is below `threshold` are discarded and
    /// their fragments flow through to the prose classification steps.
    /// Use `0.0` to accept every detection; `1.0` to reject all.
    pub fn with_min_table_confidence(mut self, threshold: f64) -> Self {
        self.min_table_confidence = threshold;
        self
    }
}

/// Classifies text fragments into typed [`Element`]s.
///
/// The partitioner applies heuristics in this order:
/// 1. Header/footer detection by Y-position
/// 2. Table detection (via `StructuredDataDetector`)
/// 3. Key-value pair detection (via colon/separator patterns)
/// 4. Title detection by font size ratio
/// 5. List item detection by bullet/number prefixes
/// 6. Remaining fragments become Paragraphs
pub struct Partitioner {
    config: PartitionConfig,
}

impl Partitioner {
    pub fn new(config: PartitionConfig) -> Self {
        Self { config }
    }

    /// Partition a page's text fragments into typed elements.
    ///
    /// * `fragments` — text fragments from one page (with `preserve_layout`)
    /// * `page` — 0-indexed page number
    /// * `page_height` — page height in PDF points (for header/footer zones)
    ///
    /// Partition fragments without page graphics (spatial table detection only).
    pub fn partition_fragments(
        &self,
        fragments: &[TextFragment],
        page: u32,
        page_height: f64,
    ) -> Vec<Element> {
        self.partition_fragments_with_graphics(fragments, None, page, page_height)
    }

    /// Partition a page's text fragments into typed elements, optionally using
    /// page graphics (vector lines) for ruling-based table detection.
    ///
    /// * `fragments` — text fragments from one page (with `preserve_layout`)
    /// * `graphics` — extracted vector lines for the page, if available
    /// * `page` — 0-indexed page number
    /// * `page_height` — page height in PDF points (for header/footer zones)
    pub fn partition_fragments_with_graphics(
        &self,
        fragments: &[TextFragment],
        graphics: Option<&ExtractedGraphics>,
        page: u32,
        page_height: f64,
    ) -> Vec<Element> {
        self.partition_fragments_with_graphics_raw(fragments, None, graphics, page, page_height)
    }

    /// Partition a page's text fragments, using a separate **raw** (un-reconstructed)
    /// fragment set for ruling-based table cell assignment.
    ///
    /// The partition pipeline (`PdfDocument::partition`) extracts text with
    /// `reconstruct_paragraphs: true`, which merges per-cell fragments into
    /// paragraph-granular fragments. The ruling-based table detector needs
    /// **cell-granular** fragments to assign text to grid cells, so this method
    /// accepts `raw_fragments` (extracted with `reconstruct_paragraphs: false`)
    /// for that purpose while `fragments` (reconstructed) drives prose
    /// classification (titles, headers, paragraphs) and fragment claiming.
    ///
    /// * `fragments` — reconstructed fragments used for classification + claiming
    /// * `raw_fragments` — cell-granular fragments for the ruling detector; when
    ///   `None`, `fragments` is used for ruling too (legacy/unit-test behavior)
    /// * `graphics` — extracted vector lines for the page, if available
    /// * `page` — 0-indexed page number
    /// * `page_height` — page height in PDF points (for header/footer zones)
    ///
    /// Internal to the partition pipeline (`do_partition_pages` supplies the raw
    /// fragments); the public entry points are `partition_fragments` and
    /// `partition_fragments_with_graphics`.
    pub(crate) fn partition_fragments_with_graphics_raw(
        &self,
        fragments: &[TextFragment],
        raw_fragments: Option<&[TextFragment]>,
        graphics: Option<&ExtractedGraphics>,
        page: u32,
        page_height: f64,
    ) -> Vec<Element> {
        if fragments.is_empty() {
            return Vec::new();
        }
        // Fragments fed to the ruling detector for cell-text assignment.
        let ruling_fragments = raw_fragments.unwrap_or(fragments);

        // Apply reading order strategy to fragments BEFORE classification
        let fragments: std::borrow::Cow<[TextFragment]> = match &self.config.reading_order {
            ReadingOrderStrategy::Simple => {
                let mut ordered = fragments.to_vec();
                SimpleReadingOrder::default().order(&mut ordered);
                std::borrow::Cow::Owned(ordered)
            }
            ReadingOrderStrategy::XYCut { min_gap } => {
                let mut ordered = fragments.to_vec();
                XYCutReadingOrder::new(*min_gap).order(&mut ordered);
                std::borrow::Cow::Owned(ordered)
            }
            ReadingOrderStrategy::None => std::borrow::Cow::Borrowed(fragments),
        };
        let fragments = fragments.as_ref();

        // Track which fragments have been claimed
        let mut claimed = vec![false; fragments.len()];
        let mut elements = Vec::new();

        // 0. Struct-tag-driven classification (issue #271).
        //    Consumes `TextFragment.struct_tag` populated by #269 Phase 1.
        //    Heading/ListItem tags are accepted here with confidence 1.0
        //    (structural ground truth); Artifact and pass-through tags
        //    (P, Span, Figure, Table, ...) fall through to the heuristics.
        for (i, f) in fragments.iter().enumerate() {
            if claimed[i] {
                continue;
            }
            let Some(tag) = f.struct_tag.as_deref() else {
                continue;
            };
            match classify_by_struct_tag(tag) {
                Some(StructTagClass::Heading) => {
                    let trimmed = f.text.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let mut meta = meta_from_fragment(f, page);
                    meta.confidence = 1.0;
                    elements.push(Element::Title(ElementData {
                        text: trimmed.to_string(),
                        metadata: meta,
                    }));
                    claimed[i] = true;
                }
                Some(StructTagClass::ListItem) => {
                    let trimmed = f.text.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let mut meta = meta_from_fragment(f, page);
                    meta.confidence = 1.0;
                    elements.push(Element::ListItem(ElementData {
                        text: trimmed.to_string(),
                        metadata: meta,
                    }));
                    claimed[i] = true;
                }
                Some(StructTagClass::List) | Some(StructTagClass::Artifact) | None => {
                    // L is an outer container — children are LI; nothing to do here.
                    // Artifact flows through to Header/Footer detection below.
                    // None: pass through.
                }
            }
        }

        // 1. Header/footer detection (issue #271: length cap + body-tag gate).
        if self.config.detect_headers_footers && page_height > 0.0 {
            let header_threshold = page_height * (1.0 - self.config.header_zone);
            let footer_threshold = page_height * self.config.footer_zone;

            for (i, f) in fragments.iter().enumerate() {
                if claimed[i] {
                    continue;
                }
                let text_too_long = f.text.chars().count() > MAX_HEADER_TEXT_LEN;
                let is_body_tagged = struct_tag_is_body(&f.struct_tag);

                if f.y >= header_threshold && !text_too_long && !is_body_tagged {
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
                } else if f.y + f.height <= footer_threshold && !text_too_long && !is_body_tagged {
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
            }
        }

        // 2. Table detection via StructuredDataDetector
        //
        // Improvements over the naive single-batch approach:
        // a) Fragment space is split into Y-separated regions so that two tables
        //    on the same page are detected independently rather than fused.
        // b) List-like regions (short left column + wide right column) are skipped
        //    before calling the detector, so numbered lists are not misclassified.
        // c) Detected tables whose confidence is below `min_table_confidence` are
        //    discarded and their fragments fall through to prose classification.
        if self.config.detect_tables {
            // Ruling-first: when the page has a drawn table grid, detect bordered
            // tables from vector lines and claim their fragments so the spatial
            // pass below only sees the remainder. region_looks_like_list is NOT
            // applied here — drawn borders are strong table evidence.
            if self.config.prefer_ruling_tables {
                if let Some(graphics) = graphics {
                    if graphics.has_table_structure() {
                        let detector = crate::text::table_detection::TableDetector::default();
                        if let Ok(tables) = detector.detect(graphics, ruling_fragments) {
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
                                let (rr, rt) = (
                                    table.bbox.x + table.bbox.width,
                                    table.bbox.y + table.bbox.height,
                                );
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

            let unclaimed_frags: Vec<&TextFragment> = fragments
                .iter()
                .enumerate()
                .filter(|(i, _)| !claimed[*i])
                .map(|(_, f)| f)
                .collect();

            let detector = crate::text::structured::StructuredDataDetector::new(Default::default());

            let regions = segment_into_table_regions(&unclaimed_frags, 2.0);

            for region in &regions {
                // Skip regions that look like numbered/bulleted lists.
                if region_looks_like_list(region) {
                    continue;
                }

                let region_owned: Vec<TextFragment> = region.iter().map(|f| (*f).clone()).collect();

                if let Ok(result) = detector.detect(&region_owned) {
                    for table in &result.tables {
                        // Apply minimum confidence filter.
                        if table.confidence < self.config.min_table_confidence {
                            continue;
                        }

                        let rows: Vec<Vec<String>> = table
                            .rows
                            .iter()
                            .map(|row| row.cells.iter().map(|c| c.text.clone()).collect())
                            .collect();

                        let bbox = ElementBBox::new(
                            table.bounding_box.x,
                            table.bounding_box.y,
                            table.bounding_box.width,
                            table.bounding_box.height,
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

                        // Claim fragments that fall within this table's bounding box.
                        for (i, f) in fragments.iter().enumerate() {
                            if !claimed[i]
                                && f.x >= table.bounding_box.x - 1.0
                                && f.x <= table.bounding_box.right() + 1.0
                                && f.y >= table.bounding_box.y - 1.0
                                && f.y <= table.bounding_box.top() + 1.0
                            {
                                claimed[i] = true;
                            }
                        }
                    }
                }
            }
        }

        // Compute body font size (most frequent) from unclaimed fragments for title detection
        let body_font_size = {
            let sizes: Vec<f64> = fragments
                .iter()
                .enumerate()
                .filter(|(i, _)| !claimed[*i])
                .map(|(_, f)| f.font_size)
                .filter(|s| *s > 0.0)
                .collect();
            if sizes.is_empty() {
                12.0
            } else {
                // Find mode (most frequent font size) — quantize to 0.5pt to handle floating point
                let mut freq = std::collections::HashMap::new();
                for s in &sizes {
                    let key = (*s * 2.0).round() as i64;
                    *freq.entry(key).or_insert(0usize) += 1;
                }
                // When frequencies are tied, prefer the smaller font size
                // (body text is typically smaller than headings)
                let mode_key = freq
                    .into_iter()
                    .max_by(|(key_a, count_a), (key_b, count_b)| {
                        count_a.cmp(count_b).then(key_b.cmp(key_a))
                    })
                    .map(|(key, _)| key)
                    .unwrap_or(24);
                mode_key as f64 / 2.0
            }
        };

        let title_threshold = body_font_size * self.config.title_min_font_ratio;

        // 3-6. Classify remaining fragments
        for (i, f) in fragments.iter().enumerate() {
            if claimed[i] {
                continue;
            }

            let meta = meta_from_fragment(f, page);
            let text = f.text.trim();
            if text.is_empty() {
                continue;
            }

            // 3. Key-value detection: "Key: Value" pattern
            if let Some(colon_pos) = text.find(':') {
                let key = text[..colon_pos].trim();
                let value = text[colon_pos + 1..].trim();
                // Heuristic: key must be a short label (max 4 words, < 40 chars),
                // non-empty with non-empty value, no periods, and no prose-like prefixes
                // that indicate a sentence structure rather than a KV pair.
                let key_word_count = key.split_whitespace().count();
                if !key.is_empty()
                    && !value.is_empty()
                    && key.len() < 40
                    && key_word_count <= 4
                    && !key.contains('.')
                    && !is_prose_prefix(key)
                {
                    let kv_confidence = compute_kv_confidence(key);
                    let mut meta = meta;
                    meta.confidence = kv_confidence;
                    elements.push(Element::KeyValue(KeyValueElementData {
                        key: key.to_string(),
                        value: value.to_string(),
                        metadata: meta,
                    }));
                    continue;
                }
            }

            // 4. Title detection — three OR'd signals (issue #271).
            //    struct_tag=P/Span suppresses the weaker bold-short heuristic
            //    (too easy to misfire on emphasized words inside paragraphs).
            //    Font-ratio and numeric-prefix heuristics still fire under P:
            //    real-world tagged PDFs (notably NCSC CAF v4.0) ship sub-section
            //    headings like "A2.a Risk Management Process" tagged as P;
            //    numeric_prefix_title's shape + capitalization guard is strict
            //    enough to win over the (mis)tag.
            let p_or_span = matches!(f.struct_tag.as_deref(), Some("P") | Some("Span"));

            let mut is_title = false;
            let mut title_confidence = 0.0_f64;

            // 4a. Font-size ratio
            if f.font_size >= title_threshold && f.font_size > body_font_size {
                let ratio = f.font_size / body_font_size;
                is_title = true;
                title_confidence = title_confidence.max(compute_title_confidence(
                    ratio,
                    self.config.title_min_font_ratio,
                ));
            }

            // 4b. Bold-short heuristic — suppressed by P/Span.
            if !p_or_span && bold_short_title(f) {
                is_title = true;
                title_confidence = title_confidence.max(0.7);
            }

            // 4c. Numeric-prefix heuristic — fires even under P (strict pattern).
            if numeric_prefix_title(f) {
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

            // 5. List item detection
            if is_list_item(text) {
                elements.push(Element::ListItem(ElementData {
                    text: text.to_string(),
                    metadata: meta,
                }));
                continue;
            }

            // 6. Default: Paragraph
            elements.push(Element::Paragraph(ElementData {
                text: text.to_string(),
                metadata: meta,
            }));
        }

        // Post-classification sort: within-page order was established by pre-sort.
        // Only sort by page to maintain multi-page document order.
        match &self.config.reading_order {
            ReadingOrderStrategy::None => {}
            _ => {
                elements.sort_by_key(|e| e.page());
            }
        }

        // Post-classification relationship pass: assign parent_heading
        let mut current_heading: Option<String> = None;
        for element in &mut elements {
            if matches!(element, Element::Title(_)) {
                current_heading = Some(element.text().to_string());
            }
            element.set_parent_heading(current_heading.clone());
        }

        elements
    }
}

/// Check if text before a colon looks like a prose phrase rather than a label.
/// Prose prefixes contain verbs or conjunctions that indicate sentence structure.
fn is_prose_prefix(key: &str) -> bool {
    let lower = key.to_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();

    // Common prose patterns: "As noted in the report", "The document states", etc.
    const PROSE_INDICATORS: &[&str] = &[
        "as",
        "the",
        "this",
        "that",
        "these",
        "those",
        "it",
        "is",
        "was",
        "were",
        "has",
        "have",
        "had",
        "will",
        "would",
        "should",
        "could",
        "may",
        "might",
        "shall",
        "can",
        "do",
        "does",
        "did",
        "being",
        "been",
        "are",
        "for",
        "with",
        "from",
        "into",
        "about",
        "after",
        "before",
        "during",
        "between",
        "through",
        "however",
        "therefore",
        "furthermore",
        "moreover",
        "although",
        "because",
        "since",
        "while",
        "when",
        "where",
        "which",
        "who",
        "whom",
        "whose",
        "according",
    ];

    // If the first word is a common prose starter, it's likely a sentence, not a label
    if let Some(first) = words.first() {
        if PROSE_INDICATORS.contains(first) {
            return true;
        }
    }

    // If any word (beyond first) is a verb/conjunction, likely prose
    if words.len() > 2 {
        for word in &words[1..] {
            if PROSE_INDICATORS.contains(word) {
                return true;
            }
        }
    }

    false
}

/// Check if text looks like a list item (bullet or numbered).
fn is_list_item(text: &str) -> bool {
    let trimmed = text.trim_start();
    // Bullet patterns: "- ", "• ", "* ", "– ", "— "
    if trimmed.starts_with("- ")
        || trimmed.starts_with("• ")
        || trimmed.starts_with("* ")
        || trimmed.starts_with("– ")
        || trimmed.starts_with("— ")
    {
        return true;
    }
    // Numbered: "1. ", "2) ", "a. ", "a) " etc.
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 3 {
        let first = bytes[0];
        let second = bytes[1];
        let third = bytes[2];
        if (first.is_ascii_digit() || first.is_ascii_lowercase())
            && (second == b'.' || second == b')')
            && third == b' '
        {
            return true;
        }
        // Multi-digit: "10. ", "11) "
        if bytes.len() >= 4
            && first.is_ascii_digit()
            && second.is_ascii_digit()
            && (bytes[2] == b'.' || bytes[2] == b')')
            && bytes[3] == b' '
        {
            return true;
        }
    }
    false
}

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

/// Splits unclaimed fragments into Y-separated table candidate regions.
///
/// Algorithm:
/// 1. Sort fragments by Y descending (top-to-bottom in PDF coordinates, where
///    higher Y values are closer to the top of the page).
/// 2. Compute the median line height across all fragments.
/// 3. Start a new region when the Y-gap between consecutive fragments exceeds
///    `median_line_height * gap_multiplier`.
/// 4. Return only regions with at least 4 fragments (minimum for meaningful
///    table detection).
fn segment_into_table_regions<'a>(
    fragments: &[&'a TextFragment],
    gap_multiplier: f64,
) -> Vec<Vec<&'a TextFragment>> {
    if fragments.is_empty() {
        return Vec::new();
    }

    // Sort a copy by Y descending (higher Y = higher on page in PDF coords).
    let mut sorted: Vec<&TextFragment> = fragments.to_vec();
    sorted.sort_by(|a, b| b.y.total_cmp(&a.y));

    // Compute median line height.
    let mut heights: Vec<f64> = sorted
        .iter()
        .map(|f| f.height)
        .filter(|h| *h > 0.0)
        .collect();
    let median_height = if heights.is_empty() {
        12.0
    } else {
        heights.sort_by(f64::total_cmp);
        let mid = heights.len() / 2;
        if heights.len() % 2 == 0 {
            (heights[mid - 1] + heights[mid]) / 2.0
        } else {
            heights[mid]
        }
    };

    let gap_threshold = median_height * gap_multiplier;

    // Build regions by splitting on large Y gaps.
    let mut regions: Vec<Vec<&TextFragment>> = Vec::new();
    let mut current_region: Vec<&TextFragment> = Vec::new();

    for frag in &sorted {
        if let Some(prev) = current_region.last() {
            // In PDF coordinates Y increases upward. After descending sort,
            // `prev.y` >= `frag.y`. The gap between the bottom of `prev`
            // (prev.y) and the top of `frag` (frag.y + frag.height) gives the
            // vertical whitespace. We compare the difference in Y positions
            // directly because fragment Y marks the baseline / bottom-left corner.
            let gap = prev.y - (frag.y + frag.height);
            if gap > gap_threshold {
                if current_region.len() >= 4 {
                    regions.push(current_region);
                }
                current_region = Vec::new();
            }
        }
        current_region.push(frag);
    }

    if current_region.len() >= 4 {
        regions.push(current_region);
    }

    regions
}

/// Returns `true` when a table candidate region looks like a numbered or
/// bulleted list rather than a genuine data table.
///
/// Heuristic: if there are exactly 2 X-position clusters and the left cluster
/// contains fragments with an average length of at most 3 characters, the
/// region is treated as a list (e.g., "1.", "2.", "-", "•", "a)").
fn region_looks_like_list(fragments: &[&TextFragment]) -> bool {
    if fragments.is_empty() {
        return false;
    }

    // Cluster X positions with a 15pt tolerance (wide enough for minor jitter).
    let tolerance = 15.0;
    let mut x_clusters: Vec<f64> = Vec::new();
    for frag in fragments {
        let x = frag.x;
        let found = x_clusters.iter().any(|&cx| (cx - x).abs() <= tolerance);
        if !found {
            x_clusters.push(x);
        }
    }

    // Only trigger on exactly 2-column layouts.
    if x_clusters.len() != 2 {
        return false;
    }

    // Sort clusters: left cluster first.
    x_clusters.sort_by(f64::total_cmp);
    let left_x = x_clusters[0];

    // Measure average text length for fragments in the left column.
    let left_frags: Vec<&TextFragment> = fragments
        .iter()
        .filter(|f| (f.x - left_x).abs() <= tolerance)
        .copied()
        .collect();

    if left_frags.is_empty() {
        return false;
    }

    let avg_left_len = left_frags
        .iter()
        .map(|f| f.text.trim().chars().count())
        .sum::<usize>() as f64
        / left_frags.len() as f64;

    // A left column averaging <= 3 chars is a bullet/number column.
    avg_left_len <= 3.0
}

fn meta_from_fragment(f: &TextFragment, page: u32) -> ElementMetadata {
    ElementMetadata {
        page,
        bbox: ElementBBox::new(f.x, f.y, f.width, f.height),
        confidence: 1.0,
        font_name: f.font_name.clone(),
        font_size: Some(f.font_size),
        is_bold: f.is_bold,
        is_italic: f.is_italic,
        parent_heading: None,
    }
}

// --- Confidence computation functions ---

/// Title confidence: maps `[min_ratio, 2*min_ratio]` → `[0.5, 1.0]`.
/// At exactly `min_ratio` → 0.5. At `2*min_ratio` or above → 1.0.
fn compute_title_confidence(actual_ratio: f64, min_ratio: f64) -> f64 {
    if min_ratio <= 0.0 {
        return 1.0;
    }
    (0.5 + 0.5 * (actual_ratio - min_ratio) / min_ratio).clamp(0.5, 1.0)
}

/// Header/footer zone confidence: `clamp(distance / zone_size, 0.5, 1.0)`
fn compute_zone_confidence(distance: f64, zone_size: f64) -> f64 {
    if zone_size <= 0.0 {
        return 0.5;
    }
    (distance / zone_size).clamp(0.5, 1.0)
}

/// KV confidence: penalizes long keys and multi-word keys.
fn compute_kv_confidence(key: &str) -> f64 {
    let len_penalty = key.len() as f64 / 40.0;
    let word_count = key.split_whitespace().count();
    let word_penalty = if word_count > 2 {
        0.1 * (word_count - 2) as f64
    } else {
        0.0
    };
    (1.0 - len_penalty - word_penalty).clamp(0.5, 1.0)
}

// --- struct_tag-driven classification (issue #271) ---

const MAX_HEADER_TEXT_LEN: usize = 100;
const MAX_BOLD_TITLE_LEN: usize = 120;
const MAX_NUMERIC_TITLE_LEN: usize = 120;
const MAX_NUMERIC_TITLE_WORDS: usize = 14;

/// `true` when the trimmed text ends with `.`, `!`, or `?`. Used by the
/// bold-short heading heuristic to exclude complete sentences.
fn ends_with_sentence_terminator(s: &str) -> bool {
    matches!(s.chars().last(), Some('.') | Some('!') | Some('?'))
}

/// Heading heuristic: `true` if the fragment is bold, has non-empty trimmed
/// text up to `MAX_BOLD_TITLE_LEN` chars, and does not end with a sentence
/// terminator. Designed for headings emitted at body font size without
/// structural tagging (e.g. NCSC `"A2.a Risk Management Process"` in bold).
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

/// Section-prefix regex matching common heading shapes:
/// - `A2.a Foo`, `A1.b Bar` (uppercase letter + digits + optional `.digit*` + optional lowercase letter)
/// - `1.1 Foo`, `3.2.1 Bar`, `1. Foo` (digits + optional `.digit*` + optional `.`)
/// - `Section 4: Foo`, `Chapter 7 Foo`
/// - `IV. Findings` (uppercase Roman numerals followed by `.`)
fn section_prefix_regex() -> &'static regex::Regex {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"^([A-Z]\d+(\.\d+)*(\.[a-z]\.?)?|\d+(\.\d+)*\.?|Section\s+\d+:?|Chapter\s+\d+:?|[IVX]+\.)\s+",
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

/// Heading heuristic: `true` if the trimmed text begins with a recognized
/// section prefix (`A2.a`, `1.1`, `Section 4:`, `IV.`) AND the next word
/// after the prefix begins with an uppercase letter.
///
/// The uppercase guard rejects measurement strings (`"1.2 million users"`)
/// and instruction items (`"1. take action"`) that match the prefix but
/// are not headings.
///
/// Additional guards: real headings rarely contain commas (sentences do)
/// and rarely exceed [`MAX_NUMERIC_TITLE_LEN`] chars; an ordered list item
/// that starts with `"N. La ..."` followed by 100+ chars of prose is
/// almost certainly body text in a Romance-language document, not a heading.
fn numeric_prefix_title(f: &TextFragment) -> bool {
    let trimmed = f.text.trim();
    let char_count = trimmed.chars().count();
    if char_count == 0 || char_count > MAX_NUMERIC_TITLE_LEN {
        return false;
    }
    if !matches_section_prefix(trimmed) {
        return false;
    }
    // List-marker discriminator: a flat single-level numbered marker
    // ("1. ", "10) ") is an ordered-list item, never a heading. Real
    // headings carry multi-level ("3.1", "4.1.1"), lettered ("A2.a"),
    // "Section N"/"Chapter N", or roman ("IV.") prefixes — none of which
    // `is_list_item` recognizes. Yielding to ListItem here keeps Title and
    // ListItem mutually exclusive on the ambiguous bare-integer prefix.
    if is_list_item(trimmed) {
        return false;
    }
    let rest = strip_section_prefix(trimmed).trim_start();
    if !matches!(rest.chars().next(), Some(c) if c.is_uppercase()) {
        return false;
    }
    // Sentence-body discriminator: comma presence almost always indicates
    // prose rather than a heading. Tradeoff: rare headings with commas
    // ("Section 1: Foundations, Architecture, and Design") are missed,
    // but the false-positive rate on Spanish/French ordered lists is much
    // worse without this guard.
    if trimmed.contains(',') {
        return false;
    }
    // Word-count guard: numbered headings in Spanish/legal text are short
    // (typically <= 14 words). Numbered list items that form full
    // sentences run much longer ("1. La vigilancia continua permitira la
    // deteccion de actividades o comportamientos anomalos y su oportuna
    // respuesta." = 17 words).
    if trimmed.split_whitespace().count() > MAX_NUMERIC_TITLE_WORDS {
        return false;
    }
    true
}

/// Classification class implied by a PDF structural tag (`/H1`, `/L`, `/Artifact`, ...).
///
/// Returned by [`classify_by_struct_tag`] when the tag carries an unambiguous
/// document-role signal. Pass-through tags (`/P`, `/Span`, ...) return `None`.
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
/// pass-through tags (`P`, `Span`, `Figure`, ...) so the fragment flows
/// into the heuristic classifier.
fn classify_by_struct_tag(tag: &str) -> Option<StructTagClass> {
    match tag {
        "H" | "H1" | "H2" | "H3" | "H4" | "H5" | "H6" | "Title" => Some(StructTagClass::Heading),
        "L" => Some(StructTagClass::List),
        "LI" | "Lbl" | "LBody" => Some(StructTagClass::ListItem),
        "Artifact" => Some(StructTagClass::Artifact),
        _ => None,
    }
}

/// `true` when the struct tag indicates body content (paragraph, span, heading,
/// list item). Used by Header/Footer detection to skip claiming fragments whose
/// author has already declared them as body. `None` (no tag) returns `false`
/// because absence of evidence does not imply body; `Artifact` returns `false`
/// because artifacts ARE page furniture.
fn struct_tag_is_body(tag: &Option<String>) -> bool {
    let Some(t) = tag.as_deref() else {
        return false;
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_by_struct_tag_recognizes_heading_tags() {
        for tag in &["H", "H1", "H2", "H3", "H4", "H5", "H6", "Title"] {
            assert_eq!(
                classify_by_struct_tag(tag),
                Some(StructTagClass::Heading),
                "tag {tag} should classify as Heading"
            );
        }
    }

    #[test]
    fn classify_by_struct_tag_recognizes_list_tags() {
        assert_eq!(classify_by_struct_tag("L"), Some(StructTagClass::List));
        assert_eq!(classify_by_struct_tag("LI"), Some(StructTagClass::ListItem));
        assert_eq!(
            classify_by_struct_tag("Lbl"),
            Some(StructTagClass::ListItem)
        );
        assert_eq!(
            classify_by_struct_tag("LBody"),
            Some(StructTagClass::ListItem)
        );
    }

    #[test]
    fn classify_by_struct_tag_recognizes_artifact() {
        assert_eq!(
            classify_by_struct_tag("Artifact"),
            Some(StructTagClass::Artifact)
        );
    }

    #[test]
    fn classify_by_struct_tag_returns_none_for_passthrough_tags() {
        for tag in &[
            "P", "Span", "Figure", "Table", "Caption", "Form", "Note", "Random",
        ] {
            assert_eq!(
                classify_by_struct_tag(tag),
                None,
                "tag {tag} should be None (fall through)"
            );
        }
    }

    #[test]
    fn struct_tag_is_body_recognizes_body_tags() {
        for tag in &[
            "P", "Span", "L", "LI", "H1", "H2", "H6", "Title", "Lbl", "LBody",
        ] {
            assert!(
                struct_tag_is_body(&Some(tag.to_string())),
                "tag {tag} should be body"
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
        assert!(!bold_short_title(&frag(
            "This is a complete sentence.",
            true,
            12.0
        )));
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
                "should match: {c}"
            );
        }
    }

    #[test]
    fn numeric_prefix_title_rejects_money_amount() {
        assert!(!numeric_prefix_title(&frag(
            "1.2 million users were affected",
            false,
            12.0
        )));
    }

    #[test]
    fn numeric_prefix_title_rejects_version_string() {
        assert!(!numeric_prefix_title(&frag(
            "version 3.0.1 release notes",
            false,
            12.0
        )));
    }

    #[test]
    fn numeric_prefix_title_rejects_lowercase_continuation() {
        assert!(!numeric_prefix_title(&frag(
            "1. take action now",
            false,
            12.0
        )));
    }

    #[test]
    fn numeric_prefix_title_rejects_flat_numbered_list_marker() {
        // Single-level "N. Word" / "N) Word" is an ordered-list item, not a
        // heading — it must yield to is_list_item (regression for #271 / PR #276).
        for c in &["1. First item", "2. Second item", "10) Tenth item"] {
            assert!(
                !numeric_prefix_title(&frag(c, false, 12.0)),
                "flat numbered list marker must not be a Title: {c}"
            );
        }
        // Multi-level and lettered prefixes remain headings.
        for c in &["1.1 Overview", "A2.a Risk Management Process"] {
            assert!(
                numeric_prefix_title(&frag(c, false, 12.0)),
                "multi-level/lettered prefix must remain a Title: {c}"
            );
        }
    }

    #[test]
    fn numeric_prefix_title_rejects_text_without_prefix() {
        assert!(!numeric_prefix_title(&frag(
            "Overview of the system",
            false,
            12.0
        )));
    }

    #[test]
    fn numeric_prefix_title_rejects_too_long() {
        let mut s = String::from("A2.a ");
        s.push_str(&"X".repeat(220));
        assert!(!numeric_prefix_title(&frag(&s, false, 12.0)));
    }

    #[test]
    fn numeric_prefix_title_rejects_text_with_comma() {
        // Spanish/French ordered list items: "1. La vigilancia, la deteccion, ..."
        // are body sentences, not headings.
        assert!(!numeric_prefix_title(&frag(
            "1. La vigilancia continua permite detectar amenazas, vulnerabilidades y errores.",
            false,
            12.0,
        )));
    }

    #[test]
    fn numeric_prefix_title_rejects_long_sentence_without_comma() {
        // Spanish numbered paragraph without commas but with many words:
        // 17 words, no comma, ends in period. Must NOT be classified as Title.
        assert!(!numeric_prefix_title(&frag(
            "1. La vigilancia continua permitira la deteccion de actividades o comportamientos anomalos y su oportuna respuesta.",
            false,
            12.0,
        )));
    }

    #[test]
    fn numeric_prefix_title_rejects_just_over_max_len() {
        // Heading-like prefix but text length 121 chars > MAX_NUMERIC_TITLE_LEN.
        let mut s = String::from("A2.a Risk Management ");
        s.push_str(&"X".repeat(120));
        assert!(!numeric_prefix_title(&frag(&s, false, 12.0)));
    }

    // --- Partitioner-level integration tests (issue #271 wiring) ---

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

    /// Narrow fragment (width 10) whose center sits at (x+5, y) — for placing
    /// text unambiguously inside a single grid cell.
    fn cell_frag(text: &str, x: f64, y: f64) -> TextFragment {
        let mut f = frag_at(text, x, y, 8.0);
        f.width = 10.0;
        f
    }

    #[test]
    fn raw_fragments_drive_cell_text_while_reconstructed_drive_claiming() {
        use crate::graphics::extraction::{ExtractedGraphics, VectorLine};

        // 2x2 grid: x in {100,200,300}, y in {100,150,200}.
        let mut graphics = ExtractedGraphics::new();
        for y in [100.0, 150.0, 200.0] {
            graphics.add_line(VectorLine::new(100.0, y, 300.0, y, 1.0, true, None));
        }
        for x in [100.0, 200.0, 300.0] {
            graphics.add_line(VectorLine::new(x, 100.0, x, 200.0, 1.0, true, None));
        }
        assert!(graphics.has_table_structure());

        // Cell-granular "raw" fragments: one per cell.
        let raw = vec![
            cell_frag("TL", 120.0, 175.0),
            cell_frag("TR", 220.0, 175.0),
            cell_frag("BL", 120.0, 125.0),
            cell_frag("BR", 220.0, 125.0),
        ];
        // Reconstructed set: a single merged fragment (what reconstruct_paragraphs
        // produces). No usable per-cell text, but its position is inside the table
        // bbox so it must be claimed and NOT resurface as a prose element.
        let reconstructed = vec![cell_frag("TL TR BL BR", 120.0, 175.0)];

        let p = Partitioner::new(PartitionConfig::default());
        let elements = p.partition_fragments_with_graphics_raw(
            &reconstructed,
            Some(&raw),
            Some(&graphics),
            0,
            842.0,
        );

        let rows = elements
            .iter()
            .find_map(|e| match e {
                Element::Table(t) => Some(t.rows.clone()),
                _ => None,
            })
            .expect("a Table element");
        // Cells came from the raw set, not the merged reconstructed fragment.
        assert_eq!(rows.len(), 2, "two grid rows, got {rows:?}");
        assert_eq!(rows[0], vec!["TL".to_string(), "TR".to_string()]);
        assert_eq!(rows[1], vec!["BL".to_string(), "BR".to_string()]);
        // The merged reconstructed fragment was claimed, so the Table is the only
        // element (it does not also appear as prose).
        assert_eq!(
            elements.len(),
            1,
            "merged fragment must be claimed, got {elements:?}"
        );
    }

    #[test]
    fn struct_tag_h1_yields_title_no_font_ratio_needed() {
        let mut f = frag_at("Section One", 50.0, 400.0, 12.0);
        f.struct_tag = Some("H1".to_string());
        let frags = vec![f];
        let partitioner = Partitioner::new(PartitionConfig::default());
        let elements = partitioner.partition_fragments(&frags, 0, 800.0);

        assert_eq!(
            elements
                .iter()
                .filter(|e| matches!(e, Element::Title(_)))
                .count(),
            1
        );
    }

    #[test]
    fn struct_tag_p_does_not_block_numeric_prefix_title() {
        // Real NCSC pattern: sub-section headings like "A2.a Risk Management Process"
        // are tagged P (only top-level "Principle Ax" carries H1). The numeric-prefix
        // pattern is strict enough to fire even under P; without this, ~26 NCSC
        // sub-section titles would be lost to the Paragraph classifier.
        let mut f = frag_at("A2.a Risk Management Process", 50.0, 400.0, 12.0);
        f.struct_tag = Some("P".to_string());
        let frags = vec![f];
        let partitioner = Partitioner::new(PartitionConfig::default());
        let elements = partitioner.partition_fragments(&frags, 0, 800.0);

        assert_eq!(
            elements
                .iter()
                .filter(|e| matches!(e, Element::Title(_)))
                .count(),
            1,
            "numeric-prefix heuristic must fire even when struct_tag=P (NCSC sub-sections)"
        );
    }

    #[test]
    fn struct_tag_p_overrides_bold_short_heuristic() {
        let mut f = frag_at("Bold Short Text", 50.0, 400.0, 12.0);
        f.is_bold = true;
        f.struct_tag = Some("P".to_string());
        let frags = vec![f];
        let partitioner = Partitioner::new(PartitionConfig::default());
        let elements = partitioner.partition_fragments(&frags, 0, 800.0);

        assert!(elements.iter().any(|e| matches!(e, Element::Paragraph(_))));
        assert_eq!(
            elements
                .iter()
                .filter(|e| matches!(e, Element::Title(_)))
                .count(),
            0
        );
    }

    #[test]
    fn bold_short_title_fires_without_struct_tag() {
        let mut f = frag_at("Risk Management", 50.0, 400.0, 12.0);
        f.is_bold = true;
        let frags = vec![f];
        let partitioner = Partitioner::new(PartitionConfig::default());
        let elements = partitioner.partition_fragments(&frags, 0, 800.0);

        assert_eq!(
            elements
                .iter()
                .filter(|e| matches!(e, Element::Title(_)))
                .count(),
            1,
            "bold-short heuristic must fire when no other signals present"
        );
    }

    #[test]
    fn numeric_prefix_title_fires_without_bold() {
        let f = frag_at("A2.a Risk Management Process", 50.0, 400.0, 12.0);
        let frags = vec![f];
        let partitioner = Partitioner::new(PartitionConfig::default());
        let elements = partitioner.partition_fragments(&frags, 0, 800.0);

        assert_eq!(
            elements
                .iter()
                .filter(|e| matches!(e, Element::Title(_)))
                .count(),
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
            elements
                .iter()
                .filter(|e| matches!(e, Element::ListItem(_)))
                .count(),
            1
        );
    }

    #[test]
    fn font_ratio_title_still_works() {
        let mut frags = vec![];
        for i in 0..5 {
            frags.push(frag_at(
                &format!("body line {i}"),
                50.0,
                400.0 - (i as f64) * 15.0,
                12.0,
            ));
        }
        frags.push(frag_at("Big Heading", 50.0, 500.0, 20.0));

        let partitioner = Partitioner::new(PartitionConfig::default());
        let elements = partitioner.partition_fragments(&frags, 0, 800.0);

        assert!(
            elements
                .iter()
                .filter(|e| matches!(e, Element::Title(_)))
                .count()
                >= 1,
            "font-ratio Title path must still fire"
        );
    }

    #[test]
    fn header_zone_rejects_long_text() {
        // page_height=800, header_zone=0.05 → threshold y >= 760.
        // Fragment at y=780 (in zone) but text = 200 chars → not Header.
        let long = "X".repeat(200);
        let frags = vec![frag_at(&long, 50.0, 780.0, 12.0)];
        let partitioner = Partitioner::new(PartitionConfig::default());
        let elements = partitioner.partition_fragments(&frags, 0, 800.0);

        let header_count = elements
            .iter()
            .filter(|e| matches!(e, Element::Header(_)))
            .count();
        assert_eq!(
            header_count, 0,
            "long text in header zone must not classify as Header"
        );
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
        assert_eq!(header_count, 0);
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
}
