//! Semantic visual masking for RAG-aligned PDF editing.
//!
//! This module's legacy operation draws opaque rectangles over content identified
//! by `SemanticEntity` bounding boxes. It does **not** remove underlying content
//! and must not be used as irreversible or security-grade redaction.

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};

use crate::fonts::Standard14Font;
use crate::graphics::Color;
use crate::parser::content::{ContentOperation, ContentParser};
use crate::parser::objects::{PdfDictionary, PdfObject};
use crate::semantic::{EntityType, SemanticEntity};
use crate::text::Font;

#[derive(Debug, Clone)]
struct VerifiedTextFont {
    base_font: String,
    encoding: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct SecureTextState {
    font: Option<(String, f32)>,
    character_spacing: f32,
    word_spacing: f32,
}

const SECURE_ENTITY_LIMIT: usize = 10_000;
const SECURE_PAGE_LIMIT: u32 = 10_000;
const SECURE_PAGE_ENTITY_LIMIT: usize = 256;
const SECURE_OPERATION_LIMIT: usize = 1_000_000;
const SECURE_INPUT_LIMIT: usize = 256 * 1024 * 1024;
const SECURE_PAGE_CONTENT_LIMIT: usize = 64 * 1024 * 1024;

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
    mode: RedactionMode,
    residual_risks: Vec<String>,
}

