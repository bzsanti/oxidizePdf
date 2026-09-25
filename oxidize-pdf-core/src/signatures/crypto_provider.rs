//! WebPKI adapter to maintained Rust verification primitives.
//!
//! WebPKI still owns path building, algorithm-identifier matching, validity,
//! constraints and CRLs. This module only decodes public keys, enforces the
//! previous provider's key bounds, and dispatches signature verification.
//! No cryptographic primitive or certificate-validation logic lives here.

use der::Decode;
use ecdsa::signature::hazmat::PrehashVerifier;
use rsa::traits::PublicKeyParts;
use rustls_pki_types::{
    alg_id, AlgorithmIdentifier, InvalidSignature, SignatureVerificationAlgorithm,
};
use sha2::{Digest, Sha256, Sha384, Sha512};

#[derive(Debug, Clone, Copy)]
enum Primitive {
    RsaPkcs1Sha256,
    RsaPkcs1Sha384,
    RsaPkcs1Sha512,
    RsaPssSha256,
    RsaPssSha384,
    RsaPssSha512,
    P256Sha256,
    P256Sha384,
    P384Sha256,
    P384Sha384,
    Ed25519,
}

#[derive(Debug)]
struct Algorithm {
    key_id: AlgorithmIdentifier,
    signature_id: AlgorithmIdentifier,
    primitive: Primitive,
}

impl SignatureVerificationAlgorithm for Algorithm {
    fn public_key_alg_id(&self) -> AlgorithmIdentifier {
        self.key_id
    }
    fn signature_alg_id(&self) -> AlgorithmIdentifier {
        self.signature_id
    }

    fn verify_signature(
        &self,
        key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), InvalidSignature> {
        use Primitive::*;
        match self.primitive {
            P256Sha256 | P256Sha384 => {
                // Preserve ring's uncompressed SEC1 key contract.
                if key.len() != 65 || key.first() != Some(&4) {
                    return Err(InvalidSignature);
                }
                let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(key)
                    .map_err(|_| InvalidSignature)?;
                let signature =
                    p256::ecdsa::Signature::from_der(signature).map_err(|_| InvalidSignature)?;
                match self.primitive {
                    P256Sha256 => key.verify_prehash(&Sha256::digest(message), &signature),
                    _ => key.verify_prehash(&Sha384::digest(message), &signature),
                }
                .map_err(|_| InvalidSignature)
            }
            P384Sha256 | P384Sha384 => {
                if key.len() != 97 || key.first() != Some(&4) {
                    return Err(InvalidSignature);
                }
                let key = p384::ecdsa::VerifyingKey::from_sec1_bytes(key)
                    .map_err(|_| InvalidSignature)?;
                let signature =
                    p384::ecdsa::Signature::from_der(signature).map_err(|_| InvalidSignature)?;
                match self.primitive {
                    P384Sha256 => key.verify_prehash(&Sha256::digest(message), &signature),
                    _ => key.verify_prehash(&Sha384::digest(message), &signature),
                }
                .map_err(|_| InvalidSignature)
            }
            Ed25519 => {
                let bytes = key.try_into().map_err(|_| InvalidSignature)?;
                let key =
                    ed25519_dalek::VerifyingKey::from_bytes(bytes).map_err(|_| InvalidSignature)?;
                let signature = ed25519_dalek::Signature::from_slice(signature)
                    .map_err(|_| InvalidSignature)?;
                key.verify_strict(message, &signature)
                    .map_err(|_| InvalidSignature)
            }
            primitive => verify_rsa(primitive, key, message, signature),
        }
    }
}

fn rsa_key(bytes: &[u8]) -> Result<rsa::RsaPublicKey, InvalidSignature> {
    let encoded = rsa::pkcs1::RsaPublicKey::from_der(bytes).map_err(|_| InvalidSignature)?;
    // Bound allocation before converting untrusted DER integers to BigUint.
    if encoded.modulus.as_bytes().len() > 1024 || encoded.public_exponent.as_bytes().len() > 5 {
        return Err(InvalidSignature);
    }
    let n = rsa::BigUint::from_bytes_be(encoded.modulus.as_bytes());
    let e = rsa::BigUint::from_bytes_be(encoded.public_exponent.as_bytes());
    // The standard DER conversion caps RSA at 4096; WebPKI/ring supported 8192.
    let key = rsa::RsaPublicKey::new_with_max_size(n, e, 8192).map_err(|_| InvalidSignature)?;
    if key.n().bits() < 2048 {
        return Err(InvalidSignature);
    }
    Ok(key)
}

