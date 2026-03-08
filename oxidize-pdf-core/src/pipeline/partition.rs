use crate::pipeline::{
    Element, ElementBBox, ElementData, ElementMetadata, KeyValueElementData, TableElementData,
};
use crate::text::extraction::TextFragment;

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
}

impl Default for PartitionConfig {
    fn default() -> Self {
        Self {
            detect_tables: true,
            detect_headers_footers: true,
            title_min_font_ratio: 1.3,
            header_zone: 0.05,
            footer_zone: 0.05,
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
    pub fn partition_fragments(
        &self,
        fragments: &[TextFragment],
        page: u32,
        page_height: f64,
    ) -> Vec<Element> {
        if fragments.is_empty() {
            return Vec::new();
        }

        // Track which fragments have been claimed
        let mut claimed = vec![false; fragments.len()];
        let mut elements = Vec::new();

        // 1. Header/footer detection
        if self.config.detect_headers_footers && page_height > 0.0 {
            let header_threshold = page_height * (1.0 - self.config.header_zone);
            let footer_threshold = page_height * self.config.footer_zone;

            for (i, f) in fragments.iter().enumerate() {
                if claimed[i] {
                    continue;
                }
                if f.y >= header_threshold {
                    elements.push(Element::Header(ElementData {
                        text: f.text.clone(),
                        metadata: meta_from_fragment(f, page),
                    }));
                    claimed[i] = true;
                } else if f.y + f.height <= footer_threshold {
                    elements.push(Element::Footer(ElementData {
                        text: f.text.clone(),
                        metadata: meta_from_fragment(f, page),
                    }));
                    claimed[i] = true;
                }
            }
        }

        // 2. Table detection via StructuredDataDetector
        if self.config.detect_tables {
            let unclaimed_frags: Vec<&TextFragment> = fragments
                .iter()
                .enumerate()
                .filter(|(i, _)| !claimed[*i])
                .map(|(_, f)| f)
                .collect();

            if unclaimed_frags.len() >= 4 {
                let detector =
                    crate::text::structured::StructuredDataDetector::new(Default::default());
                let unclaimed_owned: Vec<TextFragment> =
                    unclaimed_frags.iter().map(|f| (*f).clone()).collect();
                if let Ok(result) = detector.detect(&unclaimed_owned) {
                    for table in &result.tables {
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

                        // Claim fragments that fall within table bounding box
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
                    elements.push(Element::KeyValue(KeyValueElementData {
                        key: key.to_string(),
                        value: value.to_string(),
                        metadata: meta,
                    }));
                    continue;
                }
            }

            // 4. Title detection by font size
            if f.font_size >= title_threshold && f.font_size > body_font_size {
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

        // Sort by reading order: Y descending within page (top-to-bottom)
        elements.sort_by(super::element::element_reading_order);
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

fn meta_from_fragment(f: &TextFragment, page: u32) -> ElementMetadata {
    ElementMetadata {
        page,
        bbox: ElementBBox::new(f.x, f.y, f.width, f.height),
        confidence: 1.0,
        font_name: f.font_name.clone(),
        font_size: Some(f.font_size),
        is_bold: f.is_bold,
        is_italic: f.is_italic,
    }
}
