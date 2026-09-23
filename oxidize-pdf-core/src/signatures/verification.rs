//! Hash computation and signature verification for PDF signatures
//!
//! This module provides functionality to compute document hashes based on
//! ByteRange and verify cryptographic signatures.
//!
//! # Features
//!
//! Requires the `signatures` feature for cryptographic verification.

use super::cms::{DetailedParsedSignature, DigestAlgorithm, ParsedSignature, SignatureAlgorithm};
use super::error::{SignatureError, SignatureResult};
use super::types::ByteRange;
use sha2::{Digest, Sha256, Sha384, Sha512};

/// Result of hash verification
#[derive(Debug, Clone)]
pub struct HashVerificationResult {
    /// The computed hash of the document bytes
    pub computed_hash: Vec<u8>,
    /// The digest algorithm used
    pub algorithm: DigestAlgorithm,
    /// The total number of bytes hashed
    pub bytes_hashed: u64,
}

impl HashVerificationResult {
    /// Returns the hash as a hex string
    pub fn hash_hex(&self) -> String {
        self.computed_hash
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

/// Result of signature verification
#[derive(Debug, Clone)]
pub struct SignatureVerificationResult {
    /// Whether the hash matches the expected value
    pub hash_valid: bool,
    /// Whether the cryptographic signature is valid
    pub signature_valid: bool,
    /// The digest algorithm used
    pub digest_algorithm: DigestAlgorithm,
    /// The signature algorithm used
    pub signature_algorithm: SignatureAlgorithm,
    /// Additional details or warnings
    pub details: Option<String>,
}

impl SignatureVerificationResult {
    /// Returns true if both hash and signature are valid
    pub fn is_valid(&self) -> bool {
        self.hash_valid && self.signature_valid
    }
}

/// Computes the hash of PDF bytes according to a ByteRange
///
/// The ByteRange specifies which portions of the document are covered by
/// the signature. Typically this includes everything except the /Contents
/// value itself.
///
/// # Arguments
///
/// * `pdf_bytes` - The complete PDF file bytes
/// * `byte_range` - The ByteRange specifying which bytes to hash
/// * `algorithm` - The digest algorithm to use
///
/// # Returns
///
/// A `HashVerificationResult` containing the computed hash.
///
/// # Errors
///
/// Returns an error if the ByteRange exceeds the document size.
///
/// # Example
///
/// ```ignore
/// use oxidize_pdf::signatures::{compute_pdf_hash, ByteRange, DigestAlgorithm};
///
/// let pdf_bytes = std::fs::read("signed.pdf")?;
/// let byte_range = ByteRange::new(vec![(0, 1000), (2000, 500)]);
/// let result = compute_pdf_hash(&pdf_bytes, &byte_range, DigestAlgorithm::Sha256)?;
/// println!("Hash: {}", result.hash_hex());
/// ```
pub fn compute_pdf_hash(
    pdf_bytes: &[u8],
    byte_range: &ByteRange,
    algorithm: DigestAlgorithm,
) -> SignatureResult<HashVerificationResult> {
    // Extract bytes from all ranges and compute hash
    let computed_hash = match algorithm {
        DigestAlgorithm::Sha256 => {
            let mut hasher = Sha256::new();
            for (offset, length) in byte_range.ranges() {
                hasher.update(byte_range_slice(pdf_bytes, *offset, *length)?);
            }
            hasher.finalize().to_vec()
        }
        DigestAlgorithm::Sha384 => {
            let mut hasher = Sha384::new();
            for (offset, length) in byte_range.ranges() {
                hasher.update(byte_range_slice(pdf_bytes, *offset, *length)?);
            }
            hasher.finalize().to_vec()
        }
        DigestAlgorithm::Sha512 => {
            let mut hasher = Sha512::new();
            for (offset, length) in byte_range.ranges() {
                hasher.update(byte_range_slice(pdf_bytes, *offset, *length)?);
            }
            hasher.finalize().to_vec()
        }
    };

    Ok(HashVerificationResult {
        computed_hash,
        algorithm,
        bytes_hashed: byte_range.total_bytes(),
    })
}

fn byte_range_slice(pdf_bytes: &[u8], offset: u64, length: u64) -> SignatureResult<&[u8]> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| SignatureError::InvalidByteRange {
            details: "ByteRange offset plus length overflows u64".to_string(),
        })?;
    if end > pdf_bytes.len() as u64 {
        return Err(SignatureError::ByteRangeExceedsDocument {
            offset,
            length,
            document_size: pdf_bytes.len() as u64,
        });
    }
    let start = usize::try_from(offset).map_err(|_| SignatureError::InvalidByteRange {
        details: "ByteRange offset does not fit in memory".to_string(),
    })?;
    let end = usize::try_from(end).map_err(|_| SignatureError::InvalidByteRange {
        details: "ByteRange end does not fit in memory".to_string(),
    })?;
    Ok(&pdf_bytes[start..end])
}

