use crate::pipeline::token_counter::{TokenCounter, WordProxyCounter};
use crate::pipeline::Element;
use std::sync::Arc;

/// Configuration for semantic chunking.
#[derive(Debug, Clone)]
pub struct SemanticChunkConfig {
    /// Maximum tokens per chunk under the selected counter (word proxy by default).
    pub max_tokens: usize,
    /// Maximum overlap tokens between consecutive chunks. Whole trailing elements
    /// are retained only when they fit this limit and the next chunk budget.
    pub overlap_tokens: usize,
    /// Whether to keep elements whole (don't split titles, tables, etc.).
    pub respect_element_boundaries: bool,
}

impl Default for SemanticChunkConfig {
    fn default() -> Self {
        Self {
            max_tokens: 512,
            overlap_tokens: 50,
            respect_element_boundaries: true,
        }
    }
}

impl SemanticChunkConfig {
    /// Create config with specified max tokens.
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            ..Default::default()
        }
    }

    /// Set overlap tokens.
    pub fn with_overlap(mut self, overlap: usize) -> Self {
        self.overlap_tokens = overlap;
        self
    }
}

/// A semantic chunk: a group of elements with metadata.
#[derive(Debug, Clone)]
pub struct SemanticChunk {
    elements: Vec<Element>,
    oversized: bool,
    token_estimate: usize,
}

impl SemanticChunk {
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

    /// Token count under the counter that produced this chunk.
    pub fn token_estimate(&self) -> usize {
        self.token_estimate
    }

    /// Page numbers spanned by this chunk.
    pub fn page_numbers(&self) -> Vec<u32> {
        let mut pages: Vec<u32> = self.elements.iter().map(|e| e.page()).collect();
        pages.sort_unstable();
        pages.dedup();
        pages
    }

    /// Whether this chunk exceeds max_tokens (e.g., a large table).
    pub fn is_oversized(&self) -> bool {
        self.oversized
    }
}

/// Semantic chunker that respects element boundaries.
pub struct SemanticChunker {
    config: SemanticChunkConfig,
    counter: Arc<dyn TokenCounter>,
}

impl Default for SemanticChunker {
    fn default() -> Self {
        Self {
            config: SemanticChunkConfig::default(),
            counter: Arc::new(WordProxyCounter),
        }
    }
}

impl SemanticChunker {
    pub fn new(config: SemanticChunkConfig) -> Self {
        Self {
            config,
            counter: Arc::new(WordProxyCounter),
        }
    }

    /// Inject a token counter governing split/oversize decisions and the stamped
    /// per-chunk `token_estimate`. Default is `WordProxyCounter`.
    pub fn with_token_counter(mut self, counter: Arc<dyn TokenCounter>) -> Self {
        self.counter = counter;
        self
    }

    fn element_tokens(&self, element: &Element) -> usize {
        self.counter.count(&element.display_text())
    }

    /// Cost of appending an element, including the emitted newline separator.
    fn appended_tokens(&self, elements: &[Element], tokens: usize, next: &Element) -> usize {
        if elements.is_empty() {
            return self.element_tokens(next);
        }
        if self.counter.is_additive_over_whitespace_join() {
            return tokens.saturating_add(self.element_tokens(next));
        }
        let text = elements
            .iter()
            .chain(std::iter::once(next))
            .map(Element::display_text)
            .collect::<Vec<_>>()
            .join("\n");
        self.counter.count(&text)
    }

    fn make_chunk(&self, elements: Vec<Element>) -> SemanticChunk {
        let text = elements
            .iter()
            .map(|e| e.display_text())
            .collect::<Vec<_>>()
            .join("\n");
        let token_estimate = self.counter.count(&text);
        SemanticChunk {
            elements,
            oversized: token_estimate > self.config.max_tokens,
            token_estimate,
        }
    }

    fn make_paragraph_chunk(
        &self,
        text: &str,
        meta: &crate::pipeline::ElementMetadata,
    ) -> SemanticChunk {
        self.make_chunk(vec![Element::Paragraph(crate::pipeline::ElementData {
            text: text.to_string(),
            metadata: meta.clone(),
        })])
    }

