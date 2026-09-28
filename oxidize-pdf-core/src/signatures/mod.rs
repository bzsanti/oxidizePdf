//! Basic digital signatures and external signing for PDF documents.
//!
//! Detect and parse signature fields, verify supported PKCS#7/CMS signatures,
//! and validate certificates with configured trust and revocation data.
//! Preparation is not a completed cryptographic signature. The caller signs
//! [`PreparedSignature::bytes_to_digest`] externally and passes DER CMS to
//! [`PreparedSignature::finalize`]. Finalization checks the outer DER SEQUENCE envelope and
//! capacity; it does not validate CMS SignedData, cryptographic validity or trust.
//!
//! PAdES is left as an extension for anyone who wants to implement it using the
//! existing external-signing interfaces. The core does not provide a complete
//! PAdES implementation; preparing/embedding CMS does not establish profile compliance.
//!
//! Inspect hash/signature/certificate results, errors, warnings and later
//! modifications. An `Ok` result alone does not mean a signature is valid.
//! Missing revocation evidence is not checked-valid. Cryptographic verification
//! requires the `signatures` feature and caller-appropriate trust configuration.
//!
//! # Prepare bytes for an external signer
//!
//! ```
//! use oxidize_pdf::{Document, Page};
//! use oxidize_pdf::signatures::{
//!     prepare_incremental_signature, SignaturePreparationOptions,
//! };
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut document = Document::new();
//! document.add_page(Page::a4());
//! let base = document.to_bytes()?;
//! let options = SignaturePreparationOptions::invisible("Approval");
//! let prepared = prepare_incremental_signature(&base, &options)?;
//! let bytes_to_sign = prepared.bytes_to_digest(); // bytes, not a digest
//! let expected: Vec<u8> = prepared.byte_range().ranges().iter()
//!     .flat_map(|&(offset, length)| {
//!         prepared.prepared_pdf()[offset as usize..(offset + length) as usize]
//!             .iter().copied()
//!     }).collect();
//! assert_eq!(bytes_to_sign, expected);
//! assert!(prepared.prepared_pdf().starts_with(&base));
//! // Supply bytes_to_sign to your external CMS signer, then finalize(&cms).
//! // Empty CMS is rejected; no signature has been created by preparation.
//! assert!(prepared.finalize(&[]).is_err());
//! # Ok(())
//! # }
//! ```
//!
//! [`SignaturePreparationOptions`] controls field/widget selection, reserved
//! capacity, filter/sub-filter, metadata, DocMDP/FieldMDP and additional entries.
//! Structural entries cannot be overridden. Encrypted PDFs and modifications
//! prohibited by existing permissions are rejected. Reprepare and sign again if
//! CMS exceeds the reserved capacity. Appearance artwork is covered by the CMS.
//! See the [operation-level contract](https://github.com/bzsanti/oxidizePdf/blob/main/docs/signatures.md).

mod certificate;
mod cms;
mod detection;
mod error;
mod permissions;
mod preparation;
mod signing;
pub use preparation::{
    complete_signature_slot, create_signature_slot, draw_signature_slot, list_signature_slots,
    read_signature_slot, remove_signature_slot, SignatureSlot,
};
mod types;
mod verification;

// Public exports
pub use certificate::{
    validate_certificate, validate_certificate_chain, CertificateValidationResult,
    DetailedCertificateValidationResult, RevocationStatus, TrustStore,
};
#[cfg(feature = "signatures")]
pub use certificate::{validate_certificate_at_time, validate_certificate_detailed_at_time};
pub use cms::{
    parse_pkcs7_signature, parse_pkcs7_signature_detailed, DetailedParsedSignature,
    DigestAlgorithm, ParsedSignature, SignatureAlgorithm,
};
pub use detection::detect_signature_fields;
pub use error::{SignatureError, SignatureResult};
pub(crate) use permissions::{ensure_modification_allowed, IncrementalModification};
pub use signing::{
    prepare_incremental_signature, prepare_incremental_signature_with_appearance,
    CertificationPermission, FieldLock, PreparedSignature, SignatureAppearance,
    SignatureAppearanceLayout, SignaturePreparationOptions, SignatureRect, SignatureTarget,
    SignatureWatermark,
};
pub use types::{ByteRange, SignatureField};
// FullSignatureValidationResult is defined below in this file
pub use verification::{
    compute_pdf_hash, has_incremental_update, hashes_match, verify_signature,
    verify_signature_detailed, HashVerificationResult, SignatureVerificationResult,
};

