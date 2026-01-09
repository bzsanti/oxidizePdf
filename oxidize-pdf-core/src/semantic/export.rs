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
#[derive(Debug, Clone, Copy, PartialEq)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{EntityMetadata, EntityType};

    fn create_test_entity(id: &str, page: usize, entity_type: EntityType) -> Entity {
        Entity {
            id: id.to_string(),
            entity_type,
            bounds: (0.0, 0.0, 100.0, 50.0),
            page,
            metadata: EntityMetadata::new(),
        }
    }

    #[test]
    fn test_entity_map_new() {
        let map = EntityMap::new();

        assert!(map.document_metadata.is_empty());
        assert!(map.pages.is_empty());
        assert!(map.schemas.is_empty());
    }

    #[test]
    fn test_entity_map_default() {
        let map = EntityMap::default();

        assert!(map.document_metadata.is_empty());
        assert!(map.pages.is_empty());
        assert!(map.schemas.is_empty());
    }

    #[test]
    fn test_add_entity() {
        let mut map = EntityMap::new();
        let entity = create_test_entity("e1", 1, EntityType::Text);

        map.add_entity(entity);

        assert!(map.pages.contains_key(&1));
        assert_eq!(map.pages.get(&1).unwrap().len(), 1);
    }

    #[test]
    fn test_add_multiple_entities_same_page() {
        let mut map = EntityMap::new();
        map.add_entity(create_test_entity("e1", 1, EntityType::Text));
        map.add_entity(create_test_entity("e2", 1, EntityType::Image));
        map.add_entity(create_test_entity("e3", 1, EntityType::Table));

        assert_eq!(map.pages.get(&1).unwrap().len(), 3);
    }

    #[test]
    fn test_add_entities_different_pages() {
        let mut map = EntityMap::new();
        map.add_entity(create_test_entity("e1", 1, EntityType::Text));
        map.add_entity(create_test_entity("e2", 2, EntityType::Image));
        map.add_entity(create_test_entity("e3", 3, EntityType::Table));

        assert_eq!(map.pages.len(), 3);
        assert!(map.pages.contains_key(&1));
        assert!(map.pages.contains_key(&2));
        assert!(map.pages.contains_key(&3));
    }

    #[test]
    fn test_entities_on_page() {
        let mut map = EntityMap::new();
        map.add_entity(create_test_entity("e1", 1, EntityType::Text));
        map.add_entity(create_test_entity("e2", 1, EntityType::Image));

        let page_entities = map.entities_on_page(1);
        assert!(page_entities.is_some());
        assert_eq!(page_entities.unwrap().len(), 2);

        let missing_page = map.entities_on_page(99);
        assert!(missing_page.is_none());
    }

    #[test]
    fn test_entities_by_type() {
        let mut map = EntityMap::new();
        map.add_entity(create_test_entity("e1", 1, EntityType::Text));
        map.add_entity(create_test_entity("e2", 1, EntityType::Text));
        map.add_entity(create_test_entity("e3", 2, EntityType::Image));
        map.add_entity(create_test_entity("e4", 2, EntityType::Text));

        let text_entities = map.entities_by_type(EntityType::Text);
        assert_eq!(text_entities.len(), 3);

        let image_entities = map.entities_by_type(EntityType::Image);
        assert_eq!(image_entities.len(), 1);

        let table_entities = map.entities_by_type(EntityType::Table);
        assert_eq!(table_entities.len(), 0);
    }

    #[test]
    fn test_export_format_default() {
        let format = ExportFormat::default();
        assert_eq!(format, ExportFormat::Json);
    }

    #[test]
    fn test_export_format_variants() {
        assert_eq!(ExportFormat::Json, ExportFormat::Json);
        assert_eq!(ExportFormat::JsonLd, ExportFormat::JsonLd);
        assert_eq!(ExportFormat::Xml, ExportFormat::Xml);
        assert_ne!(ExportFormat::Json, ExportFormat::JsonLd);
    }

    #[test]
    fn test_document_metadata() {
        let mut map = EntityMap::new();
        map.document_metadata
            .insert("title".to_string(), "Test Document".to_string());
        map.document_metadata
            .insert("author".to_string(), "Test Author".to_string());

        assert_eq!(
            map.document_metadata.get("title"),
            Some(&"Test Document".to_string())
        );
        assert_eq!(
            map.document_metadata.get("author"),
            Some(&"Test Author".to_string())
        );
    }

    #[test]
    fn test_schemas() {
        let mut map = EntityMap::new();
        map.schemas.push("https://schema.org".to_string());
        map.schemas.push("https://example.com/schema".to_string());

        assert_eq!(map.schemas.len(), 2);
        assert!(map.schemas.contains(&"https://schema.org".to_string()));
    }

    #[cfg(any(feature = "semantic", test))]
    #[test]
    fn test_to_json() {
        let mut map = EntityMap::new();
        map.add_entity(create_test_entity("e1", 1, EntityType::Text));

        let json = map.to_json();
        assert!(json.is_ok());

        let json_str = json.unwrap();
        assert!(json_str.contains("pages"));
        assert!(json_str.contains("e1"));
    }

    #[cfg(any(feature = "semantic", test))]
    #[test]
    fn test_to_json_compact() {
        let mut map = EntityMap::new();
        map.add_entity(create_test_entity("e1", 1, EntityType::Text));

        let json = map.to_json_compact();
        assert!(json.is_ok());

        let json_str = json.unwrap();
        // Compact JSON should not have newlines
        assert!(!json_str.contains("\n  ")); // No indented newlines
    }

    #[cfg(any(feature = "semantic", test))]
    #[test]
    fn test_to_json_ld() {
        let mut map = EntityMap::new();
        map.add_entity(create_test_entity("e1", 1, EntityType::Text));
        map.schemas.push("https://schema.org".to_string());

        let json_ld = map.to_json_ld();
        assert!(json_ld.is_ok());

        let json_str = json_ld.unwrap();
        assert!(json_str.contains("@context"));
        assert!(json_str.contains("schema.org"));
        assert!(json_str.contains("DigitalDocument"));
        assert!(json_str.contains("hasPart"));
    }

    #[cfg(any(feature = "semantic", test))]
    #[test]
    fn test_entity_type_to_schema_org_financial() {
        assert_eq!(entity_type_to_schema_org(&EntityType::Invoice), "Invoice");
        assert_eq!(
            entity_type_to_schema_org(&EntityType::InvoiceNumber),
            "identifier"
        );
        assert_eq!(
            entity_type_to_schema_org(&EntityType::TotalAmount),
            "totalPrice"
        );
        assert_eq!(
            entity_type_to_schema_org(&EntityType::TaxAmount),
            "taxAmount"
        );
        assert_eq!(
            entity_type_to_schema_org(&EntityType::DueDate),
            "paymentDueDate"
        );
    }

    #[cfg(any(feature = "semantic", test))]
    #[test]
    fn test_entity_type_to_schema_org_identity() {
        assert_eq!(entity_type_to_schema_org(&EntityType::PersonName), "Person");
        assert_eq!(
            entity_type_to_schema_org(&EntityType::OrganizationName),
            "Organization"
        );
        assert_eq!(
            entity_type_to_schema_org(&EntityType::Address),
            "PostalAddress"
        );
        assert_eq!(
            entity_type_to_schema_org(&EntityType::PhoneNumber),
            "telephone"
        );
        assert_eq!(entity_type_to_schema_org(&EntityType::Email), "email");
    }

    #[cfg(any(feature = "semantic", test))]
    #[test]
    fn test_entity_type_to_schema_org_structure() {
        assert_eq!(entity_type_to_schema_org(&EntityType::Heading), "Heading");
        assert_eq!(
            entity_type_to_schema_org(&EntityType::Paragraph),
            "Paragraph"
        );
        assert_eq!(entity_type_to_schema_org(&EntityType::Table), "Table");
        assert_eq!(entity_type_to_schema_org(&EntityType::List), "ItemList");
        assert_eq!(entity_type_to_schema_org(&EntityType::Image), "ImageObject");
        assert_eq!(entity_type_to_schema_org(&EntityType::Text), "Text");
    }

    #[cfg(any(feature = "semantic", test))]
    #[test]
    fn test_entity_type_to_schema_org_custom() {
        assert_eq!(
            entity_type_to_schema_org(&EntityType::Custom("MyType".to_string())),
            "Thing"
        );
    }

    #[cfg(any(feature = "semantic", test))]
    #[test]
    fn test_entity_to_schema_org_basic() {
        let entity = create_test_entity("test_id", 1, EntityType::Text);
        let json = entity_to_schema_org(&entity, 1);

        assert_eq!(json["@type"], "Text");
        assert_eq!(json["@id"], "test_id");
        assert_eq!(json["pageStart"], 2); // page_num + 1
    }

    #[cfg(any(feature = "semantic", test))]
    #[test]
    fn test_entity_to_schema_org_with_metadata() {
        let mut entity = create_test_entity("test_id", 0, EntityType::Invoice);
        entity.metadata = entity.metadata.with_property("total", "1000.00");
        entity.metadata = entity.metadata.with_confidence(0.95);
        entity.metadata = entity.metadata.with_schema("https://schema.org/Invoice");

        let json = entity_to_schema_org(&entity, 0);

        assert_eq!(json["@type"], "Invoice");
        assert_eq!(json["total"], "1000.00");
        // Use approximate comparison for f32 -> f64 conversion
        let confidence = json["confidence"].as_f64().unwrap();
        assert!((confidence - 0.95).abs() < 0.001);
        assert_eq!(json["conformsTo"], "https://schema.org/Invoice");
    }
}