    /// Chunk a list of elements into semantic chunks.
    pub fn chunk(&self, elements: &[Element]) -> Vec<SemanticChunk> {
        if elements.is_empty() {
            return Vec::new();
        }

        let mut chunks = Vec::new();
        let mut current_elements: Vec<Element> = Vec::new();
        let mut current_tokens = 0usize;

        for element in elements {
            let elem_tokens = self.element_tokens(element);
            let mut joined_tokens =
                self.appended_tokens(&current_elements, current_tokens, element);

            // Non-splittable elements (Table, Title, Header, Footer, Image)
            if !is_splittable(element) {
                // If adding this would overflow and we have content, flush first
                if !current_elements.is_empty()
                    && joined_tokens > self.config.max_tokens
                    && self.config.respect_element_boundaries
                {
                    self.flush_chunk(
                        &mut chunks,
                        &mut current_elements,
                        &mut current_tokens,
                        element,
                    );
                    joined_tokens =
                        self.appended_tokens(&current_elements, current_tokens, element);
                }

                // If element alone exceeds max_tokens, it gets its own oversized chunk
                if elem_tokens > self.config.max_tokens && current_elements.is_empty() {
                    chunks.push(self.make_chunk(vec![element.clone()]));
                    continue;
                }

                current_tokens = joined_tokens;
                current_elements.push(element.clone());
                continue;
            }

            // Splittable elements (Paragraph, ListItem, CodeBlock, KeyValue)
            if joined_tokens <= self.config.max_tokens {
                // Fits in current chunk
                current_tokens = joined_tokens;
                current_elements.push(element.clone());
            } else if elem_tokens <= self.config.max_tokens {
                // Doesn't fit but element itself is within limit — start new chunk
                if !current_elements.is_empty() {
                    self.flush_chunk(
                        &mut chunks,
                        &mut current_elements,
                        &mut current_tokens,
                        element,
                    );
                }
                current_tokens = self.appended_tokens(&current_elements, current_tokens, element);
                current_elements.push(element.clone());
            } else {
                // Element exceeds max_tokens — split by sentences. Overlap is
                // omitted here so it cannot consume the sentence budget.
                if !current_elements.is_empty() {
                    chunks.push(self.make_chunk(std::mem::take(&mut current_elements)));
                }
                current_tokens = 0;

                let sentences = split_sentences(element.text());
                let meta = element.metadata().clone();
                let mut sentence_buf = String::new();
                let mut sentence_tokens = 0usize;

                for sentence in &sentences {
                    let single_tokens = self.counter.count(sentence);
                    let joined_tokens = if sentence_buf.is_empty() {
                        single_tokens
                    } else if self.counter.is_additive_over_whitespace_join() {
                        sentence_tokens.saturating_add(single_tokens)
                    } else {
                        self.counter.count(&format!("{sentence_buf} {sentence}"))
                    };
                    if joined_tokens > self.config.max_tokens && !sentence_buf.is_empty() {
                        chunks.push(self.make_paragraph_chunk(&sentence_buf, &meta));
                        sentence_buf.clear();
                        sentence_tokens = single_tokens;
                    } else {
                        sentence_tokens = joined_tokens;
                    }
                    if !sentence_buf.is_empty() {
                        sentence_buf.push(' ');
                    }
                    sentence_buf.push_str(sentence);
                }

                if !sentence_buf.is_empty() {
                    current_tokens = self.counter.count(&sentence_buf);
                    current_elements.push(Element::Paragraph(crate::pipeline::ElementData {
                        text: sentence_buf,
                        metadata: meta,
                    }));
                }
            }
        }

        // Flush remaining
        if !current_elements.is_empty() {
            chunks.push(self.make_chunk(current_elements));
        }

        chunks
    }

    /// Emit the buffer and retain a suffix only if it fits beside the next element.
    fn flush_chunk(
        &self,
        chunks: &mut Vec<SemanticChunk>,
        current_elements: &mut Vec<Element>,
        current_tokens: &mut usize,
        next: &Element,
    ) {
        let flushed = std::mem::take(current_elements);
        *current_tokens = 0;
        if self.config.overlap_tokens > 0 {
            let mut retained_start = flushed.len();
            let mut tokens = 0usize;
            for start in (0..flushed.len()).rev() {
                let suffix = &flushed[start..];
                tokens = if self.counter.is_additive_over_whitespace_join() {
                    tokens.saturating_add(self.element_tokens(&flushed[start]))
                } else {
                    let text = suffix
                        .iter()
                        .map(Element::display_text)
                        .collect::<Vec<_>>()
                        .join("\n");
                    self.counter.count(&text)
                };
                if tokens > self.config.overlap_tokens
                    || self.appended_tokens(suffix, tokens, next) > self.config.max_tokens
                {
                    // Overlap is best effort; keep the last suffix verified to
                    // fit. We do not assume larger BPE suffixes cost more.
                    break;
                }
                retained_start = start;
                *current_tokens = tokens;
            }
            current_elements.extend_from_slice(&flushed[retained_start..]);
        }
        chunks.push(self.make_chunk(flushed));
    }
}

/// Whether an element can be split across chunks.
fn is_splittable(element: &Element) -> bool {
    matches!(
        element,
        Element::Paragraph(_) | Element::ListItem(_) | Element::CodeBlock(_) | Element::KeyValue(_)
    )
}

/// Split text into sentences.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        current.push(ch);
        if (ch == '.' || ch == '!' || ch == '?') && !current.trim().is_empty() {
            sentences.push(current.trim().to_string());
            current.clear();
        }
    }

    if !current.trim().is_empty() {
        // Leftover without sentence terminator — append to last sentence or make new
        if let Some(last) = sentences.last_mut() {
            last.push(' ');
            last.push_str(current.trim());
        } else {
            sentences.push(current.trim().to_string());
        }
    }

    sentences
}
