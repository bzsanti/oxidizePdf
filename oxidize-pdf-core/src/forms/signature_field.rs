//! Digital signature fields implementation according to ISO 32000-1 Section 12.7.4.5
//!
//! This module provides signature field support including visual representation,
//! signature metadata, and lock fields after signing.

use crate::error::PdfError;
use crate::graphics::Color;
use crate::objects::{Dictionary, Object};
use crate::text::Font;
use chrono::{DateTime, Utc};

/// Signature field for digital signatures
#[derive(Debug, Clone)]
pub struct SignatureField {
    /// Field name (unique identifier)
    pub name: String,
    /// Signer information
    pub signer: Option<SignerInfo>,
    /// Signature value (placeholder for actual signature)
    pub signature_value: Option<SignatureValue>,
    /// Fields to lock after signing
    pub lock_fields: Vec<String>,
    /// Whether signature is required
    pub required: bool,
    /// Signature reason
    pub reason: Option<String>,
    /// Signature location
    pub location: Option<String>,
    /// Contact information
    pub contact_info: Option<String>,
    /// Visual appearance settings
    pub appearance: SignatureAppearance,
}

/// Information about the signer
#[derive(Debug, Clone)]
pub struct SignerInfo {
    /// Name of the signer
    pub name: String,
    /// Distinguished name (DN)
    pub distinguished_name: Option<String>,
    /// Email address
    pub email: Option<String>,
    /// Organization
    pub organization: Option<String>,
    /// Organizational unit
    pub organizational_unit: Option<String>,
}

/// Signature value and metadata
#[derive(Debug, Clone)]
pub struct SignatureValue {
    /// Timestamp of signature
    pub timestamp: DateTime<Utc>,
    /// Hash of the document
    pub document_hash: Vec<u8>,
    /// Signature algorithm
    pub algorithm: SignatureAlgorithm,
    /// Certificate chain (placeholder)
    pub certificates: Vec<Certificate>,
    /// Actual signature bytes (placeholder)
    pub signature_bytes: Vec<u8>,
}

/// Signature algorithms
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignatureAlgorithm {
    /// RSA with SHA-256
    RsaSha256,
    /// RSA with SHA-384
    RsaSha384,
    /// RSA with SHA-512
    RsaSha512,
    /// ECDSA with SHA-256
    EcdsaSha256,
    /// DSA with SHA-256
    DsaSha256,
}

/// Certificate placeholder
#[derive(Debug, Clone)]
pub struct Certificate {
    /// Subject name
    pub subject: String,
    /// Issuer name
    pub issuer: String,
    /// Serial number
    pub serial_number: String,
    /// Not before date
    pub not_before: DateTime<Utc>,
    /// Not after date
    pub not_after: DateTime<Utc>,
    /// Public key info
    pub public_key_info: String,
}

/// Visual appearance settings for signature field
#[derive(Debug, Clone)]
pub struct SignatureAppearance {
    /// Show signer name
    pub show_name: bool,
    /// Show date/time
    pub show_date: bool,
    /// Show reason
    pub show_reason: bool,
    /// Show location
    pub show_location: bool,
    /// Show distinguished name
    pub show_dn: bool,
    /// Show labels
    pub show_labels: bool,
    /// Background color
    pub background_color: Option<Color>,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: f64,
    /// Text color
    pub text_color: Color,
    /// Font for text
    pub font: Font,
    /// Font size
    pub font_size: f64,
    /// Custom logo/image
    pub logo_data: Option<Vec<u8>>,
}

impl Default for SignatureAppearance {
    fn default() -> Self {
        Self {
            show_name: true,
            show_date: true,
            show_reason: true,
            show_location: false,
            show_dn: false,
            show_labels: true,
            background_color: Some(Color::gray(0.95)),
            border_color: Color::black(),
            border_width: 1.0,
            text_color: Color::black(),
            font: Font::Helvetica,
            font_size: 10.0,
            logo_data: None,
        }
    }
}

