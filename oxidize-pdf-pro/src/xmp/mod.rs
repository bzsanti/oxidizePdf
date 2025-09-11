use crate::error::{ProError, Result};
use chrono::{DateTime, Utc};
use oxidize_pdf::{
    semantic::{BoundingBox, EntityType, SemanticEntity},
    Document,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod embedder;
pub mod schema_org;
pub mod validator;

pub use embedder::XmpEmbedder;
pub use schema_org::SchemaOrgValidator;
pub use validator::MetadataValidator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XmpMetadata {
    pub title: Option<String>,
    pub creator: Option<String>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub subject: Option<String>,
    pub description: Option<String>,
    pub schema_org_entities: Vec<SchemaOrgEntity>,
    pub custom_properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaOrgEntity {
    pub id: String,
    pub entity_type: String,
    pub schema_type: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub bounding_box: Option<BoundingBox>,
    pub content: Option<String>,
    pub confidence: Option<f32>,
    pub relationships: Vec<EntityRelationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub target_id: String,
    pub relation_type: String,
    pub confidence: Option<f32>,
}

impl Default for XmpMetadata {
    fn default() -> Self {
        Self::new()
    }
}

impl XmpMetadata {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            title: None,
            creator: None,
            created: now,
            modified: now,
            subject: None,
            description: None,
            schema_org_entities: Vec::new(),
            custom_properties: HashMap::new(),
        }
    }

    pub fn from_document(document: &Document) -> Result<Self> {
        let mut metadata = Self::new();

        // Extract semantic entities and convert to Schema.org format
        for entity in document.get_semantic_entities() {
            let schema_entity = SchemaOrgEntity::from_semantic_entity(&entity.id, entity)?;
            metadata.schema_org_entities.push(schema_entity);
        }

        Ok(metadata)
    }

    pub fn to_xmp_xml(&self) -> Result<String> {
        let json_ld = self.to_json_ld()?;
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="oxidize-pdf-pro">
    <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
        <rdf:Description rdf:about=""
            xmlns:dc="http://purl.org/dc/elements/1.1/"
            xmlns:xmp="http://ns.adobe.com/xap/1.0/"
            xmlns:schema="http://schema.org/"
            xmlns:oxidize="http://oxidizepdf.dev/ns/">
            {}
            {}
            <oxidize:semanticEntities><![CDATA[{}]]></oxidize:semanticEntities>
        </rdf:Description>
    </rdf:RDF>
</x:xmpmeta>"#,
            self.dublin_core_xml(),
            self.xmp_basic_xml(),
            json_ld
        );
        Ok(xml)
    }

    pub fn to_json_ld(&self) -> Result<String> {
        let mut context = serde_json::Map::new();
        context.insert(
            "@context".to_string(),
            serde_json::json!("https://schema.org/"),
        );
        context.insert("@type".to_string(), serde_json::json!("Document"));

        if let Some(title) = &self.title {
            context.insert("name".to_string(), serde_json::json!(title));
        }

        if let Some(creator) = &self.creator {
            context.insert(
                "creator".to_string(),
                serde_json::json!({
                    "@type": "Person",
                    "name": creator
                }),
            );
        }

        context.insert(
            "dateCreated".to_string(),
            serde_json::json!(self.created.to_rfc3339()),
        );
        context.insert(
            "dateModified".to_string(),
            serde_json::json!(self.modified.to_rfc3339()),
        );

        let mut entities = Vec::new();
        for entity in &self.schema_org_entities {
            entities.push(entity.to_json_ld()?);
        }

        if !entities.is_empty() {
            context.insert("hasPart".to_string(), serde_json::json!(entities));
        }

        serde_json::to_string_pretty(&context)
            .map_err(|e| ProError::XmpSerialization(e.to_string()))
    }

    fn dublin_core_xml(&self) -> String {
        let mut xml = String::new();

        if let Some(title) = &self.title {
            xml.push_str(&format!(
                "            <dc:title><![CDATA[{}]]></dc:title>\n",
                title
            ));
        }

        if let Some(creator) = &self.creator {
            xml.push_str(&format!(
                "            <dc:creator><![CDATA[{}]]></dc:creator>\n",
                creator
            ));
        }

        if let Some(subject) = &self.subject {
            xml.push_str(&format!(
                "            <dc:subject><![CDATA[{}]]></dc:subject>\n",
                subject
            ));
        }

        if let Some(description) = &self.description {
            xml.push_str(&format!(
                "            <dc:description><![CDATA[{}]]></dc:description>\n",
                description
            ));
        }

        xml
    }

    fn xmp_basic_xml(&self) -> String {
        format!(
            r#"            <xmp:CreateDate>{}</xmp:CreateDate>
            <xmp:ModifyDate>{}</xmp:ModifyDate>
            <xmp:MetadataDate>{}</xmp:MetadataDate>
            <xmp:CreatorTool>oxidize-pdf-pro</xmp:CreatorTool>"#,
            self.created.to_rfc3339(),
            self.modified.to_rfc3339(),
            Utc::now().to_rfc3339()
        )
    }
}

