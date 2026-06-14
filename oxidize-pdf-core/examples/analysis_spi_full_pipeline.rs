//! Analysis SPI — a full pipeline: classify → chunk on the labels → enrich.
//!
//! This mirrors how a closed, domain-specific crate (think `oxidize-legal`)
//! would extend the RAG pipeline WITHOUT forking the MIT core: it plugs in three
//! seams and the core never learns the domain semantics — it only transports the
//! opaque `ClassLabel` string and the open `extra` bag.
//!
//!   1. `ElementClassifier` labels each element before chunking
//!      (here: paragraphs starting with "CLAUSE" → "clause").
//!   2. `ChunkingStrategy` reads the labels to decide boundaries
//!      (here: every clause starts a new chunk; other text appends to it).
//!   3. `MetadataEnricher` writes provider-specific fields into `extra`
//!      (here: `legal.is_clause` and `legal.clause_number`).
//!
//! Run with:
//!   cargo run --example analysis_spi_full_pipeline --features "unstable-spi semantic"

#[cfg(all(feature = "unstable-spi", feature = "semantic"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::parser::{PdfDocument, PdfReader};
    use oxidize_pdf::pipeline::{
        AnalysisPipeline, ChunkGroup, ChunkMetadata, ChunkingStrategy, ClassLabel, ClassifyContext,
        Element, ElementClassifier, EnrichContext, MetadataEnricher,
    };
    use oxidize_pdf::text::Font;
    use oxidize_pdf::{Document, Page};
    use std::io::Cursor;

    // ---- Build a throwaway "contract" PDF -------------------------------
    // One section per page. The partitioner merges adjacent same-font lines
    // within a page into a single element but never across pages, so each page
    // yields one element whose text starts with the section's leading line —
    // exactly what the clause classifier keys on.
    let pdf_bytes = {
        let mut doc = Document::new();
        let pages: [&[&str]; 3] = [
            &["This Agreement is entered into by the parties named below."],
            &[
                "CLAUSE 1 Each party shall perform its obligations in good faith.",
                "The foregoing applies to all schedules attached to this Agreement.",
            ],
            &[
                "CLAUSE 2 Either party may terminate on thirty days written notice.",
                "Notices must be delivered to the registered address of record.",
            ],
        ];
        for lines in pages {
            let mut page = Page::a4();
            let mut y = 780.0;
            for line in lines {
                page.text()
                    .set_font(Font::Helvetica, 11.0)
                    .at(50.0, y)
                    .write(line)?;
                y -= 25.0;
            }
            doc.add_page(page);
        }
        doc.to_bytes()?
    };

    // ---- Seam 1: classify elements --------------------------------------
    struct ClauseClassifier;
    impl ElementClassifier for ClauseClassifier {
        fn classify(&self, element: &Element, _ctx: &ClassifyContext) -> Option<ClassLabel> {
            element
                .text()
                .starts_with("CLAUSE")
                .then(|| ClassLabel::new("clause"))
        }
    }

    // ---- Seam 2: chunk on the labels ------------------------------------
    // A clause-labelled element opens a new chunk; unlabelled text appends to
    // the open chunk. The built-in HybridChunker cannot do this — it has no
    // notion of "clause".
    struct SplitOnClause;
    impl ChunkingStrategy for SplitOnClause {
        fn chunk(&self, elements: &[Element]) -> Vec<ChunkGroup> {
            let mut groups: Vec<ChunkGroup> = Vec::new();
            for e in elements {
                let opens_clause = e.metadata().class_label.as_deref() == Some("clause");
                if opens_clause || groups.is_empty() {
                    groups.push(ChunkGroup::new(vec![e.clone()], None));
                } else {
                    groups.last_mut().unwrap().elements.push(e.clone());
                }
            }
            groups
        }
    }

    // ---- Seam 3: enrich `extra` -----------------------------------------
    struct ClauseEnricher;
    impl MetadataEnricher for ClauseEnricher {
        fn enrich(&self, ctx: &EnrichContext, meta: &mut ChunkMetadata) {
            let is_clause = ctx
                .elements
                .iter()
                .any(|e| e.metadata().class_label.as_deref() == Some("clause"));
            meta.extra
                .insert("legal.is_clause".to_string(), serde_json::json!(is_clause));
            // Pull the clause number out of "CLAUSE <n> ..." if present.
            if let Some(num) = ctx
                .elements
                .iter()
                .find_map(|e| e.text().strip_prefix("CLAUSE "))
                .and_then(|rest| rest.split_whitespace().next())
            {
                meta.extra
                    .insert("legal.clause_number".to_string(), serde_json::json!(num));
            }
        }
    }

    // ---- Assemble and run the pipeline ----------------------------------
    let pipeline = AnalysisPipeline::new()
        .with_classifier(Box::new(ClauseClassifier))
        .with_chunking(Box::new(SplitOnClause))
        .with_enricher(Box::new(ClauseEnricher));

    let doc = PdfDocument::new(PdfReader::new(Cursor::new(&pdf_bytes))?);
    let chunks = doc.rag_chunks_with_pipeline(&pipeline)?;

    println!(
        "{} chunk(s) produced by the legal-style pipeline:\n",
        chunks.len()
    );
    for c in &chunks {
        let is_clause = c
            .metadata
            .extra
            .get("legal.is_clause")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let number = c
            .metadata
            .extra
            .get("legal.clause_number")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        println!(
            "• clause={is_clause:<5} number={number:<3} text={:?}",
            c.text
        );
        // Show the serialized metadata.extra a RAG store would receive.
        println!("    extra = {}", serde_json::to_string(&c.metadata.extra)?);
    }

    // The preamble (no clause) is one chunk; each CLAUSE opens another →
    // exactly three chunks, two of them clause-labelled.
    let clause_chunks = chunks
        .iter()
        .filter(|c| c.metadata.extra.get("legal.is_clause") == Some(&serde_json::json!(true)))
        .count();
    assert_eq!(chunks.len(), 3);
    assert_eq!(clause_chunks, 2);
    Ok(())
}

#[cfg(not(all(feature = "unstable-spi", feature = "semantic")))]
fn main() {
    eprintln!("This example requires the `unstable-spi` and `semantic` features:");
    eprintln!(
        "  cargo run --example analysis_spi_full_pipeline --features \"unstable-spi semantic\""
    );
}