fn verify_rsa(
    primitive: Primitive,
    encoded: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<(), InvalidSignature> {
    let key = rsa_key(encoded)?;
    if signature.len() != key.size() {
        return Err(InvalidSignature);
    }
    use Primitive::*;
    let result = match primitive {
        RsaPkcs1Sha256 => key.verify(
            rsa::Pkcs1v15Sign::new::<Sha256>(),
            &Sha256::digest(message),
            signature,
        ),
        RsaPkcs1Sha384 => key.verify(
            rsa::Pkcs1v15Sign::new::<Sha384>(),
            &Sha384::digest(message),
            signature,
        ),
        RsaPkcs1Sha512 => key.verify(
            rsa::Pkcs1v15Sign::new::<Sha512>(),
            &Sha512::digest(message),
            signature,
        ),
        RsaPssSha256 => key.verify(
            rsa::Pss::new::<Sha256>(),
            &Sha256::digest(message),
            signature,
        ),
        RsaPssSha384 => key.verify(
            rsa::Pss::new::<Sha384>(),
            &Sha384::digest(message),
            signature,
        ),
        RsaPssSha512 => key.verify(
            rsa::Pss::new::<Sha512>(),
            &Sha512::digest(message),
            signature,
        ),
        _ => return Err(InvalidSignature),
    };
    result.map_err(|_| InvalidSignature)
}

// DER AlgorithmIdentifier contents: rsa-with-SHA2 OIDs with absent parameters.
// NULL and absent forms are both accepted by the previous WebPKI/ring provider.
const RSA_SHA256_ABSENT: AlgorithmIdentifier =
    AlgorithmIdentifier::from_slice(&[6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 11]);
const RSA_SHA384_ABSENT: AlgorithmIdentifier =
    AlgorithmIdentifier::from_slice(&[6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 12]);
const RSA_SHA512_ABSENT: AlgorithmIdentifier =
    AlgorithmIdentifier::from_slice(&[6, 9, 42, 134, 72, 134, 247, 13, 1, 1, 13]);

// Explicit list: dependency feature unification must never select a native
// provider (ureq in dev-dependencies can independently enable webpki/ring).
// RSA_SHA384 covers 2048..8192, including the redundant 3072..8192 ring entry.
pub(super) static ALL_VERIFICATION_ALGS: &[&dyn SignatureVerificationAlgorithm] = &[
    &Algorithm {
        key_id: alg_id::ECDSA_P256,
        signature_id: alg_id::ECDSA_SHA256,
        primitive: Primitive::P256Sha256,
    },
    &Algorithm {
        key_id: alg_id::ECDSA_P256,
        signature_id: alg_id::ECDSA_SHA384,
        primitive: Primitive::P256Sha384,
    },
    &Algorithm {
        key_id: alg_id::ECDSA_P384,
        signature_id: alg_id::ECDSA_SHA256,
        primitive: Primitive::P384Sha256,
    },
    &Algorithm {
        key_id: alg_id::ECDSA_P384,
        signature_id: alg_id::ECDSA_SHA384,
        primitive: Primitive::P384Sha384,
    },
    &Algorithm {
        key_id: alg_id::ED25519,
        signature_id: alg_id::ED25519,
        primitive: Primitive::Ed25519,
    },
    &Algorithm {
        key_id: alg_id::RSA_ENCRYPTION,
        signature_id: alg_id::RSA_PKCS1_SHA256,
        primitive: Primitive::RsaPkcs1Sha256,
    },
    &Algorithm {
        key_id: alg_id::RSA_ENCRYPTION,
        signature_id: alg_id::RSA_PSS_SHA256,
        primitive: Primitive::RsaPssSha256,
    },
    &Algorithm {
        key_id: alg_id::RSA_ENCRYPTION,
        signature_id: RSA_SHA256_ABSENT,
        primitive: Primitive::RsaPkcs1Sha256,
    },
    &Algorithm {
        key_id: alg_id::RSA_ENCRYPTION,
        signature_id: alg_id::RSA_PKCS1_SHA384,
        primitive: Primitive::RsaPkcs1Sha384,
    },
    &Algorithm {
        key_id: alg_id::RSA_ENCRYPTION,
        signature_id: alg_id::RSA_PSS_SHA384,
        primitive: Primitive::RsaPssSha384,
    },
    &Algorithm {
        key_id: alg_id::RSA_ENCRYPTION,
        signature_id: RSA_SHA384_ABSENT,
        primitive: Primitive::RsaPkcs1Sha384,
    },
    &Algorithm {
        key_id: alg_id::RSA_ENCRYPTION,
        signature_id: alg_id::RSA_PKCS1_SHA512,
        primitive: Primitive::RsaPkcs1Sha512,
    },
    &Algorithm {
        key_id: alg_id::RSA_ENCRYPTION,
        signature_id: alg_id::RSA_PSS_SHA512,
        primitive: Primitive::RsaPssSha512,
    },
    &Algorithm {
        key_id: alg_id::RSA_ENCRYPTION,
        signature_id: RSA_SHA512_ABSENT,
        primitive: Primitive::RsaPkcs1Sha512,
    },
];

#[cfg(test)]
#[path = "crypto_provider_tests.rs"]
mod tests;
