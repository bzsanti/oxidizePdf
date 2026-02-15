//! PKCS#7/CMS signature parsing
//!
//! This module provides functionality to parse CMS (Cryptographic Message Syntax)
//! structures used in PDF digital signatures.
//!
//! # Features
//!
//! Requires the `signatures` feature to be enabled.
//!
//! # BER Support
//!
//! PDF signatures may use BER (indefinite-length) encoding. This module
//! automatically converts BER to DER before parsing.

#[cfg(feature = "signatures")]
use cms::content_info::ContentInfo;
#[cfg(feature = "signatures")]
use cms::signed_data::SignedData;
#[cfg(feature = "signatures")]
use der::{Decode, Encode};
#[cfg(feature = "signatures")]
use x509_cert::Certificate;

use super::error::{SignatureError, SignatureResult};

// =============================================================================
// BER Security Limits (DoS Protection)
// =============================================================================

/// Maximum nesting depth for BER structures.
/// Prevents stack overflow from deeply nested malicious input.
#[cfg(feature = "signatures")]
const MAX_BER_DEPTH: usize = 100;

/// Maximum output size for BER-to-DER conversion (10 MB).
/// Prevents memory exhaustion from malicious BER expansion.
#[cfg(feature = "signatures")]
const MAX_BER_OUTPUT_SIZE: usize = 10 * 1024 * 1024;

/// Convert BER-encoded data to DER by resolving indefinite-length encodings
///
/// BER allows indefinite-length encoding where a SEQUENCE or other constructed
/// type has length byte 0x80 and is terminated by 0x00 0x00. DER requires
/// definite-length encoding. This function recursively converts all indefinite
/// lengths to definite lengths.
///
/// # Security
///
/// This function includes DoS protection:
/// - Maximum nesting depth of 100 levels
/// - Maximum output size of 10 MB
#[cfg(feature = "signatures")]
fn ber_to_der(input: &[u8]) -> SignatureResult<Vec<u8>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    // Check if this is already DER (no indefinite length markers)
    if !contains_indefinite_length(input) {
        return Ok(input.to_vec());
    }

    // Calculate size budget: min of 100x expansion or absolute max
    let size_budget = std::cmp::min(input.len().saturating_mul(100), MAX_BER_OUTPUT_SIZE);

    // Parse and convert recursively with depth=0 and size budget
    convert_element(input, 0, size_budget).map(|(output, _)| output)
}

/// Check if the data starts with indefinite-length BER encoding
///
/// Indefinite length (0x80) is only valid for constructed types (bit 6 set).
/// This function verifies both conditions to avoid false positives when
/// 0x80 appears as a regular length value in primitive types.
#[cfg(feature = "signatures")]
fn contains_indefinite_length(data: &[u8]) -> bool {
    if data.len() < 2 {
        return false;
    }

    let tag = data[0];
    let length_byte = data[1];

    // Indefinite length (0x80) is only valid for constructed types
    // Bit 6 (0x20) indicates constructed encoding
    let is_constructed = (tag & 0x20) != 0;

    is_constructed && length_byte == 0x80
}

