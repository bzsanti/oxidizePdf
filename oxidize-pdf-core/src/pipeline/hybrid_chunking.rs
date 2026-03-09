use crate::pipeline::graph::ElementGraph;
use crate::pipeline::Element;

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
}

impl Default for HybridChunkConfig {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            overlap_tokens: 50,
            merge_adjacent: true,
            propagate_headings: true,
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
                && same_merge_group(buffer.last().unwrap(), element)
                && buffer_tokens + elem_tokens <= self.config.max_tokens;

            if can_merge {
                buffer.push(element.clone());
                buffer_tokens += elem_tokens;
                continue;
            }

            // Can't merge — check if buffer needs flushing
            if !buffer.is_empty() {
                // Flush if: adding would overflow, or types differ
                if buffer_tokens + elem_tokens > self.config.max_tokens
                    || !same_merge_group(buffer.last().unwrap(), element)
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
                chunks.push(HybridChunk {
                    elements: vec![element.clone()],
                    heading_context: elem_heading,
                    oversized: true,
                });
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

/// Whether two adjacent elements can be merged in the same chunk.
fn same_merge_group(a: &Element, b: &Element) -> bool {
    matches!(
        (a, b),
        (Element::Paragraph(_), Element::Paragraph(_))
            | (Element::ListItem(_), Element::ListItem(_))
    )
}
