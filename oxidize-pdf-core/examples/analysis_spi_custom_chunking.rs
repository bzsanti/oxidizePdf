//! Analysis SPI — plugging a custom `ChunkingStrategy`.
//!
//! The SPI (behind the `unstable-spi` feature) lets you decide how elements
//! group into chunks while the pipeline keeps ownership of everything
//! downstream: chunk ids, prev/next links, the `oversized` flag, and the full
//! `ChunkMetadata`. A custom strategy therefore cannot corrupt ids or metadata —
//! it only controls boundaries.
//!
//! Run with:
//!   cargo run --example analysis_spi_custom_chunking --features unstable-spi
//!
//! This example is self-contained: it generates a small PDF in memory, then
//! chunks it two ways (the built-in `HybridChunker` vs a custom strategy) to
//! show the difference.

#[cfg(feature = "unstable-spi")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::parser::{PdfDocument, PdfReader};
    use oxidize_pdf::pipeline::{
        AnalysisPipeline, ChunkGroup, ChunkingStrategy, Element, HybridChunker,
    };
    use oxidize_pdf::text::Font;
    use oxidize_pdf::{Document, Page};
    use std::io::Cursor;

    // 1. Build a throwaway PDF with a heading and a few paragraphs.
    let pdf_bytes = {
        let mut doc = Document::new();
        let mut page = Page::a4();
        let lines = [
            (770.0, 18.0, Font::HelveticaBold, "Quarterly Report"),
            (
                740.0,
                11.0,
                Font::Helvetica,
                "Revenue grew across every region this quarter.",
            ),
            (
                715.0,
                11.0,
                Font::Helvetica,
                "Operating costs stayed flat versus last quarter.",
            ),
            (
                690.0,
                11.0,
                Font::Helvetica,
                "Headcount increased by twelve people in total.",
            ),
            (
                665.0,
                11.0,
                Font::Helvetica,
                "The board approved the expansion plan unanimously.",
            ),
        ];
        for (y, size, font, text) in lines {
            page.text().set_font(font, size).at(50.0, y).write(text)?;
        }
        doc.add_page(page);
        doc.to_bytes()?
    };

    // A custom strategy: collapse the whole document into a single chunk
    // (useful when the downstream store wants one vector per document). It
    // ignores the token budget entirely — the pipeline still computes the
    // `oversized` flag from its own budget, so the chunk is correctly flagged.
    struct WholeDocumentChunk;
    impl ChunkingStrategy for WholeDocumentChunk {
        fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup> {
            if elements.is_empty() {
                return Vec::new();
            }
            // One group holding every element, in order.
            vec![ChunkGroup::new(elements.to_vec(), None)]
        }
    }

    // 2a. Default pipeline == rag_chunks(): the built-in HybridChunker merges
    //     adjacent paragraphs up to the token budget.
    let doc = PdfDocument::new(PdfReader::new(Cursor::new(&pdf_bytes))?);
    let default_chunks = doc.rag_chunks_with_pipeline(&AnalysisPipeline::new())?;

    // 2b. Same document, custom strategy.
    let doc = PdfDocument::new(PdfReader::new(Cursor::new(&pdf_bytes))?);
    let custom_chunks = doc.rag_chunks_with_pipeline(
        &AnalysisPipeline::new().with_chunking(Box::new(WholeDocumentChunk)),
    )?;

    println!("Default HybridChunker → {} chunk(s):", default_chunks.len());
    for c in &default_chunks {
        println!(
            "  [{}] {:?}",
            c.metadata.chunk_id.get(..8).unwrap_or(""),
            truncate(&c.text, 60)
        );
    }

    println!(
        "\nCustom whole-document strategy → {} chunk(s):",
        custom_chunks.len()
    );
    for c in &custom_chunks {
        // The pipeline — not the strategy — derived the id, prev/next links, and
        // the oversized flag (this single chunk holds the whole document).
        println!(
            "  [{}] oversized={} :: {:?}",
            c.metadata.chunk_id.get(..8).unwrap_or(""),
            c.is_oversized,
            truncate(&c.text, 60)
        );
    }

    // The default merges by token budget; the custom strategy collapses to one.
    assert!(custom_chunks.len() <= default_chunks.len());
    assert_eq!(custom_chunks.len(), 1);
    // Decorator-ready: a strategy can also hold a `HybridChunker`, call it for
    // base groups, and refine — "delegate to the default and refine".
    let _wrap_the_default = HybridChunker::default();
    Ok(())
}

#[cfg(feature = "unstable-spi")]
fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let cut: String = s.chars().take(n).collect();
        format!("{cut}…")
    }
}

#[cfg(not(feature = "unstable-spi"))]
fn main() {
    eprintln!("This example requires the `unstable-spi` feature:");
    eprintln!("  cargo run --example analysis_spi_custom_chunking --features unstable-spi");
}
