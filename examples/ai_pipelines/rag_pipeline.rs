//! Advanced RAG Pipeline Example
//!
//! This example demonstrates a complete RAG (Retrieval Augmented Generation) pipeline
//! using oxidize-pdf for document chunking, including:
//! - PDF loading and text extraction with page tracking
//! - Intelligent chunking with sentence boundaries
//! - Mock embedding generation (in production, use OpenAI/Cohere API)
//! - Vector store preparation format (Pinecone/Qdrant compatible)
//! - Query and retrieval simulation
//!
//! Run with:
//! ```bash
//! cargo run --example rag_pipeline
//! ```

use oxidize_pdf::ai::DocumentChunker;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::{Document, Font, Page, Result};
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🤖 Advanced RAG Pipeline Example\n");
    println!("{}", "=".repeat(80));

    // Step 1: Create and save a sample multi-page document
    println!("\n📄 Step 1: Creating sample document...");
    let mut doc = create_comprehensive_document()?;
    let pdf_path = "examples/results/rag_pipeline_demo.pdf";
    doc.save(pdf_path)?;
    println!("✅ Created: {}", pdf_path);

    // Step 2: Load and extract text with page tracking
    println!("\n📖 Step 2: Extracting text from PDF with page tracking...");
    let reader = PdfReader::open(pdf_path)?;
    let pdf_doc = PdfDocument::new(reader);
    let text_pages = pdf_doc.extract_text()?;

    // Convert to format suitable for chunking with page numbers
    let page_texts: Vec<(usize, String)> = text_pages
        .iter()
        .enumerate()
        .map(|(idx, page)| (idx + 1, page.text.clone()))
        .collect();

    println!("✅ Extracted {} pages", page_texts.len());

    // Step 3: Chunk document with intelligent boundaries
    println!("\n🔪 Step 3: Chunking document (512 tokens, 50 overlap)...");
    let chunker = DocumentChunker::new(512, 50);
    let chunks = chunker.chunk_text_with_pages(&page_texts)?;
    println!("✅ Created {} chunks", chunks.len());

    // Step 4: Simulate embedding generation (in production, use OpenAI/Cohere)
    println!("\n🧮 Step 4: Generating embeddings (simulated)...");
    let embeddings = generate_mock_embeddings(&chunks);
    println!(
        "✅ Generated {} embeddings (dimensions: 1536)",
        embeddings.len()
    );

    // Step 5: Prepare data for vector store (Pinecone/Qdrant format)
    println!("\n💾 Step 5: Preparing vector store format...");
    let vector_records = prepare_for_vector_store(&chunks, &embeddings)?;
    println!("✅ Prepared {} vector records", vector_records.len());

    // Step 6: Display vector store records
    println!("\n📊 Vector Store Records (first 3):");
    println!("{}", "-".repeat(80));
    for (i, record) in vector_records.iter().take(3).enumerate() {
        println!("\nRecord {}:", i);
        println!("  ID: {}", record.id);
        println!("  Pages: {:?}", record.metadata.get("pages"));
        println!(
            "  Chunk index: {}",
            record.metadata.get("chunk_index").unwrap()
        );
        println!("  Tokens: {}", record.metadata.get("tokens").unwrap());
        println!(
            "  Sentence boundary: {}",
            record.metadata.get("sentence_boundary").unwrap()
        );
        println!(
            "  Text preview: {}...",
            record.text.chars().take(100).collect::<String>()
        );
        println!("  Embedding dims: {}", record.embedding.len());
    }

    // Step 7: Simulate RAG query
    println!("\n{}", "=".repeat(80));
    println!("\n🔍 Step 7: Simulating RAG Query...");
    let query = "What are the key features of document chunking?";
    println!("Query: \"{}\"", query);

    // In production: generate query embedding with same model as documents
    let query_embedding = vec![0.1; 1536]; // Mock embedding

    // Simulate similarity search (in production: use vector DB)
    let top_k = 3;
    let relevant_chunks = simulate_similarity_search(&query_embedding, &vector_records, top_k);

    println!("\n📚 Top {} relevant chunks:", top_k);
    println!("{}", "-".repeat(80));
    for (i, (record, score)) in relevant_chunks.iter().enumerate() {
        println!("\n{}. Score: {:.3}", i + 1, score);
        println!("   Pages: {:?}", record.metadata.get("pages"));
        println!(
            "   Text: {}",
            record.text.chars().take(150).collect::<String>() + "..."
        );
    }

    // Step 8: Show typical LLM prompt format
    println!("\n{}", "=".repeat(80));
    println!("\n💬 Step 8: LLM Prompt Format");
    println!("{}", "-".repeat(80));

    let context = relevant_chunks
        .iter()
        .map(|(r, _)| r.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");

    let prompt = format!(
        "Context:\n{}\n\nQuestion: {}\n\nAnswer based on the context above:",
        context, query
    );

    println!("{}", prompt.chars().take(500).collect::<String>() + "...");

    // Summary
    println!("\n{}", "=".repeat(80));
    println!("\n✅ RAG Pipeline Complete!");
    println!("\n📈 Pipeline Summary:");
    println!("  • Pages processed: {}", page_texts.len());
    println!("  • Chunks created: {}", chunks.len());
    println!("  • Embeddings generated: {}", embeddings.len());
    println!("  • Vector records: {}", vector_records.len());
    println!("  • Query results: {} relevant chunks", top_k);

    println!("\n💡 Next Steps:");
    println!("  1. Replace mock embeddings with real API (OpenAI/Cohere)");
    println!("  2. Index vectors in actual vector DB (Pinecone/Qdrant/Weaviate)");
    println!("  3. Implement real similarity search");
    println!("  4. Send context + query to LLM for answer generation");
    println!("  5. Add metadata filtering (by page, date, etc.)");

    Ok(())
}

