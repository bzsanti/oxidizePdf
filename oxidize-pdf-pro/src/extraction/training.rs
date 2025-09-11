use crate::error::{ProError, Result};
use oxidize_pdf::semantic::{BoundingBox, EntityType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataset {
    pub examples: Vec<TrainingExample>,
    pub metadata: DatasetMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingExample {
    pub id: String,
    pub input_text: String,
    pub target_entity_type: EntityType,
    pub bounding_box: Option<BoundingBox>,
    pub context: HashMap<String, String>,
    pub confidence: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub total_examples: usize,
    pub entity_type_distribution: HashMap<String, usize>,
}

#[derive(Debug, Clone)]
pub struct ExtractionTarget {
    pub entity_type: EntityType,
    pub required_fields: Vec<String>,
    pub confidence_threshold: f32,
}

impl TrainingDataset {
    pub fn new() -> Self {
        Self {
            examples: Vec::new(),
            metadata: DatasetMetadata {
                created_at: chrono::Utc::now(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                total_examples: 0,
                entity_type_distribution: HashMap::new(),
            },
        }
    }

    pub fn add_example(&mut self, example: TrainingExample) {
        let entity_type_str = format!("{:?}", example.target_entity_type);
        *self
            .metadata
            .entity_type_distribution
            .entry(entity_type_str)
            .or_insert(0) += 1;

        self.examples.push(example);
        self.metadata.total_examples = self.examples.len();
    }

    pub fn export_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(ProError::Serialization)
    }

    pub fn export_csv(&self) -> Result<String> {
        let mut csv = String::new();
        csv.push_str("id,input_text,target_entity_type,confidence,x,y,width,height\n");

        for example in &self.examples {
            let bbox = example.bounding_box.as_ref();
            csv.push_str(&format!(
                "{},{},{:?},{},{},{},{},{}\n",
                example.id,
                example.input_text.replace(',', ";"),
                example.target_entity_type,
                example.confidence,
                bbox.map(|b| b.x.to_string()).unwrap_or_default(),
                bbox.map(|b| b.y.to_string()).unwrap_or_default(),
                bbox.map(|b| b.width.to_string()).unwrap_or_default(),
                bbox.map(|b| b.height.to_string()).unwrap_or_default(),
            ));
        }

        Ok(csv)
    }
}

impl Default for TrainingDataset {
    fn default() -> Self {
        Self::new()
    }
}