/// Security semantics of a redaction result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionMode {
    /// The result only obscures visible content; source data remains recoverable.
    VisualMask,
    /// Targeted data was removed and the configured forensic checks passed.
    Irreversible,
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

    /// Whether the operation only covered content visually or removed it.
    pub fn mode(&self) -> RedactionMode {
        self.mode
    }

    /// Known ways sensitive data may remain recoverable from the result.
    pub fn residual_risks(&self) -> &[String] {
        &self.residual_risks
    }

    /// True only for a completed irreversible-redaction operation.
    pub fn is_irreversible(&self) -> bool {
        self.mode == RedactionMode::Irreversible && self.residual_risks.is_empty()
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

impl SemanticRedactorError {
    #[allow(non_snake_case)]
    fn SecureRedactionUnsupported(reason: String) -> Self {
        Self::ParseFailed(format!("secure redaction refused: {reason}"))
    }
}

/// Result type for semantic redactor operations.
pub type SemanticRedactorResult<T> = Result<T, SemanticRedactorError>;

/// Visually masks content based on semantic entity bounding boxes.
///
/// Given PDF bytes and a set of `SemanticEntity`s with bounding boxes, this
/// draws opaque rectangles over the specified entity types. Underlying text,
/// images, annotations, metadata, attachments, and prior revisions may remain
/// recoverable. Use [`SemanticRedactor::redact_irreversible`] when a security
/// guarantee is required; unsupported documents are rejected without output.
pub struct SemanticRedactor;

impl SemanticRedactor {
    /// Apply visual masks, returning the modified bytes and an explicit risk report.
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
                    mode: RedactionMode::VisualMask,
                    residual_risks: visual_mask_risks(),
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
                mode: RedactionMode::VisualMask,
                residual_risks: visual_mask_risks(),
            },
        ))
    }

    /// Remove targeted information irreversibly or fail without producing output.
    ///
    /// The initial secure engine accepts only exact, non-empty ASCII matches that
    /// occupy a complete literal `Tj` operand or the concatenated literal strings
    /// of one complete `TJ` array in direct page content. `Tj` and `TJ` removal
    /// replaces glyph data with a numeric adjustment that preserves the original advance,
    /// including AFM widths, kerning, character spacing, and word spacing. It
    /// rebuilds the document, dropping prior revisions and document-level
    /// auxiliary data, and verifies that no target remains in any output page text
    /// operation. Annotations, XObjects, inline images, abbreviated text-showing
    /// operators, marked content, ambiguous matches, and malformed streams are
    /// rejected.
    pub fn redact_irreversible(
        pdf_bytes: &[u8],
        entities: &[SemanticEntity],
        config: RedactionConfig,
    ) -> SemanticRedactorResult<(Vec<u8>, RedactionReport)> {
        if pdf_bytes.len() > SECURE_INPUT_LIMIT {
            return secure_unsupported("input exceeds the secure-redaction byte limit");
        }
        if entities.len() > SECURE_ENTITY_LIMIT {
            return secure_unsupported(format!(
                "entity limit exceeded: maximum is {SECURE_ENTITY_LIMIT}"
            ));
        }
        let to_redact: Vec<&SemanticEntity> = entities
            .iter()
            .filter(|entity| config.entity_types.contains(&entity.entity_type))
            .collect();
        if to_redact.is_empty() {
            return secure_unsupported("at least one matching entity is required");
        }
        for entity in &to_redact {
            if entity.content.is_empty() {
                return secure_unsupported(format!("entity '{}' has empty content", entity.id));
            }
            if !entity.content.is_ascii() {
                return secure_unsupported(format!(
                    "entity '{}' is not representable by the initial ASCII text engine",
                    entity.id
                ));
            }
            let bounds = &entity.bounds;
            if bounds.page == 0
                || !bounds.x.is_finite()
                || !bounds.y.is_finite()
                || !bounds.width.is_finite()
                || !bounds.height.is_finite()
                || bounds.width <= 0.0
                || bounds.height <= 0.0
            {
                return secure_unsupported(format!(
                    "entity '{}' has invalid page or bounding box",
                    entity.id
                ));
            }
        }

        let reader = crate::parser::PdfReader::new(Cursor::new(pdf_bytes))
            .map_err(|error| SemanticRedactorError::ParseFailed(error.to_string()))?;
        let document = reader.into_document();
        let page_count = document
            .page_count()
            .map_err(|error| SemanticRedactorError::PageReconstructionFailed(error.to_string()))?;
        if page_count > SECURE_PAGE_LIMIT {
            return secure_unsupported(format!(
                "page limit exceeded: maximum is {SECURE_PAGE_LIMIT}"
            ));
        }
        let mut by_page: HashMap<u32, Vec<&SemanticEntity>> = HashMap::new();
        for entity in &to_redact {
            if entity.bounds.page > page_count {
                return secure_unsupported(format!(
                    "entity '{}' references missing page {}",
                    entity.id, entity.bounds.page
                ));
            }
            by_page.entry(entity.bounds.page).or_default().push(entity);
        }
        if by_page
            .values()
            .any(|page_entities| page_entities.len() > SECURE_PAGE_ENTITY_LIMIT)
        {
            return secure_unsupported(format!(
                "per-page entity limit exceeded: maximum is {SECURE_PAGE_ENTITY_LIMIT}"
            ));
        }

        let mut output_doc = crate::document::Document::new();
        let mut report_entries = Vec::new();
        for page_idx in 0..page_count {
            let parsed_page = document.get_page(page_idx).map_err(|error| {
                SemanticRedactorError::PageReconstructionFailed(error.to_string())
            })?;
            if parsed_page.has_annotations() {
                return secure_unsupported(format!("page {} contains annotations", page_idx + 1));
            }
            if parsed_page
                .get_resources()
                .is_some_and(|resources| resources.get("XObject").is_some())
            {
                return secure_unsupported(format!(
                    "page {} contains XObject resources",
                    page_idx + 1
                ));
            }

            let streams = parsed_page
                .content_streams_with_document(&document)
                .map_err(|error| {
                    SemanticRedactorError::PageReconstructionFailed(error.to_string())
                })?;
            let mut content = Vec::new();
            for stream in streams {
                validate_initial_secure_operations(&stream, page_idx + 1)?;
                if content.len().saturating_add(stream.len()) > SECURE_PAGE_CONTENT_LIMIT {
                    return secure_unsupported(format!(
                        "page {} exceeds the decoded-content byte limit",
                        page_idx + 1
                    ));
                }
                content.extend_from_slice(&stream);
                content.push(b'\n');
            }
            let verified_fonts = verified_ascii_fonts(&parsed_page, &document, page_idx + 1)?;

            if let Some(page_entities) = by_page.get(&(page_idx + 1)) {
                for entity in page_entities {
                    let bounds = &entity.bounds;
                    let media_box = parsed_page.media_box;
                    if f64::from(bounds.x) < media_box[0]
                        || f64::from(bounds.y) < media_box[1]
                        || f64::from(bounds.x + bounds.width) > media_box[2]
                        || f64::from(bounds.y + bounds.height) > media_box[3]
                    {
                        return secure_unsupported(format!(
                            "entity '{}' bounding box lies outside page {} MediaBox",
                            entity.id,
                            page_idx + 1
                        ));
                    }
                    let verified_tj_matches = count_verified_text_matches(
                        &content,
                        entity.content.as_bytes(),
                        &verified_fonts,
                        page_idx + 1,
                    )?;
                    let tj_replacement = find_verified_tj_replacement(
                        &content,
                        entity.content.as_bytes(),
                        &verified_fonts,
                        page_idx + 1,
                    )?;
                    let (rewritten_tj, tj_matches) = remove_exact_literal_tj(
                        &content,
                        entity.content.as_bytes(),
                        tj_replacement,
                    )?;
                    let tj_array_replacement = find_verified_tj_array_replacement(
                        &content,
                        entity.content.as_bytes(),
                        &verified_fonts,
                        page_idx + 1,
                    )?;
                    let total_matches = tj_matches + usize::from(tj_array_replacement.is_some());
                    let verified_matches =
                        verified_tj_matches + usize::from(tj_array_replacement.is_some());
                    if total_matches != 1 || verified_matches != 1 {
                        return secure_unsupported(format!(
                            "entity '{}' must match exactly one complete Tj operand or TJ array using a verified Standard-14 text font; found {} verified of {} total",
                            entity.id, verified_matches, total_matches
                        ));
                    }
                    if !target_intersects_bounds(
                        &content,
                        entity.content.as_bytes(),
                        bounds,
                        &verified_fonts,
                        page_idx + 1,
                    )? {
                        return secure_unsupported(format!(
                            "entity '{}' bounding box does not intersect its verified text match",
                            entity.id
                        ));
                    }
                    content = if let Some(replacement) = tj_array_replacement {
                        rewrite_exact_tj_array(&content, entity.content.as_bytes(), replacement)?
                    } else {
                        rewritten_tj
                    };
                    report_entries.push(RedactionEntry {
                        entity_id: entity.id.clone(),
                        entity_type: entity.entity_type.clone(),
                        page: page_idx + 1,
                    });
                }
            }

            let mut page = crate::page::Page::from_parsed_with_content(&parsed_page, &document)
                .map_err(|error| {
                    SemanticRedactorError::PageReconstructionFailed(error.to_string())
                })?;
            page.set_content(content);
            output_doc.add_page(page);
        }

        let output = output_doc
            .to_bytes()
            .map_err(|error| SemanticRedactorError::WriteFailed(error.to_string()))?;
        verify_targets_absent(&output, &to_redact)?;

        Ok((
            output,
            RedactionReport {
                entries: report_entries,
                mode: RedactionMode::Irreversible,
                residual_risks: Vec::new(),
            },
        ))
    }
}