/// Create a comprehensive multi-page document for RAG demo
fn create_comprehensive_document() -> Result<Document> {
    let mut doc = Document::new();
    doc.set_title("RAG Pipeline Documentation");
    doc.set_author("oxidize-pdf");

    // Page 1: Introduction to Document Chunking
    let mut page1 = Page::a4();
    page1
        .text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 750.0)
        .write("Document Chunking for RAG Systems")?;

    let intro_text = [
        "Document chunking is the process of splitting long documents into smaller,",
        "manageable pieces suitable for Large Language Model processing. This is crucial",
        "for RAG (Retrieval Augmented Generation) systems where documents need to be",
        "indexed in vector databases and retrieved based on semantic similarity.",
        "",
        "Key features of effective document chunking include:",
        "• Fixed-size chunks with configurable token limits",
        "• Overlap between chunks to preserve context",
        "• Respect for sentence boundaries to avoid breaking mid-sentence",
        "• Rich metadata for accurate retrieval and provenance tracking",
    ];

    let mut y = 700.0;
    for line in &intro_text {
        page1
            .text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, y)
            .write(line)?;
        y -= 18.0;
    }

    doc.add_page(page1);

    // Page 2: Technical Implementation
    let mut page2 = Page::a4();
    page2
        .text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Technical Implementation")?;

    let tech_text = [
        "The chunking algorithm follows these steps:",
        "",
        "1. Text Extraction: Extract text from PDF preserving page information",
        "2. Tokenization: Split text into tokens (words) for accurate counting",
        "3. Chunk Creation: Create fixed-size chunks with specified overlap",
        "4. Boundary Detection: Adjust chunk boundaries to respect sentences",
        "5. Metadata Enrichment: Add page numbers, position, and confidence scores",
        "",
        "Performance considerations:",
        "• Linear scaling with document size",
        "• Efficient memory usage with streaming processing",
        "• Optimized for documents up to 1000+ pages",
        "• Target performance: <100ms for 100 pages",
    ];

    y = 700.0;
    for line in &tech_text {
        page2
            .text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, y)
            .write(line)?;
        y -= 16.0;
    }

    doc.add_page(page2);

    // Page 3: Integration with Vector Stores
    let mut page3 = Page::a4();
    page3
        .text()
        .set_font(Font::HelveticaBold, 16.0)
        .at(50.0, 750.0)
        .write("Vector Store Integration")?;

    let vector_text = [
        "After chunking, documents are typically indexed in vector databases.",
        "The process involves:",
        "",
        "1. Generate embeddings for each chunk using an embedding model",
        "2. Store embeddings along with metadata in a vector database",
        "3. Enable similarity search for query-time retrieval",
        "4. Return relevant chunks with metadata for LLM context",
        "",
        "Supported vector stores include Pinecone, Qdrant, Weaviate, and Chroma.",
        "Each chunk includes metadata like page numbers, position, and confidence",
        "scores to enable precise provenance tracking and result filtering.",
    ];

    y = 700.0;
    for line in &vector_text {
        page3
            .text()
            .set_font(Font::Helvetica, 11.0)
            .at(50.0, y)
            .write(line)?;
        y -= 16.0;
    }

    doc.add_page(page3);

    Ok(doc)
}

