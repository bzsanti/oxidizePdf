//! PKCS#7/CMS signature parsing
//!
//! This module provides functionality to parse CMS (Cryptographic Message Syntax)
//! structures used in PDF digital signatures.
//!
//! # Features
//!
//! Requires the `signatures` feature to be enabled.

#[cfg(feature = "signatures")]
use cms::content_info::ContentInfo;
#[cfg(feature = "signatures")]
use cms::signed_data::SignedData;
#[cfg(feature = "signatures")]
use der::{Decode, Encode};
#[cfg(feature = "signatures")]
use x509_cert::Certificate;

use super::error::{SignatureError, SignatureResult};

/// Digest algorithm used for signature hash computation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DigestAlgorithm {
    /// SHA-256 (recommended)
    Sha256,
    /// SHA-384
    Sha384,
    /// SHA-512
    Sha512,
}

impl DigestAlgorithm {
    /// Returns the OID string for this algorithm
    pub fn oid(&self) -> &'static str {
        match self {
            DigestAlgorithm::Sha256 => "2.16.840.1.101.3.4.2.1",
            DigestAlgorithm::Sha384 => "2.16.840.1.101.3.4.2.2",
            DigestAlgorithm::Sha512 => "2.16.840.1.101.3.4.2.3",
        }
    }

    /// Returns the algorithm name
    pub fn name(&self) -> &'static str {
        match self {
            DigestAlgorithm::Sha256 => "SHA-256",
            DigestAlgorithm::Sha384 => "SHA-384",
            DigestAlgorithm::Sha512 => "SHA-512",
        }
    }
}

/// Signature algorithm used for signing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    /// RSA with SHA-256
    RsaSha256,
    /// RSA with SHA-384
    RsaSha384,
    /// RSA with SHA-512
    RsaSha512,
    /// ECDSA with SHA-256 (P-256 curve)
    EcdsaSha256,
    /// ECDSA with SHA-384 (P-384 curve)
    EcdsaSha384,
}

impl SignatureAlgorithm {
    /// Returns the algorithm name
    pub fn name(&self) -> &'static str {
        match self {
            SignatureAlgorithm::RsaSha256 => "RSA-SHA256",
            SignatureAlgorithm::RsaSha384 => "RSA-SHA384",
            SignatureAlgorithm::RsaSha512 => "RSA-SHA512",
            SignatureAlgorithm::EcdsaSha256 => "ECDSA-SHA256",
            SignatureAlgorithm::EcdsaSha384 => "ECDSA-SHA384",
        }
    }

    /// Returns the digest algorithm used by this signature algorithm
    pub fn digest_algorithm(&self) -> DigestAlgorithm {
        match self {
            SignatureAlgorithm::RsaSha256 | SignatureAlgorithm::EcdsaSha256 => {
                DigestAlgorithm::Sha256
            }
            SignatureAlgorithm::RsaSha384 | SignatureAlgorithm::EcdsaSha384 => {
                DigestAlgorithm::Sha384
            }
            SignatureAlgorithm::RsaSha512 => DigestAlgorithm::Sha512,
        }
    }
}

/// Parsed PKCS#7/CMS signature structure
#[derive(Debug, Clone)]
pub struct ParsedSignature {
    /// The digest algorithm used
    pub digest_algorithm: DigestAlgorithm,
    /// The signature algorithm used
    pub signature_algorithm: SignatureAlgorithm,
    /// The raw signature value bytes
    pub signature_value: Vec<u8>,
    /// The signer's certificate in DER format
    pub signer_certificate_der: Vec<u8>,
    /// Optional signing time from signed attributes
    pub signing_time: Option<String>,
}