/// Verifies a PDF signature against the document bytes
///
/// This function performs the complete signature verification process:
/// 1. Computes the document hash from the ByteRange
/// 2. Verifies the cryptographic signature using the signer's public key
///
/// # Arguments
///
/// * `pdf_bytes` - The complete PDF file bytes
/// * `signature` - The parsed PKCS#7/CMS signature
/// * `byte_range` - The ByteRange specifying which bytes are signed
///
/// # Returns
///
/// A `SignatureVerificationResult` indicating whether the signature is valid.
///
/// # Errors
///
/// Returns an error if verification cannot be performed (e.g., unsupported algorithm).
#[cfg(feature = "signatures")]
pub fn verify_signature(
    pdf_bytes: &[u8],
    signature: &ParsedSignature,
    byte_range: &ByteRange,
) -> SignatureResult<SignatureVerificationResult> {
    verify_signature_inputs(pdf_bytes, signature, byte_range, None, None)
}

/// Verify a signature using retained CMS signed attributes and `/Contents` bytes.
#[cfg(feature = "signatures")]
pub fn verify_signature_detailed(
    pdf_bytes: &[u8],
    signature: &DetailedParsedSignature,
    byte_range: &ByteRange,
) -> SignatureResult<SignatureVerificationResult> {
    byte_range
        .validate()
        .map_err(|details| SignatureError::InvalidByteRange { details })?;
    validate_contents_exclusion(pdf_bytes, byte_range, &signature.cms_contents)?;
    verify_signature_inputs(
        pdf_bytes,
        &signature.signature,
        byte_range,
        signature.signed_attributes_der.as_deref(),
        signature.message_digest.as_deref(),
    )
}

#[cfg(feature = "signatures")]
fn verify_signature_inputs(
    pdf_bytes: &[u8],
    signature: &ParsedSignature,
    byte_range: &ByteRange,
    signed_attributes_der: Option<&[u8]>,
    message_digest: Option<&[u8]>,
) -> SignatureResult<SignatureVerificationResult> {
    use der::{Decode, Encode};
    use x509_cert::Certificate;

    byte_range
        .validate()
        .map_err(|details| SignatureError::InvalidByteRange { details })?;

    // Compute the document hash
    let hash_result = compute_pdf_hash(pdf_bytes, byte_range, signature.digest_algorithm)?;

    // Parse the certificate to get the public key
    let cert = Certificate::from_der(&signature.signer_certificate_der).map_err(|e| {
        SignatureError::CertificateExtractionFailed {
            details: format!("Failed to parse certificate: {}", e),
        }
    })?;

    // Get SPKI as DER bytes
    let spki_der = cert
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|e| SignatureError::CertificateExtractionFailed {
            details: format!("Failed to encode SPKI: {}", e),
        })?;

    let (hash_valid, signed_bytes, input_is_prehashed) =
        match (signed_attributes_der, message_digest) {
            (Some(attributes), Some(expected_digest)) => (
                hashes_match(&hash_result.computed_hash, expected_digest),
                attributes,
                false,
            ),
            (Some(_), None) => {
                return Err(SignatureError::HashVerificationFailed {
                    details: "CMS signed attributes are missing messageDigest".to_string(),
                })
            }
            (None, Some(_)) => {
                return Err(SignatureError::CmsParsingFailed {
                    details: "messageDigest is present without signed attributes".to_string(),
                })
            }
            (None, None) => (true, hash_result.computed_hash.as_slice(), true),
        };

    // Verify the signature over canonical signed attributes when CMS carries them.
    let signature_valid = match signature.signature_algorithm {
        SignatureAlgorithm::RsaSha256
        | SignatureAlgorithm::RsaSha384
        | SignatureAlgorithm::RsaSha512 => verify_rsa_signature(
            &spki_der,
            &signature.signature_value,
            signed_bytes,
            input_is_prehashed,
            signature,
        )?,
        SignatureAlgorithm::EcdsaSha256 | SignatureAlgorithm::EcdsaSha384 => {
            verify_ecdsa_signature(
                &spki_der,
                &signature.signature_value,
                signed_bytes,
                input_is_prehashed,
                signature,
            )?
        }
    };

    Ok(SignatureVerificationResult {
        hash_valid,
        signature_valid,
        digest_algorithm: signature.digest_algorithm,
        signature_algorithm: signature.signature_algorithm,
        details: None,
    })
}

