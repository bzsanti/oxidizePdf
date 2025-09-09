use crate::error::{ProError, Result};
use crate::license::FeatureGate;
use crate::xmp::XmpMetadata;
use oxidize_pdf::{
    semantic::{BoundingBox, EntityType, SemanticEntity},
    Document,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub mod analysis;
pub mod extractor;
pub mod training;

pub use analysis::{EntityAnalyzer, ExtractionMetrics};
pub use extractor::SemanticExtractor;
pub use training::{ExtractionTarget, TrainingDataset, TrainingExample};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub entities: HashMap<String, ExtractedEntity>,
    pub confidence_score: f32,
    pub processing_time_ms: u64,
    pub extraction_metadata: ExtractionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub id: String,
    pub entity_type: EntityType,
    pub content: Option<String>,
    pub bounding_box: Option<BoundingBox>,
    pub confidence: f32,
    pub properties: HashMap<String, serde_json::Value>,
    pub relationships: Vec<EntityRelationship>,
    pub extraction_method: ExtractionMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub target_id: String,
    pub relation_type: String,
    pub confidence: f32,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionMethod {
    TextPattern,
    SpatialAnalysis,
    MLModel,
    RuleEngine,
    OCR,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetadata {
    pub document_path: Option<String>,
    pub page_count: u32,
    pub extraction_timestamp: chrono::DateTime<chrono::Utc>,
    pub extractor_version: String,
    pub processing_stats: ProcessingStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub total_entities_found: u32,
    pub entities_by_type: HashMap<String, u32>,
    pub average_confidence: f32,
    pub pages_processed: u32,
    pub ocr_words_processed: u32,
    pub pattern_matches: u32,
}

#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    pub min_confidence_threshold: f32,
    pub enable_ocr: bool,
    pub enable_spatial_analysis: bool,
    pub enable_relationship_detection: bool,
    pub target_entity_types: Option<Vec<EntityType>>,
    pub extraction_regions: Option<Vec<BoundingBox>>,
    pub custom_patterns: HashMap<EntityType, Vec<String>>,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            min_confidence_threshold: 0.5,
            enable_ocr: true,
            enable_spatial_analysis: true,
            enable_relationship_detection: true,
            target_entity_types: None,
            extraction_regions: None,
            custom_patterns: HashMap::new(),
        }
    }
}

impl ExtractionResult {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            confidence_score: 0.0,
            processing_time_ms: 0,
            extraction_metadata: ExtractionMetadata {
                document_path: None,
                page_count: 0,
                extraction_timestamp: chrono::Utc::now(),
                extractor_version: env!("CARGO_PKG_VERSION").to_string(),
                processing_stats: ProcessingStats {
                    total_entities_found: 0,
                    entities_by_type: HashMap::new(),
                    average_confidence: 0.0,
                    pages_processed: 0,
                    ocr_words_processed: 0,
                    pattern_matches: 0,
                },
            },
        }
    }

    pub fn add_entity(&mut self, entity: ExtractedEntity) {
        let entity_type_name = format!("{:?}", entity.entity_type);
        *self
            .extraction_metadata
            .processing_stats
            .entities_by_type
            .entry(entity_type_name)
            .or_insert(0) += 1;

        self.extraction_metadata
            .processing_stats
            .total_entities_found += 1;
        self.entities.insert(entity.id.clone(), entity);

        self.update_confidence_score();
    }

    pub fn get_entities_by_type(&self, entity_type: &EntityType) -> Vec<&ExtractedEntity> {
        self.entities
            .values()
            .filter(|e| &e.entity_type == entity_type)
            .collect()
    }

    pub fn get_high_confidence_entities(&self, threshold: f32) -> Vec<&ExtractedEntity> {
        self.entities
            .values()
            .filter(|e| e.confidence >= threshold)
            .collect()
    }

    fn update_confidence_score(&mut self) {
        if self.entities.is_empty() {
            self.confidence_score = 0.0;
            return;
        }

        let total_confidence: f32 = self.entities.values().map(|e| e.confidence).sum();
        self.confidence_score = total_confidence / self.entities.len() as f32;
        self.extraction_metadata.processing_stats.average_confidence = self.confidence_score;
    }

    pub fn to_training_dataset(&self) -> Result<TrainingDataset> {
        FeatureGate::check_extraction_features()?;

        let mut dataset = TrainingDataset::new();

        for entity in self.entities.values() {
            let example = TrainingExample {
                id: entity.id.clone(),
                input_text: entity.content.clone().unwrap_or_default(),
                target_entity_type: entity.entity_type.clone(),
                bounding_box: entity.bounding_box.clone(),
                context: HashMap::new(),
                confidence: entity.confidence,
                metadata: HashMap::new(),
            };

            dataset.add_example(example);
        }

        Ok(dataset)
    }

    pub fn export_to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| ProError::Serialization(e))
    }

    pub fn export_to_csv(&self) -> Result<String> {
        let mut csv = String::new();
        csv.push_str("id,entity_type,content,confidence,x,y,width,height,extraction_method\n");

        for entity in self.entities.values() {
            let bbox = entity.bounding_box.as_ref();
            csv.push_str(&format!(
                "{},{:?},{},{},{},{},{},{},{:?}\n",
                entity.id,
                entity.entity_type,
                entity
                    .content
                    .as_ref()
                    .unwrap_or(&"".to_string())
                    .replace(',', ";"),
                entity.confidence,
                bbox.map(|b| b.x.to_string()).unwrap_or_default(),
                bbox.map(|b| b.y.to_string()).unwrap_or_default(),
                bbox.map(|b| b.width.to_string()).unwrap_or_default(),
                bbox.map(|b| b.height.to_string()).unwrap_or_default(),
                entity.extraction_method
            ));
        }

        Ok(csv)
    }
}

