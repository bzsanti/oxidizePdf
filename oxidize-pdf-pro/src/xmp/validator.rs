use super::{SchemaOrgEntity, XmpMetadata};
use crate::error::{ProError, Result};
use std::collections::{HashMap, HashSet};

pub struct MetadataValidator {
    enabled_checks: HashSet<ValidationCheck>,
    custom_validators: HashMap<String, Box<dyn CustomValidator>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum ValidationCheck {
    RequiredFields,
    DataTypes,
    Relationships,
    Duplicates,
    SpatialBounds,
    SchemaCompliance,
    ContentConsistency,
}

pub trait CustomValidator {
    fn validate(&self, entity: &SchemaOrgEntity) -> ValidationResult;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: IssueSeverity,
    pub entity_id: Option<String>,
    pub field: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

impl MetadataValidator {
    pub fn new() -> Self {
        let mut checks = HashSet::new();
        checks.insert(ValidationCheck::RequiredFields);
        checks.insert(ValidationCheck::DataTypes);
        checks.insert(ValidationCheck::Relationships);
        checks.insert(ValidationCheck::SchemaCompliance);

        Self {
            enabled_checks: checks,
            custom_validators: HashMap::new(),
        }
    }

    pub fn enable_check(&mut self, check: ValidationCheck) -> &mut Self {
        self.enabled_checks.insert(check);
        self
    }

    pub fn disable_check(&mut self, check: ValidationCheck) -> &mut Self {
        self.enabled_checks.remove(&check);
        self
    }

    pub fn add_custom_validator<V: CustomValidator + 'static>(
        &mut self,
        validator: V,
    ) -> &mut Self {
        self.custom_validators
            .insert(validator.name().to_string(), Box::new(validator));
        self
    }

    pub fn validate(&self, metadata: &XmpMetadata) -> Result<ValidationResult> {
        let mut all_issues = Vec::new();

        if self
            .enabled_checks
            .contains(&ValidationCheck::RequiredFields)
        {
            let mut result = self.validate_required_fields(metadata)?;
            all_issues.append(&mut result.issues);
        }

        if self.enabled_checks.contains(&ValidationCheck::DataTypes) {
            let mut result = self.validate_data_types(metadata)?;
            all_issues.append(&mut result.issues);
        }

        if self
            .enabled_checks
            .contains(&ValidationCheck::Relationships)
        {
            let mut result = self.validate_relationships(metadata)?;
            all_issues.append(&mut result.issues);
        }

        if self.enabled_checks.contains(&ValidationCheck::Duplicates) {
            let mut result = self.validate_duplicates(metadata)?;
            all_issues.append(&mut result.issues);
        }

        if self
            .enabled_checks
            .contains(&ValidationCheck::SpatialBounds)
        {
            let mut result = self.validate_spatial_bounds(metadata)?;
            all_issues.append(&mut result.issues);
        }

        if self
            .enabled_checks
            .contains(&ValidationCheck::ContentConsistency)
        {
            let mut result = self.validate_content_consistency(metadata)?;
            all_issues.append(&mut result.issues);
        }

        // Run custom validators
        for entity in &metadata.schema_org_entities {
            if let Some(validator) = self.custom_validators.get(&entity.schema_type) {
                let mut result = validator.validate(entity);
                all_issues.append(&mut result.issues);
            }
        }

        let has_errors = all_issues
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error);

        Ok(ValidationResult {
            passed: !has_errors,
            issues: all_issues,
        })
    }

