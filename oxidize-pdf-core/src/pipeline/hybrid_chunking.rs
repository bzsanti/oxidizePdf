use crate::pipeline::graph::ElementGraph;
use crate::pipeline::{Element, ElementData, ElementMetadata};

/// Policy for which adjacent element types can be merged into a single chunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MergePolicy {
    /// Only merge Paragraph+Paragraph and ListItem+ListItem (legacy behavior).
    SameTypeOnly,
    /// Merge any adjacent non-structural elements (Paragraph, ListItem, KeyValue).
    /// Titles, Tables, Images, and CodeBlocks always start a new chunk.
    AnyInlineContent,
}

/// Configuration for hybrid chunking.
#[derive(Debug, Clone)]
pub struct HybridChunkConfig {
    /// Maximum tokens per chunk (approximate — uses word count as proxy).
    pub max_tokens: usize,
    /// Number of overlap tokens between consecutive chunks.
    pub overlap_tokens: usize,
    /// Whether to merge adjacent elements of the same type (Paragraph+Paragraph, ListItem+ListItem).
    pub merge_adjacent: bool,
    /// Whether to propagate heading context from `parent_heading` metadata.
    pub propagate_headings: bool,
    /// Merge policy for adjacent elements. Default: `MergePolicy::AnyInlineContent`.
    pub merge_policy: MergePolicy,
}

impl Default for HybridChunkConfig {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            overlap_tokens: 50,
            merge_adjacent: true,
            propagate_headings: true,
            merge_policy: MergePolicy::AnyInlineContent,
        }
    }
}

/// A hybrid chunk: a group of elements with heading context.
#[derive(Debug, Clone)]
pub struct HybridChunk {
    elements: Vec<Element>,
    /// The heading context for this chunk (from `parent_heading` of its elements).
    pub heading_context: Option<String>,
    oversized: bool,
}

impl HybridChunk {
    /// The elements in this chunk.
    pub fn elements(&self) -> &[Element] {
        &self.elements
    }

