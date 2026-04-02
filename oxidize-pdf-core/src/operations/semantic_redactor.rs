//! Semantic redactor for RAG-aligned PDF editing
//!
//! Draws opaque rectangles over content identified by `SemanticEntity` bounding boxes,
//! removing sensitive information (PII, confidential data) before LLM ingestion while
//! preserving document structure for retrieval.

use std::collections::HashMap;
use std::io::Cursor;

use crate::graphics::Color;
use crate::semantic::{EntityType, SemanticEntity};
use crate::text::Font;

/// Visual style for redacted regions.
#[derive(Debug, Clone)]
pub enum RedactionStyle {
    /// Opaque black rectangle covering the content
    BlackBox,
    /// Black rectangle with white placeholder text on top
    Placeholder(String),
}

impl Default for RedactionStyle {
    fn default() -> Self {
        Self::BlackBox
    }
}

/// Configuration for what and how to redact.
#[derive(Debug, Clone)]
pub struct RedactionConfig {
    /// Entity types to redact (empty = redact nothing)
    pub entity_types: Vec<EntityType>,
    /// Visual style for redacted areas
    pub style: RedactionStyle,
}

impl Default for RedactionConfig {
    fn default() -> Self {
        Self {
            entity_types: Vec::new(),
            style: RedactionStyle::BlackBox,
        }
    }
}

impl RedactionConfig {
    /// Create a new empty config.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set entity types to redact.
    pub fn with_types(mut self, types: Vec<EntityType>) -> Self {
        self.entity_types = types;
        self
    }

    /// Set the redaction style.
    pub fn with_style(mut self, style: RedactionStyle) -> Self {
        self.style = style;
        self
    }
}

/// A record of a single redaction applied.
#[derive(Debug, Clone)]
pub struct RedactionEntry {
    /// ID of the redacted entity
    pub entity_id: String,
    /// Type of the redacted entity
    pub entity_type: EntityType,
    /// Page number (1-indexed, matching BoundingBox convention)
    pub page: u32,
}

/// Report of all redactions applied to a document.
#[derive(Debug)]
pub struct RedactionReport {
    entries: Vec<RedactionEntry>,
}

impl RedactionReport {
    /// Total number of redactions applied.
    pub fn redacted_count(&self) -> usize {
        self.entries.len()
    }

    /// Filter entries by entity type.
    pub fn by_type(&self, entity_type: &EntityType) -> Vec<&RedactionEntry> {
        self.entries
            .iter()
            .filter(|e| &e.entity_type == entity_type)
            .collect()
    }

    /// Unique pages affected by redactions (1-indexed).
    pub fn pages_affected(&self) -> Vec<u32> {
        let mut pages: Vec<u32> = self.entries.iter().map(|e| e.page).collect();
        pages.sort();
        pages.dedup();
        pages
    }

    /// All entries in the report.
    pub fn entries(&self) -> &[RedactionEntry] {
        &self.entries
    }
}

/// Errors that can occur during semantic redaction.
#[derive(Debug, thiserror::Error)]
pub enum SemanticRedactorError {
    /// Failed to parse the input PDF
    #[error("parse failed: {0}")]
    ParseFailed(String),

    /// Failed to reconstruct a page
    #[error("page reconstruction failed: {0}")]
    PageReconstructionFailed(String),

    /// Failed to write the output PDF
    #[error("write failed: {0}")]
    WriteFailed(String),
}

/// Result type for semantic redactor operations.
pub type SemanticRedactorResult<T> = Result<T, SemanticRedactorError>;

/// Redacts sensitive content from PDFs based on semantic entity bounding boxes.
///
/// Given PDF bytes and a set of `SemanticEntity`s with bounding boxes, this
/// draws opaque rectangles over the specified entity types, producing a
/// redacted PDF suitable for LLM ingestion.
pub struct SemanticRedactor;

impl SemanticRedactor {
    /// Redact entities from a PDF, returning the modified bytes and a report.
    ///
    /// # Arguments
    ///
    /// * `pdf_bytes` - The original PDF file bytes
    /// * `entities` - Semantic entities with bounding boxes
    /// * `config` - What to redact and how
    ///
    /// # Returns
    ///
    /// A tuple of (modified PDF bytes, redaction report).
    pub fn redact(
        pdf_bytes: &[u8],
        entities: &[SemanticEntity],
        config: RedactionConfig,
    ) -> SemanticRedactorResult<(Vec<u8>, RedactionReport)> {
        // Filter entities by configured types
        let to_redact: Vec<&SemanticEntity> = if config.entity_types.is_empty() {
            Vec::new()
        } else {
            entities
                .iter()
                .filter(|e| config.entity_types.contains(&e.entity_type))
                .collect()
        };

        // If nothing to redact, return original bytes
        if to_redact.is_empty() {
            return Ok((
                pdf_bytes.to_vec(),
                RedactionReport {
                    entries: Vec::new(),
                },
            ));
        }

        // Group entities by page (BoundingBox.page is 1-indexed)
        let mut by_page: HashMap<u32, Vec<&SemanticEntity>> = HashMap::new();
        for entity in &to_redact {
            by_page.entry(entity.bounds.page).or_default().push(entity);
        }

        // Parse the PDF
        let cursor = Cursor::new(pdf_bytes);
        let reader = crate::parser::PdfReader::new(cursor)
            .map_err(|e| SemanticRedactorError::ParseFailed(e.to_string()))?;
        let document = reader.into_document();

        let page_count = document
            .page_count()
            .map_err(|e| SemanticRedactorError::PageReconstructionFailed(e.to_string()))?;

        let mut output_doc = crate::document::Document::new();
        let mut report_entries = Vec::new();

        for page_idx in 0..page_count {
            let parsed_page = document
                .get_page(page_idx)
                .map_err(|e| SemanticRedactorError::PageReconstructionFailed(e.to_string()))?;

            let mut page = crate::page::Page::from_parsed_with_content(&parsed_page, &document)
                .map_err(|e| SemanticRedactorError::PageReconstructionFailed(e.to_string()))?;

            // page_idx is 0-indexed, BoundingBox.page is 1-indexed
            let page_num_1indexed = (page_idx + 1) as u32;

            if let Some(page_entities) = by_page.get(&page_num_1indexed) {
                for entity in page_entities {
                    let bbox = &entity.bounds;

                    // Draw opaque black rectangle over the entity
                    page.graphics()
                        .set_fill_color(Color::black())
                        .rect(
                            bbox.x as f64,
                            bbox.y as f64,
                            bbox.width as f64,
                            bbox.height as f64,
                        )
                        .fill();

                    // If placeholder style, add white text on top
                    if let RedactionStyle::Placeholder(ref text) = config.style {
                        let font_size = (bbox.height as f64 * 0.6).min(10.0).max(4.0);
                        let text_ctx = page.text();
                        text_ctx.set_font(Font::Helvetica, font_size);
                        text_ctx.set_fill_color(Color::white());
                        text_ctx.at(
                            bbox.x as f64 + 2.0,
                            bbox.y as f64 + (bbox.height as f64 - font_size) / 2.0,
                        );
                        let _ = text_ctx.write(text);
                    }

                    report_entries.push(RedactionEntry {
                        entity_id: entity.id.clone(),
                        entity_type: entity.entity_type.clone(),
                        page: page_num_1indexed,
                    });
                }
            }

            output_doc.add_page(page);
        }

        let output_bytes = output_doc
            .to_bytes()
            .map_err(|e| SemanticRedactorError::WriteFailed(e.to_string()))?;

        Ok((
            output_bytes,
            RedactionReport {
                entries: report_entries,
            },
        ))
    }
}
