//! PDF signature and fingerprinting system
//!
//! This module provides a multi-layer identification system for PDFs generated with oxidize-pdf:
//!
//! 1. **Build signature**: A cryptographic hash that uniquely identifies the version and build
//! 2. **Feature fingerprinting**: Automatic detection of features used in the document
//! 3. **Edition tagging**: Identifies Community, PRO, or Enterprise edition
//!
//! These fields are written to the PDF's Info Dictionary and are NOT exposed in the public API,
//! making them resistant to spoofing while remaining non-intrusive to legitimate users.

use crate::document::Document;
use crate::objects::{Dictionary, Object};
use sha2::{Digest, Sha256};

/// Edition of oxidize-pdf used to generate the PDF
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Community edition (AGPL-3.0 license)
    Community,
    /// PRO edition (Commercial license)
    #[allow(dead_code)]
    Pro,
    /// Enterprise edition (Commercial license with advanced features)
    #[allow(dead_code)]
    Enterprise,
}

impl Edition {
    /// Get the edition as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Edition::Community => "Community",
            Edition::Pro => "PRO",
            Edition::Enterprise => "Enterprise",
        }
    }
}

/// PDF signature containing build information and feature fingerprint
pub struct PdfSignature {
    /// Version of oxidize-pdf (e.g., "1.2.5")
    #[allow(dead_code)]
    version: String,
    /// Edition used to generate the PDF
    edition: Edition,
    /// Cryptographic hash of build (version + edition + build timestamp)
    build_hash: String,
    /// Bit flags representing features used in the document
    features_fingerprint: u16,
}

impl PdfSignature {
    /// Create a new PDF signature for a document
    ///
    /// # Arguments
    ///
    /// * `document` - The PDF document to sign
    /// * `edition` - The edition of oxidize-pdf being used
    pub fn new(document: &Document, edition: Edition) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            edition,
            build_hash: Self::generate_build_hash(edition),
            features_fingerprint: Self::compute_features(document),
        }
    }

    /// Generate a cryptographic build hash
    ///
    /// This hash uniquely identifies the version, edition, and build of oxidize-pdf.
    /// It cannot be easily spoofed without access to the source code.
    fn generate_build_hash(edition: Edition) -> String {
        let mut hasher = Sha256::new();

        // Hash version
        hasher.update(env!("CARGO_PKG_VERSION").as_bytes());

        // Hash edition
        hasher.update(edition.as_str().as_bytes());

        // Hash build timestamp (if available) or use a constant
        // In production, this would be set during build with a build script
        let build_time = option_env!("BUILD_TIMESTAMP").unwrap_or("2024-10-05");
        hasher.update(build_time.as_bytes());

        // Hash a secret salt (makes it harder to reverse engineer)
        hasher.update(b"oxidize-pdf-signature-v1");

        let hash = hasher.finalize();

        // Use first 8 bytes (16 hex chars) for the signature
        format!("oxpdf-{}", hex_encode(&hash[..8]))
    }

    /// Compute feature fingerprint from document
    ///
    /// Uses bit flags to represent which features are present in the document.
    /// This helps identify advanced feature usage for licensing purposes.
    fn compute_features(document: &Document) -> u16 {
        let mut features = 0u16;

        // Bit 0: Encryption
        if document.encryption.is_some() {
            features |= 0x0001;
        }

        // Bit 1: Semantic entities (AI-Ready PDFs)
        if !document.semantic_entities.is_empty() {
            features |= 0x0002;
        }

        // Bit 2: Document outline/bookmarks
        if document.outline.is_some() {
            features |= 0x0004;
        }

        // Bit 3: Interactive forms (AcroForm)
        if document.acro_form.is_some() {
            features |= 0x0008;
        }

        // Bit 4: Named destinations
        if document.named_destinations.is_some() {
            features |= 0x0010;
        }

        // Bit 5: Page labels
        if document.page_labels.is_some() {
            features |= 0x0020;
        }

        // Bit 6: Open action
        if document.open_action.is_some() {
            features |= 0x0040;
        }

        // Bit 7: Viewer preferences
        if document.viewer_preferences.is_some() {
            features |= 0x0080;
        }

        // Bit 8: Custom fonts
        if !document.custom_fonts.is_empty() {
            features |= 0x0100;
        }

        // Bit 9: Compressed streams
        if document.compress {
            features |= 0x0200;
        }

        // Bit 10: XRef streams (PDF 1.5+)
        if document.use_xref_streams {
            features |= 0x0400;
        }

        // Bits 11-15: Reserved for future use

        features
    }

    /// Write signature fields to the PDF Info Dictionary
    ///
    /// These fields are NOT exposed in the public API and cannot be overridden by users.
    /// They provide a technical fingerprint for anti-spoofing and licensing purposes.
    pub fn write_to_info_dict(&self, info_dict: &mut Dictionary) {
        // Build signature (cryptographic hash)
        info_dict.set("oxidize-pdf-build", Object::String(self.build_hash.clone()));

        // Feature fingerprint (hex encoded bit flags)
        info_dict.set(
            "oxidize-pdf-features",
            Object::String(format!("{:04x}", self.features_fingerprint)),
        );

        // Edition marker (only for PRO/Enterprise builds)
        if self.edition != Edition::Community {
            info_dict.set(
                "oxidize-pdf-edition",
                Object::String(self.edition.as_str().to_string()),
            );
        }
    }

    /// Get the build hash
    #[allow(dead_code)]
    pub fn build_hash(&self) -> &str {
        &self.build_hash
    }

    /// Get the features fingerprint
    #[allow(dead_code)]
    pub fn features(&self) -> u16 {
        self.features_fingerprint
    }
}