fn verified_ascii_fonts<R: Read + Seek>(
    page: &crate::parser::page_tree::ParsedPage,
    document: &crate::parser::document::PdfDocument<R>,
    page_number: u32,
) -> SemanticRedactorResult<HashMap<String, VerifiedTextFont>> {
    let Some(resources) = page.get_resources() else {
        return Ok(HashMap::new());
    };
    for (name, _) in &resources.0 {
        if !matches!(name.0.as_str(), "Font" | "ProcSet") {
            return secure_unsupported(format!(
                "page {page_number} contains unsupported /{} resources",
                name.0
            ));
        }
    }
    let Some(fonts_object) = resources.get("Font") else {
        return Ok(HashMap::new());
    };
    let fonts_object = resolve_parser_object(fonts_object, document, page_number, "Font resource")?;
    let PdfObject::Dictionary(fonts) = fonts_object else {
        return secure_unsupported(format!(
            "page {page_number} Font resource is not a dictionary"
        ));
    };

    let mut verified = HashMap::new();
    for (resource_name, font_object) in &fonts.0 {
        let font_object = resolve_parser_object(
            font_object,
            document,
            page_number,
            &format!("font /{}", resource_name.0),
        )?;
        let PdfObject::Dictionary(font) = font_object else {
            return secure_unsupported(format!(
                "page {page_number} font /{} is not a dictionary",
                resource_name.0
            ));
        };
        let subtype_is_type1 = font
            .get("Subtype")
            .and_then(PdfObject::as_name)
            .is_some_and(|name| name.0 == "Type1");
        let standard_font = font
            .get("BaseFont")
            .and_then(PdfObject::as_name)
            .and_then(|name| Standard14Font::from_name(&name.0));
        if !subtype_is_type1 || standard_font.is_none() {
            return secure_unsupported(format!(
                "page {page_number} font /{} is not an audited Standard-14 Type1 resource",
                resource_name.0
            ));
        }
        let encoding_is_audited = match font.get("Encoding") {
            None => true,
            Some(PdfObject::Name(name)) => matches!(
                name.0.as_str(),
                "StandardEncoding" | "WinAnsiEncoding" | "MacRomanEncoding"
            ),
            Some(_) => false,
        };
        if !encoding_is_audited {
            return secure_unsupported(format!(
                "page {page_number} font /{} uses an unsupported encoding",
                resource_name.0
            ));
        }
        if let Some(verified_font) = verified_ascii_standard14_font(&font) {
            verified.insert(resource_name.0.clone(), verified_font);
        }
    }
    Ok(verified)
}

fn resolve_parser_object<R: Read + Seek>(
    object: &PdfObject,
    document: &crate::parser::document::PdfDocument<R>,
    page_number: u32,
    description: &str,
) -> SemanticRedactorResult<PdfObject> {
    match object {
        PdfObject::Reference(number, generation) => {
            document.get_object(*number, *generation).map_err(|error| {
                SemanticRedactorError::SecureRedactionUnsupported(format!(
                    "page {page_number} cannot resolve {description}: {error}"
                ))
            })
        }
        other => Ok(other.clone()),
    }
}

