//! PDF to LLM Formats - Complete Integration Example
//!
//! This example demonstrates the complete pipeline from loading a PDF
//! to exporting it in AI/ML-optimized formats.
//!
//! Usage:
//!   cargo run --example pdf_to_llm_formats --features semantic <path/to/document.pdf>

use oxidize_pdf::ai;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get PDF path from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path/to/document.pdf>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  cargo run --example pdf_to_llm_formats --features semantic tests/fixtures/sample.pdf");
        std::process::exit(1);
    }

    let pdf_path = &args[1];

    println!("=== PDF to LLM Formats - Integration Demo ===\n");
    println!("Loading PDF: {}\n", pdf_path);

    // Load the PDF document
    let reader = PdfReader::open(pdf_path)?;
    let document = PdfDocument::new(reader);

    // Get document info
    let page_count = document.page_count()?;
    let metadata = document.metadata()?;

    println!("Document Information:");
    println!(
        "  Title: {:?}",
        metadata.title.as_deref().unwrap_or("Untitled")
    );
    println!(
        "  Author: {:?}",
        metadata.author.as_deref().unwrap_or("Unknown")
    );
    println!("  Pages: {}", page_count);
    println!("  PDF Version: {}", document.version()?);
    println!();

    // ========================================
    // 1. MARKDOWN EXPORT
    // ========================================
    println!("1. EXPORTING TO MARKDOWN");
    println!("========================\n");

    let markdown = ai::export_to_markdown(&document)?;
    let md_path = "examples/results/exported_markdown.md";
    fs::write(md_path, &markdown)?;

    println!("✓ Exported to: {}", md_path);
    println!("Preview (first 500 chars):\n");
    println!("{}\n", &markdown.chars().take(500).collect::<String>());
    println!("...\n");

    // ========================================
    // 2. CONTEXTUAL FORMAT EXPORT
    // ========================================
    println!("2. EXPORTING TO CONTEXTUAL FORMAT");
    println!("==================================\n");

    let contextual = ai::export_to_contextual(&document)?;
    let ctx_path = "examples/results/exported_contextual.txt";
    fs::write(ctx_path, &contextual)?;

    println!("✓ Exported to: {}", ctx_path);
    println!("Preview (first 500 chars):\n");
    println!("{}\n", &contextual.chars().take(500).collect::<String>());
    println!("...\n");

    // ========================================
    // 3. JSON EXPORT (requires 'semantic' feature)
    // ========================================
    #[cfg(feature = "semantic")]
    {
        println!("3. EXPORTING TO JSON");
        println!("====================\n");

        let json = ai::export_to_json(&document)?;
        let json_path = "examples/results/exported_json.json";
        fs::write(json_path, &json)?;

        println!("✓ Exported to: {}", json_path);
        println!("Preview (first 500 chars):\n");
        println!("{}\n", &json.chars().take(500).collect::<String>());
        println!("...\n");
    }

    // ========================================
    // 4. CHUNKED EXPORT FOR RAG (requires 'semantic' feature)
    // ========================================
    #[cfg(feature = "semantic")]
    {
        println!("4. EXPORTING WITH CHUNKING FOR RAG");
        println!("===================================\n");

        let chunks = ai::export_to_chunks(&document, 1000, 200)?;
        let chunks_path = "examples/results/exported_chunks.json";
        fs::write(chunks_path, &chunks)?;

        println!("✓ Exported to: {}", chunks_path);
        println!("Chunk settings:");
        println!("  - Chunk size: 1000 characters");
        println!("  - Overlap: 200 characters");
        println!("\nPreview (first 500 chars):\n");
        println!("{}\n", &chunks.chars().take(500).collect::<String>());
        println!("...\n");
    }

    // ========================================
    // 5. EXAMPLE: LLM PROMPT CONSTRUCTION
    // ========================================
    println!("5. EXAMPLE: LLM PROMPT CONSTRUCTION");
    println!("====================================\n");

    let document_context = ai::export_to_contextual(&document)?;

    let llm_prompt = format!(
        "You are a helpful assistant analyzing documents.\n\n\
         Below is the content of a document:\n\n\
         {}\n\n\
         Question: Can you summarize the main points of this document?\n\n\
         Please provide a concise summary based on the document content.",
        document_context
    );

    let prompt_path = "examples/results/example_prompt.txt";
    fs::write(prompt_path, &llm_prompt)?;

    println!("✓ Example prompt saved to: {}", prompt_path);
    println!("Preview (first 700 chars):\n");
    println!("{}\n", &llm_prompt.chars().take(700).collect::<String>());
    println!("...\n");

    // ========================================
    // 6. SUMMARY
    // ========================================
    println!("✅ ALL EXPORTS COMPLETED SUCCESSFULLY!\n");
    println!("Generated files:");
    println!("  - {}", md_path);
    println!("  - {}", ctx_path);
    #[cfg(feature = "semantic")]
    println!("  - {}", "examples/results/exported_json.json");
    #[cfg(feature = "semantic")]
    println!("  - {}", "examples/results/exported_chunks.json");
    println!("  - {}", prompt_path);
    println!();
    println!("Next steps:");
    println!("  1. Review the exported files");
    println!("  2. Use Markdown for human-readable documentation");
    println!("  3. Use JSON for API integration");
    println!("  4. Use Contextual format for direct LLM prompts");
    println!("  5. Use chunks for RAG pipelines with vector databases");

    Ok(())
}
