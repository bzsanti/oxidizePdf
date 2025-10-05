//! Export functionality for semantic entities

use super::Entity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(any(feature = "semantic", test))]
use super::EntityType;

#[cfg(any(feature = "semantic", test))]
use serde_json::{json, Value};

/// Map of entities organized by page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMap {
    /// Document-level metadata
    pub document_metadata: HashMap<String, String>,
    /// Entities organized by page number
    pub pages: HashMap<usize, Vec<Entity>>,
    /// Schema definitions used
    pub schemas: Vec<String>,
}

impl Default for EntityMap {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityMap {
    pub fn new() -> Self {
        Self {
            document_metadata: HashMap::new(),
            pages: HashMap::new(),
            schemas: Vec::new(),
        }
    }

    /// Add an entity to the map
    pub fn add_entity(&mut self, entity: Entity) {
        self.pages.entry(entity.page).or_default().push(entity);
    }

    /// Export to JSON string (requires serde_json feature)
    #[cfg(any(feature = "semantic", test))]
    #[allow(unexpected_cfgs)]
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Export to JSON with custom options (requires serde_json feature)
    #[cfg(any(feature = "semantic", test))]
    #[allow(unexpected_cfgs)]
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Get all entities of a specific type
    pub fn entities_by_type(&self, entity_type: super::EntityType) -> Vec<&Entity> {
        self.pages
            .values()
            .flat_map(|entities| entities.iter())
            .filter(|e| e.entity_type == entity_type)
            .collect()
    }

    /// Get all entities on a specific page
    pub fn entities_on_page(&self, page: usize) -> Option<&Vec<Entity>> {
        self.pages.get(&page)
    }

    /// Export to JSON-LD format with Schema.org context
    #[cfg(any(feature = "semantic", test))]
    #[allow(unexpected_cfgs)]
    pub fn to_json_ld(&self) -> Result<String, serde_json::Error> {
        let mut json_ld = json!({
            "@context": "https://schema.org",
            "@type": "DigitalDocument",
            "additionalType": "AI-Ready PDF",
            "hasPart": []
        });

        let mut parts = Vec::new();

        for (page_num, entities) in &self.pages {
            for entity in entities {
                let entity_json = entity_to_schema_org(entity, *page_num);
                parts.push(entity_json);
            }
        }

        json_ld["hasPart"] = Value::Array(parts);

        // Add schemas if any
        if !self.schemas.is_empty() {
            json_ld["conformsTo"] = json!(self.schemas);
        }

        // Add document metadata
        if !self.document_metadata.is_empty() {
            for (key, value) in &self.document_metadata {
                json_ld[key] = json!(value);
            }
        }

        serde_json::to_string_pretty(&json_ld)
    }
}

/// Convert EntityType to Schema.org type
#[cfg(any(feature = "semantic", test))]
fn entity_type_to_schema_org(entity_type: &EntityType) -> &'static str {
    match entity_type {
        // Financial Documents
        EntityType::Invoice => "Invoice",
        EntityType::InvoiceNumber => "identifier",
        EntityType::CustomerName => "customer",
        EntityType::TotalAmount => "totalPrice",
        EntityType::TaxAmount => "taxAmount",
        EntityType::DueDate => "paymentDueDate",
        EntityType::LineItem => "LineItem",
        EntityType::PaymentAmount => "price",

        // Identity & Contact
        EntityType::PersonName => "Person",
        EntityType::OrganizationName => "Organization",
        EntityType::Address => "PostalAddress",
        EntityType::PhoneNumber => "telephone",
        EntityType::Email => "email",
        EntityType::Website => "url",

        // Legal Documents
        EntityType::Contract => "DigitalDocument",
        EntityType::ContractParty => "Party",
        EntityType::ContractTerm => "OfferCatalog",
        EntityType::EffectiveDate => "datePublished",
        EntityType::ContractValue => "price",
        EntityType::Signature => "signatureValue",

        // Document Structure
        EntityType::Heading => "Heading",
        EntityType::Paragraph => "Paragraph",
        EntityType::Table => "Table",
        EntityType::List => "ItemList",
        EntityType::Image => "ImageObject",
        EntityType::Text => "Text",
        EntityType::Header => "WPHeader",
        EntityType::Footer => "WPFooter",
        EntityType::PageNumber => "pageStart",

        // Dates and Numbers
        EntityType::Date => "Date",
        EntityType::Amount => "MonetaryAmount",
        EntityType::Quantity => "quantityValue",
        EntityType::Percentage => "ratingValue",

        // Custom
        EntityType::Custom(_) => "Thing",
    }
}

/// Convert Entity to Schema.org JSON-LD
#[cfg(any(feature = "semantic", test))]
fn entity_to_schema_org(entity: &Entity, page_num: usize) -> Value {
    let schema_type = entity_type_to_schema_org(&entity.entity_type);

    let mut json = json!({
        "@type": schema_type,
        "spatialCoverage": {
            "@type": "Place",
            "geo": {
                "@type": "GeoCoordinates",
                "box": format!("{},{},{},{}", entity.bounds.0, entity.bounds.1,
                              entity.bounds.2, entity.bounds.3)
            }
        },
        "pageStart": page_num + 1
    });

    // Add ID if present
    if !entity.id.is_empty() {
        json["@id"] = json!(entity.id);
    }

    // Add properties from metadata
    for (key, value) in &entity.metadata.properties {
        json[key] = json!(value);
    }

    // Add confidence if present
    if let Some(confidence) = entity.metadata.confidence {
        json["confidence"] = json!(confidence);
    }

    // Add schema if present
    if let Some(schema) = &entity.metadata.schema {
        json["conformsTo"] = json!(schema);
    }

    json
}

/// Export format options
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    /// JSON format (default)
    Json,
    /// JSON-LD with schema.org context
    JsonLd,
    /// XML format
    Xml,
}

impl Default for ExportFormat {
    fn default() -> Self {
        Self::Json
    }
}