fn verified_ascii_standard14_font(font: &PdfDictionary) -> Option<VerifiedTextFont> {
    let subtype_is_type1 = font
        .get("Subtype")
        .and_then(PdfObject::as_name)
        .is_some_and(|name| name.0 == "Type1");
    let Some(base_font) = font.get("BaseFont").and_then(PdfObject::as_name) else {
        return None;
    };
    let Some(standard_font) = Standard14Font::from_name(&base_font.0) else {
        return None;
    };
    if standard_font.is_symbolic() || !subtype_is_type1 {
        return None;
    }
    let encoding = match font.get("Encoding") {
        None => None,
        Some(PdfObject::Name(name))
            if matches!(
                name.0.as_str(),
                "StandardEncoding" | "WinAnsiEncoding" | "MacRomanEncoding"
            ) =>
        {
            Some(name.0.clone())
        }
        Some(_) => return None,
    };
    Some(VerifiedTextFont {
        base_font: base_font.0.clone(),
        encoding,
    })
}

fn text_advance_units(
    shown: &[u8],
    adjustments: f32,
    state: &SecureTextState,
    fonts: &HashMap<String, VerifiedTextFont>,
    page: u32,
) -> SemanticRedactorResult<f32> {
    let Some((font_name, font_size)) = &state.font else {
        return secure_unsupported(format!("page {page} text has no active font"));
    };
    if *font_size <= 0.0 || !font_size.is_finite() {
        return secure_unsupported(format!("page {page} text has an invalid font size"));
    }
    let Some(font) = fonts.get(font_name) else {
        return secure_unsupported(format!(
            "page {page} target does not use a verified Standard-14 text font"
        ));
    };
    let metrics = crate::text::fonts::standard::get_standard_font_metrics_by_name(&font.base_font)
        .ok_or_else(|| {
            SemanticRedactorError::SecureRedactionUnsupported(format!(
                "page {page} lacks AFM metrics for /{}",
                font.base_font
            ))
        })?;
    let glyph_units = shown.iter().try_fold(0_f32, |sum, byte| {
        metrics
            .encoded_char_width(font.encoding.as_deref(), None, *byte)
            .map(|width| sum + width as f32)
            .ok_or_else(|| {
                SemanticRedactorError::SecureRedactionUnsupported(format!(
                    "page {page} cannot resolve byte {byte} through /{}",
                    font.base_font
                ))
            })
    })?;
    let spacing = state.character_spacing * shown.len() as f32 * 1000.0 / font_size
        + state.word_spacing * shown.iter().filter(|byte| **byte == b' ').count() as f32 * 1000.0
            / font_size;
    let advance = glyph_units - adjustments + spacing;
    if advance.is_finite() {
        Ok(advance)
    } else {
        secure_unsupported(format!("page {page} text advance is non-finite"))
    }
}

fn find_verified_tj_replacement(
    content: &[u8],
    target: &[u8],
    fonts: &HashMap<String, VerifiedTextFont>,
    page: u32,
) -> SemanticRedactorResult<Option<f32>> {
    let operations = ContentParser::parse_strict(content).map_err(|error| {
        SemanticRedactorError::SecureRedactionUnsupported(format!(
            "page {page} content cannot be parsed strictly: {error}"
        ))
    })?;
    let mut state = SecureTextState::default();
    let mut replacement = None;
    for operation in operations {
        match operation {
            ContentOperation::SetFont(name, size) => state.font = Some((name, size)),
            ContentOperation::SetCharSpacing(value) => state.character_spacing = value,
            ContentOperation::SetWordSpacing(value) => state.word_spacing = value,
            ContentOperation::ShowText(text) if text == target => {
                let candidate = -text_advance_units(&text, 0.0, &state, fonts, page)?;
                if replacement.replace(candidate).is_some() {
                    return secure_unsupported("target matches more than one complete Tj operand");
                }
            }
            _ => {}
        }
    }
    Ok(replacement)
}