#[cfg(feature = "signatures")]
fn validate_contents_exclusion(
    pdf_bytes: &[u8],
    byte_range: &ByteRange,
    cms_contents: &[u8],
) -> SignatureResult<()> {
    let ranges = byte_range.ranges();
    let gap_start =
        ranges[0]
            .0
            .checked_add(ranges[0].1)
            .ok_or_else(|| SignatureError::InvalidByteRange {
                details: "First ByteRange end overflows u64".to_string(),
            })?;
    let gap_end = ranges[1].0;
    if gap_start == gap_end {
        return Ok(());
    }
    let gap = byte_range_slice(pdf_bytes, gap_start, gap_end - gap_start)?;
    if gap.len() < 2 || gap.first() != Some(&b'<') || gap.last() != Some(&b'>') {
        return Err(SignatureError::InvalidByteRange {
            details: "ByteRange exclusion is not a PDF hexadecimal string".to_string(),
        });
    }
    let mut decoded = Vec::with_capacity((gap.len() - 2) / 2);
    let mut high_nibble = None;
    for byte in &gap[1..gap.len() - 1] {
        if byte.is_ascii_whitespace() {
            continue;
        }
        let nibble = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => {
                return Err(SignatureError::InvalidByteRange {
                    details: "ByteRange exclusion contains non-hexadecimal data".to_string(),
                })
            }
        };
        if let Some(high) = high_nibble.take() {
            decoded.push((high << 4) | nibble);
        } else {
            high_nibble = Some(nibble);
        }
    }
    if let Some(high) = high_nibble {
        decoded.push(high << 4);
    }
    if !hashes_match(&decoded, cms_contents) {
        return Err(SignatureError::InvalidByteRange {
            details: "ByteRange exclusion does not match the CMS /Contents value".to_string(),
        });
    }
    Ok(())
}

#[cfg(not(feature = "signatures"))]
pub fn verify_signature(
    _pdf_bytes: &[u8],
    _signature: &ParsedSignature,
    _byte_range: &ByteRange,
) -> SignatureResult<SignatureVerificationResult> {
    Err(SignatureError::SignatureVerificationFailed {
        details: "signatures feature not enabled".to_string(),
    })
}

#[cfg(not(feature = "signatures"))]
pub fn verify_signature_detailed(
    _pdf_bytes: &[u8],
    _signature: &DetailedParsedSignature,
    _byte_range: &ByteRange,
) -> SignatureResult<SignatureVerificationResult> {
    Err(SignatureError::SignatureVerificationFailed {
        details: "signatures feature not enabled".to_string(),
    })
}

/// Verifies an RSA signature
#[cfg(feature = "signatures")]
fn verify_rsa_signature(
    spki_der: &[u8],
    signature_bytes: &[u8],
    signed_bytes: &[u8],
    input_is_prehashed: bool,
    parsed_sig: &ParsedSignature,
) -> SignatureResult<bool> {
    use rsa::pkcs1v15::{Signature as RsaSignature, VerifyingKey};
    use rsa::signature::hazmat::PrehashVerifier;
    use rsa::signature::Verifier;
    use rsa::RsaPublicKey;
    use spki::DecodePublicKey;

    // Parse RSA public key from SPKI DER
    let public_key = RsaPublicKey::from_public_key_der(spki_der).map_err(|e| {
        SignatureError::CertificateExtractionFailed {
            details: format!("Failed to parse RSA public key: {}", e),
        }
    })?;

    // Create RSA signature from bytes
    let signature = RsaSignature::try_from(signature_bytes).map_err(|e| {
        SignatureError::SignatureVerificationFailed {
            details: format!("Invalid RSA signature format: {}", e),
        }
    })?;

    // CMS RSA signatures use the digest-specific PKCS#1 v1.5 DigestInfo prefix.
    let is_valid = match parsed_sig.digest_algorithm {
        DigestAlgorithm::Sha256 => {
            let verifying_key = VerifyingKey::<Sha256>::new(public_key);
            if input_is_prehashed {
                verifying_key
                    .verify_prehash(signed_bytes, &signature)
                    .is_ok()
            } else {
                verifying_key.verify(signed_bytes, &signature).is_ok()
            }
        }
        DigestAlgorithm::Sha384 => {
            let verifying_key = VerifyingKey::<Sha384>::new(public_key);
            if input_is_prehashed {
                verifying_key
                    .verify_prehash(signed_bytes, &signature)
                    .is_ok()
            } else {
                verifying_key.verify(signed_bytes, &signature).is_ok()
            }
        }
        DigestAlgorithm::Sha512 => {
            let verifying_key = VerifyingKey::<Sha512>::new(public_key);
            if input_is_prehashed {
                verifying_key
                    .verify_prehash(signed_bytes, &signature)
                    .is_ok()
            } else {
                verifying_key.verify(signed_bytes, &signature).is_ok()
            }
        }
    };

    Ok(is_valid)
}

