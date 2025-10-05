//! Basic Document Chunking Example
//!
//! This example demonstrates how to chunk a PDF document for use with
//! Large Language Models (LLMs) in RAG (Retrieval Augmented Generation) pipelines.
//!
//! Run with:
//! ```bash
//! cargo run --example basic_chunking
//! ```

use oxidize_pdf::ai::DocumentChunker;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::{Document, Font, Page, Result};

fn main() -> Result<()> {
    println!("🤖 Document Chunking for RAG Example\n");
    println!("Creating a sample multi-page PDF document...\n");

    // Create a sample document with multiple pages
    let mut doc = create_sample_document()?;

    // Save the document first
    let pdf_path = "examples/results/sample_for_chunking.pdf";
    doc.save(pdf_path)?;
    println!("✅ Created sample PDF: {}\n", pdf_path);

    // Re-open the document for chunking using the parser
    let reader = PdfReader::open(pdf_path)?;
    let pdf_doc = PdfDocument::new(reader);

    // Extract text from the PDF
    let text_pages = pdf_doc.extract_text()?;
    let full_text = text_pages
        .iter()
        .map(|page| page.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");

    // Create a chunker with typical settings for GPT-3.5/4
    // 512 tokens per chunk, 50 tokens overlap
    println!("📊 Chunking Configuration:");
    println!("   Chunk size: 512 tokens");
    println!("   Overlap: 50 tokens\n");

    let chunker = DocumentChunker::new(512, 50);

    // Chunk the text directly
    println!("🔪 Chunking document...");
    let chunks = chunker.chunk_text(&full_text)?;

    println!("✅ Created {} chunks\n", chunks.len());

    // Display information about each chunk
    println!("📄 Chunk Details:");
    println!("{:-<80}", "");

    for (i, chunk) in chunks.iter().enumerate().take(5) {
        println!("Chunk {}: {}", i, chunk.id);
        println!("  Tokens: {}", chunk.tokens);
        println!("  Content length: {} chars", chunk.content.len());

        // Show first 100 characters of content
        let preview = if chunk.content.len() > 100 {
            format!("{}...", &chunk.content[..100])
        } else {
            chunk.content.clone()
        };
        println!("  Preview: {}", preview.replace('\n', " "));
        println!("{:-<80}", "");
    }

    if chunks.len() > 5 {
        println!("... ({} more chunks)", chunks.len() - 5);
        println!("{:-<80}", "");
    }

    // Show overlap verification between first two chunks (if they exist)
    if chunks.len() >= 2 {
        println!("\n🔗 Overlap Verification:");
        let chunk0_words: Vec<&str> = chunks[0].content.split_whitespace().collect();
        let chunk1_words: Vec<&str> = chunks[1].content.split_whitespace().collect();

        if chunk0_words.len() >= 10 && chunk1_words.len() >= 10 {
            let chunk0_end: Vec<_> = chunk0_words.iter().rev().take(5).rev().collect();
            let chunk1_start: Vec<_> = chunk1_words.iter().take(5).collect();

            println!("Last 5 words of Chunk 0: {:?}", chunk0_end);
            println!("First 5 words of Chunk 1: {:?}", chunk1_start);

            let overlap_exists = chunk0_end.iter().any(|w| chunk1_start.contains(w));
            if overlap_exists {
                println!("✅ Overlap detected between chunks!");
            } else {
                println!("⚠️  No overlap detected (may be expected for large chunks)");
            }
        }
    }

    // Show token estimation
    println!("\n📈 Token Estimation:");
    let sample_text = "The quick brown fox jumps over the lazy dog";
    let estimated = DocumentChunker::estimate_tokens(sample_text);
    println!("Text: \"{}\"", sample_text);
    println!("Estimated tokens: {}", estimated);
    println!("(Approximation: ~1.33 tokens per word)");

    // Show typical use case
    println!("\n💡 Typical RAG Pipeline Usage:");
    println!("1. Chunk document (✅ Done!)");
    println!("2. Generate embeddings for each chunk (use OpenAI/Cohere API)");
    println!("3. Store chunks + embeddings in vector database (Pinecone/Qdrant)");
    println!("4. Query with user question");
    println!("5. Retrieve relevant chunks");
    println!("6. Send chunks + question to LLM for answer generation");

    println!("\n✅ Document chunking complete!");

    Ok(())
}

/// Create a sample multi-page document for demonstration
fn create_sample_document() -> Result<Document> {
    let mut doc = Document::new();
    doc.set_title("Sample Document for Chunking");
    doc.set_author("oxidize-pdf");

    // Page 1: Introduction
    let mut page1 = Page::a4();
    page1
        .text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("Document Chunking for RAG")?;

    page1
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 700.0)
        .write("This document demonstrates how PDF content is split into chunks suitable")?;

    page1
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 685.0)
        .write("for Large Language Model processing. Each chunk contains a portion of the")?;

    page1
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 670.0)
        .write("document with overlap to preserve context between chunks. This is crucial")?;

    page1
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 655.0)
        .write("for Retrieval Augmented Generation (RAG) systems where documents are")?;

    page1
        .text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 640.0)
        .write("indexed in vector databases and retrieved based on semantic similarity.")?;

    doc.add_page(page1);

    // Page 2: Technical Details
    let mut page2 = Page::a4();
    page2
        .text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 750.0)
        .write("Technical Details")?;

    let mut y = 720.0;
    let details = [
        "The chunking algorithm splits text into fixed-size pieces measured in tokens.",
        "Tokens are approximate word-level units used by LLMs for processing.",
        "An overlap between chunks ensures context is preserved across boundaries.",
        "Typical chunk sizes range from 256 to 2048 tokens depending on the LLM.",
        "GPT-3.5 and GPT-4 work well with 512-token chunks and 50-token overlap.",
        "Larger models like Claude can handle chunks up to 4096 tokens or more.",
        "The chunking process preserves document structure and maintains readability.",
    ];

    for detail in &details {
        page2
            .text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, y)
            .write(detail)?;
        y -= 20.0;
    }

    doc.add_page(page2);

    // Page 3: More content to ensure multiple chunks
    let mut page3 = Page::a4();
    page3
        .text()
        .set_font(Font::HelveticaBold, 14.0)
        .at(50.0, 750.0)
        .write("Additional Content")?;

    let mut y = 720.0;
    for i in 0..30 {
        page3
            .text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, y)
            .write(&format!(
                "Line {}: This is additional content to ensure we have enough text for multiple chunks.",
                i + 1
            ))?;
        y -= 15.0;
        if y < 50.0 {
            break;
        }
    }

    doc.add_page(page3);

    Ok(doc)
}