impl ParsedSignature {
    /// Returns the signer's common name from the certificate
    #[cfg(feature = "signatures")]
    pub fn signer_common_name(&self) -> SignatureResult<String> {
        use der::asn1::{PrintableStringRef, Utf8StringRef};

        let cert = Certificate::from_der(&self.signer_certificate_der).map_err(|e| {
            SignatureError::CmsParsingFailed {
                details: format!("Failed to parse certificate: {}", e),
            }
        })?;

        // Extract CN from subject
        for rdn in cert.tbs_certificate.subject.0.iter() {
            for atv in rdn.0.iter() {
                // OID for commonName: 2.5.4.3
                if atv.oid.to_string() == "2.5.4.3" {
                    // Try to decode as UTF8String first, then PrintableString
                    if let Ok(utf8) = Utf8StringRef::try_from(&atv.value) {
                        return Ok(utf8.as_str().to_string());
                    }
                    if let Ok(printable) = PrintableStringRef::try_from(&atv.value) {
                        return Ok(printable.as_str().to_string());
                    }
                    // Fallback: return raw bytes as hex
                    return Ok(format!("<binary CN: {} bytes>", atv.value.value().len()));
                }
            }
        }

        Err(SignatureError::CmsParsingFailed {
            details: "Certificate has no common name".to_string(),
        })
    }

    #[cfg(not(feature = "signatures"))]
    pub fn signer_common_name(&self) -> SignatureResult<String> {
        Err(SignatureError::CmsParsingFailed {
            details: "signatures feature not enabled".to_string(),
        })
    }
}

/// Parses a PKCS#7/CMS signature from raw bytes (DER encoded)
///
/// # Arguments
///
/// * `contents` - The raw signature bytes from the PDF /Contents field
///
/// # Returns
///
/// A `ParsedSignature` containing the extracted signature information.
///
/// # Errors
///
/// Returns an error if the DER structure is invalid or unsupported.
#[cfg(feature = "signatures")]
pub fn parse_pkcs7_signature(contents: &[u8]) -> SignatureResult<ParsedSignature> {
    use const_oid::ObjectIdentifier;

    // Parse ContentInfo (top-level CMS structure)
    let content_info =
        ContentInfo::from_der(contents).map_err(|e| SignatureError::CmsParsingFailed {
            details: format!("Failed to parse ContentInfo: {}", e),
        })?;

    // Verify it's SignedData (OID 1.2.840.113549.1.7.2)
    const SIGNED_DATA_OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");
    if content_info.content_type != SIGNED_DATA_OID {
        return Err(SignatureError::CmsParsingFailed {
            details: format!(
                "Expected SignedData, got OID: {}",
                content_info.content_type
            ),
        });
    }

    // Extract SignedData
    let signed_data_bytes =
        content_info
            .content
            .to_der()
            .map_err(|e| SignatureError::CmsParsingFailed {
                details: format!("Failed to encode content: {}", e),
            })?;

    let signed_data =
        SignedData::from_der(&signed_data_bytes).map_err(|e| SignatureError::CmsParsingFailed {
            details: format!("Failed to parse SignedData: {}", e),
        })?;

    // Extract signer info (we expect exactly one)
    let signer_infos: Vec<_> = signed_data.signer_infos.0.iter().collect();
    if signer_infos.is_empty() {
        return Err(SignatureError::CmsParsingFailed {
            details: "No signer info found in SignedData".to_string(),
        });
    }
    let signer_info = &signer_infos[0];

    // Extract digest algorithm from signer info
    let digest_algorithm = parse_digest_algorithm(&signer_info.digest_alg.oid.to_string())?;

    // Extract signature algorithm
    let signature_algorithm = parse_signature_algorithm(
        &signer_info.signature_algorithm.oid.to_string(),
        digest_algorithm,
    )?;

    // Extract signature value
    let signature_value = signer_info.signature.as_bytes().to_vec();

    // Extract certificate
    let certificates =
        signed_data
            .certificates
            .as_ref()
            .ok_or_else(|| SignatureError::CmsParsingFailed {
                details: "No certificates in SignedData".to_string(),
            })?;

    // Find the signer's certificate (first one for simplicity)
    let cert_choices: Vec<_> = certificates.0.iter().collect();
    if cert_choices.is_empty() {
        return Err(SignatureError::CmsParsingFailed {
            details: "No certificates found".to_string(),
        });
    }

    // Get certificate DER
    let signer_certificate_der = match &cert_choices[0] {
        cms::cert::CertificateChoices::Certificate(cert) => {
            cert.to_der()
                .map_err(|e| SignatureError::CmsParsingFailed {
                    details: format!("Failed to encode certificate: {}", e),
                })?
        }
        _ => {
            return Err(SignatureError::CmsParsingFailed {
                details: "Unsupported certificate type".to_string(),
            })
        }
    };

    // Extract signing time from signed attributes if present
    let signing_time = extract_signing_time(signer_info);

    Ok(ParsedSignature {
        digest_algorithm,
        signature_algorithm,
        signature_value,
        signer_certificate_der,
        signing_time,
    })
}

