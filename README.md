# oxidize-pdf

[![Crates.io](https://img.shields.io/crates/v/oxidize-pdf.svg)](https://crates.io/crates/oxidize-pdf)
[![Documentation](https://docs.rs/oxidize-pdf/badge.svg)](https://docs.rs/oxidize-pdf)
[![Downloads](https://img.shields.io/crates/d/oxidize-pdf)](https://crates.io/crates/oxidize-pdf)
[![Coverage](https://img.shields.io/badge/coverage-72%25-yellow)](https://github.com/bzsanti/oxidizePdf)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tests](https://img.shields.io/badge/tests-7%2C993-brightgreen)](https://github.com/bzsanti/oxidizePdf)
[![Rust](https://img.shields.io/badge/rust-%3E%3D1.77-orange.svg)](https://www.rust-lang.org)

**The Rust PDF library built for AI.** Parse any PDF into structure-aware, embedding-ready chunks with one line of code. Pure Rust, zero C dependencies, 99.3% success rate on 9,000+ real-world PDFs.

```rust
let chunks = PdfDocument::open("paper.pdf")?.rag_chunks()?;
// Each chunk: text, pages, bounding boxes, element types, heading context, token estimate
```

## Why oxidize-pdf for RAG?

Most PDF libraries give you a wall of text. oxidize-pdf gives you **structured, metadata-rich chunks** ready for your vector store:

| What you get | Why it matters |
|---|---|
| `chunk.full_text` | Heading context prepended -- better embeddings |
| `chunk.page_numbers` | Citation back to source pages |
| `chunk.bounding_boxes` | Spatial position for visual grounding |
| `chunk.element_types` | Filter by "table", "title", "paragraph" |
| `chunk.token_estimate` | Right-size chunks for your model's context window |
| `chunk.heading_context` | Section awareness without post-processing |

**Performance**: Pure Rust, 3,000-4,000 pages/sec generation, 85ms full-text extraction for a 930KB PDF.

## Quick Start

```toml
[dependencies]
oxidize-pdf = "2.3"
```

### RAG Pipeline -- One Liner

```rust
use oxidize_pdf::parser::PdfDocument;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let doc = PdfDocument::open("document.pdf")?;

    // Structure-aware chunking with full metadata
    let chunks = doc.rag_chunks()?;

    for chunk in &chunks {
        println!("Chunk {}: pages {:?}, ~{} tokens",
            chunk.chunk_index, chunk.page_numbers, chunk.token_estimate);
        println!("  Types: {}", chunk.element_types.join(", "));
        if let Some(heading) = &chunk.heading_context {
            println!("  Section: {}", heading);
        }

        // Use chunk.full_text for embeddings (includes heading context)
        // Use chunk.text for display (content only)
    }

    Ok(())
}
```

### Custom Chunk Size

```rust
use oxidize_pdf::pipeline::HybridChunkConfig;

// Smaller chunks for more precise retrieval
let config = HybridChunkConfig {
    max_tokens: 256,
    ..HybridChunkConfig::default()
};
let chunks = doc.rag_chunks_with(config)?;
```

### JSON for Vector Store Ingestion

```rust
// Serialize all chunks to JSON (requires `semantic` feature)
let json = doc.rag_chunks_json()?;
std::fs::write("chunks.json", json)?;
```

### Element Partitioning

For fine-grained control, access the typed element pipeline directly:

```rust
use oxidize_pdf::pipeline::ExtractionProfile;

let doc = PdfDocument::open("document.pdf")?;

// Partition into typed elements
let elements = doc.partition()?;
for el in &elements {
    println!("page {} : {}", el.page(), el.text());
}

// Or with a pre-configured profile
let elements = doc.partition_with_profile(ExtractionProfile::Academic)?;

// Build a relationship graph (parent/child sections)
let (elements, graph) = doc.partition_graph(Default::default())?;
for section in graph.top_level_sections() {
    println!("Section: {}", elements[section].text());
}
```

## Also in the box

Beyond RAG, the same crate also handles PDF parsing (99.3 % success on 9,000+ real-world PDFs, CJK, lenient recovery), generation (3,000–4,000 pages/sec), encryption (RC4-40/128, AES-128, AES-256 R5/R6 — read and write), digital signatures (PKCS#7 verification), PDF/A validation (8 conformance levels), JBIG2 image decoding (pure-Rust ITU-T T.88), invoice extraction (ES/EN/DE/IT), and split/merge/rotate operations. One dependency for the full pipeline.

See [`oxidize-pdf-core/examples/`](https://github.com/bzsanti/oxidizePdf/tree/main/oxidize-pdf-core/examples) for working samples (133 examples) and [docs.rs](https://docs.rs/oxidize-pdf) for the API surface.

## Full Feature Set

### AI/RAG Pipeline
- Structure-aware chunking with `RagChunk` metadata (pages, bboxes, types, headings)
- Element partitioning: Title, Paragraph, Table, ListItem, Image, CodeBlock, KeyValue
- `ElementGraph` for parent/child section relationships
- 6 extraction profiles (Standard, Academic, Form, Government, Dense, Presentation)
- Reading order strategies (Simple, XYCut)
- LLM-optimized export formats (Markdown, Contextual, JSON)
- Invoice data extraction (ES, EN, DE, IT)

### PDF Processing
- Parse PDF 1.0-1.7 with 99.3% success rate (9,000+ PDFs tested)
- Generate multi-page documents with text, graphics, images
- Encryption: RC4-40/128, AES-128, AES-256 (R5/R6) -- read and write
- Digital signatures: detection, PKCS#7 verification, certificate validation
- PDF/A validation: 8 conformance levels (1a/b, 2a/b/u, 3a/b/u)
- JBIG2 decoder: pure Rust (ITU-T T.88)
- Split, merge, rotate operations
- CJK text support (Chinese, Japanese, Korean)
- Corruption recovery and lenient parsing
- Decompression bomb protection

## Performance

| Operation | Speed |
|---|---|
| PDF generation | 3,000-4,000 pages/sec |
| Full text extraction (930KB) | 85 ms |
| Page text extraction | 546 us |
| File loading | 738 us |

Benchmarked with Criterion. Baseline: `v2.0.0-profiling`.

## Testing

7,993 tests across unit, integration, and doc tests. 7-tier corpus (T0-T6) with 9,000+ PDFs.

```bash
cargo test --workspace         # Full test suite
cargo clippy -- -D warnings    # Lint check
cargo run --example rag_pipeline -- path/to/file.pdf
```

## License

MIT -- see [LICENSE](https://github.com/bzsanti/oxidizePdf/blob/main/LICENSE).

## Links

- [Documentation (docs.rs)](https://docs.rs/oxidize-pdf)
- [Crates.io](https://crates.io/crates/oxidize-pdf)
- [GitHub](https://github.com/bzsanti/oxidizePdf)
- [Issue Tracker](https://github.com/bzsanti/oxidizePdf/issues)