/// Generate mock embeddings (in production, use OpenAI/Cohere API)
fn generate_mock_embeddings(chunks: &[oxidize_pdf::ai::DocumentChunk]) -> Vec<Vec<f32>> {
    chunks
        .iter()
        .map(|_chunk| {
            // In production: openai_api.create_embedding(chunk.content)
            // For now, return mock 1536-dimensional embedding (OpenAI text-embedding-ada-002 size)
            vec![0.1; 1536]
        })
        .collect()
}

/// Vector store record (Pinecone/Qdrant compatible format)
#[derive(Debug, Clone)]
struct VectorRecord {
    id: String,
    embedding: Vec<f32>,
    text: String,
    metadata: HashMap<String, String>,
}

/// Prepare chunks for vector store ingestion
fn prepare_for_vector_store(
    chunks: &[oxidize_pdf::ai::DocumentChunk],
    embeddings: &[Vec<f32>],
) -> Result<Vec<VectorRecord>> {
    let records: Vec<VectorRecord> = chunks
        .iter()
        .zip(embeddings.iter())
        .map(|(chunk, embedding)| {
            let mut metadata = HashMap::new();
            metadata.insert("chunk_index".to_string(), chunk.chunk_index.to_string());
            metadata.insert("tokens".to_string(), chunk.tokens.to_string());
            metadata.insert("pages".to_string(), format!("{:?}", chunk.page_numbers));
            metadata.insert(
                "first_page".to_string(),
                chunk.metadata.position.first_page.to_string(),
            );
            metadata.insert(
                "last_page".to_string(),
                chunk.metadata.position.last_page.to_string(),
            );
            metadata.insert(
                "sentence_boundary".to_string(),
                chunk.metadata.sentence_boundary_respected.to_string(),
            );
            metadata.insert(
                "confidence".to_string(),
                chunk.metadata.confidence.to_string(),
            );

            VectorRecord {
                id: chunk.id.clone(),
                embedding: embedding.clone(),
                text: chunk.content.clone(),
                metadata,
            }
        })
        .collect();

    Ok(records)
}

/// Simulate similarity search (in production, use vector DB)
fn simulate_similarity_search(
    _query_embedding: &[f32],
    records: &[VectorRecord],
    top_k: usize,
) -> Vec<(VectorRecord, f32)> {
    // In production: vector_db.search(query_embedding, top_k)
    // For now, return first top_k records with mock scores
    records
        .iter()
        .take(top_k)
        .enumerate()
        .map(|(i, record)| (record.clone(), 0.95 - (i as f32 * 0.05)))
        .collect()
}