#[cfg(not(feature = "signatures"))]
pub fn parse_pkcs7_signature(_contents: &[u8]) -> SignatureResult<ParsedSignature> {
    Err(SignatureError::CmsParsingFailed {
        details: "signatures feature not enabled".to_string(),
    })
}

/// Parses a digest algorithm OID string
#[cfg(feature = "signatures")]
fn parse_digest_algorithm(oid: &str) -> SignatureResult<DigestAlgorithm> {
    match oid {
        "2.16.840.1.101.3.4.2.1" => Ok(DigestAlgorithm::Sha256),
        "2.16.840.1.101.3.4.2.2" => Ok(DigestAlgorithm::Sha384),
        "2.16.840.1.101.3.4.2.3" => Ok(DigestAlgorithm::Sha512),
        _ => Err(SignatureError::UnsupportedAlgorithm {
            algorithm: format!("digest OID: {}", oid),
        }),
    }
}

/// Parses a signature algorithm OID string
#[cfg(feature = "signatures")]
fn parse_signature_algorithm(
    oid: &str,
    digest: DigestAlgorithm,
) -> SignatureResult<SignatureAlgorithm> {
    match oid {
        // RSA PKCS#1 v1.5
        "1.2.840.113549.1.1.1" => match digest {
            DigestAlgorithm::Sha256 => Ok(SignatureAlgorithm::RsaSha256),
            DigestAlgorithm::Sha384 => Ok(SignatureAlgorithm::RsaSha384),
            DigestAlgorithm::Sha512 => Ok(SignatureAlgorithm::RsaSha512),
        },
        // RSA with SHA-256
        "1.2.840.113549.1.1.11" => Ok(SignatureAlgorithm::RsaSha256),
        // RSA with SHA-384
        "1.2.840.113549.1.1.12" => Ok(SignatureAlgorithm::RsaSha384),
        // RSA with SHA-512
        "1.2.840.113549.1.1.13" => Ok(SignatureAlgorithm::RsaSha512),
        // ECDSA with SHA-256
        "1.2.840.10045.4.3.2" => Ok(SignatureAlgorithm::EcdsaSha256),
        // ECDSA with SHA-384
        "1.2.840.10045.4.3.3" => Ok(SignatureAlgorithm::EcdsaSha384),
        _ => Err(SignatureError::UnsupportedAlgorithm {
            algorithm: format!("signature OID: {}", oid),
        }),
    }
}