/// Parse and convert a single BER element to DER
///
/// # Arguments
///
/// * `input` - The BER-encoded bytes to convert
/// * `depth` - Current nesting depth (for DoS protection)
/// * `size_budget` - Remaining bytes allowed in output (for DoS protection)
///
/// # Returns
///
/// (converted_bytes, bytes_consumed) or error if limits exceeded
#[cfg(feature = "signatures")]
fn convert_element(
    input: &[u8],
    depth: usize,
    size_budget: usize,
) -> SignatureResult<(Vec<u8>, usize)> {
    // Fix #1: Check recursion depth limit
    if depth > MAX_BER_DEPTH {
        return Err(SignatureError::CmsParsingFailed {
            details: format!(
                "BER nesting too deep: {} levels (max {})",
                depth, MAX_BER_DEPTH
            ),
        });
    }

    if input.is_empty() {
        return Err(SignatureError::CmsParsingFailed {
            details: "Empty input in BER conversion".to_string(),
        });
    }

    let tag = input[0];
    if input.len() < 2 {
        return Err(SignatureError::CmsParsingFailed {
            details: "BER element too short".to_string(),
        });
    }

    let length_byte = input[1];

    // Check for indefinite length
    if length_byte == 0x80 {
        // Indefinite length - must find end-of-contents (0x00 0x00)
        let is_constructed = (tag & 0x20) != 0;
        if !is_constructed {
            return Err(SignatureError::CmsParsingFailed {
                details: "Indefinite length on primitive type".to_string(),
            });
        }

        // Find the matching end-of-contents by parsing nested elements
        let content_start = 2;
        let (content, content_len) =
            parse_indefinite_content(&input[content_start..], depth + 1, size_budget)?;

        // Fix #2: Check output size limit
        let output_size = 1 + 5 + content.len(); // tag + max_length_bytes + content
        if output_size > size_budget {
            return Err(SignatureError::CmsParsingFailed {
                details: format!(
                    "BER output exceeds size limit: {} bytes (max {} MB)",
                    output_size,
                    MAX_BER_OUTPUT_SIZE / 1024 / 1024
                ),
            });
        }

        // Build DER output with definite length
        let mut output = vec![tag];
        encode_der_length(&mut output, content.len());
        output.extend(content);

        // Total consumed: tag + 0x80 + content + 0x00 0x00
        let consumed = 2 + content_len + 2;
        Ok((output, consumed))
    } else if length_byte < 0x80 {
        // Short form length
        let length = length_byte as usize;
        let total = 2 + length;
        if input.len() < total {
            return Err(SignatureError::CmsParsingFailed {
                details: format!(
                    "BER element truncated (short form): need {} bytes, got {}",
                    total,
                    input.len()
                ),
            });
        }

        // Check if this is a constructed type that needs recursive conversion
        let is_constructed = (tag & 0x20) != 0;
        if is_constructed && length > 0 {
            let content = &input[2..2 + length];
            let converted_content = convert_constructed_content(content, depth + 1, size_budget)?;

            // Fix #2: Check output size limit
            let output_size = 1 + 5 + converted_content.len();
            if output_size > size_budget {
                return Err(SignatureError::CmsParsingFailed {
                    details: format!(
                        "BER output exceeds size limit: {} bytes (max {} MB)",
                        output_size,
                        MAX_BER_OUTPUT_SIZE / 1024 / 1024
                    ),
                });
            }

            let mut output = vec![tag];
            encode_der_length(&mut output, converted_content.len());
            output.extend(converted_content);
            Ok((output, total))
        } else {
            // Primitive or empty - copy as-is
            Ok((input[..total].to_vec(), total))
        }
    } else {
        // Long form length
        let num_octets = (length_byte & 0x7F) as usize;

        // Fix #3: Validate length bytes BEFORE reading them
        if num_octets == 0 {
            return Err(SignatureError::CmsParsingFailed {
                details: "Invalid BER length: zero octets in long form".to_string(),
            });
        }
        if num_octets > 4 {
            return Err(SignatureError::CmsParsingFailed {
                details: format!(
                    "BER length too large: {} octets (max 4, would exceed 4GB)",
                    num_octets
                ),
            });
        }
        if input.len() < 2 + num_octets {
            return Err(SignatureError::CmsParsingFailed {
                details: format!(
                    "BER truncated reading length: need {} bytes, got {}",
                    2 + num_octets,
                    input.len()
                ),
            });
        }

        // Now safe to read length bytes
        let mut length: usize = 0;
        for i in 0..num_octets {
            length = (length << 8) | (input[2 + i] as usize);
        }

        let content_start = 2 + num_octets;
        let total = content_start + length;
        if input.len() < total {
            return Err(SignatureError::CmsParsingFailed {
                details: format!(
                    "BER element truncated: declared {} bytes content, got {}",
                    length,
                    input.len().saturating_sub(content_start)
                ),
            });
        }

        // Check if this is a constructed type that needs recursive conversion
        let is_constructed = (tag & 0x20) != 0;
        if is_constructed && length > 0 {
            let content = &input[content_start..total];
            let converted_content = convert_constructed_content(content, depth + 1, size_budget)?;

            // Fix #2: Check output size limit
            let output_size = 1 + 5 + converted_content.len();
            if output_size > size_budget {
                return Err(SignatureError::CmsParsingFailed {
                    details: format!(
                        "BER output exceeds size limit: {} bytes (max {} MB)",
                        output_size,
                        MAX_BER_OUTPUT_SIZE / 1024 / 1024
                    ),
                });
            }

            let mut output = vec![tag];
            encode_der_length(&mut output, converted_content.len());
            output.extend(converted_content);
            Ok((output, total))
        } else {
            // Primitive - copy as-is
            Ok((input[..total].to_vec(), total))
        }
    }
}