impl SchemaOrgEntity {
    fn from_semantic_entity(id: &str, entity: &SemanticEntity) -> Result<Self> {
        let schema_type = entity_type_to_schema_org(&entity.entity_type);
        let mut properties = HashMap::new();

        let content = &entity.content;
        if !content.is_empty() {
            properties.insert("text".to_string(), serde_json::json!(content));
        }

        // Add bounding box as spatial properties
        let bbox = &entity.bounds;
        properties.insert(
            "spatialCoverage".to_string(),
            serde_json::json!({
                "@type": "Place",
                "geo": {
                    "@type": "GeoCoordinates",
                    "latitude": bbox.y + bbox.height / 2.0,
                    "longitude": bbox.x + bbox.width / 2.0
                }
            }),
        );

        let relationships = entity
            .relationships
            .iter()
            .map(|rel| EntityRelationship {
                target_id: rel.target_id.clone(),
                relation_type: format!("{:?}", rel.relation_type),
                confidence: None,
            })
            .collect();

        Ok(Self {
            id: id.to_string(),
            entity_type: format!("{:?}", entity.entity_type),
            schema_type,
            properties,
            bounding_box: Some(entity.bounds.clone()),
            content: Some(entity.content.clone()),
            confidence: None,
            relationships,
        })
    }

    fn to_json_ld(&self) -> Result<serde_json::Value> {
        let mut obj = serde_json::Map::new();
        obj.insert("@type".to_string(), serde_json::json!(self.schema_type));
        obj.insert("@id".to_string(), serde_json::json!(self.id));

        for (key, value) in &self.properties {
            obj.insert(key.clone(), value.clone());
        }

        if let Some(content) = &self.content {
            obj.insert("text".to_string(), serde_json::json!(content));
        }

        if let Some(confidence) = self.confidence {
            obj.insert("confidence".to_string(), serde_json::json!(confidence));
        }

        Ok(serde_json::json!(obj))
    }
}

fn entity_type_to_schema_org(entity_type: &EntityType) -> String {
    match entity_type {
        // Available entity types from the core library
        EntityType::Invoice => "Invoice".to_string(),
        EntityType::InvoiceNumber => "Order".to_string(),
        EntityType::CustomerName => "Person".to_string(),
        EntityType::LineItem => "Offer".to_string(),
        EntityType::TotalAmount => "MonetaryAmount".to_string(),
        EntityType::TaxAmount => "MonetaryAmount".to_string(),
        EntityType::PaymentAmount => "MonetaryAmount".to_string(),
        EntityType::DueDate => "Date".to_string(),
        EntityType::Contract => "Contract".to_string(),
        EntityType::ContractParty => "Person".to_string(),
        EntityType::PersonName => "Person".to_string(),
        EntityType::OrganizationName => "Organization".to_string(),
        EntityType::Address => "PostalAddress".to_string(),
        EntityType::PhoneNumber => "Text".to_string(),
        EntityType::Email => "Text".to_string(),
        EntityType::Website => "URL".to_string(),
        EntityType::Table => "Table".to_string(),
        EntityType::Text => "Text".to_string(),
        EntityType::Image => "ImageObject".to_string(),
        EntityType::Heading => "Text".to_string(),
        EntityType::Paragraph => "Text".to_string(),
        EntityType::List => "List".to_string(),
        EntityType::PageNumber => "Text".to_string(),
        EntityType::Header => "Text".to_string(),
        EntityType::Footer => "Text".to_string(),
        EntityType::ContractTerm => "Text".to_string(),
        EntityType::EffectiveDate => "Date".to_string(),
        EntityType::ContractValue => "MonetaryAmount".to_string(),
        EntityType::Signature => "Text".to_string(),
        EntityType::Date => "Date".to_string(),
        EntityType::Quantity => "Number".to_string(),
        EntityType::Percentage => "Number".to_string(),
        EntityType::Amount => "MonetaryAmount".to_string(),
        EntityType::Custom(name) => format!("Thing_{}", name),
    }
}
