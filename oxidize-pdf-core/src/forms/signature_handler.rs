//! Signature handler for managing signature fields and signing process
//!
//! This module provides the integration between signature fields and the PDF document,
//! handling the signing process, field locking, and signature validation.

use crate::error::PdfError;
use crate::forms::signature_field::{SignatureAlgorithm, SignatureField, SignerInfo};
use crate::objects::{Dictionary, Object};
use chrono::Utc;
use std::collections::{HashMap, HashSet};

/// Handler for managing signature fields in a document
pub struct SignatureHandler {
    /// All signature fields in the document
    signature_fields: HashMap<String, SignatureField>,
    /// Fields that are locked by signatures
    locked_fields: HashSet<String>,
    /// Document-wide signature settings
    settings: SignatureSettings,
}

/// Settings for signature handling
#[derive(Debug, Clone)]
pub struct SignatureSettings {
    /// Allow incremental saves for signatures
    pub incremental_save: bool,
    /// Require all signature fields to be signed
    pub require_all_signatures: bool,
    /// Lock all fields after any signature
    pub lock_all_after_first_signature: bool,
    /// Default signature algorithm
    pub default_algorithm: SignatureAlgorithm,
    /// Enable signature timestamps
    pub enable_timestamps: bool,
}

impl Default for SignatureSettings {
    fn default() -> Self {
        Self {
            incremental_save: true,
            require_all_signatures: false,
            lock_all_after_first_signature: false,
            default_algorithm: SignatureAlgorithm::RsaSha256,
            enable_timestamps: true,
        }
    }
}

/// Signature validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Field name
    pub field_name: String,
    /// Whether signature is valid
    pub is_valid: bool,
    /// Signer information
    pub signer: Option<SignerInfo>,
    /// Validation timestamp
    pub validated_at: chrono::DateTime<Utc>,
    /// Validation errors if any
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

#[allow(clippy::derivable_impls)]
impl Default for SignatureHandler {
    fn default() -> Self {
        Self {
            signature_fields: HashMap::new(),
            locked_fields: HashSet::new(),
            settings: SignatureSettings::default(),
        }
    }
}

impl SignatureHandler {
    /// Create a new signature handler
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom settings
    pub fn with_settings(settings: SignatureSettings) -> Self {
        Self {
            signature_fields: HashMap::new(),
            locked_fields: HashSet::new(),
            settings,
        }
    }

    /// Add a signature field
    pub fn add_signature_field(&mut self, field: SignatureField) -> Result<(), PdfError> {
        if self.signature_fields.contains_key(&field.name) {
            return Err(PdfError::DuplicateField(format!(
                "Signature field '{}' already exists",
                field.name
            )));
        }

        self.signature_fields.insert(field.name.clone(), field);
        Ok(())
    }

    /// Get a signature field by name
    pub fn get_field(&self, name: &str) -> Option<&SignatureField> {
        self.signature_fields.get(name)
    }

    /// Get mutable signature field by name
    pub fn get_field_mut(&mut self, name: &str) -> Option<&mut SignatureField> {
        self.signature_fields.get_mut(name)
    }

    /// Sign a field
    pub fn sign_field(
        &mut self,
        field_name: &str,
        signer: SignerInfo,
        reason: Option<String>,
    ) -> Result<(), PdfError> {
        // Check if field is locked
        if self.locked_fields.contains(field_name) {
            return Err(PdfError::InvalidOperation(format!(
                "Field '{}' is locked and cannot be signed",
                field_name
            )));
        }

        // Get the field and sign it
        let field = self
            .signature_fields
            .get_mut(field_name)
            .ok_or_else(|| PdfError::FieldNotFound(field_name.to_string()))?;

        field.sign(signer, reason)?;

        // Lock fields as specified
        let fields_to_lock = field.lock_fields.clone();
        for field_to_lock in fields_to_lock {
            self.locked_fields.insert(field_to_lock);
        }

        // Lock all fields if setting is enabled
        if self.settings.lock_all_after_first_signature {
            for field_name in self.signature_fields.keys() {
                self.locked_fields.insert(field_name.clone());
            }
        }

        Ok(())
    }