/// Parse content with indefinite length until we find end-of-contents
///
/// # Arguments
///
/// * `input` - The content bytes (after the 0x80 length marker)
/// * `depth` - Current nesting depth
/// * `size_budget` - Remaining bytes allowed in output
///
/// # Returns
///
/// (converted_content, bytes_consumed_excluding_eoc)
#[cfg(feature = "signatures")]
fn parse_indefinite_content(
    input: &[u8],
    depth: usize,
    size_budget: usize,
) -> SignatureResult<(Vec<u8>, usize)> {
    let mut output = Vec::new();
    let mut pos = 0;

    while pos < input.len() {
        // Check for end-of-contents (0x00 0x00)
        if input.len() >= pos + 2 && input[pos] == 0x00 && input[pos + 1] == 0x00 {
            return Ok((output, pos));
        }

        // Check remaining budget before parsing next element
        let remaining_budget = size_budget.saturating_sub(output.len());
        if remaining_budget == 0 {
            return Err(SignatureError::CmsParsingFailed {
                details: format!(
                    "BER output exceeds size limit during indefinite content parsing (max {} MB)",
                    MAX_BER_OUTPUT_SIZE / 1024 / 1024
                ),
            });
        }

        // Parse next element with depth and budget propagation
        let (element, consumed) = convert_element(&input[pos..], depth, remaining_budget)?;
        output.extend(element);
        pos += consumed;
    }

    Err(SignatureError::CmsParsingFailed {
        details: "End-of-contents not found in indefinite-length encoding".to_string(),
    })
}

/// Convert all elements in a constructed type's content
///
/// # Arguments
///
/// * `input` - The content bytes of a constructed type
/// * `depth` - Current nesting depth
/// * `size_budget` - Remaining bytes allowed in output
#[cfg(feature = "signatures")]
fn convert_constructed_content(
    input: &[u8],
    depth: usize,
    size_budget: usize,
) -> SignatureResult<Vec<u8>> {
    let mut output = Vec::new();
    let mut pos = 0;

    while pos < input.len() {
        // Check remaining budget before parsing next element
        let remaining_budget = size_budget.saturating_sub(output.len());
        if remaining_budget == 0 {
            return Err(SignatureError::CmsParsingFailed {
                details: format!(
                    "BER output exceeds size limit during constructed content parsing (max {} MB)",
                    MAX_BER_OUTPUT_SIZE / 1024 / 1024
                ),
            });
        }

        let (element, consumed) = convert_element(&input[pos..], depth, remaining_budget)?;
        output.extend(element);
        pos += consumed;
    }

    Ok(output)
}