    /// Concatenated text of all elements.
    pub fn text(&self) -> String {
        self.elements
            .iter()
            .map(|e| e.display_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Text optimized for RAG embedding: heading context prepended (if any) + chunk content.
    /// Use this for embedding generation. Use `text()` for display.
    pub fn full_text(&self) -> String {
        match &self.heading_context {
            Some(heading) => format!("{}\n\n{}", heading, self.text()),
            None => self.text(),
        }
    }

    /// Approximate token count (word count proxy).
    pub fn token_estimate(&self) -> usize {
        estimate_tokens(&self.text())
    }

    /// Whether this chunk exceeds max_tokens (e.g., a large table).
    pub fn is_oversized(&self) -> bool {
        self.oversized
    }
}

/// Hybrid chunker that merges adjacent elements and propagates heading context.
pub struct HybridChunker {
    config: HybridChunkConfig,
}

impl Default for HybridChunker {
    fn default() -> Self {
        Self {
            config: HybridChunkConfig::default(),
        }
    }
}

impl HybridChunker {
    pub fn new(config: HybridChunkConfig) -> Self {
        Self { config }
    }

    /// Chunk a list of elements into hybrid chunks.
    pub fn chunk(&self, elements: &[Element]) -> Vec<HybridChunk> {
        if elements.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut buffer: Vec<Element> = Vec::new();
        let mut buffer_tokens = 0usize;
        let mut buffer_heading: Option<String> = None;

        for element in elements {
            let elem_tokens = estimate_tokens(&element.display_text());
            let elem_heading = if self.config.propagate_headings {
                element.metadata().parent_heading.clone()
            } else {
                None
            };

            // Check if this element can merge with the buffer
            let can_merge = self.config.merge_adjacent
                && !buffer.is_empty()
                && can_merge_elements(buffer.last().unwrap(), element, &self.config.merge_policy)
                && buffer_tokens + elem_tokens <= self.config.max_tokens;

            if can_merge {
                buffer.push(element.clone());
                buffer_tokens += elem_tokens;
                continue;
            }

            // Can't merge — check if buffer needs flushing
            if !buffer.is_empty() {
                // Flush if: adding would overflow, or types differ, or merge disabled
                if buffer_tokens + elem_tokens > self.config.max_tokens
                    || !can_merge_elements(
                        buffer.last().unwrap(),
                        element,
                        &self.config.merge_policy,
                    )
                    || !self.config.merge_adjacent
                {
                    self.flush_buffer(
                        &mut chunks,
                        &mut buffer,
                        &mut buffer_tokens,
                        &mut buffer_heading,
                    );
                }
            }

            // Handle oversized element
            if elem_tokens > self.config.max_tokens && buffer.is_empty() {
                if is_splittable_element(element) {
                    let text = element.display_text();
                    let fragments = split_by_sentences(&text, self.config.max_tokens);
                    for fragment in fragments {
                        let fragment_element = make_text_fragment_element(element, fragment.trim());
                        chunks.push(HybridChunk {
                            elements: vec![fragment_element],
                            heading_context: elem_heading.clone(),
                            oversized: false,
                        });
                    }
                } else {
                    // Table, image, code: atomic oversized chunk
                    chunks.push(HybridChunk {
                        elements: vec![element.clone()],
                        heading_context: elem_heading,
                        oversized: true,
                    });
                }
                continue;
            }

            // Start or append to buffer
            if buffer.is_empty() {
                buffer_heading = elem_heading;
            }
            buffer.push(element.clone());
            buffer_tokens += elem_tokens;
        }

        // Flush remaining
        if !buffer.is_empty() {
            chunks.push(HybridChunk {
                elements: std::mem::take(&mut buffer),
                heading_context: buffer_heading,
                oversized: false,
            });
        }

        chunks
    }

    /// Chunk a list of elements using the relationship graph to keep sections together.
    ///
    /// This method uses graph structure to group elements by section (all children of
    /// a title element), then attempts to pack each section into a single chunk.  If
    /// a section exceeds `max_tokens`, it delegates to [`chunk`](Self::chunk) for that
    /// section's elements, ensuring all resulting sub-chunks still carry the section's
    /// heading context.
    ///
    /// Elements that have no parent section (preamble elements before any title) are
    /// chunked with the standard `chunk()` strategy.
    pub fn chunk_with_graph(&self, elements: &[Element], graph: &ElementGraph) -> Vec<HybridChunk> {
        if elements.is_empty() {
            return Vec::new();
        }

        let mut chunks: Vec<HybridChunk> = Vec::new();

        // Collect preamble: indices with no parent AND not a title.
        let top_sections = graph.top_level_sections();

        // Determine the index of the first title so we know the preamble boundary.
        let first_title_idx = top_sections.first().copied().unwrap_or(elements.len());

        // ── Preamble (elements before the first title section) ────────────────
        if first_title_idx > 0 {
            let preamble: Vec<Element> = elements[..first_title_idx].to_vec();
            chunks.extend(self.chunk(&preamble));
        }

        // ── Process each top-level section ────────────────────────────────────
        for &title_idx in &top_sections {
            let title_heading = elements[title_idx]
                .metadata()
                .parent_heading
                .clone()
                .or_else(|| Some(elements[title_idx].text().to_string()));

            let child_indices = graph.elements_in_section(title_idx);

            // Gather section elements: title + all children.
            let mut section_elements: Vec<Element> = Vec::with_capacity(1 + child_indices.len());
            section_elements.push(elements[title_idx].clone());
            for &ci in &child_indices {
                section_elements.push(elements[ci].clone());
            }

            let section_tokens: usize = section_elements
                .iter()
                .map(|e| estimate_tokens(&e.display_text()))
                .sum();

            if section_tokens <= self.config.max_tokens {
                // Entire section fits in one chunk.
                chunks.push(HybridChunk {
                    elements: section_elements,
                    heading_context: title_heading,
                    oversized: false,
                });
            } else {
                // Section is too large — split with standard chunker, then fix heading.
                let mut sub_chunks = self.chunk(&section_elements);
                for sub in &mut sub_chunks {
                    sub.heading_context = title_heading.clone();
                }
                chunks.extend(sub_chunks);
            }
        }

        chunks
    }

    fn flush_buffer(
        &self,
        chunks: &mut Vec<HybridChunk>,
        buffer: &mut Vec<Element>,
        buffer_tokens: &mut usize,
        buffer_heading: &mut Option<String>,
    ) {
        let flushed = std::mem::take(buffer);
        let heading = buffer_heading.take();

        chunks.push(HybridChunk {
            elements: flushed.clone(),
            heading_context: heading,
            oversized: false,
        });

        // Apply overlap: carry trailing elements from flushed chunk into the next
        if self.config.overlap_tokens > 0 {
            let mut overlap_tokens = 0usize;
            let mut overlap_elements = Vec::new();

            for elem in flushed.iter().rev() {
                let t = estimate_tokens(&elem.display_text());
                if overlap_tokens + t > self.config.overlap_tokens && !overlap_elements.is_empty() {
                    break;
                }
                overlap_elements.push(elem.clone());
                overlap_tokens += t;
            }

            overlap_elements.reverse();
            *buffer = overlap_elements;
            *buffer_tokens = overlap_tokens;
            // Preserve heading from overlap elements
            if let Some(first) = buffer.first() {
                *buffer_heading = first.metadata().parent_heading.clone();
            }
        } else {
            *buffer_tokens = 0;
        }
    }
}

/// Simple token estimator: word count (split by whitespace).
fn estimate_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Whether two adjacent elements can be merged according to the given policy.
fn can_merge_elements(a: &Element, b: &Element, policy: &MergePolicy) -> bool {
    match policy {
        MergePolicy::SameTypeOnly => matches!(
            (a, b),
            (Element::Paragraph(_), Element::Paragraph(_))
                | (Element::ListItem(_), Element::ListItem(_))
        ),
        MergePolicy::AnyInlineContent => is_inline_element(a) && is_inline_element(b),
    }
}

/// Returns true for text-based elements that can be merged with adjacent elements.
/// Structural elements (Title, Table, Image) and code blocks always start a new chunk.
fn is_inline_element(e: &Element) -> bool {
    matches!(
        e,
        Element::Paragraph(_) | Element::ListItem(_) | Element::KeyValue(_)
    )
}

/// Returns true for elements whose text content can be split at sentence boundaries.
fn is_splittable_element(e: &Element) -> bool {
    matches!(e, Element::Paragraph(_) | Element::ListItem(_))
}

/// Split text at sentence boundaries (`. `, `! `, `? `, `\n`) into fragments of at most
/// `max_tokens` words. Greedily accumulates sentences; if a single sentence still exceeds
/// `max_tokens`, it is emitted as a single fragment (cannot split further without a
/// semantic break). Never returns an empty Vec.
fn split_by_sentences(text: &str, max_tokens: usize) -> Vec<String> {
    // Split into sentences preserving the delimiter as part of the sentence.
    let sentences = split_into_sentences(text);

    let mut fragments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_tokens = 0usize;

    for sentence in sentences {
        let sentence = sentence.trim();
        if sentence.is_empty() {
            continue;
        }
        let sentence_tokens = estimate_tokens(sentence);

        if current.is_empty() {
            // Starting a new fragment
            current.push_str(sentence);
            current_tokens = sentence_tokens;
        } else if current_tokens + 1 + sentence_tokens <= max_tokens {
            // Adding a space separator between sentences
            current.push(' ');
            current.push_str(sentence);
            current_tokens += 1 + sentence_tokens;
        } else {
            // Current sentence doesn't fit: flush and start new fragment
            fragments.push(current.clone());
            current = sentence.to_string();
            current_tokens = sentence_tokens;
        }
    }

    if !current.is_empty() {
        fragments.push(current);
    }

    if fragments.is_empty() {
        // Fallback: return the original text as a single fragment
        fragments.push(text.to_string());
    }

    fragments
}

/// Split text into sentence-like segments preserving punctuation.
/// Splits on `. `, `! `, `? `, and `\n`.
fn split_into_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        current.push(ch);

        if matches!(ch, '.' | '!' | '?') && i + 1 < len && chars[i + 1] == ' ' {
            sentences.push(current.trim().to_string());
            current = String::new();
            i += 2; // skip the space after delimiter
            continue;
        } else if ch == '\n' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current = String::new();
        }

        i += 1;
    }

    let remaining = current.trim().to_string();
    if !remaining.is_empty() {
        sentences.push(remaining);
    }

    sentences
}

/// Create a new Paragraph element from an existing element's metadata, replacing the text.
/// Preserves provenance (page, bbox, parent_heading).
fn make_text_fragment_element(source: &Element, fragment_text: &str) -> Element {
    let metadata = source.metadata().clone();
    Element::Paragraph(ElementData {
        text: fragment_text.to_string(),
        metadata: ElementMetadata {
            page: metadata.page,
            bbox: metadata.bbox,
            parent_heading: metadata.parent_heading,
            ..Default::default()
        },
    })
}