/// Extracts signing time from signer info signed attributes
#[cfg(feature = "signatures")]
fn extract_signing_time(signer_info: &cms::signed_data::SignerInfo) -> Option<String> {
    // OID for signingTime: 1.2.840.113549.1.9.5
    const SIGNING_TIME_OID: &str = "1.2.840.113549.1.9.5";

    signer_info.signed_attrs.as_ref().and_then(|attrs| {
        for attr in attrs.iter() {
            if attr.oid.to_string() == SIGNING_TIME_OID {
                // The attribute value contains the time
                // For now, return a placeholder - full parsing would decode ASN.1 time
                return Some("(signing time present)".to_string());
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Algorithm enum tests

    #[test]
    fn test_digest_algorithm_oid() {
        assert_eq!(DigestAlgorithm::Sha256.oid(), "2.16.840.1.101.3.4.2.1");
        assert_eq!(DigestAlgorithm::Sha384.oid(), "2.16.840.1.101.3.4.2.2");
        assert_eq!(DigestAlgorithm::Sha512.oid(), "2.16.840.1.101.3.4.2.3");
    }

    #[test]
    fn test_digest_algorithm_name() {
        assert_eq!(DigestAlgorithm::Sha256.name(), "SHA-256");
        assert_eq!(DigestAlgorithm::Sha384.name(), "SHA-384");
        assert_eq!(DigestAlgorithm::Sha512.name(), "SHA-512");
    }

    #[test]
    fn test_signature_algorithm_name() {
        assert_eq!(SignatureAlgorithm::RsaSha256.name(), "RSA-SHA256");
        assert_eq!(SignatureAlgorithm::EcdsaSha256.name(), "ECDSA-SHA256");
    }

    #[test]
    fn test_signature_algorithm_digest() {
        assert_eq!(
            SignatureAlgorithm::RsaSha256.digest_algorithm(),
            DigestAlgorithm::Sha256
        );
        assert_eq!(
            SignatureAlgorithm::RsaSha384.digest_algorithm(),
            DigestAlgorithm::Sha384
        );
        assert_eq!(
            SignatureAlgorithm::EcdsaSha384.digest_algorithm(),
            DigestAlgorithm::Sha384
        );
    }

    #[test]
    fn test_digest_algorithm_clone_copy() {
        let alg = DigestAlgorithm::Sha256;
        let cloned = alg.clone();
        let copied = alg;
        assert_eq!(alg, cloned);
        assert_eq!(alg, copied);
    }

    #[test]
    fn test_signature_algorithm_clone_copy() {
        let alg = SignatureAlgorithm::RsaSha256;
        let cloned = alg.clone();
        let copied = alg;
        assert_eq!(alg, cloned);
        assert_eq!(alg, copied);
    }

    #[test]
    fn test_digest_algorithm_debug() {
        let debug = format!("{:?}", DigestAlgorithm::Sha256);
        assert!(debug.contains("Sha256"));
    }

    #[test]
    fn test_signature_algorithm_debug() {
        let debug = format!("{:?}", SignatureAlgorithm::EcdsaSha256);
        assert!(debug.contains("EcdsaSha256"));
    }

    // Parsing tests (require signatures feature)

    #[cfg(feature = "signatures")]
    #[test]
    fn test_parse_digest_algorithm_sha256() {
        let result = parse_digest_algorithm("2.16.840.1.101.3.4.2.1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), DigestAlgorithm::Sha256);
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_parse_digest_algorithm_sha384() {
        let result = parse_digest_algorithm("2.16.840.1.101.3.4.2.2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), DigestAlgorithm::Sha384);
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_parse_digest_algorithm_sha512() {
        let result = parse_digest_algorithm("2.16.840.1.101.3.4.2.3");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), DigestAlgorithm::Sha512);
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_parse_digest_algorithm_unsupported() {
        let result = parse_digest_algorithm("1.2.3.4.5");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SignatureError::UnsupportedAlgorithm { .. }));
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_parse_signature_algorithm_rsa_sha256() {
        let result = parse_signature_algorithm("1.2.840.113549.1.1.11", DigestAlgorithm::Sha256);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SignatureAlgorithm::RsaSha256);
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_parse_signature_algorithm_ecdsa_sha256() {
        let result = parse_signature_algorithm("1.2.840.10045.4.3.2", DigestAlgorithm::Sha256);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), SignatureAlgorithm::EcdsaSha256);
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_parse_signature_algorithm_unsupported() {
        let result = parse_signature_algorithm("1.2.3.4.5", DigestAlgorithm::Sha256);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SignatureError::UnsupportedAlgorithm { .. }));
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_parse_pkcs7_invalid_der() {
        let invalid = vec![0x00, 0x01, 0x02, 0x03];
        let result = parse_pkcs7_signature(&invalid);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SignatureError::CmsParsingFailed { .. }));
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_parse_pkcs7_empty_input() {
        let result = parse_pkcs7_signature(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parsed_signature_debug() {
        let sig = ParsedSignature {
            digest_algorithm: DigestAlgorithm::Sha256,
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            signature_value: vec![1, 2, 3],
            signer_certificate_der: vec![4, 5, 6],
            signing_time: Some("2024-01-01".to_string()),
        };
        let debug = format!("{:?}", sig);
        assert!(debug.contains("Sha256"));
        assert!(debug.contains("RsaSha256"));
    }

    #[test]
    fn test_parsed_signature_clone() {
        let sig = ParsedSignature {
            digest_algorithm: DigestAlgorithm::Sha256,
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            signature_value: vec![1, 2, 3],
            signer_certificate_der: vec![4, 5, 6],
            signing_time: None,
        };
        let cloned = sig.clone();
        assert_eq!(sig.digest_algorithm, cloned.digest_algorithm);
        assert_eq!(sig.signature_value, cloned.signature_value);
    }
}
