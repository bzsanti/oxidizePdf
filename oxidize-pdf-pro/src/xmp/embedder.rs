use super::{SchemaOrgEntity, XmpMetadata};
use crate::error::{ProError, Result};
use oxidize_pdf::Document;
use std::io::{Read, Seek, Write};

pub struct XmpEmbedder {
    include_schema_org: bool,
    include_spatial_info: bool,
    compression_enabled: bool,
}

impl Default for XmpEmbedder {
    fn default() -> Self {
        Self::new()
    }
}

impl XmpEmbedder {
    pub fn new() -> Self {
        Self {
            include_schema_org: true,
            include_spatial_info: true,
            compression_enabled: true,
        }
    }

    pub fn with_schema_org(mut self, enabled: bool) -> Self {
        self.include_schema_org = enabled;
        self
    }

    pub fn with_spatial_info(mut self, enabled: bool) -> Self {
        self.include_spatial_info = enabled;
        self
    }

    pub fn with_compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = enabled;
        self
    }

    pub fn embed_entities(&self, document: &mut Document) -> Result<()> {
        let metadata = XmpMetadata::from_document(document)?;
        self.embed_metadata(document, &metadata)
    }

    pub fn embed_metadata(&self, document: &mut Document, metadata: &XmpMetadata) -> Result<()> {
        let xmp_xml = metadata.to_xmp_xml()?;

        // Create XMP metadata stream
        let metadata_obj_id = document
            .add_xmp_metadata(&xmp_xml)
            .map_err(|e| ProError::XmpEmbedding(format!("Failed to add XMP metadata: {}", e)))?;

        tracing::info!(
            "Embedded XMP metadata with {} entities",
            metadata.schema_org_entities.len()
        );

        Ok(())
    }

    pub fn extract_metadata(&self, document: &Document) -> Result<Option<XmpMetadata>> {
        match document.get_xmp_metadata() {
            Ok(Some(xmp_data)) => {
                let metadata = self.parse_xmp_data(&xmp_data)?;
                Ok(Some(metadata))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(ProError::XmpExtraction(format!(
                "Failed to extract XMP: {}",
                e
            ))),
        }
    }

    fn parse_xmp_data(&self, xmp_data: &str) -> Result<XmpMetadata> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xmp_data);
        reader.trim_text(true);

        let mut metadata = XmpMetadata::new();
        let mut buf = Vec::new();
        let mut current_element = String::new();
        let mut in_semantic_entities = false;
        let mut semantic_data = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if current_element == "oxidize:semanticEntities" {
                        in_semantic_entities = true;
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().unwrap().into_owned();
                    if in_semantic_entities {
                        semantic_data.push_str(&text);
                    } else {
                        match current_element.as_str() {
                            "dc:title" => metadata.title = Some(text),
                            "dc:creator" => metadata.creator = Some(text),
                            "dc:subject" => metadata.subject = Some(text),
                            "dc:description" => metadata.description = Some(text),
                            _ => {}
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let element_name = e.name();
                    let element = String::from_utf8_lossy(element_name.as_ref());
                    if element == "oxidize:semanticEntities" {
                        in_semantic_entities = false;
                        // Parse JSON-LD semantic data
                        if !semantic_data.is_empty() {
                            metadata.schema_org_entities =
                                self.parse_semantic_entities(&semantic_data)?;
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(ProError::XmpParsing(format!("XML parsing error: {}", e))),
                _ => {}
            }
            buf.clear();
        }

        Ok(metadata)
    }

    fn parse_semantic_entities(&self, json_data: &str) -> Result<Vec<SchemaOrgEntity>> {
        let parsed: serde_json::Value = serde_json::from_str(json_data)
            .map_err(|e| ProError::XmpParsing(format!("JSON parsing error: {}", e)))?;

        let mut entities = Vec::new();

        if let Some(parts) = parsed.get("hasPart").and_then(|p| p.as_array()) {
            for part in parts {
                if let Ok(entity) = self.parse_schema_org_entity(part) {
                    entities.push(entity);
                }
            }
        }

        Ok(entities)
    }

    fn parse_schema_org_entity(&self, value: &serde_json::Value) -> Result<SchemaOrgEntity> {
        let id = value
            .get("@id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let schema_type = value
            .get("@type")
            .and_then(|v| v.as_str())
            .unwrap_or("Thing")
            .to_string();

        let mut properties = std::collections::HashMap::new();
        if let Some(obj) = value.as_object() {
            for (key, val) in obj {
                if !key.starts_with('@') {
                    properties.insert(key.clone(), val.clone());
                }
            }
        }

        let content = value
            .get("text")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let confidence = value
            .get("confidence")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32);

        Ok(SchemaOrgEntity {
            id,
            entity_type: "Unknown".to_string(), // Would need reverse mapping
            schema_type,
            properties,
            bounding_box: None, // Would need to parse spatial data
            content,
            confidence,
            relationships: Vec::new(), // Would need relationship parsing
        })
    }

    pub fn validate_schema_org(&self, metadata: &XmpMetadata) -> Result<Vec<String>> {
        let mut issues = Vec::new();

        for entity in &metadata.schema_org_entities {
            // Validate required Schema.org properties
            match entity.schema_type.as_str() {
                "Invoice" => {
                    if !entity.properties.contains_key("totalPaymentDue") {
                        issues.push(format!("Invoice {} missing totalPaymentDue", entity.id));
                    }
                }
                "Person" => {
                    if !entity.properties.contains_key("name") && entity.content.is_none() {
                        issues.push(format!("Person {} missing name", entity.id));
                    }
                }
                "PostalAddress" => {
                    if !entity.properties.contains_key("streetAddress") {
                        issues.push(format!("PostalAddress {} missing streetAddress", entity.id));
                    }
                }
                "MonetaryAmount" => {
                    if !entity.properties.contains_key("value") {
                        issues.push(format!("MonetaryAmount {} missing value", entity.id));
                    }
                }
                _ => {} // Basic validation passed
            }
        }

        Ok(issues)
    }
}
