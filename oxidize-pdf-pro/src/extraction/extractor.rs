use super::{ExtractedEntity, ExtractionConfig, ExtractionMethod, ExtractionResult};
use crate::error::{ProError, Result};
use crate::license::FeatureGate;
use oxidize_pdf::{
    semantic::{BoundingBox, EntityType},
    Document,
};
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

pub struct SemanticExtractor {
    config: ExtractionConfig,
    pattern_extractors: HashMap<EntityType, PatternExtractor>,
    spatial_analyzer: SpatialAnalyzer,
}

struct PatternExtractor {
    patterns: Vec<regex::Regex>,
    #[allow(dead_code)]
    entity_type: EntityType,
    confidence_base: f32,
}

struct SpatialAnalyzer {
    enabled: bool,
}

impl SemanticExtractor {
    pub fn new() -> Self {
        Self {
            config: ExtractionConfig::default(),
            pattern_extractors: HashMap::new(),
            spatial_analyzer: SpatialAnalyzer { enabled: true },
        }
    }

    pub fn with_config(mut self, config: ExtractionConfig) -> Self {
        let spatial_analysis_enabled = config.enable_spatial_analysis;
        self.config = config;
        self.spatial_analyzer.enabled = spatial_analysis_enabled;
        self.load_default_patterns();
        self
    }

    pub fn from_pdf<P: AsRef<Path>>(_path: P) -> Result<Self> {
        FeatureGate::check_extraction_features()?;

        let mut extractor = Self::new();
        extractor.load_default_patterns();
        Ok(extractor)
    }

    pub fn from_document(_document: &Document) -> Result<Self> {
        FeatureGate::check_extraction_features()?;

        let mut extractor = Self::new();
        extractor.load_default_patterns();
        Ok(extractor)
    }

    pub fn extract_from_document(&mut self, document: &Document) -> Result<ExtractionResult> {
        FeatureGate::check_extraction_features()?;
        FeatureGate::record_document_processed(1)?;

        let start_time = Instant::now();
        let mut result = ExtractionResult::new();

        // Extract existing semantic entities first
        for entity in document.get_semantic_entities() {
            let extracted = ExtractedEntity::from(entity).with_confidence(1.0);
            result.add_entity(extracted);
        }

        // Extract text content and apply patterns
        if let Ok(text_content) = document.extract_text() {
            let pattern_entities = self.extract_with_patterns(&text_content)?;
            for entity in pattern_entities {
                result.add_entity(entity);
            }
        }

        // Apply spatial analysis if enabled
        if self.config.enable_spatial_analysis {
            let spatial_entities = self.extract_with_spatial_analysis(document)?;
            for entity in spatial_entities {
                result.add_entity(entity);
            }
        }

        // Detect relationships if enabled
        if self.config.enable_relationship_detection {
            self.detect_relationships(&mut result)?;
        }

        // Filter by confidence threshold
        result
            .entities
            .retain(|_, entity| entity.confidence >= self.config.min_confidence_threshold);

        result.processing_time_ms = start_time.elapsed().as_millis() as u64;
        result.update_confidence_score();

        Ok(result)
    }

    pub fn extract_from_pdf<P: AsRef<Path>>(&mut self, _path: P) -> Result<ExtractionResult> {
        FeatureGate::check_extraction_features()?;

        // TODO: Implement PDF loading once core library supports it
        // For now, create a placeholder document
        let document = Document::new();

        self.extract_from_document(&document)
    }

    pub fn add_custom_pattern(&mut self, entity_type: EntityType, pattern: &str) -> Result<()> {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| ProError::Extraction(format!("Invalid regex pattern: {}", e)))?;

        self.pattern_extractors
            .entry(entity_type.clone())
            .or_insert_with(|| PatternExtractor {
                patterns: Vec::new(),
                entity_type: entity_type.clone(),
                confidence_base: 0.7,
            })
            .patterns
            .push(regex);