    /// Validate all signatures in the document
    pub fn validate_all(&self) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        for (name, field) in &self.signature_fields {
            results.push(self.validate_field(name, field));
        }

        results
    }

    /// Validate a single signature field
    fn validate_field(&self, name: &str, field: &SignatureField) -> ValidationResult {
        let mut result = ValidationResult {
            field_name: name.to_string(),
            is_valid: false,
            signer: field.signer.clone(),
            validated_at: Utc::now(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        if !field.is_signed() {
            if field.required {
                result
                    .errors
                    .push("Required signature field is not signed".to_string());
            } else {
                result.warnings.push("Field is not signed".to_string());
            }
            return result;
        }

        // Perform validation (placeholder - always succeeds for now)
        match field.verify() {
            Ok(valid) => {
                result.is_valid = valid;
                if !valid {
                    result
                        .errors
                        .push("Signature verification failed".to_string());
                }
            }
            Err(e) => {
                result.errors.push(format!("Validation error: {}", e));
            }
        }

        // Check certificate validity (placeholder)
        if let Some(ref sig_value) = field.signature_value {
            // In a real implementation, check certificate chain
            if sig_value.certificates.is_empty() {
                result
                    .warnings
                    .push("No certificates found in signature".to_string());
            }
        }

        result
    }

    /// Check if a field is locked
    pub fn is_field_locked(&self, field_name: &str) -> bool {
        self.locked_fields.contains(field_name)
    }

    /// Get all unsigned required fields
    pub fn get_unsigned_required_fields(&self) -> Vec<String> {
        self.signature_fields
            .iter()
            .filter(|(_, field)| field.required && !field.is_signed())
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Check if all required signatures are present
    pub fn all_required_signed(&self) -> bool {
        self.get_unsigned_required_fields().is_empty()
    }

    /// Get signing order based on dependencies
    pub fn get_signing_order(&self) -> Vec<String> {
        // Simple implementation - fields with no lock dependencies first
        let mut order = Vec::new();
        let mut added = HashSet::new();

        // First add fields that don't lock any others
        for (name, field) in &self.signature_fields {
            if field.lock_fields.is_empty() && !field.is_signed() {
                order.push(name.clone());
                added.insert(name.clone());
            }
        }

        // Then add remaining fields
        for (name, field) in &self.signature_fields {
            if !added.contains(name) && !field.is_signed() {
                order.push(name.clone());
            }
        }

        order
    }

    /// Export signature fields to PDF dictionary format
    pub fn export_to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        let mut fields = Vec::new();

        for field in self.signature_fields.values() {
            fields.push(Object::Dictionary(field.to_dict()));
        }

        dict.set("Fields", Object::Array(fields));
        dict.set("SigFlags", Object::Integer(3)); // Signatures exist and are append-only

        dict
    }

    /// Create a signature summary report
    pub fn generate_summary(&self) -> SignatureSummary {
        let total = self.signature_fields.len();
        let signed = self
            .signature_fields
            .values()
            .filter(|f| f.is_signed())
            .count();
        let required = self
            .signature_fields
            .values()
            .filter(|f| f.required)
            .count();
        let required_signed = self
            .signature_fields
            .values()
            .filter(|f| f.required && f.is_signed())
            .count();

        SignatureSummary {
            total_fields: total,
            signed_fields: signed,
            unsigned_fields: total - signed,
            required_fields: required,
            required_signed,
            required_unsigned: required - required_signed,
            all_required_complete: required == required_signed,
            locked_fields: self.locked_fields.len(),
        }
    }
}

/// Summary of signature status in document
#[derive(Debug, Clone)]
pub struct SignatureSummary {
    /// Total number of signature fields
    pub total_fields: usize,
    /// Number of signed fields
    pub signed_fields: usize,
    /// Number of unsigned fields
    pub unsigned_fields: usize,
    /// Number of required fields
    pub required_fields: usize,
    /// Number of required fields that are signed
    pub required_signed: usize,
    /// Number of required fields that are unsigned
    pub required_unsigned: usize,
    /// Whether all required fields are signed
    pub all_required_complete: bool,
    /// Number of locked fields
    pub locked_fields: usize,
}

impl SignatureSummary {
    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.total_fields == 0 {
            100.0
        } else {
            (self.signed_fields as f64 / self.total_fields as f64) * 100.0
        }
    }

    /// Check if document is ready (all required signed)
    pub fn is_ready(&self) -> bool {
        self.all_required_complete
    }

    /// Generate a text report
    pub fn to_report(&self) -> String {
        format!(
            "Signature Summary:\n\
             - Total fields: {}\n\
             - Signed: {} ({:.1}%)\n\
             - Unsigned: {}\n\
             - Required fields: {} ({} signed, {} unsigned)\n\
             - Status: {}\n\
             - Locked fields: {}",
            self.total_fields,
            self.signed_fields,
            self.completion_percentage(),
            self.unsigned_fields,
            self.required_fields,
            self.required_signed,
            self.required_unsigned,
            if self.is_ready() {
                "Ready"
            } else {
                "Incomplete"
            },
            self.locked_fields
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_handler_creation() {
        let handler = SignatureHandler::new();
        assert_eq!(handler.signature_fields.len(), 0);
        assert_eq!(handler.locked_fields.len(), 0);
    }

    #[test]
    fn test_add_signature_field() {
        let mut handler = SignatureHandler::new();
        let field = SignatureField::new("sig1");

        assert!(handler.add_signature_field(field.clone()).is_ok());
        assert_eq!(handler.signature_fields.len(), 1);

        // Cannot add duplicate
        assert!(handler.add_signature_field(field).is_err());
    }

    #[test]
    fn test_sign_field() {
        let mut handler = SignatureHandler::new();
        let field =
            SignatureField::new("sig1").lock_fields_after_signing(vec!["field1".to_string()]);

        handler.add_signature_field(field).unwrap();

        let signer = SignerInfo::new("John Doe");
        assert!(handler
            .sign_field("sig1", signer, Some("Approved".to_string()))
            .is_ok());

        // Check that field1 is now locked
        assert!(handler.is_field_locked("field1"));
    }

    #[test]
    fn test_validation() {
        let mut handler = SignatureHandler::new();
        let mut field = SignatureField::new("sig1").required();

        handler.add_signature_field(field.clone()).unwrap();

        let results = handler.validate_all();
        assert_eq!(results.len(), 1);
        assert!(!results[0].is_valid);
        assert!(!results[0].errors.is_empty());
    }

    #[test]
    fn test_signing_order() {
        let mut handler = SignatureHandler::new();

        // Field that locks others should come last
        let field1 =
            SignatureField::new("sig1").lock_fields_after_signing(vec!["sig2".to_string()]);
        let field2 = SignatureField::new("sig2");

        handler.add_signature_field(field1).unwrap();
        handler.add_signature_field(field2).unwrap();

        let order = handler.get_signing_order();
        assert_eq!(order[0], "sig2"); // Should be signed first
        assert_eq!(order[1], "sig1"); // Should be signed second
    }

    #[test]
    fn test_summary() {
        let mut handler = SignatureHandler::new();

        handler
            .add_signature_field(SignatureField::new("sig1").required())
            .unwrap();
        handler
            .add_signature_field(SignatureField::new("sig2"))
            .unwrap();

        let summary = handler.generate_summary();
        assert_eq!(summary.total_fields, 2);
        assert_eq!(summary.required_fields, 1);
        assert_eq!(summary.signed_fields, 0);
        assert!(!summary.is_ready());

        // Sign the required field
        handler
            .sign_field("sig1", SignerInfo::new("Signer"), None)
            .unwrap();

        let summary = handler.generate_summary();
        assert_eq!(summary.signed_fields, 1);
        assert!(summary.is_ready());
    }
}