/// Helper function to encode bytes as hex string
fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    #[test]
    fn test_edition_as_str() {
        assert_eq!(Edition::Community.as_str(), "Community");
        assert_eq!(Edition::Pro.as_str(), "PRO");
        assert_eq!(Edition::Enterprise.as_str(), "Enterprise");
    }

    #[test]
    fn test_build_hash_format() {
        let hash = PdfSignature::generate_build_hash(Edition::Community);
        assert!(hash.starts_with("oxpdf-"));
        assert_eq!(hash.len(), 22); // "oxpdf-" + 16 hex chars
    }

    #[test]
    fn test_build_hash_uniqueness() {
        let hash1 = PdfSignature::generate_build_hash(Edition::Community);
        let hash2 = PdfSignature::generate_build_hash(Edition::Pro);
        assert_ne!(
            hash1, hash2,
            "Different editions should have different hashes"
        );
    }

    #[test]
    fn test_compute_features_empty() {
        let doc = Document::new();
        let features = PdfSignature::compute_features(&doc);

        // Should have compression enabled by default (bit 9)
        assert!(features & 0x0200 != 0, "Compression should be enabled");
    }

    #[test]
    fn test_compute_features_with_encryption() {
        let mut doc = Document::new();

        // Create encryption settings with default permissions
        let permissions = crate::encryption::Permissions::default();
        let encryption = crate::document::DocumentEncryption::new(
            "user_password",
            "owner_password",
            permissions,
            crate::document::EncryptionStrength::Rc4_128bit,
        );
        doc.set_encryption(encryption);

        let features = PdfSignature::compute_features(&doc);

        // Should have encryption bit set (bit 0)
        assert!(features & 0x0001 != 0, "Encryption bit should be set");
    }

    #[test]
    fn test_pdf_signature_creation() {
        let doc = Document::new();
        let signature = PdfSignature::new(&doc, Edition::Community);

        assert_eq!(signature.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(signature.edition, Edition::Community);
        assert!(signature.build_hash.starts_with("oxpdf-"));
        assert!(signature.features_fingerprint > 0); // At least compression should be set
    }

    #[test]
    fn test_write_to_info_dict_community() {
        let doc = Document::new();
        let signature = PdfSignature::new(&doc, Edition::Community);
        let mut dict = Dictionary::new();

        signature.write_to_info_dict(&mut dict);

        // Should have build hash
        assert!(dict.get("oxidize-pdf-build").is_some());

        // Should have features fingerprint
        assert!(dict.get("oxidize-pdf-features").is_some());

        // Should NOT have edition marker for Community
        assert!(dict.get("oxidize-pdf-edition").is_none());
    }

    #[test]
    fn test_write_to_info_dict_pro() {
        let doc = Document::new();
        let signature = PdfSignature::new(&doc, Edition::Pro);
        let mut dict = Dictionary::new();

        signature.write_to_info_dict(&mut dict);

        // Should have edition marker for PRO
        let edition = dict.get("oxidize-pdf-edition");
        assert!(edition.is_some());
        if let Some(Object::String(ed)) = edition {
            assert_eq!(ed, "PRO");
        } else {
            panic!("Edition should be a string");
        }
    }

    #[test]
    fn test_features_fingerprint_format() {
        let doc = Document::new();
        let signature = PdfSignature::new(&doc, Edition::Community);
        let mut dict = Dictionary::new();

        signature.write_to_info_dict(&mut dict);

        let features = dict.get("oxidize-pdf-features").unwrap();
        if let Object::String(features_str) = features {
            // Should be 4 hex digits
            assert_eq!(features_str.len(), 4);
            assert!(
                features_str.chars().all(|c| c.is_ascii_hexdigit()),
                "Features should be hex encoded"
            );
        } else {
            panic!("Features should be a string");
        }
    }
}