/// Complete signature validation result combining all verification steps
///
/// This struct provides a comprehensive view of a signature's validity,
/// including hash verification, cryptographic signature verification,
/// and certificate validation.
///
/// # Example
///
/// ```ignore
/// use oxidize_pdf::parser::PdfReader;
///
/// let mut reader = PdfReader::open("signed.pdf")?;
/// let results = reader.verify_signatures()?;
///
/// for result in &results {
///     if result.is_valid() {
///         println!("Valid signature from: {}", result.signer_name());
///     } else {
///         println!("Invalid signature: {:?}", result.validation_errors());
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FullSignatureValidationResult {
    /// The signature field information
    pub field: SignatureField,
    /// The signer's common name (extracted from certificate)
    pub signer_name: Option<String>,
    /// Signing time (if present in signed attributes)
    pub signing_time: Option<String>,
    /// Whether the document hash is valid
    pub hash_valid: bool,
    /// Whether the cryptographic signature is valid
    pub signature_valid: bool,
    /// Certificate validation result (if available)
    pub certificate_result: Option<CertificateValidationResult>,
    /// Whether there are modifications after the signature (incremental update)
    pub has_modifications_after_signing: bool,
    /// Validation errors encountered
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

impl FullSignatureValidationResult {
    /// Returns true if the signature is completely valid
    ///
    /// A signature is valid when:
    /// - The document hash matches
    /// - The cryptographic signature verifies
    /// - A certificate result is present and valid
    /// - There are no modifications after signing
    pub fn is_valid(&self) -> bool {
        self.hash_valid
            && self.signature_valid
            && self.errors.is_empty()
            && !self.has_modifications_after_signing
            && self
                .certificate_result
                .as_ref()
                .map(|c| c.is_valid())
                .unwrap_or(false)
    }

    /// Returns the signer's name, or a placeholder if unknown
    pub fn signer_name(&self) -> &str {
        self.signer_name.as_deref().unwrap_or("<unknown>")
    }

    /// Returns true if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
            || self
                .certificate_result
                .as_ref()
                .map(|c| c.has_warnings())
                .unwrap_or(false)
    }

    /// Returns all validation errors
    pub fn validation_errors(&self) -> &[String] {
        &self.errors
    }

    /// Returns all warnings (including certificate warnings)
    pub fn all_warnings(&self) -> Vec<String> {
        let mut all = self.warnings.clone();
        if let Some(cert) = &self.certificate_result {
            all.extend(cert.warnings.clone());
        }
        if self.has_modifications_after_signing {
            all.push("Document was modified after signing (incremental update)".to_string());
        }
        all
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_signature_validation_result_is_valid_all_pass() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100), (200, 100)]),
                vec![1, 2, 3],
            ),
            signer_name: Some("Test User".to_string()),
            signing_time: Some("2024-01-01T12:00:00Z".to_string()),
            hash_valid: true,
            signature_valid: true,
            certificate_result: Some(CertificateValidationResult {
                subject: "CN=Test User".to_string(),
                issuer: "CN=Test CA".to_string(),
                valid_from: "2024-01-01".to_string(),
                valid_to: "2025-01-01".to_string(),
                is_time_valid: true,
                is_trusted: true,
                is_signature_capable: true,
                warnings: vec![],
            }),
            has_modifications_after_signing: false,
            errors: vec![],
            warnings: vec![],
        };
        assert!(result.is_valid());
    }

    #[test]
    fn test_full_signature_validation_result_invalid_hash() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100), (200, 100)]),
                vec![1, 2, 3],
            ),
            signer_name: None,
            signing_time: None,
            hash_valid: false,
            signature_valid: true,
            certificate_result: None,
            has_modifications_after_signing: false,
            errors: vec!["Hash mismatch".to_string()],
            warnings: vec![],
        };
        assert!(!result.is_valid());
    }

    #[test]
    fn test_full_signature_validation_result_invalid_signature() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100), (200, 100)]),
                vec![1, 2, 3],
            ),
            signer_name: None,
            signing_time: None,
            hash_valid: true,
            signature_valid: false,
            certificate_result: None,
            has_modifications_after_signing: false,
            errors: vec!["Signature verification failed".to_string()],
            warnings: vec![],
        };
        assert!(!result.is_valid());
    }

    #[test]
    fn test_full_signature_validation_result_modified_after_signing() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100), (200, 100)]),
                vec![1, 2, 3],
            ),
            signer_name: Some("Test".to_string()),
            signing_time: None,
            hash_valid: true,
            signature_valid: true,
            certificate_result: None,
            has_modifications_after_signing: true,
            errors: vec![],
            warnings: vec![],
        };
        assert!(!result.is_valid());
    }

    #[test]
    fn test_full_signature_validation_result_signer_name() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100)]),
                vec![],
            ),
            signer_name: Some("John Doe".to_string()),
            signing_time: None,
            hash_valid: true,
            signature_valid: true,
            certificate_result: None,
            has_modifications_after_signing: false,
            errors: vec![],
            warnings: vec![],
        };
        assert_eq!(result.signer_name(), "John Doe");
    }

    #[test]
    fn test_full_signature_validation_result_signer_name_unknown() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100)]),
                vec![],
            ),
            signer_name: None,
            signing_time: None,
            hash_valid: true,
            signature_valid: true,
            certificate_result: None,
            has_modifications_after_signing: false,
            errors: vec![],
            warnings: vec![],
        };
        assert_eq!(result.signer_name(), "<unknown>");
    }

    #[test]
    fn test_full_signature_validation_result_has_warnings() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100)]),
                vec![],
            ),
            signer_name: None,
            signing_time: None,
            hash_valid: true,
            signature_valid: true,
            certificate_result: None,
            has_modifications_after_signing: false,
            errors: vec![],
            warnings: vec!["Test warning".to_string()],
        };
        assert!(result.has_warnings());
    }

    #[test]
    fn test_full_signature_validation_result_all_warnings() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100)]),
                vec![],
            ),
            signer_name: None,
            signing_time: None,
            hash_valid: true,
            signature_valid: true,
            certificate_result: Some(CertificateValidationResult {
                subject: "CN=Test".to_string(),
                issuer: "CN=CA".to_string(),
                valid_from: "2024-01-01".to_string(),
                valid_to: "2025-01-01".to_string(),
                is_time_valid: true,
                is_trusted: true,
                is_signature_capable: true,
                warnings: vec!["Self-signed certificate".to_string()],
            }),
            has_modifications_after_signing: true,
            errors: vec![],
            warnings: vec!["Generic warning".to_string()],
        };

        let all = result.all_warnings();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&"Generic warning".to_string()));
        assert!(all.contains(&"Self-signed certificate".to_string()));
        assert!(all.iter().any(|w| w.contains("modified after signing")));
    }

    #[test]
    fn test_full_signature_validation_result_invalid_certificate() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100)]),
                vec![],
            ),
            signer_name: Some("Test".to_string()),
            signing_time: None,
            hash_valid: true,
            signature_valid: true,
            certificate_result: Some(CertificateValidationResult {
                subject: "CN=Test".to_string(),
                issuer: "CN=CA".to_string(),
                valid_from: "2024-01-01".to_string(),
                valid_to: "2025-01-01".to_string(),
                is_time_valid: false, // Expired
                is_trusted: true,
                is_signature_capable: true,
                warnings: vec![],
            }),
            has_modifications_after_signing: false,
            errors: vec![],
            warnings: vec![],
        };
        assert!(!result.is_valid()); // Certificate is expired
    }

    #[test]
    fn test_full_signature_validation_result_clone() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100)]),
                vec![],
            ),
            signer_name: Some("Test".to_string()),
            signing_time: None,
            hash_valid: true,
            signature_valid: true,
            certificate_result: None,
            has_modifications_after_signing: false,
            errors: vec![],
            warnings: vec![],
        };
        let cloned = result.clone();
        assert_eq!(result.signer_name, cloned.signer_name);
        assert_eq!(result.hash_valid, cloned.hash_valid);
    }

    #[test]
    fn test_full_signature_validation_result_debug() {
        let result = FullSignatureValidationResult {
            field: SignatureField::new(
                "Adobe.PPKLite".to_string(),
                ByteRange::new(vec![(0, 100)]),
                vec![],
            ),
            signer_name: Some("Test".to_string()),
            signing_time: None,
            hash_valid: true,
            signature_valid: true,
            certificate_result: None,
            has_modifications_after_signing: false,
            errors: vec![],
            warnings: vec![],
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("hash_valid"));
        assert!(debug.contains("signature_valid"));
    }
}