fn target_intersects_bounds(
    content: &[u8],
    target: &[u8],
    bounds: &crate::semantic::BoundingBox,
    fonts: &HashMap<String, VerifiedTextFont>,
    page: u32,
) -> SemanticRedactorResult<bool> {
    let operations = ContentParser::parse_strict(content).map_err(|error| {
        SemanticRedactorError::SecureRedactionUnsupported(format!(
            "page {page} content cannot be parsed strictly: {error}"
        ))
    })?;
    let mut state = SecureTextState::default();
    let (mut x, mut y, mut line_x, mut line_y) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
    let mut horizontal_scale = 1.0_f32;
    for operation in operations {
        match operation {
            ContentOperation::BeginText => {
                (x, y, line_x, line_y) = (0.0, 0.0, 0.0, 0.0);
            }
            ContentOperation::SetFont(name, size) => state.font = Some((name, size)),
            ContentOperation::SetCharSpacing(value) => state.character_spacing = value,
            ContentOperation::SetWordSpacing(value) => state.word_spacing = value,
            ContentOperation::SetHorizontalScaling(value) => horizontal_scale = value / 100.0,
            ContentOperation::MoveText(tx, ty) | ContentOperation::MoveTextSetLeading(tx, ty) => {
                line_x += tx;
                line_y += ty;
                x = line_x;
                y = line_y;
            }
            ContentOperation::SetTextMatrix(a, b, c, d, e, f) => {
                if b != 0.0 || c != 0.0 || a <= 0.0 || d <= 0.0 {
                    return secure_unsupported(format!(
                        "page {page} uses unsupported rotated or reflected text geometry"
                    ));
                }
                x = e;
                y = f;
                line_x = e;
                line_y = f;
                horizontal_scale *= a;
            }
            ContentOperation::ShowText(text) => {
                let advance = text_advance_units(&text, 0.0, &state, fonts, page)?;
                if text == target
                    && text_rect_intersects(x, y, advance, horizontal_scale, &state, bounds)
                {
                    return Ok(true);
                }
                let size = state.font.as_ref().map_or(0.0, |(_, size)| *size);
                x += advance * size / 1000.0 * horizontal_scale;
            }
            ContentOperation::ShowTextArray(elements) => {
                let shown: Vec<u8> = elements
                    .iter()
                    .filter_map(|element| match element {
                        crate::parser::content::TextElement::Text(text) => Some(text.as_slice()),
                        crate::parser::content::TextElement::Spacing(_) => None,
                    })
                    .flatten()
                    .copied()
                    .collect();
                let adjustments = elements
                    .iter()
                    .filter_map(|element| match element {
                        crate::parser::content::TextElement::Spacing(value) => Some(*value),
                        crate::parser::content::TextElement::Text(_) => None,
                    })
                    .sum();
                let advance = text_advance_units(&shown, adjustments, &state, fonts, page)?;
                if shown == target
                    && text_rect_intersects(x, y, advance, horizontal_scale, &state, bounds)
                {
                    return Ok(true);
                }
                let size = state.font.as_ref().map_or(0.0, |(_, size)| *size);
                x += advance * size / 1000.0 * horizontal_scale;
            }
            _ => {}
        }
    }
    Ok(false)
}

fn text_rect_intersects(
    x: f32,
    baseline: f32,
    advance_units: f32,
    horizontal_scale: f32,
    state: &SecureTextState,
    bounds: &crate::semantic::BoundingBox,
) -> bool {
    let font_size = state.font.as_ref().map_or(0.0, |(_, size)| *size);
    let width = advance_units.abs() * font_size / 1000.0 * horizontal_scale.abs();
    let left = x.min(x + width);
    let right = x.max(x + width);
    let bottom = baseline - font_size * 0.25;
    let top = baseline + font_size;
    left < bounds.x + bounds.width
        && right > bounds.x
        && bottom < bounds.y + bounds.height
        && top > bounds.y
}

