//! Optional build, edition and feature metadata for generated PDFs.
//!
//! These source-derived identifiers are ordinary Info entries, not digital
//! signatures, proof of origin, secrets or a licensing enforcement mechanism.
//! Document::set_build_identification controls whether the writer emits them.

use super::WriterConfig;
use crate::document::Document;
use crate::objects::{Dictionary, Object};
use sha2::{Digest, Sha256};

/// Edition of oxidize-pdf used to generate the PDF
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Open source edition (MIT license)
    OpenSource,
}

impl Edition {
    /// Get the edition as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            Edition::OpenSource => "OpenSource",
        }
    }
}

/// Source-derived build information and feature metadata
pub struct PdfBuildIdentification {
    /// Version of oxidize-pdf (e.g., "1.2.5")
    #[allow(dead_code)]
    version: String,
    /// Edition used to generate the PDF
    edition: Edition,
    /// Truncated SHA-256 identifier derived from version, edition and build timestamp.
    build_hash: String,
    /// Bit flags representing features used in the document
    features_fingerprint: u16,
}

impl PdfBuildIdentification {
    /// Collect descriptive build and feature metadata for a document
    ///
    /// # Arguments
    ///
    /// * `document` - The PDF document being generated
    /// * `edition` - The edition of oxidize-pdf being used
    pub fn new(document: &Document, edition: Edition, config: &WriterConfig) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            edition,
            build_hash: Self::generate_build_hash(edition),
            features_fingerprint: Self::compute_features(document, config),
        }
    }

    /// Generate the compatibility identifier from public build metadata.
    ///
    /// The inputs are public version/edition/build values, not a secret or proof
    /// of origin. The truncated hash is not guaranteed to uniquely identify a build.
    fn generate_build_hash(edition: Edition) -> String {
        let mut hasher = Sha256::new();

        // Hash version
        hasher.update(env!("CARGO_PKG_VERSION").as_bytes());

        // Hash edition
        hasher.update(edition.as_str().as_bytes());

        // Hash build timestamp (if available) or use a constant
        // Without BUILD_TIMESTAMP, builds with the same version and edition share
        // this input. Neither source code nor binary contents enter the hash.
        let build_time = option_env!("BUILD_TIMESTAMP").unwrap_or("2024-10-05");
        hasher.update(build_time.as_bytes());

        // Public domain-separation constant; not a secret salt.
        hasher.update(b"oxidize-pdf-signature-v1");

        let hash = hasher.finalize();

        // Keep the existing 8-byte (16 hex digit) identifier format.
        format!("oxpdf-{}", hex_encode(&hash[..8]))
    }

    /// Compute feature fingerprint from document
    ///
    /// Uses bit flags to represent which features are present in the document.
    /// This is descriptive metadata, not a licensing or authenticity check.
    fn compute_features(document: &Document, config: &WriterConfig) -> u16 {
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
        if config.compress_streams {
            features |= 0x0200;
        }

        // Bit 10: XRef streams (PDF 1.5+)
        if config.use_xref_streams {
            features |= 0x0400;
        }

        // Bits 11-15: Reserved for future use

        features
    }

    /// Write descriptive identification fields to the PDF Info Dictionary
    ///
    /// The caller decides whether to emit these optional, source-derived fields.
    pub fn write_to_info_dict(&self, info_dict: &mut Dictionary) {
        // Descriptive build identifier; not an authenticity check.
        info_dict.set("oxidize-pdf-build", Object::String(self.build_hash.clone()));

        // Feature fingerprint (hex encoded bit flags)
        info_dict.set(
            "oxidize-pdf-features",
            Object::String(format!("{:04x}", self.features_fingerprint)),
        );

        // Edition marker
        info_dict.set(
            "oxidize-pdf-edition",
            Object::String(self.edition.as_str().to_string()),
        );
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
        assert_eq!(Edition::OpenSource.as_str(), "OpenSource");
    }

    #[test]
    fn test_build_hash_format() {
        let hash = PdfBuildIdentification::generate_build_hash(Edition::OpenSource);
        assert!(hash.starts_with("oxpdf-"));
        assert_eq!(hash.len(), 22); // "oxpdf-" + 16 hex chars
    }

    #[test]
    fn test_compute_features_empty() {
        let doc = Document::new();
        let features = PdfBuildIdentification::compute_features(&doc, &WriterConfig::default());

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

        let features = PdfBuildIdentification::compute_features(&doc, &WriterConfig::default());

        // Should have encryption bit set (bit 0)
        assert!(features & 0x0001 != 0, "Encryption bit should be set");
    }

    #[test]
    fn test_build_identification_creation() {
        let doc = Document::new();
        let identification =
            PdfBuildIdentification::new(&doc, Edition::OpenSource, &WriterConfig::default());

        assert_eq!(identification.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(identification.edition, Edition::OpenSource);
        assert!(identification.build_hash.starts_with("oxpdf-"));
        assert!(identification.features_fingerprint > 0); // At least compression should be set
    }

    #[test]
    fn test_write_to_info_dict() {
        let doc = Document::new();
        let identification =
            PdfBuildIdentification::new(&doc, Edition::OpenSource, &WriterConfig::default());
        let mut dict = Dictionary::new();

        identification.write_to_info_dict(&mut dict);

        // Should have build hash
        assert!(dict.get("oxidize-pdf-build").is_some());

        // Should have features fingerprint
        assert!(dict.get("oxidize-pdf-features").is_some());
    }

    #[test]
    fn test_features_fingerprint_format() {
        let doc = Document::new();
        let identification =
            PdfBuildIdentification::new(&doc, Edition::OpenSource, &WriterConfig::default());
        let mut dict = Dictionary::new();

        identification.write_to_info_dict(&mut dict);

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