impl SignatureField {
    /// Create a new signature field
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            signer: None,
            signature_value: None,
            lock_fields: Vec::new(),
            required: false,
            reason: None,
            location: None,
            contact_info: None,
            appearance: SignatureAppearance::default(),
        }
    }

    /// Set the signer information
    pub fn with_signer(mut self, signer: SignerInfo) -> Self {
        self.signer = Some(signer);
        self
    }

    /// Set the signature reason
    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set the signature location
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set contact information
    pub fn with_contact(mut self, contact: impl Into<String>) -> Self {
        self.contact_info = Some(contact.into());
        self
    }

    /// Add fields to lock after signing
    pub fn lock_fields_after_signing(mut self, fields: Vec<String>) -> Self {
        self.lock_fields = fields;
        self
    }

    /// Mark field as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Customize appearance
    pub fn with_appearance(mut self, appearance: SignatureAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Check if field is signed
    pub fn is_signed(&self) -> bool {
        self.signature_value.is_some()
    }

    /// Sign the field (placeholder implementation)
    pub fn sign(&mut self, signer: SignerInfo, reason: Option<String>) -> Result<(), PdfError> {
        if self.is_signed() {
            return Err(PdfError::InvalidOperation(
                "Field is already signed".to_string(),
            ));
        }

        // Create signature value (placeholder)
        let signature_value = SignatureValue {
            timestamp: Utc::now(),
            document_hash: vec![0; 32], // Placeholder hash
            algorithm: SignatureAlgorithm::RsaSha256,
            certificates: vec![],
            signature_bytes: vec![0; 256], // Placeholder signature
        };

        self.signer = Some(signer);
        if let Some(r) = reason {
            self.reason = Some(r);
        }
        self.signature_value = Some(signature_value);

        Ok(())
    }

    /// Verify signature (placeholder implementation)
    pub fn verify(&self) -> Result<bool, PdfError> {
        if !self.is_signed() {
            return Ok(false);
        }

        // Placeholder verification - always returns true for now
        // In a real implementation, this would verify the signature
        // against the document hash and certificate chain
        Ok(true)
    }

    /// Generate appearance stream for the signature field
    pub fn generate_appearance(&self, width: f64, height: f64) -> Result<Vec<u8>, PdfError> {
        let mut stream = Vec::new();

        // Background
        if let Some(bg_color) = self.appearance.background_color {
            match bg_color {
                Color::Rgb(r, g, b) => {
                    stream.extend(format!("{} {} {} rg\n", r, g, b).as_bytes());
                }
                Color::Gray(v) => {
                    stream.extend(format!("{} g\n", v).as_bytes());
                }
                Color::Cmyk(c, m, y, k) => {
                    stream.extend(format!("{} {} {} {} k\n", c, m, y, k).as_bytes());
                }
            }
            stream.extend(format!("0 0 {} {} re f\n", width, height).as_bytes());
        }

        // Border
        match self.appearance.border_color {
            Color::Rgb(r, g, b) => {
                stream.extend(format!("{} {} {} RG\n", r, g, b).as_bytes());
            }
            Color::Gray(v) => {
                stream.extend(format!("{} G\n", v).as_bytes());
            }
            Color::Cmyk(c, m, y, k) => {
                stream.extend(format!("{} {} {} {} K\n", c, m, y, k).as_bytes());
            }
        }
        stream.extend(format!("{} w\n", self.appearance.border_width).as_bytes());
        stream.extend(format!("0 0 {} {} re S\n", width, height).as_bytes());

        // Text content
        stream.extend(b"BT\n");
        stream.extend(
            format!(
                "/{} {} Tf\n",
                self.appearance.font.pdf_name(),
                self.appearance.font_size
            )
            .as_bytes(),
        );
        match self.appearance.text_color {
            Color::Rgb(r, g, b) => {
                stream.extend(format!("{} {} {} rg\n", r, g, b).as_bytes());
            }
            Color::Gray(v) => {
                stream.extend(format!("{} g\n", v).as_bytes());
            }
            Color::Cmyk(c, m, y, k) => {
                stream.extend(format!("{} {} {} {} k\n", c, m, y, k).as_bytes());
            }
        }

        let mut y_pos = height - self.appearance.font_size - 5.0;
        let x_pos = 5.0;

        if self.is_signed() {
            // Signed appearance
            if let Some(ref signer) = self.signer {
                if self.appearance.show_name {
                    let label = if self.appearance.show_labels {
                        "Digitally signed by: "
                    } else {
                        ""
                    };
                    stream.extend(format!("{} {} Td\n", x_pos, y_pos).as_bytes());
                    stream.extend(format!("({}{}) Tj\n", label, signer.name).as_bytes());
                    y_pos -= self.appearance.font_size + 2.0;
                }

                if self.appearance.show_dn && signer.distinguished_name.is_some() {
                    let dn = signer.distinguished_name.as_ref().unwrap();
                    stream.extend(format!("{} {} Td\n", x_pos, y_pos).as_bytes());
                    stream.extend(format!("(DN: {}) Tj\n", dn).as_bytes());
                    y_pos -= self.appearance.font_size + 2.0;
                }
            }

            if self.appearance.show_date {
                if let Some(ref sig_value) = self.signature_value {
                    let label = if self.appearance.show_labels {
                        "Date: "
                    } else {
                        ""
                    };
                    let date_str = sig_value
                        .timestamp
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string();
                    stream.extend(format!("{} {} Td\n", x_pos, y_pos).as_bytes());
                    stream.extend(format!("({}{}) Tj\n", label, date_str).as_bytes());
                    y_pos -= self.appearance.font_size + 2.0;
                }
            }

            if self.appearance.show_reason && self.reason.is_some() {
                let label = if self.appearance.show_labels {
                    "Reason: "
                } else {
                    ""
                };
                stream.extend(format!("{} {} Td\n", x_pos, y_pos).as_bytes());
                stream.extend(
                    format!("({}{}) Tj\n", label, self.reason.as_ref().unwrap()).as_bytes(),
                );
                y_pos -= self.appearance.font_size + 2.0;
            }

            if self.appearance.show_location && self.location.is_some() {
                let label = if self.appearance.show_labels {
                    "Location: "
                } else {
                    ""
                };
                stream.extend(format!("{} {} Td\n", x_pos, y_pos).as_bytes());
                stream.extend(
                    format!("({}{}) Tj\n", label, self.location.as_ref().unwrap()).as_bytes(),
                );
            }
        } else {
            // Unsigned appearance - show placeholder
            stream.extend(format!("{} {} Td\n", x_pos, y_pos).as_bytes());
            stream.extend(b"(Click to sign) Tj\n");
        }

        stream.extend(b"ET\n");

        Ok(stream)
    }

    /// Convert to PDF dictionary
    pub fn to_dict(&self) -> Dictionary {
        let mut dict = Dictionary::new();

        dict.set("Type", Object::Name("Annot".to_string()));
        dict.set("Subtype", Object::Name("Widget".to_string()));
        dict.set("FT", Object::Name("Sig".to_string()));
        dict.set("T", Object::String(self.name.clone()));

        // Field flags
        let mut flags = 0;
        if self.required {
            flags |= 2; // Required flag
        }
        dict.set("Ff", Object::Integer(flags));

        // Signature dictionary
        if self.is_signed() {
            let mut sig_dict = Dictionary::new();
            sig_dict.set("Type", Object::Name("Sig".to_string()));

            if let Some(ref signer) = self.signer {
                sig_dict.set("Name", Object::String(signer.name.clone()));
                if let Some(ref email) = signer.email {
                    sig_dict.set("ContactInfo", Object::String(email.clone()));
                }
            }

            if let Some(ref reason) = self.reason {
                sig_dict.set("Reason", Object::String(reason.clone()));
            }

            if let Some(ref location) = self.location {
                sig_dict.set("Location", Object::String(location.clone()));
            }

            if let Some(ref sig_value) = self.signature_value {
                sig_dict.set(
                    "M",
                    Object::String(sig_value.timestamp.format("%Y%m%d%H%M%S%z").to_string()),
                );
            }

            dict.set("V", Object::Dictionary(sig_dict));
        }

        // Lock dictionary for fields to lock after signing
        if !self.lock_fields.is_empty() {
            let mut lock_dict = Dictionary::new();
            lock_dict.set("Type", Object::Name("SigFieldLock".to_string()));

            let fields: Vec<Object> = self
                .lock_fields
                .iter()
                .map(|f| Object::String(f.clone()))
                .collect();
            lock_dict.set("Fields", Object::Array(fields));

            dict.set("Lock", Object::Dictionary(lock_dict));
        }

        dict
    }
}