fn find_verified_tj_array_replacement(
    content: &[u8],
    target: &[u8],
    verified_fonts: &HashMap<String, VerifiedTextFont>,
    page: u32,
) -> SemanticRedactorResult<Option<f32>> {
    let operations = ContentParser::parse_strict(content).map_err(|error| {
        SemanticRedactorError::SecureRedactionUnsupported(format!(
            "page {page} content cannot be parsed strictly: {error}"
        ))
    })?;
    let mut state = SecureTextState::default();
    let mut graphics_stack = Vec::new();
    let mut replacement = None;
    for operation in operations {
        match operation {
            ContentOperation::SetFont(name, size) => state.font = Some((name, size)),
            ContentOperation::SetCharSpacing(value) => state.character_spacing = value,
            ContentOperation::SetWordSpacing(value) => state.word_spacing = value,
            ContentOperation::SaveGraphicsState => graphics_stack.push(state.clone()),
            ContentOperation::RestoreGraphicsState => {
                state = graphics_stack.pop().ok_or_else(|| {
                    SemanticRedactorError::SecureRedactionUnsupported(format!(
                        "page {page} restores an unmatched graphics state"
                    ))
                })?;
            }
            ContentOperation::ShowTextArray(elements) => {
                let shown: Vec<u8> = elements
                    .iter()
                    .filter_map(|element| match element {
                        crate::parser::content::TextElement::Text(text) => Some(text.as_slice()),
                        crate::parser::content::TextElement::Spacing(_) => None,
                    })
                    .flatten()
                    .copied()
                    .collect();
                if shown != target {
                    if shown.windows(target.len()).any(|part| part == target) {
                        return secure_unsupported(
                            "target occurs inside a larger TJ array; only a complete TJ array is supported",
                        );
                    }
                    continue;
                }
                let Some((font_name, font_size)) = &state.font else {
                    return secure_unsupported(format!("page {page} TJ target has no active font"));
                };
                if *font_size <= 0.0 || !font_size.is_finite() {
                    return secure_unsupported(format!(
                        "page {page} TJ target has an invalid font size"
                    ));
                }
                let Some(font) = verified_fonts.get(font_name) else {
                    return secure_unsupported(format!(
                        "page {page} TJ target does not use a verified Standard-14 text font"
                    ));
                };
                let metrics = crate::text::fonts::standard::get_standard_font_metrics_by_name(
                    &font.base_font,
                )
                .ok_or_else(|| {
                    SemanticRedactorError::SecureRedactionUnsupported(format!(
                        "page {page} lacks AFM metrics for /{}",
                        font.base_font
                    ))
                })?;
                let glyph_units = shown.iter().try_fold(0_f32, |sum, byte| {
                    metrics
                        .encoded_char_width(font.encoding.as_deref(), None, *byte)
                        .map(|width| sum + width as f32)
                        .ok_or_else(|| {
                            SemanticRedactorError::SecureRedactionUnsupported(format!(
                                "page {page} cannot resolve byte {byte} through /{}",
                                font.base_font
                            ))
                        })
                })?;
                let adjustments: f32 = elements
                    .iter()
                    .filter_map(|element| match element {
                        crate::parser::content::TextElement::Spacing(value) => Some(*value),
                        crate::parser::content::TextElement::Text(_) => None,
                    })
                    .sum();
                if !state.character_spacing.is_finite() || !state.word_spacing.is_finite() {
                    return secure_unsupported(format!(
                        "page {page} TJ target has non-finite text spacing"
                    ));
                }
                let character_spacing_units =
                    state.character_spacing * shown.len() as f32 * 1000.0 / font_size;
                let word_spacing_units = state.word_spacing
                    * shown.iter().filter(|byte| **byte == b' ').count() as f32
                    * 1000.0
                    / font_size;
                let candidate =
                    adjustments - glyph_units - character_spacing_units - word_spacing_units;
                if !candidate.is_finite() {
                    return secure_unsupported(format!(
                        "page {page} TJ replacement advance is non-finite"
                    ));
                }
                if replacement.replace(candidate).is_some() {
                    return secure_unsupported("target matches more than one complete TJ array");
                }
            }
            _ => {}
        }
    }
    Ok(replacement)
}

fn rewrite_exact_tj_array(
    content: &[u8],
    target: &[u8],
    replacement: f32,
) -> SemanticRedactorResult<Vec<u8>> {
    let mut output = Vec::with_capacity(content.len());
    let mut cursor = 0;
    let mut replaced = false;
    while cursor < content.len() {
        if content[cursor] != b'[' {
            output.push(content[cursor]);
            cursor += 1;
            continue;
        }
        let Some((shown, array_end, operator_end)) = parse_literal_tj_array(content, cursor)?
        else {
            output.push(content[cursor]);
            cursor += 1;
            continue;
        };
        if shown == target {
            if replaced {
                return secure_unsupported("target matches more than one complete TJ array");
            }
            use std::io::Write as _;
            write!(&mut output, "[{replacement:.6}] TJ").expect("writing to Vec<u8> never fails");
            cursor = operator_end;
            replaced = true;
        } else {
            output.extend_from_slice(&content[cursor..array_end]);
            cursor = array_end;
        }
    }
    if !replaced {
        return secure_unsupported("verified TJ array could not be rewritten lexically");
    }
    Ok(output)
}