        Ok(())
    }

    pub fn set_extraction_region(&mut self, region: BoundingBox) {
        if self.config.extraction_regions.is_none() {
            self.config.extraction_regions = Some(Vec::new());
        }
        self.config
            .extraction_regions
            .as_mut()
            .unwrap()
            .push(region);
    }

    fn load_default_patterns(&mut self) {
        let patterns = [
            (
                EntityType::InvoiceNumber,
                r"(?i)invoice\s*#?\s*:?\s*([A-Z0-9-]+)",
            ),
            (
                EntityType::CustomerName,
                r"(?i)bill\s+to\s*:?\s*([A-Za-z\s]+)",
            ),
            (
                EntityType::TotalAmount,
                r"(?i)total\s*:?\s*\$?([\d,]+\.?\d*)",
            ),
            (
                EntityType::DueDate,
                r"(?i)due\s+date\s*:?\s*(\d{1,2}[/-]\d{1,2}[/-]\d{2,4})",
            ),
            (
                EntityType::Email,
                r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b",
            ),
            (
                EntityType::PhoneNumber,
                r"(?:\+?1[-.\s]?)?\(?([0-9]{3})\)?[-.\s]?([0-9]{3})[-.\s]?([0-9]{4})",
            ),
            (EntityType::TaxAmount, r"(?i)tax\s*:?\s*\$?([\d,]+\.?\d*)"),
            (
                EntityType::PaymentAmount,
                r"\$?([\d,]+\.?\d*)\s*(?:each|per|@)",
            ),
        ];

        for (entity_type, pattern_str) in patterns.iter() {
            if let Ok(regex) = regex::Regex::new(pattern_str) {
                self.pattern_extractors.insert(
                    entity_type.clone(),
                    PatternExtractor {
                        patterns: vec![regex],
                        entity_type: entity_type.clone(),
                        confidence_base: 0.8,
                    },
                );
            }
        }
    }

    fn extract_with_patterns(&self, text: &str) -> Result<Vec<ExtractedEntity>> {
        let mut entities = Vec::new();
        let mut entity_counter = 0;

        for (entity_type, extractor) in &self.pattern_extractors {
            // Skip if we have target types and this isn't one of them
            if let Some(targets) = &self.config.target_entity_types {
                if !targets.contains(entity_type) {
                    continue;
                }
            }

            for pattern in &extractor.patterns {
                for captures in pattern.captures_iter(text) {
                    if let Some(matched) = captures.get(1).or_else(|| captures.get(0)) {
                        let content = matched.as_str().trim().to_string();
                        if !content.is_empty() {
                            let confidence = self.calculate_pattern_confidence(
                                &content,
                                entity_type,
                                extractor.confidence_base,
                            );

                            let entity = ExtractedEntity::new(
                                format!("extracted_{}", entity_counter),
                                entity_type.clone(),
                            )
                            .with_content(content)
                            .with_confidence(confidence)
                            .with_extraction_method(ExtractionMethod::TextPattern);

                            entities.push(entity);
                            entity_counter += 1;
                        }
                    }
                }
            }
        }

        Ok(entities)
    }

    fn extract_with_spatial_analysis(&self, document: &Document) -> Result<Vec<ExtractedEntity>> {
        if !self.spatial_analyzer.enabled {
            return Ok(Vec::new());
        }

        let mut entities = Vec::new();
        let _entity_counter = 0;

        // Analyze document structure for tables, headers, etc.
        for page_num in 0..document.page_count() {
            // Simulated spatial analysis - in reality, this would analyze
            // text positioning, line spacing, font sizes, etc.

            if let Ok(page_text) = document.extract_page_text(page_num) {
                // Look for table-like structures
                let table_entities = self.detect_table_structures(&page_text, page_num as u32)?;
                entities.extend(table_entities);

                // Look for header/title structures
                let header_entities = self.detect_headers(&page_text, page_num as u32)?;
                entities.extend(header_entities);
            }
        }

        Ok(entities)
    }

    fn detect_table_structures(&self, text: &str, page_num: u32) -> Result<Vec<ExtractedEntity>> {
        let mut entities = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            // Simple heuristic: lines with multiple tab-separated or space-aligned values
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                // Looks like a table row
                let entity = ExtractedEntity::new(
                    format!("table_row_{}_{}", page_num, line_idx),
                    EntityType::Table,
                )
                .with_content(line.to_string())
                .with_confidence(0.6)
                .with_extraction_method(ExtractionMethod::SpatialAnalysis)
                .with_bounding_box(BoundingBox::new(
                    50.0,
                    700.0 - (line_idx as f32 * 12.0),
                    500.0,
                    12.0,
                    page_num,
                ));

                entities.push(entity);
            }
        }

        Ok(entities)
    }

    fn detect_headers(&self, text: &str, page_num: u32) -> Result<Vec<ExtractedEntity>> {
        let mut entities = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Heuristics for headers:
            // - Short lines (< 60 chars)
            // - All caps or title case
            // - Not too many numbers
            if trimmed.len() < 60 && trimmed.len() > 5 {
                let is_likely_header = trimmed.chars().filter(|c| c.is_uppercase()).count() as f32
                    / trimmed.len() as f32
                    > 0.3;

                if is_likely_header {
                    let entity = ExtractedEntity::new(
                        format!("header_{}_{}", page_num, line_idx),
                        EntityType::Heading,
                    )
                    .with_content(trimmed.to_string())
                    .with_confidence(0.5)
                    .with_extraction_method(ExtractionMethod::SpatialAnalysis)
                    .with_bounding_box(BoundingBox::new(
                        50.0,
                        750.0 - (line_idx as f32 * 15.0),
                        400.0,
                        15.0,
                        page_num,
                    ));

                    entities.push(entity);
                }
            }
        }

        Ok(entities)
    }

    fn detect_relationships(&self, result: &mut ExtractionResult) -> Result<()> {
        let entities: Vec<_> = result.entities.values().cloned().collect();

        for entity in &entities {
            match entity.entity_type {
                EntityType::Invoice => {
                    // Link invoice to customer, total amount, etc.
                    for other in &entities {
                        match other.entity_type {
                            EntityType::CustomerName => {
                                if let Some(entity_mut) = result.entities.get_mut(&entity.id) {
                                    entity_mut.add_relationship(
                                        other.id.clone(),
                                        "billsTo".to_string(),
                                        0.8,
                                    );
                                }
                            }
                            EntityType::TotalAmount => {
                                if let Some(entity_mut) = result.entities.get_mut(&entity.id) {
                                    entity_mut.add_relationship(
                                        other.id.clone(),
                                        "totalPaymentDue".to_string(),
                                        0.9,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
                EntityType::LineItem => {
                    // Link line items to products, prices, etc.
                    for other in &entities {
                        match other.entity_type {
                            EntityType::OrganizationName | EntityType::PaymentAmount => {
                                // Simple spatial relationship check
                                if let (Some(bbox1), Some(bbox2)) =
                                    (&entity.bounding_box, &other.bounding_box)
                                {
                                    let distance = calculate_distance(bbox1, bbox2);
                                    if distance < 100.0 {
                                        // Within 100 points
                                        if let Some(entity_mut) =
                                            result.entities.get_mut(&entity.id)
                                        {
                                            entity_mut.add_relationship(
                                                other.id.clone(),
                                                "includes".to_string(),
                                                0.7,
                                            );
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn calculate_pattern_confidence(
        &self,
        content: &str,
        entity_type: &EntityType,
        base: f32,
    ) -> f32 {
        let mut confidence = base;

        // Adjust confidence based on content characteristics
        match entity_type {
            EntityType::Email => {
                if content.contains('@') && content.contains('.') {
                    confidence += 0.1;
                }
            }
            EntityType::PhoneNumber => {
                let digit_count = content.chars().filter(|c| c.is_ascii_digit()).count();
                if digit_count >= 10 {
                    confidence += 0.1;
                }
            }
            EntityType::TotalAmount | EntityType::TaxAmount => {
                if content.contains('$') || content.chars().any(|c| c.is_ascii_digit()) {
                    confidence += 0.1;
                }
            }
            _ => {}
        }

        // Penalize very short or very long content
        match content.len() {
            0..=2 => confidence -= 0.2,
            3..=50 => {} // Good length
            51..=100 => confidence -= 0.1,
            _ => confidence -= 0.2,
        }

        confidence.clamp(0.0, 1.0)
    }
}

impl Default for SemanticExtractor {
    fn default() -> Self {
        Self::new()
    }
}

fn calculate_distance(bbox1: &BoundingBox, bbox2: &BoundingBox) -> f32 {
    let center1_x = bbox1.x + bbox1.width / 2.0;
    let center1_y = bbox1.y + bbox1.height / 2.0;
    let center2_x = bbox2.x + bbox2.width / 2.0;
    let center2_y = bbox2.y + bbox2.height / 2.0;

    let dx = center1_x - center2_x;
    let dy = center1_y - center2_y;

    (dx * dx + dy * dy).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_extraction() {
        let extractor = SemanticExtractor::new();
        let text = "Invoice #12345 for John Doe. Total: $1,250.50";

        let entities = extractor.extract_with_patterns(text).unwrap();
        assert!(!entities.is_empty());

        let invoice_entities: Vec<_> = entities
            .iter()
            .filter(|e| e.entity_type == EntityType::InvoiceNumber)
            .collect();
        assert!(!invoice_entities.is_empty());
        assert!(invoice_entities[0]
            .content
            .as_ref()
            .unwrap()
            .contains("12345"));
    }

    #[test]
    fn test_confidence_calculation() {
        let extractor = SemanticExtractor::new();

        let email_confidence =
            extractor.calculate_pattern_confidence("test@example.com", &EntityType::Email, 0.8);
        assert!(email_confidence > 0.8);

        let short_confidence =
            extractor.calculate_pattern_confidence("a", &EntityType::CustomerName, 0.8);
        assert!(short_confidence < 0.8);
    }
}