impl ExtractedEntity {
    pub fn new(id: String, entity_type: EntityType) -> Self {
        Self {
            id,
            entity_type,
            content: None,
            bounding_box: None,
            confidence: 0.0,
            properties: HashMap::new(),
            relationships: Vec::new(),
            extraction_method: ExtractionMethod::TextPattern,
        }
    }

    pub fn with_content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    pub fn with_bounding_box(mut self, bbox: BoundingBox) -> Self {
        self.bounding_box = Some(bbox);
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_property(mut self, key: String, value: serde_json::Value) -> Self {
        self.properties.insert(key, value);
        self
    }

    pub fn with_extraction_method(mut self, method: ExtractionMethod) -> Self {
        self.extraction_method = method;
        self
    }

    pub fn add_relationship(&mut self, target_id: String, relation_type: String, confidence: f32) {
        self.relationships.push(EntityRelationship {
            target_id,
            relation_type,
            confidence: confidence.clamp(0.0, 1.0),
            context: None,
        });
    }

    pub fn to_semantic_entity(&self, id: String, bounds: BoundingBox) -> SemanticEntity {
        SemanticEntity {
            id,
            entity_type: self.entity_type.clone(),
            bounds,
            content: self.content.clone().unwrap_or_default(),
            metadata: oxidize_pdf::semantic::EntityMetadata::new(),
            relationships: Vec::new(),
        }
    }
}

impl From<&SemanticEntity> for ExtractedEntity {
    fn from(semantic: &SemanticEntity) -> Self {
        ExtractedEntity {
            id: semantic.id.clone(),
            entity_type: semantic.entity_type.clone(),
            content: Some(semantic.content.clone()),
            bounding_box: Some(semantic.bounds.clone()),
            confidence: 1.0, // Default confidence for existing entities
            properties: HashMap::new(),
            relationships: Vec::new(),
            extraction_method: ExtractionMethod::RuleEngine,
        }
    }
}

/// Utility functions for working with extraction results
pub mod utils {
    use super::*;

    pub fn merge_extraction_results(results: Vec<ExtractionResult>) -> Result<ExtractionResult> {
        if results.is_empty() {
            return Ok(ExtractionResult::new());
        }

        let mut merged = ExtractionResult::new();
        let mut total_processing_time = 0u64;

        for result in results {
            total_processing_time += result.processing_time_ms;

            for (id, entity) in result.entities {
                merged.entities.insert(id, entity);
            }

            // Merge stats
            merged.extraction_metadata.processing_stats.pages_processed +=
                result.extraction_metadata.processing_stats.pages_processed;
            merged
                .extraction_metadata
                .processing_stats
                .ocr_words_processed += result
                .extraction_metadata
                .processing_stats
                .ocr_words_processed;
            merged.extraction_metadata.processing_stats.pattern_matches +=
                result.extraction_metadata.processing_stats.pattern_matches;
        }

        merged.processing_time_ms = total_processing_time;
        merged.update_confidence_score();

        Ok(merged)
    }

    pub fn filter_entities_by_confidence(
        result: &ExtractionResult,
        min_confidence: f32,
    ) -> HashMap<String, ExtractedEntity> {
        result
            .entities
            .iter()
            .filter(|(_, entity)| entity.confidence >= min_confidence)
            .map(|(id, entity)| (id.clone(), entity.clone()))
            .collect()
    }

    pub fn group_entities_by_type(
        result: &ExtractionResult,
    ) -> HashMap<EntityType, Vec<&ExtractedEntity>> {
        let mut grouped = HashMap::new();

        for entity in result.entities.values() {
            grouped
                .entry(entity.entity_type.clone())
                .or_insert_with(Vec::new)
                .push(entity);
        }

        grouped
    }

    pub fn calculate_extraction_quality_score(result: &ExtractionResult) -> f32 {
        if result.entities.is_empty() {
            return 0.0;
        }

        let confidence_score = result.confidence_score;
        let entity_density =
            result.entities.len() as f32 / result.extraction_metadata.page_count.max(1) as f32;
        let relationship_score = result
            .entities
            .values()
            .map(|e| e.relationships.len() as f32)
            .sum::<f32>()
            / result.entities.len() as f32;

        // Weighted combination
        (confidence_score * 0.5)
            + (entity_density.min(1.0) * 0.3)
            + (relationship_score.min(1.0) * 0.2)
    }
}
