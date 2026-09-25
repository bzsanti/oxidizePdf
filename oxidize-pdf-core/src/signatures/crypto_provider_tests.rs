use super::ALL_VERIFICATION_ALGS;
use der::Decode;
use rustls_pki_types::alg_id;

#[test]
fn openssl_vectors_accept_valid_and_reject_tampered_inputs() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/signatures/provider_627");
    let message = std::fs::read(root.join("message.bin")).unwrap();
    let cases = [
        ("rsa", "rsa_pkcs1_sha256", alg_id::RSA_PKCS1_SHA256),
        ("rsa", "rsa_pkcs1_sha384", alg_id::RSA_PKCS1_SHA384),
        ("rsa", "rsa_pkcs1_sha512", alg_id::RSA_PKCS1_SHA512),
        ("rsa", "rsa_pss_sha256", alg_id::RSA_PSS_SHA256),
        ("rsa", "rsa_pss_sha384", alg_id::RSA_PSS_SHA384),
        ("rsa", "rsa_pss_sha512", alg_id::RSA_PSS_SHA512),
        ("p256", "p256_ecdsa_sha256", alg_id::ECDSA_SHA256),
        ("p256", "p256_ecdsa_sha384", alg_id::ECDSA_SHA384),
        ("p384", "p384_ecdsa_sha256", alg_id::ECDSA_SHA256),
        ("p384", "p384_ecdsa_sha384", alg_id::ECDSA_SHA384),
        ("ed25519", "ed25519", alg_id::ED25519),
    ];
    for (key_name, signature_name, signature_id) in cases {
        let der = std::fs::read(root.join(format!("{key_name}.spki"))).unwrap();
        let spki = spki::SubjectPublicKeyInfoRef::from_der(&der).unwrap();
        let key = spki.subject_public_key.as_bytes().unwrap();
        let key_id = match key_name {
            "rsa" => alg_id::RSA_ENCRYPTION,
            "p256" => alg_id::ECDSA_P256,
            "p384" => alg_id::ECDSA_P384,
            _ => alg_id::ED25519,
        };
        let algorithm = ALL_VERIFICATION_ALGS
            .iter()
            .find(|alg| alg.public_key_alg_id() == key_id && alg.signature_alg_id() == signature_id)
            .expect(signature_name);
        let signature = std::fs::read(root.join(format!("{signature_name}.sig"))).unwrap();
        assert!(
            algorithm
                .verify_signature(key, &message, &signature)
                .is_ok(),
            "{signature_name}"
        );
        let mut altered_message = message.clone();
        altered_message[0] ^= 1;
        assert!(
            algorithm
                .verify_signature(key, &altered_message, &signature)
                .is_err(),
            "{signature_name}: message"
        );
        let mut altered_signature = signature.clone();
        *altered_signature.last_mut().unwrap() ^= 1;
        assert!(
            algorithm
                .verify_signature(key, &message, &altered_signature)
                .is_err(),
            "{signature_name}: signature"
        );
        let wrong_der = std::fs::read(root.join(format!("{key_name}_wrong.spki"))).unwrap();
        let wrong_spki = spki::SubjectPublicKeyInfoRef::from_der(&wrong_der).unwrap();
        let wrong_key = wrong_spki.subject_public_key.as_bytes().unwrap();
        assert!(
            algorithm
                .verify_signature(wrong_key, &message, &signature)
                .is_err(),
            "{signature_name}: valid wrong key"
        );
        for malformed in [&[][..], &[0][..], &[0xff; 16][..]] {
            assert!(
                algorithm
                    .verify_signature(malformed, &message, &signature)
                    .is_err(),
                "{signature_name}: malformed key"
            );
            assert!(
                algorithm
                    .verify_signature(key, &message, malformed)
                    .is_err(),
                "{signature_name}: malformed signature"
            );
        }
    }
}

#[test]
fn rsa_key_size_bounds_preserve_8192_bit_support() {
    use der::{asn1::UintRef, Encode};
    for (bits, accepted) in [
        (1024usize, false),
        (2047, false),
        (2048, true),
        (4096, true),
        (8192, true),
        (8193, false),
    ] {
        let mut modulus = vec![0xff; bits.div_ceil(8)];
        if bits % 8 != 0 {
            modulus[0] = (1u8 << (bits % 8)) - 1;
        }
        let encoded = rsa::pkcs1::RsaPublicKey {
            modulus: UintRef::new(&modulus).unwrap(),
            public_exponent: UintRef::new(&[1, 0, 1]).unwrap(),
        }
        .to_der()
        .unwrap();
        assert_eq!(super::rsa_key(&encoded).is_ok(), accepted, "{bits} bits");
    }
}

#[test]
fn absent_rsa_parameters_keep_the_same_verification_semantics() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/signatures/provider_627");
    let message = std::fs::read(root.join("message.bin")).unwrap();
    let der = std::fs::read(root.join("rsa.spki")).unwrap();
    let spki = spki::SubjectPublicKeyInfoRef::from_der(&der).unwrap();
    let key = spki.subject_public_key.as_bytes().unwrap();
    for (digest, absent) in [
        ("sha256", super::RSA_SHA256_ABSENT),
        ("sha384", super::RSA_SHA384_ABSENT),
        ("sha512", super::RSA_SHA512_ABSENT),
    ] {
        let alg = ALL_VERIFICATION_ALGS
            .iter()
            .find(|a| a.signature_alg_id() == absent)
            .unwrap();
        let signature = std::fs::read(root.join(format!("rsa_pkcs1_{digest}.sig"))).unwrap();
        assert!(
            alg.verify_signature(key, &message, &signature).is_ok(),
            "{digest}"
        );
        assert!(
            alg.verify_signature(key, b"wrong message", &signature)
                .is_err(),
            "{digest}"
        );
    }
}

#[test]
fn ed25519_small_order_identity_key_is_rejected() {
    let alg = ALL_VERIFICATION_ALGS
        .iter()
        .find(|a| a.signature_alg_id() == alg_id::ED25519)
        .unwrap();
    let mut identity = [0u8; 32];
    identity[0] = 1;
    let mut signature = [0u8; 64];
    signature[0] = 1;
    assert!(alg
        .verify_signature(&identity, b"forged", &signature)
        .is_err());
}

#[test]
fn rsa_pss_requires_the_algorithm_identifiers_digest_length_salt() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/signatures/provider_627");
    let message = std::fs::read(root.join("message.bin")).unwrap();
    let der = std::fs::read(root.join("rsa_wrong.spki")).unwrap();
    let spki = spki::SubjectPublicKeyInfoRef::from_der(&der).unwrap();
    let key = spki.subject_public_key.as_bytes().unwrap();
    let alg = ALL_VERIFICATION_ALGS
        .iter()
        .find(|a| a.signature_alg_id() == alg_id::RSA_PSS_SHA256)
        .unwrap();
    let valid = std::fs::read(root.join("rsa_pss_salt_32.sig")).unwrap();
    let invalid = std::fs::read(root.join("rsa_pss_salt_0.sig")).unwrap();
    assert!(alg.verify_signature(key, &message, &valid).is_ok());
    assert!(alg.verify_signature(key, &message, &invalid).is_err());
}