/// Encode a length in DER format
#[cfg(feature = "signatures")]
fn encode_der_length(output: &mut Vec<u8>, length: usize) {
    if length < 128 {
        output.push(length as u8);
    } else if length < 256 {
        output.push(0x81);
        output.push(length as u8);
    } else if length < 65536 {
        output.push(0x82);
        output.push((length >> 8) as u8);
        output.push(length as u8);
    } else if length < 16777216 {
        output.push(0x83);
        output.push((length >> 16) as u8);
        output.push((length >> 8) as u8);
        output.push(length as u8);
    } else {
        output.push(0x84);
        output.push((length >> 24) as u8);
        output.push((length >> 16) as u8);
        output.push((length >> 8) as u8);
        output.push(length as u8);
    }
}

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

    // Convert BER to DER if necessary (PDF signatures may use BER encoding)
    let der_contents = ber_to_der(contents)?;

    // Parse ContentInfo (top-level CMS structure)
    let content_info =
        ContentInfo::from_der(&der_contents).map_err(|e| SignatureError::CmsParsingFailed {
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

    // ==========================================================================
    // Security tests for BER-to-DER conversion (DoS protection)
    // ==========================================================================

    #[cfg(feature = "signatures")]
    #[test]
    fn test_ber_to_der_depth_limit_protection() {
        // Fix #1: Create deeply nested BER structure (>MAX_BER_DEPTH levels)
        // Each level: 0x30 0x80 (SEQUENCE indefinite) ... 0x00 0x00 (EOC)
        let depth = MAX_BER_DEPTH + 50; // 150 levels, exceeds limit of 100

        let mut ber = Vec::new();
        // Open 150 nested SEQUENCEs with indefinite length
        for _ in 0..depth {
            ber.push(0x30); // SEQUENCE tag (constructed)
            ber.push(0x80); // Indefinite length
        }
        // Add a primitive element at the deepest level
        ber.extend_from_slice(&[0x02, 0x01, 0x00]); // INTEGER 0
                                                    // Close all levels with end-of-contents
        for _ in 0..depth {
            ber.push(0x00);
            ber.push(0x00);
        }

        let result = ber_to_der(&ber);
        assert!(result.is_err(), "Should reject deeply nested BER");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("nesting too deep") || err_msg.contains("depth"),
            "Error should mention depth limit, got: {}",
            err_msg
        );
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_ber_to_der_size_limit_protection() {
        // Fix #2: Create BER with indefinite length containing huge nested content
        // The size limit is checked during BER-to-DER conversion when indefinite length is used
        //
        // Note: ber_to_der only processes data with indefinite length markers.
        // For regular DER with definite lengths, it passes through unchanged.
        // The size check happens during recursive conversion.

        // Create a valid BER with indefinite length, but simulate the budget check
        // by directly testing convert_element with a tiny budget
        let ber = vec![
            0x30, 0x80, // SEQUENCE indefinite length
            0x04, 0x82, 0x00, 0x10, // OCTET STRING with 16 bytes
        ];
        // Add 16 bytes of content
        let mut full_ber = ber;
        full_ber.extend(vec![0x41; 16]);
        full_ber.extend_from_slice(&[0x00, 0x00]); // End of contents

        // With a tiny size budget of 5, this should fail
        let result = convert_element(&full_ber, 0, 5);
        assert!(
            result.is_err(),
            "Should reject BER when output exceeds size budget"
        );

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("exceeds") || err_msg.contains("limit"),
            "Error should mention size limit, got: {}",
            err_msg
        );
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_ber_to_der_buffer_overread_protection() {
        // Fix #3: Create truncated BER inside indefinite content
        // This tests that convert_element properly validates lengths before reading

        // Outer SEQUENCE with indefinite length, containing truncated inner element
        let truncated_ber = vec![
            0x30, 0x80, // SEQUENCE indefinite
            0x30, 0x85, // Inner SEQUENCE with long form (5 bytes for length - invalid)
            0x01,
            0x02, // Only 2 length bytes provided
                  // Missing: 3 more length bytes, then content, then EOC
        ];

        let result = ber_to_der(&truncated_ber);
        assert!(
            result.is_err(),
            "Should reject BER with invalid/truncated length"
        );

        let err_msg = result.unwrap_err().to_string();
        // Should fail with "too large" (5 octets > 4 max) or truncation error
        assert!(
            err_msg.contains("too large")
                || err_msg.contains("truncated")
                || err_msg.contains("octets"),
            "Error should mention length issue, got: {}",
            err_msg
        );
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_ber_to_der_length_zero_octets() {
        // Fix #3: Long form with 0 octets (0x80 without being indefinite)
        // This is actually indefinite length for constructed types, but
        // for primitive types it's invalid
        let invalid = vec![0x02, 0x80]; // INTEGER with indefinite length (invalid)

        let result = ber_to_der(&invalid);
        // Either passes as DER (since primitive 0x80 = 128 byte length)
        // or fails as truncated - both are acceptable
        // The key is it shouldn't panic or buffer overread
        let _ = result; // Just ensure no panic
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_contains_indefinite_length_primitive_false_positive() {
        // Fix #4: Primitive type with 0x80 as length (128 bytes) - NOT indefinite
        // OCTET STRING with 128 bytes of content
        let primitive = vec![0x04, 0x80]; // OCTET STRING, length 128 (short form max)
        assert!(
            !contains_indefinite_length(&primitive),
            "Should not detect indefinite length on primitive type"
        );

        // INTEGER with 128-byte length
        let integer = vec![0x02, 0x80];
        assert!(
            !contains_indefinite_length(&integer),
            "Should not detect indefinite length on INTEGER"
        );
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_contains_indefinite_length_constructed_true() {
        // Fix #4: Constructed type with indefinite length - SHOULD detect
        let sequence = vec![0x30, 0x80]; // SEQUENCE indefinite
        assert!(
            contains_indefinite_length(&sequence),
            "Should detect indefinite length on SEQUENCE"
        );

        let set = vec![0x31, 0x80]; // SET indefinite
        assert!(
            contains_indefinite_length(&set),
            "Should detect indefinite length on SET"
        );

        // Context-specific constructed [0] with indefinite length
        let context = vec![0xA0, 0x80]; // [0] EXPLICIT/constructed
        assert!(
            contains_indefinite_length(&context),
            "Should detect indefinite length on context-specific constructed"
        );
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_ber_to_der_valid_conversion() {
        // Ensure valid BER still converts correctly
        // Simple SEQUENCE with INTEGER inside, using indefinite length
        let ber = vec![
            0x30, 0x80, // SEQUENCE indefinite length
            0x02, 0x01, 0x42, // INTEGER = 66
            0x00, 0x00, // End of contents
        ];

        let result = ber_to_der(&ber);
        assert!(result.is_ok(), "Valid BER should convert successfully");

        let der = result.unwrap();
        // DER should have definite length
        assert_eq!(der[0], 0x30, "Should be SEQUENCE");
        assert_eq!(der[1], 0x03, "Length should be 3 (definite)");
        assert_eq!(&der[2..5], &[0x02, 0x01, 0x42], "Content preserved");
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_ber_to_der_already_der() {
        // DER input should pass through unchanged
        let der = vec![
            0x30, 0x03, // SEQUENCE, length 3
            0x02, 0x01, 0x42, // INTEGER = 66
        ];

        let result = ber_to_der(&der);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), der, "DER should pass through unchanged");
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_ber_to_der_empty_input() {
        let result = ber_to_der(&[]);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_ber_to_der_moderate_nesting_ok() {
        // Nesting within limits should work (e.g., 10 levels)
        let depth = 10;

        let mut ber = Vec::new();
        for _ in 0..depth {
            ber.push(0x30);
            ber.push(0x80);
        }
        ber.extend_from_slice(&[0x02, 0x01, 0x00]); // INTEGER 0
        for _ in 0..depth {
            ber.push(0x00);
            ber.push(0x00);
        }

        let result = ber_to_der(&ber);
        assert!(
            result.is_ok(),
            "Moderate nesting ({} levels) should work",
            depth
        );
    }
}