fn parse_literal_tj_array(
    content: &[u8],
    start: usize,
) -> SemanticRedactorResult<Option<(Vec<u8>, usize, usize)>> {
    let mut shown = Vec::new();
    let mut cursor = start + 1;
    loop {
        while cursor < content.len() && content[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if content.get(cursor) == Some(&b'%') {
            while cursor < content.len() && !matches!(content[cursor], b'\r' | b'\n') {
                cursor += 1;
            }
            continue;
        }
        match content.get(cursor) {
            Some(b']') => {
                cursor += 1;
                break;
            }
            Some(b'(') => {
                let (decoded, end) = parse_literal_string(content, cursor)?;
                shown.extend_from_slice(&decoded);
                cursor = end;
            }
            Some(b'+' | b'-' | b'.' | b'0'..=b'9') => {
                cursor += 1;
                while cursor < content.len() && matches!(content[cursor], b'0'..=b'9' | b'.') {
                    cursor += 1;
                }
            }
            Some(_) => return Ok(None),
            None => return secure_unsupported("unterminated TJ array"),
        }
    }
    let array_end = cursor;
    while cursor < content.len() && content[cursor].is_ascii_whitespace() {
        cursor += 1;
    }
    if content.get(cursor..cursor + 2) != Some(b"TJ") {
        return Ok(None);
    }
    Ok(Some((shown, array_end, cursor + 2)))
}

fn count_verified_text_matches(
    content: &[u8],
    target: &[u8],
    verified_fonts: &HashMap<String, VerifiedTextFont>,
    page: u32,
) -> SemanticRedactorResult<usize> {
    let operations = ContentParser::parse_strict(content).map_err(|error| {
        SemanticRedactorError::SecureRedactionUnsupported(format!(
            "page {page} content cannot be parsed strictly: {error}"
        ))
    })?;
    let mut current_font = None;
    let mut graphics_stack = Vec::new();
    let mut matches = 0;
    for operation in operations {
        match operation {
            ContentOperation::SetFont(name, _) => current_font = Some(name),
            ContentOperation::SaveGraphicsState => graphics_stack.push(current_font.clone()),
            ContentOperation::RestoreGraphicsState => {
                current_font = graphics_stack.pop().ok_or_else(|| {
                    SemanticRedactorError::SecureRedactionUnsupported(format!(
                        "page {page} restores an unmatched graphics state"
                    ))
                })?;
            }
            ContentOperation::ShowText(text) if text == target => {
                if current_font
                    .as_ref()
                    .is_some_and(|name| verified_fonts.contains_key(name))
                {
                    matches += 1;
                }
            }
            _ => {}
        }
    }
    if !graphics_stack.is_empty() {
        return secure_unsupported(format!("page {page} leaves graphics states unbalanced"));
    }
    Ok(matches)
}

fn secure_unsupported<T>(reason: impl Into<String>) -> SemanticRedactorResult<T> {
    Err(SemanticRedactorError::SecureRedactionUnsupported(
        reason.into(),
    ))
}

fn validate_initial_secure_operations(content: &[u8], page: u32) -> SemanticRedactorResult<()> {
    let operations = ContentParser::parse_strict(content).map_err(|error| {
        SemanticRedactorError::SecureRedactionUnsupported(format!(
            "page {page} content cannot be parsed strictly: {error}"
        ))
    })?;
    if operations.len() > SECURE_OPERATION_LIMIT {
        return secure_unsupported(format!("page {page} exceeds the content-operation limit"));
    }
    for operation in operations {
        if matches!(
            operation,
            ContentOperation::NextLineShowText(_)
                | ContentOperation::SetSpacingNextLineShowText(_, _, _)
                | ContentOperation::PaintXObject(_)
                | ContentOperation::ShadingFill(_)
                | ContentOperation::SetGraphicsStateParams(_)
                | ContentOperation::SetStrokingColorSpace(_)
                | ContentOperation::SetNonStrokingColorSpace(_)
                | ContentOperation::SetTransformMatrix(_, _, _, _, _, _)
                | ContentOperation::BeginInlineImage
                | ContentOperation::InlineImage { .. }
                | ContentOperation::BeginMarkedContent(_)
                | ContentOperation::BeginMarkedContentWithProps(_, _)
                | ContentOperation::EndMarkedContent
                | ContentOperation::DefineMarkedContentPoint(_)
                | ContentOperation::DefineMarkedContentPointWithProps(_, _)
        ) {
            return secure_unsupported(format!(
                "page {page} uses a text or graphics operator unsupported by the initial secure engine"
            ));
        }
    }
    Ok(())
}

fn remove_exact_literal_tj(
    content: &[u8],
    target: &[u8],
    replacement: Option<f32>,
) -> SemanticRedactorResult<(Vec<u8>, usize)> {
    let mut output = Vec::with_capacity(content.len());
    let mut cursor = 0;
    let mut matches = 0;
    while cursor < content.len() {
        if content[cursor] != b'(' {
            output.push(content[cursor]);
            cursor += 1;
            continue;
        }
        let (decoded, end) = parse_literal_string(content, cursor)?;
        let mut operator = end;
        while operator < content.len() && content[operator].is_ascii_whitespace() {
            operator += 1;
        }
        let is_tj = content.get(operator..operator + 2) == Some(b"Tj")
            && content
                .get(operator + 2)
                .is_none_or(|byte| byte.is_ascii_whitespace() || b"/%[]<>()".contains(byte));
        if is_tj && decoded == target {
            let replacement = replacement.ok_or_else(|| {
                SemanticRedactorError::SecureRedactionUnsupported(
                    "Tj target lacks a verified advance replacement".to_string(),
                )
            })?;
            use std::io::Write as _;
            write!(&mut output, "[{replacement:.6}] TJ").expect("writing to Vec<u8> never fails");
            cursor = operator + 2;
            matches += 1;
        } else {
            if is_tj && decoded.windows(target.len()).any(|part| part == target) {
                return secure_unsupported(
                    "target occurs inside a larger Tj operand; only complete operands are supported",
                );
            }
            output.extend_from_slice(&content[cursor..end]);
        }
        if cursor < end {
            cursor = end;
        }
    }
    Ok((output, matches))
}

fn parse_literal_string(content: &[u8], start: usize) -> SemanticRedactorResult<(Vec<u8>, usize)> {
    let mut decoded = Vec::new();
    let mut cursor = start + 1;
    let mut depth = 1_u32;
    while cursor < content.len() {
        let byte = content[cursor];
        cursor += 1;
        match byte {
            b'(' => {
                depth += 1;
                decoded.push(byte);
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((decoded, cursor));
                }
                decoded.push(byte);
            }
            b'\\' => {
                let escaped = *content.get(cursor).ok_or_else(|| {
                    SemanticRedactorError::SecureRedactionUnsupported(
                        "unterminated escape in literal string".to_string(),
                    )
                })?;
                cursor += 1;
                match escaped {
                    b'n' => decoded.push(b'\n'),
                    b'r' => decoded.push(b'\r'),
                    b't' => decoded.push(b'\t'),
                    b'b' => decoded.push(8),
                    b'f' => decoded.push(12),
                    b'\n' => {}
                    b'\r' => {
                        if content.get(cursor) == Some(&b'\n') {
                            cursor += 1;
                        }
                    }
                    b'0'..=b'7' => {
                        let mut value = escaped - b'0';
                        for _ in 0..2 {
                            match content.get(cursor) {
                                Some(next @ b'0'..=b'7') => {
                                    value = value.wrapping_mul(8).wrapping_add(*next - b'0');
                                    cursor += 1;
                                }
                                _ => break,
                            }
                        }
                        decoded.push(value);
                    }
                    other => decoded.push(other),
                }
            }
            other => decoded.push(other),
        }
    }
    secure_unsupported("unterminated literal string")
}

