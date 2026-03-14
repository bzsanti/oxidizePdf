//! RAG Pipeline Example
//!
//! Demonstrates the one-liner RAG API:
//!   open PDF → rag_chunks() → iterate chunks → print metadata
//!
//! Run with:
//!   cargo run --example rag_pipeline -- path/to/document.pdf

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::pipeline::HybridChunkConfig;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("document.pdf");

    let reader = PdfReader::open(path)?;
    let doc = PdfDocument::new(reader);

    // One-liner: default config (512 tokens, AnyInlineContent merge)
    let chunks = doc.rag_chunks()?;

    println!("Produced {} RAG chunks\n", chunks.len());

    for chunk in &chunks {
        println!("--- Chunk {} ---", chunk.chunk_index);
        if let Some(ref heading) = chunk.heading_context {
            println!("  Heading:  {}", heading);
        }
        println!("  Pages:    {:?}", chunk.page_numbers);
        println!("  Elements: {}", chunk.element_types.join(", "));
        println!("  Tokens:   ~{}", chunk.token_estimate);
        if chunk.is_oversized {
            println!("  ⚠ oversized chunk");
        }
        let preview: String = chunk.text.chars().take(80).collect();
        println!("  Preview:  {}", preview);
        println!();
    }

    // Custom config: 256 tokens
    println!("=== Custom config (256 tokens) ===");
    let config = HybridChunkConfig {
        max_tokens: 256,
        ..HybridChunkConfig::default()
    };
    let small_chunks = doc.rag_chunks_with(config)?;
    println!(
        "Produced {} chunks with 256-token limit\n",
        small_chunks.len()
    );

    Ok(())
}
