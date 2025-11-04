//! LLM-Optimized Export Formats Example
//!
//! This example demonstrates all three export formats for AI/ML pipelines:
//! 1. Markdown - Structured format with YAML frontmatter
//! 2. JSON - Machine-readable format for APIs
//! 3. Contextual - Natural language format for LLM prompts
//!
//! Each format is optimized for different use cases in AI/ML workflows.

use oxidize_pdf::ai::{ContextualFormat, DocumentMetadata, MarkdownExporter};

#[cfg(feature = "semantic")]
use oxidize_pdf::ai::{DocumentChunker, JsonExporter};

fn main() -> oxidize_pdf::Result<()> {
    println!("=== LLM-Optimized Export Formats Demo ===\n");

    // Sample document data
    let metadata = DocumentMetadata {
        title: "Quarterly Financial Report".to_string(),
        page_count: 3,
        created_at: Some("2025-10-13".to_string()),
        author: Some("Finance Team".to_string()),
    };

    let pages = vec![
        (
            1,
            "Executive Summary\n\nThis quarter shows strong growth across all sectors.".to_string(),
        ),
        (
            2,
            "Revenue Analysis\n\nTotal revenue increased by 25% compared to last quarter."
                .to_string(),
        ),
        (
            3,
            "Conclusion\n\nWe recommend continued investment in key growth areas.".to_string(),
        ),
    ];

    // Simple text for basic examples
    let simple_text = "This is a sample document for AI/ML processing.";

    // ========================================
    // 1. MARKDOWN EXPORT
    // ========================================
    println!("1. MARKDOWN EXPORT");
    println!("==================\n");

    // Simple markdown
    println!("--- Simple Markdown ---");
    let md_simple = MarkdownExporter::export_text(simple_text)?;
    println!("{}\n", md_simple);

    // Markdown with metadata
    println!("--- Markdown with Metadata ---");
    let md_with_meta = MarkdownExporter::export_with_metadata(simple_text, &metadata)?;
    println!("{}\n", md_with_meta);

    // Multi-page markdown
    println!("--- Multi-Page Markdown ---");
    let md_pages = MarkdownExporter::export_with_pages(&pages)?;
    println!("{}\n", md_pages);

    // ========================================
    // 2. JSON EXPORT (requires 'semantic' feature)
    // ========================================
    #[cfg(feature = "semantic")]
    {
        println!("\n2. JSON EXPORT");
        println!("==============\n");

        // Simple JSON
        println!("--- Simple JSON ---");
        let json_simple = JsonExporter::export_simple(simple_text)?;
        println!("{}\n", json_simple);

        // JSON with metadata
        println!("--- JSON with Metadata ---");
        let json_with_meta = JsonExporter::export_with_metadata(simple_text, &metadata)?;
        println!("{}\n", json_with_meta);

        // JSON with pages
        println!("--- JSON with Pages ---");
        let json_pages = JsonExporter::export_pages(&pages)?;
        println!("{}\n", json_pages);

        // JSON with chunks (for RAG pipelines)
        println!("--- JSON with Chunks (RAG) ---");
        let chunker = DocumentChunker::new(512, 50);
        let long_text = "This is a longer document that will be chunked for RAG pipelines. \
                         It contains multiple sentences that will be split into manageable chunks \
                         for embedding and retrieval. Each chunk will have metadata about its position \
                         in the original document.";
        let chunks = chunker.chunk_text(long_text)?;
        let json_chunks = JsonExporter::export_with_chunks(&chunks)?;
        println!("{}\n", json_chunks);
    }

    // ========================================
    // 3. CONTEXTUAL FORMAT
    // ========================================
    println!("\n3. CONTEXTUAL FORMAT (LLM Prompt Injection)");
    println!("============================================\n");

    // Simple contextual
    println!("--- Simple Contextual ---");
    let ctx_simple = ContextualFormat::export_simple(simple_text)?;
    println!("{}\n", ctx_simple);

    // Contextual with metadata
    println!("--- Contextual with Metadata ---");
    let ctx_with_meta = ContextualFormat::export_with_metadata(simple_text, &metadata)?;
    println!("{}\n", ctx_with_meta);

    // Multi-page contextual
    println!("--- Multi-Page Contextual ---");
    let ctx_pages = ContextualFormat::export_with_pages(&pages)?;
    println!("{}\n", ctx_pages);

    // Full contextual (metadata + pages)
    println!("--- Full Contextual (Metadata + Pages) ---");
    let ctx_full = ContextualFormat::export_with_metadata_and_pages(&pages, &metadata)?;
    println!("{}\n", ctx_full);

    // ========================================
    // 4. USE CASE COMPARISON
    // ========================================
    println!("\n4. USE CASE COMPARISON");
    println!("======================\n");

    println!("📝 MARKDOWN:");
    println!("  - Best for: Human-readable documents, documentation");
    println!("  - Features: YAML frontmatter, structured headers");
    println!("  - Use when: You need both human and LLM readability\n");

    #[cfg(feature = "semantic")]
    println!("🔧 JSON:");
    println!("  - Best for: API integration, structured data processing");
    println!("  - Features: Machine-parseable, type-safe structure");
    println!("  - Use when: Feeding data into pipelines or APIs\n");

    println!("💬 CONTEXTUAL:");
    println!("  - Best for: Direct LLM prompt injection, Q&A systems");
    println!("  - Features: Natural language, conversational style");
    println!("  - Use when: LLM needs to understand document context naturally\n");

    // ========================================
    // 5. EXAMPLE: LLM PROMPT CONSTRUCTION
    // ========================================
    println!("\n5. EXAMPLE: LLM PROMPT CONSTRUCTION");
    println!("====================================\n");

    let document_context = ContextualFormat::export_with_metadata_and_pages(&pages, &metadata)?;

    let llm_prompt = format!(
        "You are a financial analyst. Below is a financial report.\n\n\
         {}\n\n\
         Question: What is the revenue growth percentage?\n\n\
         Please provide a concise answer based on the document.",
        document_context
    );

    println!("--- Complete LLM Prompt ---");
    println!("{}\n", llm_prompt);

    println!("✅ All export formats demonstrated successfully!");
    println!("\nNext steps:");
    println!("  1. Choose the format that fits your use case");
    println!("  2. Integrate with your LLM API (OpenAI, Anthropic, etc.)");
    println!("  3. Process PDF documents and export to your chosen format");
    println!("  4. Use chunking for large documents (RAG pipelines)");

    Ok(())
}