fn verify_targets_absent(
    output: &[u8],
    entities: &[&SemanticEntity],
) -> SemanticRedactorResult<()> {
    let reader = crate::parser::PdfReader::new(Cursor::new(output)).map_err(|error| {
        SemanticRedactorError::WriteFailed(format!("forensic reparse failed: {error}"))
    })?;
    let document = reader.into_document();
    let metadata = document
        .metadata()
        .map_err(|error| SemanticRedactorError::WriteFailed(error.to_string()))?;
    for value in [
        metadata.title.as_deref(),
        metadata.author.as_deref(),
        metadata.subject.as_deref(),
        metadata.keywords.as_deref(),
        metadata.creator.as_deref(),
        metadata.producer.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        for entity in entities {
            if value.contains(&entity.content) {
                return secure_unsupported(format!(
                    "forensic verification recovered entity '{}' in metadata",
                    entity.id
                ));
            }
        }
    }
    let page_count = document
        .page_count()
        .map_err(|error| SemanticRedactorError::WriteFailed(error.to_string()))?;
    for page_idx in 0..page_count {
        let page = document
            .get_page(page_idx)
            .map_err(|error| SemanticRedactorError::WriteFailed(error.to_string()))?;
        if page.has_annotations() {
            return secure_unsupported(format!(
                "forensic verification found annotations on page {}",
                page_idx + 1
            ));
        }
        verified_ascii_fonts(&page, &document, page_idx + 1)?;
        for stream in page
            .content_streams_with_document(&document)
            .map_err(|error| SemanticRedactorError::WriteFailed(error.to_string()))?
        {
            validate_initial_secure_operations(&stream, page_idx + 1)?;
            let operations = ContentParser::parse_strict(&stream).map_err(|error| {
                SemanticRedactorError::WriteFailed(format!(
                    "forensic content parse failed: {error}"
                ))
            })?;
            for operation in operations {
                let recovered = match operation {
                    ContentOperation::ShowText(text) => Some(text),
                    ContentOperation::ShowTextArray(elements) => Some(
                        elements
                            .into_iter()
                            .filter_map(|element| match element {
                                crate::parser::content::TextElement::Text(text) => Some(text),
                                crate::parser::content::TextElement::Spacing(_) => None,
                            })
                            .flatten()
                            .collect(),
                    ),
                    _ => None,
                };
                if let Some(recovered) = recovered {
                    for entity in entities {
                        if recovered
                            .windows(entity.content.len())
                            .any(|part| part == entity.content.as_bytes())
                        {
                            return secure_unsupported(format!(
                                "forensic verification recovered entity '{}'",
                                entity.id
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn visual_mask_risks() -> Vec<String> {
    vec![
        "underlying page content remains extractable".to_string(),
        "sensitive data may remain in annotations, forms, metadata, attachments, or prior revisions"
            .to_string(),
    ]
}