    fn validate_required_fields(&self, metadata: &XmpMetadata) -> Result<ValidationResult> {
        let mut issues = Vec::new();

        // Basic metadata validation
        if metadata.title.is_none() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                entity_id: None,
                field: Some("title".to_string()),
                message: "Document title is missing".to_string(),
                suggestion: Some("Add a descriptive title to improve discoverability".to_string()),
            });
        }

        if metadata.creator.is_none() {
            issues.push(ValidationIssue {
                severity: IssueSeverity::Warning,
                entity_id: None,
                field: Some("creator".to_string()),
                message: "Document creator is missing".to_string(),
                suggestion: Some("Specify the document creator for proper attribution".to_string()),
            });
        }

        // Entity-specific validations
        for entity in &metadata.schema_org_entities {
            match entity.schema_type.as_str() {
                "Invoice" => {
                    if !entity.properties.contains_key("totalPaymentDue") {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Error,
                            entity_id: Some(entity.id.clone()),
                            field: Some("totalPaymentDue".to_string()),
                            message: "Invoice must have totalPaymentDue".to_string(),
                            suggestion: Some(
                                "Add the total amount due for this invoice".to_string(),
                            ),
                        });
                    }
                }
                "Person" => {
                    if entity.content.is_none() && !entity.properties.contains_key("name") {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Error,
                            entity_id: Some(entity.id.clone()),
                            field: Some("name".to_string()),
                            message: "Person must have a name".to_string(),
                            suggestion: Some("Add name property or text content".to_string()),
                        });
                    }
                }
                "MonetaryAmount" => {
                    if !entity.properties.contains_key("value") {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Error,
                            entity_id: Some(entity.id.clone()),
                            field: Some("value".to_string()),
                            message: "MonetaryAmount must have a value".to_string(),
                            suggestion: Some("Specify the monetary value".to_string()),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(ValidationResult {
            passed: issues.iter().all(|i| i.severity != IssueSeverity::Error),
            issues,
        })
    }

    fn validate_data_types(&self, metadata: &XmpMetadata) -> Result<ValidationResult> {
        let mut issues = Vec::new();

        for entity in &metadata.schema_org_entities {
            for (prop_name, prop_value) in &entity.properties {
                let issue = match (entity.schema_type.as_str(), prop_name.as_str()) {
                    ("MonetaryAmount", "value") => {
                        if !prop_value.is_number() && !prop_value.is_string() {
                            Some("MonetaryAmount.value must be numeric or string")
                        } else if let Some(str_val) = prop_value.as_str() {
                            if str_val.parse::<f64>().is_err() {
                                Some("MonetaryAmount.value string must be parseable as number")
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    ("Person", "email") => {
                        if let Some(email) = prop_value.as_str() {
                            if !email.contains('@') || !email.contains('.') {
                                Some("Invalid email format")
                            } else {
                                None
                            }
                        } else {
                            Some("Email must be a string")
                        }
                    }
                    ("PostalAddress", "postalCode") => {
                        if let Some(postal) = prop_value.as_str() {
                            if postal.trim().is_empty() {
                                Some("Postal code cannot be empty")
                            } else {
                                None
                            }
                        } else {
                            Some("Postal code must be a string")
                        }
                    }
                    (_, "dateCreated" | "dateModified" | "datePublished") => {
                        if let Some(date_str) = prop_value.as_str() {
                            if chrono::DateTime::parse_from_rfc3339(date_str).is_err() {
                                Some("Date must be in ISO 8601 format")
                            } else {
                                None
                            }
                        } else {
                            Some("Date must be a string in ISO 8601 format")
                        }
                    }
                    _ => None,
                };

                if let Some(issue_msg) = issue {
                    issues.push(ValidationIssue {
                        severity: IssueSeverity::Error,
                        entity_id: Some(entity.id.clone()),
                        field: Some(prop_name.clone()),
                        message: issue_msg.to_string(),
                        suggestion: None,
                    });
                }
            }

            // Validate confidence scores
            if let Some(confidence) = entity.confidence {
                if !(0.0..=1.0).contains(&confidence) {
                    issues.push(ValidationIssue {
                        severity: IssueSeverity::Warning,
                        entity_id: Some(entity.id.clone()),
                        field: Some("confidence".to_string()),
                        message: "Confidence score should be between 0.0 and 1.0".to_string(),
                        suggestion: Some(
                            "Normalize confidence scores to 0.0-1.0 range".to_string(),
                        ),
                    });
                }
            }
        }

        Ok(ValidationResult {
            passed: issues.iter().all(|i| i.severity != IssueSeverity::Error),
            issues,
        })
    }

    fn validate_relationships(&self, metadata: &XmpMetadata) -> Result<ValidationResult> {
        let mut issues = Vec::new();
        let entity_ids: HashSet<_> = metadata.schema_org_entities.iter().map(|e| &e.id).collect();

        for entity in &metadata.schema_org_entities {
            for relationship in &entity.relationships {
                if !entity_ids.contains(&relationship.target_id) {
                    issues.push(ValidationIssue {
                        severity: IssueSeverity::Error,
                        entity_id: Some(entity.id.clone()),
                        field: Some("relationships".to_string()),
                        message: format!(
                            "Relationship target '{}' does not exist",
                            relationship.target_id
                        ),
                        suggestion: Some(
                            "Ensure all relationship targets are valid entity IDs".to_string(),
                        ),
                    });
                }

                if let Some(confidence) = relationship.confidence {
                    if !(0.0..=1.0).contains(&confidence) {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Warning,
                            entity_id: Some(entity.id.clone()),
                            field: Some("relationships".to_string()),
                            message: "Relationship confidence should be between 0.0 and 1.0"
                                .to_string(),
                            suggestion: None,
                        });
                    }
                }
            }
        }

        Ok(ValidationResult {
            passed: issues.iter().all(|i| i.severity != IssueSeverity::Error),
            issues,
        })
    }

    fn validate_duplicates(&self, metadata: &XmpMetadata) -> Result<ValidationResult> {
        let mut issues = Vec::new();
        let mut seen_ids = HashMap::new();

        for entity in &metadata.schema_org_entities {
            if let Some(existing_entity) = seen_ids.insert(&entity.id, entity) {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Error,
                    entity_id: Some(entity.id.clone()),
                    field: None,
                    message: "Duplicate entity ID found".to_string(),
                    suggestion: Some("Ensure all entity IDs are unique".to_string()),
                });
            }
        }

        Ok(ValidationResult {
            passed: issues.is_empty(),
            issues,
        })
    }

    fn validate_spatial_bounds(&self, metadata: &XmpMetadata) -> Result<ValidationResult> {
        let mut issues = Vec::new();

        for entity in &metadata.schema_org_entities {
            if let Some(bbox) = &entity.bounding_box {
                if bbox.width <= 0.0 || bbox.height <= 0.0 {
                    issues.push(ValidationIssue {
                        severity: IssueSeverity::Error,
                        entity_id: Some(entity.id.clone()),
                        field: Some("bounding_box".to_string()),
                        message: "Bounding box must have positive width and height".to_string(),
                        suggestion: Some("Check bounding box dimensions".to_string()),
                    });
                }

                if bbox.x < 0.0 || bbox.y < 0.0 {
                    issues.push(ValidationIssue {
                        severity: IssueSeverity::Warning,
                        entity_id: Some(entity.id.clone()),
                        field: Some("bounding_box".to_string()),
                        message: "Bounding box has negative coordinates".to_string(),
                        suggestion: Some(
                            "Verify coordinate system and page boundaries".to_string(),
                        ),
                    });
                }
            }
        }

        Ok(ValidationResult {
            passed: issues.iter().all(|i| i.severity != IssueSeverity::Error),
            issues,
        })
    }

    fn validate_content_consistency(&self, metadata: &XmpMetadata) -> Result<ValidationResult> {
        let mut issues = Vec::new();

        for entity in &metadata.schema_org_entities {
            // Check if content matches expected schema type
            match entity.schema_type.as_str() {
                "MonetaryAmount" => {
                    if let Some(content) = &entity.content {
                        let has_currency_symbols = content.chars().any(|c| "$€£¥".contains(c));
                        let has_digits = content.chars().any(|c| c.is_ascii_digit());

                        if !has_digits {
                            issues.push(ValidationIssue {
                                severity: IssueSeverity::Warning,
                                entity_id: Some(entity.id.clone()),
                                field: Some("content".to_string()),
                                message: "MonetaryAmount content should contain numeric values"
                                    .to_string(),
                                suggestion: Some(
                                    "Verify that the content represents a monetary amount"
                                        .to_string(),
                                ),
                            });
                        }
                    }
                }
                "Person" => {
                    if let Some(content) = &entity.content {
                        if content.chars().any(|c| c.is_ascii_digit()) && content.len() > 10 {
                            issues.push(ValidationIssue {
                                severity: IssueSeverity::Info,
                                entity_id: Some(entity.id.clone()),
                                field: Some("content".to_string()),
                                message: "Person content contains many digits, verify it's actually a person name".to_string(),
                                suggestion: None,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(ValidationResult {
            passed: true, // Content consistency issues are typically warnings
            issues,
        })
    }
}

impl Default for MetadataValidator {
    fn default() -> Self {
        Self::new()
    }
}

// Example custom validator implementation
pub struct InvoiceValidator;

impl CustomValidator for InvoiceValidator {
    fn validate(&self, entity: &SchemaOrgEntity) -> ValidationResult {
        let mut issues = Vec::new();

        if entity.schema_type == "Invoice" {
            // Custom invoice validation logic
            if !entity.properties.contains_key("customer") {
                issues.push(ValidationIssue {
                    severity: IssueSeverity::Warning,
                    entity_id: Some(entity.id.clone()),
                    field: Some("customer".to_string()),
                    message: "Invoice should specify customer".to_string(),
                    suggestion: Some("Add customer information for better compliance".to_string()),
                });
            }

            if let Some(total) = entity.properties.get("totalPaymentDue") {
                if let Some(total_num) = total.as_f64() {
                    if total_num < 0.0 {
                        issues.push(ValidationIssue {
                            severity: IssueSeverity::Error,
                            entity_id: Some(entity.id.clone()),
                            field: Some("totalPaymentDue".to_string()),
                            message: "Invoice total cannot be negative".to_string(),
                            suggestion: None,
                        });
                    }
                }
            }
        }

        ValidationResult {
            passed: issues.iter().all(|i| i.severity != IssueSeverity::Error),
            issues,
        }
    }

    fn name(&self) -> &str {
        "InvoiceValidator"
    }
}
