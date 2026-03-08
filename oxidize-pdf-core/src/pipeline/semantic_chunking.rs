use crate::pipeline::Element;

/// Configuration for semantic chunking.
#[derive(Debug, Clone)]
pub struct SemanticChunkConfig {
    /// Maximum tokens per chunk (approximate — uses word count as proxy).
    pub max_tokens: usize,
    /// Number of overlap tokens between consecutive chunks.
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
            .map(|e| element_text(e))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Approximate token count (word count proxy).
    pub fn token_estimate(&self) -> usize {
        estimate_tokens(&self.text())
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
}

impl Default for SemanticChunker {
    fn default() -> Self {
        Self {
            config: SemanticChunkConfig::default(),
        }
    }
}

impl SemanticChunker {
    pub fn new(config: SemanticChunkConfig) -> Self {
        Self { config }
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
            let elem_tokens = element_token_count(element);

            // Non-splittable elements (Table, Title, Header, Footer, Image)
            if !is_splittable(element) {
                // If adding this would overflow and we have content, flush first
                if current_tokens > 0
                    && current_tokens + elem_tokens > self.config.max_tokens
                    && self.config.respect_element_boundaries
                {
                    chunks.push(SemanticChunk {
                        elements: std::mem::take(&mut current_elements),
                        oversized: false,
                    });
                    current_tokens = 0;
                }

                // If element alone exceeds max_tokens, it gets its own oversized chunk
                if elem_tokens > self.config.max_tokens && current_elements.is_empty() {
                    chunks.push(SemanticChunk {
                        elements: vec![element.clone()],
                        oversized: true,
                    });
                    continue;
                }

                current_elements.push(element.clone());
                current_tokens += elem_tokens;
                continue;
            }

            // Splittable elements (Paragraph, ListItem, CodeBlock, KeyValue)
            if current_tokens + elem_tokens <= self.config.max_tokens {
                // Fits in current chunk
                current_elements.push(element.clone());
                current_tokens += elem_tokens;
            } else if elem_tokens <= self.config.max_tokens {
                // Doesn't fit but element itself is within limit — start new chunk
                if !current_elements.is_empty() {
                    chunks.push(SemanticChunk {
                        elements: std::mem::take(&mut current_elements),
                        oversized: false,
                    });
                }
                current_elements.push(element.clone());
                current_tokens = elem_tokens;
            } else {
                // Element exceeds max_tokens — split by sentences
                if !current_elements.is_empty() {
                    chunks.push(SemanticChunk {
                        elements: std::mem::take(&mut current_elements),
                        oversized: false,
                    });
                    current_tokens = 0;
                }

                let sentences = split_sentences(element.text());
                let meta = element.metadata().clone();
                let mut sentence_buf = String::new();
                let mut buf_tokens = 0;

                for sentence in &sentences {
                    let s_tokens = estimate_tokens(sentence);
                    if buf_tokens + s_tokens > self.config.max_tokens && !sentence_buf.is_empty() {
                        chunks.push(make_paragraph_chunk(&sentence_buf, &meta));
                        sentence_buf.clear();
                        buf_tokens = 0;
                    }
                    if !sentence_buf.is_empty() {
                        sentence_buf.push(' ');
                    }
                    sentence_buf.push_str(sentence);
                    buf_tokens += s_tokens;
                }

                if !sentence_buf.is_empty() {
                    current_elements.push(Element::Paragraph(crate::pipeline::ElementData {
                        text: sentence_buf,
                        metadata: meta,
                    }));
                    current_tokens = buf_tokens;
                }
            }
        }

        // Flush remaining
        if !current_elements.is_empty() {
            chunks.push(SemanticChunk {
                elements: current_elements,
                oversized: false,
            });
        }

        chunks
    }
}

/// Whether an element can be split across chunks.
fn is_splittable(element: &Element) -> bool {
    matches!(
        element,
        Element::Paragraph(_) | Element::ListItem(_) | Element::CodeBlock(_) | Element::KeyValue(_)
    )
}

/// Approximate token count for an element.
fn element_token_count(element: &Element) -> usize {
    let text = element_text(element);
    estimate_tokens(&text)
}

/// Get text representation for token counting.
fn element_text(element: &Element) -> String {
    match element {
        Element::Table(t) => {
            // Represent table as rows for token counting
            t.rows
                .iter()
                .map(|row| row.join(" | "))
                .collect::<Vec<_>>()
                .join("\n")
        }
        Element::Image(img) => img.alt_text.clone().unwrap_or_default(),
        Element::KeyValue(kv) => format!("{}: {}", kv.key, kv.value),
        _ => element.text().to_string(),
    }
}

/// Simple token estimator: word count (split by whitespace).
fn estimate_tokens(text: &str) -> usize {
    text.split_whitespace().count()
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

fn make_paragraph_chunk(text: &str, meta: &crate::pipeline::ElementMetadata) -> SemanticChunk {
    SemanticChunk {
        elements: vec![Element::Paragraph(crate::pipeline::ElementData {
            text: text.to_string(),
            metadata: meta.clone(),
        })],
        oversized: false,
    }
}
