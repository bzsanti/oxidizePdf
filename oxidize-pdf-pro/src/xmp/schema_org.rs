use super::{SchemaOrgEntity, XmpMetadata};
use crate::error::{ProError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct SchemaOrgValidator {
    strict_mode: bool,
    custom_schemas: HashMap<String, SchemaDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub required_properties: Vec<String>,
    pub optional_properties: Vec<String>,
    pub parent_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub entity_id: String,
    pub message: String,
    pub property: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub entity_id: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl SchemaOrgValidator {
    pub fn new() -> Self {
        let mut validator = Self {
            strict_mode: false,
            custom_schemas: HashMap::new(),
        };
        validator.load_standard_schemas();
        validator
    }

    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    pub fn add_custom_schema(&mut self, schema_type: String, definition: SchemaDefinition) {
        self.custom_schemas.insert(schema_type, definition);
    }

    pub fn validate_metadata(&self, metadata: &XmpMetadata) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Validate each entity
        for entity in &metadata.schema_org_entities {
            self.validate_entity(entity, &mut result)?;
        }

        // Validate relationships
        self.validate_relationships(&metadata.schema_org_entities, &mut result)?;

        result.is_valid = result.errors.is_empty();
        Ok(result)
    }

    fn validate_entity(
        &self,
        entity: &SchemaOrgEntity,
        result: &mut ValidationResult,
    ) -> Result<()> {
        let schema = self.get_schema_definition(&entity.schema_type)?;

        // Check required properties
        for required_prop in &schema.required_properties {
            if !entity.properties.contains_key(required_prop)
                && !self.is_property_in_content(required_prop, entity)
            {
                result.errors.push(ValidationError {
                    entity_id: entity.id.clone(),
                    message: format!("Missing required property: {}", required_prop),
                    property: Some(required_prop.clone()),
                });
            }
        }

        // Validate property types
        for (prop_name, prop_value) in &entity.properties {
            if let Err(error_msg) =
                self.validate_property_type(&entity.schema_type, prop_name, prop_value)
            {
                result.errors.push(ValidationError {
                    entity_id: entity.id.clone(),
                    message: error_msg,
                    property: Some(prop_name.clone()),
                });
            }
        }

        // Check for recommendations
        self.add_recommendations(entity, result);

        Ok(())
    }

    fn validate_relationships(
        &self,
        entities: &[SchemaOrgEntity],
        result: &mut ValidationResult,
    ) -> Result<()> {
        let entity_ids: std::collections::HashSet<_> = entities.iter().map(|e| &e.id).collect();

        for entity in entities {
            for relationship in &entity.relationships {
                if !entity_ids.contains(&relationship.target_id) {
                    result.errors.push(ValidationError {
                        entity_id: entity.id.clone(),
                        message: format!(
                            "Relationship points to non-existent entity: {}",
                            relationship.target_id
                        ),
                        property: None,
                    });
                }
            }
        }

        Ok(())
    }

    fn get_schema_definition(&self, schema_type: &str) -> Result<SchemaDefinition> {
        if let Some(custom_schema) = self.custom_schemas.get(schema_type) {
            return Ok(custom_schema.clone());
        }

        // Standard Schema.org definitions
        let definition = match schema_type {
            "Invoice" => SchemaDefinition {
                required_properties: vec!["totalPaymentDue".to_string(), "customer".to_string()],
                optional_properties: vec![
                    "paymentDueDate".to_string(),
                    "paymentMethod".to_string(),
                    "billingAddress".to_string(),
                    "provider".to_string(),
                ],
                parent_types: vec!["Intangible".to_string()],
            },
            "Person" => SchemaDefinition {
                required_properties: vec!["name".to_string()],
                optional_properties: vec![
                    "email".to_string(),
                    "telephone".to_string(),
                    "address".to_string(),
                    "jobTitle".to_string(),
                ],
                parent_types: vec!["Thing".to_string()],
            },
            "PostalAddress" => SchemaDefinition {
                required_properties: vec!["streetAddress".to_string()],
                optional_properties: vec![
                    "addressLocality".to_string(),
                    "addressRegion".to_string(),
                    "postalCode".to_string(),
                    "addressCountry".to_string(),
                ],
                parent_types: vec!["ContactPoint".to_string()],
            },
            "MonetaryAmount" => SchemaDefinition {
                required_properties: vec!["value".to_string()],
                optional_properties: vec![
                    "currency".to_string(),
                    "maxValue".to_string(),
                    "minValue".to_string(),
                ],
                parent_types: vec!["StructuredValue".to_string()],
            },
            "Offer" => SchemaDefinition {
                required_properties: vec!["itemOffered".to_string()],
                optional_properties: vec![
                    "price".to_string(),
                    "priceCurrency".to_string(),
                    "availability".to_string(),
                    "seller".to_string(),
                ],
                parent_types: vec!["Intangible".to_string()],
            },
            "Product" => SchemaDefinition {
                required_properties: vec!["name".to_string()],
                optional_properties: vec![
                    "description".to_string(),
                    "sku".to_string(),
                    "brand".to_string(),
                    "category".to_string(),
                ],
                parent_types: vec!["Thing".to_string()],
            },
            "Contract" => SchemaDefinition {
                required_properties: vec!["name".to_string()],
                optional_properties: vec![
                    "contractType".to_string(),
                    "party".to_string(),
                    "startDate".to_string(),
                    "endDate".to_string(),
                ],
                parent_types: vec!["CreativeWork".to_string()],
            },
            "Report" => SchemaDefinition {
                required_properties: vec!["name".to_string()],
                optional_properties: vec![
                    "author".to_string(),
                    "datePublished".to_string(),
                    "about".to_string(),
                ],
                parent_types: vec!["CreativeWork".to_string()],
            },
            _ => {
                if self.strict_mode {
                    return Err(ProError::SchemaValidation(format!(
                        "Unknown schema type: {}",
                        schema_type
                    )));
                } else {
                    // Default minimal schema for unknown types
                    SchemaDefinition {
                        required_properties: vec![],
                        optional_properties: vec!["name".to_string(), "description".to_string()],
                        parent_types: vec!["Thing".to_string()],
                    }
                }
            }
        };

        Ok(definition)
    }

    fn validate_property_type(
        &self,
        schema_type: &str,
        property: &str,
        value: &serde_json::Value,
    ) -> std::result::Result<(), String> {
        match (schema_type, property) {
            ("MonetaryAmount", "value") => {
                if !value.is_number() && !value.is_string() {
                    return Err("MonetaryAmount.value must be a number or string".to_string());
                }
            }
            ("Person", "email") => {
                if let Some(email_str) = value.as_str() {
                    if !email_str.contains('@') {
                        return Err("Invalid email format".to_string());
                    }
                }
            }
            ("PostalAddress", "postalCode") => {
                if let Some(postal_str) = value.as_str() {
                    if postal_str.trim().is_empty() {
                        return Err("Postal code cannot be empty".to_string());
                    }
                }
            }
            _ => {} // No specific validation
        }

        Ok(())
    }

    fn is_property_in_content(&self, property: &str, entity: &SchemaOrgEntity) -> bool {
        if let Some(content) = &entity.content {
            // Simple heuristic: check if content might contain the property
            match property {
                "name" => !content.trim().is_empty(),
                "value" => content.chars().any(|c| c.is_ascii_digit()),
                _ => false,
            }
        } else {
            false
        }
    }

    fn add_recommendations(&self, entity: &SchemaOrgEntity, result: &mut ValidationResult) {
        match entity.schema_type.as_str() {
            "Invoice" => {
                if !entity.properties.contains_key("paymentDueDate") {
                    result.warnings.push(ValidationWarning {
                        entity_id: entity.id.clone(),
                        message: "Consider adding payment due date".to_string(),
                        suggestion: Some("Add paymentDueDate property".to_string()),
                    });
                }
            }
            "Person" => {
                if !entity.properties.contains_key("email")
                    && !entity.properties.contains_key("telephone")
                {
                    result.warnings.push(ValidationWarning {
                        entity_id: entity.id.clone(),
                        message: "Consider adding contact information".to_string(),
                        suggestion: Some("Add email or telephone property".to_string()),
                    });
                }
            }
            "MonetaryAmount" => {
                if !entity.properties.contains_key("currency") {
                    result.warnings.push(ValidationWarning {
                        entity_id: entity.id.clone(),
                        message: "Currency not specified".to_string(),
                        suggestion: Some("Add currency property (e.g., 'USD', 'EUR')".to_string()),
                    });
                }
            }
            _ => {}
        }
    }

    fn load_standard_schemas(&mut self) {
        // Pre-load commonly used schema definitions
        // This could be expanded or loaded from external configuration
    }

    pub fn get_schema_recommendation(&self, entity_type: &str) -> Option<String> {
        match entity_type {
            "Invoice" => Some(
                "Consider using Invoice schema with totalPaymentDue, customer, and paymentDueDate"
                    .to_string(),
            ),
            "Person" => {
                Some("Use Person schema with name, email, and address properties".to_string())
            }
            "Product" => {
                Some("Use Product schema with name, description, and sku properties".to_string())
            }
            _ => None,
        }
    }
}

impl Default for SchemaOrgValidator {
    fn default() -> Self {
        Self::new()
    }
}