impl SignerInfo {
    /// Create new signer info
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            distinguished_name: None,
            email: None,
            organization: None,
            organizational_unit: None,
        }
    }

    /// Set email
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set organization
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }

    /// Build distinguished name
    pub fn build_dn(&mut self) {
        let mut dn_parts = vec![format!("CN={}", self.name)];

        if let Some(ref email) = self.email {
            dn_parts.push(format!("emailAddress={}", email));
        }

        if let Some(ref org) = self.organization {
            dn_parts.push(format!("O={}", org));
        }

        if let Some(ref ou) = self.organizational_unit {
            dn_parts.push(format!("OU={}", ou));
        }

        self.distinguished_name = Some(dn_parts.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_field_creation() {
        let field = SignatureField::new("sig1");
        assert_eq!(field.name, "sig1");
        assert!(!field.is_signed());
        assert!(!field.required);
    }

    #[test]
    fn test_signer_info() {
        let mut signer = SignerInfo::new("John Doe")
            .with_email("john@example.com")
            .with_organization("ACME Corp");

        signer.build_dn();
        assert!(signer.distinguished_name.is_some());
        assert!(signer.distinguished_name.unwrap().contains("CN=John Doe"));
    }

    #[test]
    fn test_sign_field() {
        let mut field = SignatureField::new("sig1");
        let signer = SignerInfo::new("Jane Smith");

        assert!(field
            .sign(signer.clone(), Some("Approval".to_string()))
            .is_ok());
        assert!(field.is_signed());
        assert_eq!(field.reason, Some("Approval".to_string()));

        // Cannot sign twice
        assert!(field.sign(signer, None).is_err());
    }

    #[test]
    fn test_signature_appearance() {
        let field = SignatureField::new("sig1");
        let appearance = field.generate_appearance(200.0, 50.0);

        assert!(appearance.is_ok());
        let stream = appearance.unwrap();
        assert!(!stream.is_empty());
    }

    #[test]
    fn test_lock_fields() {
        let field = SignatureField::new("sig1")
            .lock_fields_after_signing(vec!["field1".to_string(), "field2".to_string()]);

        assert_eq!(field.lock_fields.len(), 2);
    }

    #[test]
    fn test_required_field() {
        let field = SignatureField::new("sig1").required();
        assert!(field.required);

        let dict = field.to_dict();
        // Check that required flag is set
        if let Some(Object::Integer(flags)) = dict.get("Ff") {
            assert_eq!(flags & 2, 2);
        }
    }
}
