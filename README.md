# oxidize-pdf

[![Crates.io](https://img.shields.io/crates/v/oxidize-pdf.svg)](https://crates.io/crates/oxidize-pdf)
[![Documentation](https://docs.rs/oxidize-pdf/badge.svg)](https://docs.rs/oxidize-pdf)
[![Downloads](https://img.shields.io/crates/d/oxidize-pdf)](https://crates.io/crates/oxidize-pdf)
[![Coverage](https://img.shields.io/badge/coverage-72%25-yellow)](https://github.com/bzsanti/oxidizePdf)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Tests](https://img.shields.io/badge/tests-7%2C993-brightgreen)](https://github.com/bzsanti/oxidizePdf)
[![Rust](https://img.shields.io/badge/rust-%3E%3D1.88-orange.svg)](https://www.rust-lang.org)

**A comprehensive PDF toolkit written in pure Rust.** Generate, parse, modify,
extract and validate PDFs without C bindings. Use one crate for ordinary PDF
workflows or continue into structure-aware chunks for AI/RAG applications.

| I need to… | Start here |
|---|---|
| Generate PDFs with text, graphics, images and tables | [`Document`](https://docs.rs/oxidize-pdf/latest/oxidize_pdf/struct.Document.html) |
| Parse PDFs and extract text | [`PdfDocument`](https://docs.rs/oxidize-pdf/latest/oxidize_pdf/parser/struct.PdfDocument.html) |
| Split, merge, rotate or reorder pages | [`operations`](https://docs.rs/oxidize-pdf/latest/oxidize_pdf/operations/) |
| Validate PDF/A or inspect signatures | [PDF/A validation](https://docs.rs/oxidize-pdf/latest/oxidize_pdf/pdfa/) and [signatures](https://docs.rs/oxidize-pdf/latest/oxidize_pdf/signatures/) |
| Build an AI/RAG ingestion pipeline | [`rag_chunks`](https://docs.rs/oxidize-pdf/latest/oxidize_pdf/parser/struct.PdfDocument.html#method.rag_chunks) |

```toml
[dependencies]
oxidize-pdf = "5.1.2"
```

Structure-aware RAG remains a first-class workflow:

```rust
let chunks = PdfDocument::open("paper.pdf")?.rag_chunks()?;
// Each chunk: text, pages, bounding boxes, element types, heading context, token estimate
```

## Why oxidize-pdf?

- **One toolkit:** generation, parsing, manipulation, extraction and validation.
- **Pure Rust deployment:** no C library, Java runtime or Python process.
- **AI-ready when needed:** page references, bounding boxes, element types,
  heading context and token estimates are available from the same parser.
- **Explicit scope:** page rasterization is not provided; use a renderer such as
  Pdfium when bitmap output is required.

### Structure-aware RAG

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
oxidize-pdf = "5.1.2"
```

### RAG Pipeline

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

### Contextual Retrieval (no-ML)

Prepend a deterministic context snippet to each chunk's `full_text` before
embedding — the no-ML analogue of [Contextual Retrieval](https://www.anthropic.com/news/contextual-retrieval)
/ Late Chunking, which cut retrieval failures substantially by situating each
chunk in its document. No LLM, no GPU: the prefix is a pure function of the
document metadata and the chunk's heading breadcrumb, so it is fully
reproducible (stable `chunk_id`). The display `text` field stays context-free.

```rust
use oxidize_pdf::pipeline::{ContextFormat, ContextMode, DocumentSource, HybridChunkConfig};

let config = HybridChunkConfig {
    context_mode: ContextMode::Contextual(ContextFormat::Labeled),
    ..HybridChunkConfig::default()
};
let source = DocumentSource::with_file(Some("annual.pdf".into()), Some("sha256-…".into()));
let chunks = doc.rag_chunks_with_source_and_config(source, config)?;

// chunk.full_text now reads, e.g.:
//   Document: Annual Report — Acme Corp
//   Section: 1 Introduction › 1.2 Scope (p. 3–4)
//
//   <chunk text>
```

`ContextFormat::Prose` renders the same facts as one natural-language sentence.
Modes: `ContextMode::None` (bare text), `Heading` (leaf heading only, the
default), `Contextual(_)` (document + section prefix).

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