/// Verifies an ECDSA signature
#[cfg(feature = "signatures")]
fn verify_ecdsa_signature(
    spki_der: &[u8],
    signature_bytes: &[u8],
    signed_bytes: &[u8],
    input_is_prehashed: bool,
    parsed_sig: &ParsedSignature,
) -> SignatureResult<bool> {
    use ecdsa::signature::hazmat::PrehashVerifier;
    use ecdsa::signature::Verifier;
    use spki::DecodePublicKey;

    match parsed_sig.signature_algorithm {
        SignatureAlgorithm::EcdsaSha256 => {
            // P-256 curve with SHA-256
            let public_key =
                p256::ecdsa::VerifyingKey::from_public_key_der(spki_der).map_err(|e| {
                    SignatureError::CertificateExtractionFailed {
                        details: format!("Failed to parse P-256 public key: {}", e),
                    }
                })?;

            let signature = p256::ecdsa::Signature::from_der(signature_bytes).map_err(|e| {
                SignatureError::SignatureVerificationFailed {
                    details: format!("Invalid ECDSA P-256 signature format: {}", e),
                }
            })?;

            Ok(if input_is_prehashed {
                public_key.verify_prehash(signed_bytes, &signature).is_ok()
            } else {
                public_key.verify(signed_bytes, &signature).is_ok()
            })
        }
        SignatureAlgorithm::EcdsaSha384 => {
            // P-384 curve with SHA-384
            let public_key =
                p384::ecdsa::VerifyingKey::from_public_key_der(spki_der).map_err(|e| {
                    SignatureError::CertificateExtractionFailed {
                        details: format!("Failed to parse P-384 public key: {}", e),
                    }
                })?;

            let signature = p384::ecdsa::Signature::from_der(signature_bytes).map_err(|e| {
                SignatureError::SignatureVerificationFailed {
                    details: format!("Invalid ECDSA P-384 signature format: {}", e),
                }
            })?;

            Ok(if input_is_prehashed {
                public_key.verify_prehash(signed_bytes, &signature).is_ok()
            } else {
                public_key.verify(signed_bytes, &signature).is_ok()
            })
        }
        _ => Err(SignatureError::UnsupportedAlgorithm {
            algorithm: format!("{:?}", parsed_sig.signature_algorithm),
        }),
    }
}

/// Detects if a PDF has been modified after signing (incremental update)
///
/// This function checks if there are any bytes after the signed region,
/// which would indicate an incremental update was applied after signing.
///
/// # Arguments
///
/// * `pdf_bytes` - The complete PDF file bytes
/// * `byte_range` - The ByteRange from the signature
///
/// # Returns
///
/// `true` if there are modifications after the signed region, `false` otherwise.
pub fn has_incremental_update(pdf_bytes: &[u8], byte_range: &ByteRange) -> bool {
    if byte_range.is_empty() {
        return false;
    }

    // Find the end of the signed region
    let ranges = byte_range.ranges();
    if let Some((last_offset, last_length)) = ranges.last() {
        let Some(signed_end) = last_offset
            .checked_add(*last_length)
            .and_then(|end| usize::try_from(end).ok())
        else {
            return true;
        };
        // If there are bytes after the signed region, it's an incremental update
        pdf_bytes.len() > signed_end
    } else {
        false
    }
}

