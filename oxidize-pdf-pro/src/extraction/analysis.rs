use super::{ExtractedEntity, ExtractionResult};
use oxidize_pdf::semantic::EntityType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct EntityAnalyzer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionMetrics {
    pub total_entities: usize,
    pub entities_by_type: HashMap<String, usize>,
    pub average_confidence: f32,
    pub confidence_distribution: ConfidenceDistribution,
    pub spatial_coverage: SpatialCoverage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceDistribution {
    pub high_confidence: usize,   // >= 0.8
    pub medium_confidence: usize, // 0.5 - 0.8
    pub low_confidence: usize,    // < 0.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialCoverage {
    pub entities_with_bounds: usize,
    pub page_coverage_percentage: f32,
    pub average_entity_size: f32,
}

impl EntityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(&self, result: &ExtractionResult) -> ExtractionMetrics {
        let total_entities = result.entities.len();
        let mut entities_by_type = HashMap::new();
        let mut total_confidence = 0.0;
        let mut high_conf = 0;
        let mut medium_conf = 0;
        let mut low_conf = 0;
        let mut entities_with_bounds = 0;
        let mut total_area = 0.0;

        for entity in result.entities.values() {
            // Count by type
            let type_name = format!("{:?}", entity.entity_type);
            *entities_by_type.entry(type_name).or_insert(0) += 1;

            // Confidence distribution
            total_confidence += entity.confidence;
            if entity.confidence >= 0.8 {
                high_conf += 1;
            } else if entity.confidence >= 0.5 {
                medium_conf += 1;
            } else {
                low_conf += 1;
            }

            // Spatial analysis
            if let Some(bbox) = &entity.bounding_box {
                entities_with_bounds += 1;
                total_area += bbox.width * bbox.height;
            }
        }

        let average_confidence = if total_entities > 0 {
            total_confidence / total_entities as f32
        } else {
            0.0
        };

        let average_entity_size = if entities_with_bounds > 0 {
            total_area / entities_with_bounds as f32
        } else {
            0.0
        };

        ExtractionMetrics {
            total_entities,
            entities_by_type,
            average_confidence,
            confidence_distribution: ConfidenceDistribution {
                high_confidence: high_conf,
                medium_confidence: medium_conf,
                low_confidence: low_conf,
            },
            spatial_coverage: SpatialCoverage {
                entities_with_bounds,
                page_coverage_percentage: 0.0, // Would need page dimensions to calculate
                average_entity_size,
            },
        }
    }
}

impl Default for EntityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