/// Compares two hashes for equality
pub fn hashes_match(hash1: &[u8], hash2: &[u8]) -> bool {
    if hash1.len() != hash2.len() {
        return false;
    }
    // Constant-time comparison to prevent timing attacks
    use subtle::ConstantTimeEq;
    hash1.ct_eq(hash2).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Hash computation tests

    #[test]
    fn test_compute_pdf_hash_sha256() {
        let pdf_bytes = b"Hello, this is a test PDF content!";
        let byte_range = ByteRange::new(vec![(0, 10), (20, 14)]);

        let result = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha256).unwrap();

        assert_eq!(result.algorithm, DigestAlgorithm::Sha256);
        assert_eq!(result.computed_hash.len(), 32); // SHA-256 = 32 bytes
        assert_eq!(result.bytes_hashed, 24); // 10 + 14 = 24
    }

    #[test]
    fn test_compute_pdf_hash_sha384() {
        let pdf_bytes = b"Test PDF content for SHA-384 hashing";
        let byte_range = ByteRange::new(vec![(0, 20), (25, 11)]);

        let result = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha384).unwrap();

        assert_eq!(result.algorithm, DigestAlgorithm::Sha384);
        assert_eq!(result.computed_hash.len(), 48); // SHA-384 = 48 bytes
    }

    #[test]
    fn test_compute_pdf_hash_sha512() {
        let pdf_bytes = b"Test PDF content for SHA-512 hashing test";
        let byte_range = ByteRange::new(vec![(0, 20), (25, 16)]);

        let result = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha512).unwrap();

        assert_eq!(result.algorithm, DigestAlgorithm::Sha512);
        assert_eq!(result.computed_hash.len(), 64); // SHA-512 = 64 bytes
    }

    #[test]
    fn test_compute_pdf_hash_byterange_exceeds_document() {
        let pdf_bytes = b"Short content";
        let byte_range = ByteRange::new(vec![(0, 10), (100, 50)]); // 100+50 > 13

        let result = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha256);

        assert!(result.is_err());
        match result.unwrap_err() {
            SignatureError::ByteRangeExceedsDocument {
                offset,
                length,
                document_size,
            } => {
                assert_eq!(offset, 100);
                assert_eq!(length, 50);
                assert_eq!(document_size, 13);
            }
            _ => panic!("Expected ByteRangeExceedsDocument error"),
        }
    }

    #[test]
    fn test_compute_pdf_hash_empty_byterange() {
        let pdf_bytes = b"Some PDF content";
        let byte_range = ByteRange::new(vec![]);

        let result = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha256).unwrap();

        // Empty input should produce the hash of empty data
        assert_eq!(result.bytes_hashed, 0);
        // SHA-256 of empty string is well-known
        let expected_empty_hash =
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(result.hash_hex(), expected_empty_hash);
    }

    #[test]
    fn test_hash_hex_format() {
        let pdf_bytes = b"Test content";
        let byte_range = ByteRange::new(vec![(0, 12)]);

        let result = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha256).unwrap();

        let hex = result.hash_hex();
        assert_eq!(hex.len(), 64); // 32 bytes * 2 hex chars
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_pdf_hash_deterministic() {
        let pdf_bytes = b"Deterministic test content";
        // Total 26 bytes (indices 0-25), so (20, 6) covers indices 20-25
        let byte_range = ByteRange::new(vec![(0, 15), (20, 6)]);

        let result1 = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha256).unwrap();
        let result2 = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha256).unwrap();

        assert_eq!(result1.computed_hash, result2.computed_hash);
    }

    // Hash comparison tests

    #[test]
    fn test_hashes_match_identical() {
        let hash = vec![0xab, 0xcd, 0xef, 0x12];
        assert!(hashes_match(&hash, &hash));
    }

    #[test]
    fn test_hashes_match_different() {
        let hash1 = vec![0xab, 0xcd, 0xef, 0x12];
        let hash2 = vec![0xab, 0xcd, 0xef, 0x13];
        assert!(!hashes_match(&hash1, &hash2));
    }

    #[test]
    fn test_hashes_match_different_length() {
        let hash1 = vec![0xab, 0xcd, 0xef];
        let hash2 = vec![0xab, 0xcd, 0xef, 0x12];
        assert!(!hashes_match(&hash1, &hash2));
    }

    #[test]
    fn test_hashes_match_empty() {
        let empty: Vec<u8> = vec![];
        assert!(hashes_match(&empty, &empty));
    }

    // Incremental update detection tests

    #[test]
    fn test_has_incremental_update_no_update() {
        let pdf_bytes = b"PDF content here";
        let byte_range = ByteRange::new(vec![(0, 10), (12, 4)]); // Ends at byte 16

        assert!(!has_incremental_update(pdf_bytes, &byte_range));
    }

    #[test]
    fn test_has_incremental_update_with_update() {
        let pdf_bytes = b"PDF content here with extra bytes after signature";
        let byte_range = ByteRange::new(vec![(0, 10), (12, 4)]); // Ends at byte 16

        assert!(has_incremental_update(pdf_bytes, &byte_range));
    }

    #[test]
    fn test_has_incremental_update_empty_byterange() {
        let pdf_bytes = b"Some content";
        let byte_range = ByteRange::new(vec![]);

        assert!(!has_incremental_update(pdf_bytes, &byte_range));
    }

    // SignatureVerificationResult tests

    #[test]
    fn test_signature_verification_result_is_valid_both_valid() {
        let result = SignatureVerificationResult {
            hash_valid: true,
            signature_valid: true,
            digest_algorithm: DigestAlgorithm::Sha256,
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            details: None,
        };
        assert!(result.is_valid());
    }

    #[test]
    fn test_signature_verification_result_is_valid_hash_invalid() {
        let result = SignatureVerificationResult {
            hash_valid: false,
            signature_valid: true,
            digest_algorithm: DigestAlgorithm::Sha256,
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            details: None,
        };
        assert!(!result.is_valid());
    }

    #[test]
    fn test_signature_verification_result_is_valid_signature_invalid() {
        let result = SignatureVerificationResult {
            hash_valid: true,
            signature_valid: false,
            digest_algorithm: DigestAlgorithm::Sha256,
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            details: None,
        };
        assert!(!result.is_valid());
    }

    #[test]
    fn test_signature_verification_result_clone() {
        let result = SignatureVerificationResult {
            hash_valid: true,
            signature_valid: true,
            digest_algorithm: DigestAlgorithm::Sha384,
            signature_algorithm: SignatureAlgorithm::EcdsaSha384,
            details: Some("test details".to_string()),
        };
        let cloned = result.clone();
        assert_eq!(result.hash_valid, cloned.hash_valid);
        assert_eq!(result.details, cloned.details);
    }

    #[test]
    fn test_signature_verification_result_debug() {
        let result = SignatureVerificationResult {
            hash_valid: true,
            signature_valid: true,
            digest_algorithm: DigestAlgorithm::Sha256,
            signature_algorithm: SignatureAlgorithm::RsaSha256,
            details: None,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("hash_valid"));
        assert!(debug.contains("signature_valid"));
    }

    #[test]
    fn test_hash_verification_result_clone() {
        let result = HashVerificationResult {
            computed_hash: vec![0x12, 0x34, 0x56],
            algorithm: DigestAlgorithm::Sha256,
            bytes_hashed: 100,
        };
        let cloned = result.clone();
        assert_eq!(result.computed_hash, cloned.computed_hash);
        assert_eq!(result.bytes_hashed, cloned.bytes_hashed);
    }

    #[test]
    fn test_hash_verification_result_debug() {
        let result = HashVerificationResult {
            computed_hash: vec![0x12, 0x34],
            algorithm: DigestAlgorithm::Sha512,
            bytes_hashed: 50,
        };
        let debug = format!("{:?}", result);
        assert!(debug.contains("computed_hash"));
        assert!(debug.contains("Sha512"));
    }

    // Verify signature without feature tests (always available)

    #[test]
    fn test_compute_pdf_hash_multiple_ranges() {
        // Simulating a typical PDF signature ByteRange [0, offset1, offset2, length2]
        let pdf_bytes = b"0123456789SIGNATURE_CONTENTS_HEREabcdefghij";
        // ByteRange covers [0..10] and [32..10], skipping the "SIGNATURE_CONTENTS_HERE" part
        let byte_range = ByteRange::new(vec![(0, 10), (32, 10)]);

        let result = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha256).unwrap();

        // The hash should be computed from "0123456789" + "abcdefghij"
        assert_eq!(result.bytes_hashed, 20);
        assert_eq!(result.computed_hash.len(), 32);
    }

    #[test]
    fn test_hash_known_value() {
        // Known SHA-256 hash of "test"
        let pdf_bytes = b"test";
        let byte_range = ByteRange::new(vec![(0, 4)]);

        let result = compute_pdf_hash(pdf_bytes, &byte_range, DigestAlgorithm::Sha256).unwrap();

        // SHA-256("test") = 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
        assert_eq!(
            result.hash_hex(),
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }
}
