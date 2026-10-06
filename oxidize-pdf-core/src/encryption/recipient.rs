//! Certificate recipient encryption: Adobe.PubSec, adbe.pkcs7.s5, AESV3.
//!
//! Experimental candidate pending security acceptance (#642). `rsa 0.9.10`
//! has the unresolved timing advisory RUSTSEC-2023-0071. OAEP with blinding
//! does not establish exemption. Do not expose this as a decryption service.
//!
//! Feature `recipient-encryption`. Certificates are DER X.509 with RSA keys
//! (2048–4096 bits); private keys are unencrypted PKCS#8 DER. Key transport is
//! RSA-OAEP/SHA-256/MGF1-SHA-256; CMS and PDF content use AES-256-CBC.
//! Metadata is encrypted. No PDF MAC, PKI trust validation, or PAdES is implied.
//! CBC does not authenticate content: successful decryption is not integrity
//! verification. See `docs/recipient-encryption.md` for the complete profile.

use super::{EncryptionKey, Permissions};
use crate::error::{PdfError, Result};
use crate::objects::{Dictionary, Object};
use crate::parser::objects::PdfDictionary;
use cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use cms::cert::IssuerAndSerialNumber;
use cms::content_info::{CmsVersion, ContentInfo};
use cms::enveloped_data::{
    EncryptedContentInfo, EnvelopedData, KeyTransRecipientInfo, RecipientIdentifier, RecipientInfo,
    RecipientInfos,
};
use const_oid::ObjectIdentifier;
use der::{
    asn1::{Any, OctetString, SetOfVec},
    Decode, Encode,
};
use rsa::{
    pkcs1::{DecodeRsaPublicKey, RsaOaepParams},
    pkcs8::DecodePrivateKey,
    rand_core::{OsRng, RngCore},
    traits::PublicKeyParts,
    Oaep, RsaPrivateKey, RsaPublicKey,
};
use sha2::{Digest, Sha256};
use spki::AlgorithmIdentifierOwned;
use x509_cert::{ext::pkix::KeyUsage, Certificate};
use zeroize::Zeroizing;

const RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");
const OAEP: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.7");
const AES256: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.1.42");
const ENVELOPED: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.3");
const DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.1");
const MAX_RECIPIENTS: usize = 32;
const MAX_DER: usize = 65536;

fn err(message: &str) -> PdfError {
    PdfError::EncryptionError(format!("recipient encryption: {message}"))
}
fn der_err(_: impl std::fmt::Display) -> PdfError {
    err("invalid DER structure")
}
fn identity(cert: &Certificate) -> IssuerAndSerialNumber {
    IssuerAndSerialNumber {
        issuer: cert.tbs_certificate.issuer.clone(),
        serial_number: cert.tbs_certificate.serial_number.clone(),
    }
}
fn certificate(bytes: &[u8]) -> Result<(Certificate, RsaPublicKey)> {
    if bytes.is_empty() || bytes.len() > MAX_DER {
        return Err(err("certificate size exceeds profile bounds"));
    }
    let cert = Certificate::from_der(bytes).map_err(der_err)?;
    let spki = &cert.tbs_certificate.subject_public_key_info;
    if spki.algorithm.oid != RSA
        || spki
            .algorithm
            .parameters
            .as_ref()
            .is_some_and(|p| !p.is_null())
    {
        return Err(err("only rsaEncryption certificate keys are supported"));
    }
    let key = RsaPublicKey::from_pkcs1_der(
        spki.subject_public_key
            .as_bytes()
            .ok_or_else(|| err("invalid RSA bit string"))?,
    )
    .map_err(der_err)?;
    if !(2048..=4096).contains(&key.n().bits()) {
        return Err(err("RSA modulus must be 2048–4096 bits"));
    }
    Ok((cert, key))
}
fn suitable(cert: &Certificate) -> Result<()> {
    let validity = cert.tbs_certificate.validity;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| err("system clock predates Unix epoch"))?;
    if now < validity.not_before.to_unix_duration() || now > validity.not_after.to_unix_duration() {
        return Err(err("recipient certificate is outside its validity period"));
    }
    if let Some((_, usage)) = cert.tbs_certificate.get::<KeyUsage>().map_err(der_err)? {
        if !usage.key_encipherment() {
            return Err(err("certificate KeyUsage does not permit keyEncipherment"));
        }
    }
    // This is suitability, not path validation: caller selects/trusts recipients.
    Ok(())
}

/// A validated set of certificate recipients, each with its own permissions.
///
/// Experimental pending #642 security acceptance: the RSA dependency has the
/// unresolved timing advisory RUSTSEC-2023-0071. This is not approved for a
/// network-facing decryption service.
///
/// No private keys are retained. Certificates are checked for RSA key size,
/// validity period and KeyUsage (if present), not chain trust or revocation.
/// The caller is responsible for selecting the correct recipients. Permission
/// enforcement depends on the reader; it does not restrict the private key owner.
/// The writer rechecks certificate validity and generates fresh key material.
#[derive(Clone)]
pub struct RecipientEncryption {
    recipients: Vec<(Certificate, RsaPublicKey, Permissions)>,
}
impl RecipientEncryption {
    /// Start with one DER X.509 recipient certificate.
    pub fn new(certificate_der: &[u8], permissions: Permissions) -> Result<Self> {
        let mut value = Self {
            recipients: Vec::new(),
        };
        value.add_recipient(certificate_der, permissions)?;
        Ok(value)
    }
    /// Add a distinct recipient (maximum 32). Duplicate issuer/serial is rejected.
    pub fn add_recipient(
        &mut self,
        certificate_der: &[u8],
        permissions: Permissions,
    ) -> Result<()> {
        if self.recipients.len() >= MAX_RECIPIENTS {
            return Err(err("too many recipients"));
        }
        let (cert, key) = certificate(certificate_der)?;
        suitable(&cert)?;
        if self
            .recipients
            .iter()
            .any(|(other, _, _)| identity(other) == identity(&cert))
        {
            return Err(err("duplicate recipient issuer and serial"));
        }
        self.recipients.push((cert, key, permissions));
        Ok(())
    }
    pub(crate) fn prepare(&self) -> Result<(Dictionary, EncryptionKey)> {
        let mut seed = Zeroizing::new([0u8; 20]);
        OsRng
            .try_fill_bytes(seed.as_mut())
            .map_err(|_| err("system randomness unavailable"))?;
        let mut recipients = Vec::new();
        for (cert, key, permissions) in &self.recipients {
            suitable(cert)?;
            // ISO public-key permission layout; bit 1 set for Acrobat compatibility,
            // bit 13 allows absence of PDF MAC. Only ordinary document rights vary.
            let p = (permissions.bits() & 0x0f3c) | 0xfffff0c1;
            let mut payload = Zeroizing::new(Vec::with_capacity(24));
            payload.extend_from_slice(seed.as_ref());
            payload.extend_from_slice(&p.to_be_bytes());
            let mut cek = Zeroizing::new([0u8; 32]);
            let mut iv = [0u8; 16];
            OsRng
                .try_fill_bytes(cek.as_mut())
                .map_err(|_| err("system randomness unavailable"))?;
            OsRng
                .try_fill_bytes(&mut iv)
                .map_err(|_| err("system randomness unavailable"))?;
            let mut buffer = Zeroizing::new([0u8; 32]);
            buffer[..24].copy_from_slice(&payload);
            let encrypted = cbc::Encryptor::<aes::Aes256>::new_from_slices(cek.as_ref(), &iv)
                .map_err(der_err)?
                .encrypt_padded_mut::<Pkcs7>(buffer.as_mut(), 24)
                .map_err(der_err)?
                .to_vec();
            let wrapped = key
                .encrypt(&mut OsRng, Oaep::new::<Sha256>(), cek.as_ref())
                .map_err(|_| err("RSA-OAEP encryption failed"))?;
            let ktri = KeyTransRecipientInfo {
                version: CmsVersion::V0,
                rid: RecipientIdentifier::IssuerAndSerialNumber(identity(cert)),
                key_enc_alg: AlgorithmIdentifierOwned {
                    oid: OAEP,
                    parameters: Some(
                        Any::encode_from(&RsaOaepParams::new::<Sha256>()).map_err(der_err)?,
                    ),
                },
                enc_key: OctetString::new(wrapped).map_err(der_err)?,
            };
            let envelope = EnvelopedData {
                version: CmsVersion::V0,
                originator_info: None,
                recip_infos: RecipientInfos(
                    SetOfVec::try_from(vec![RecipientInfo::Ktri(ktri)]).map_err(der_err)?,
                ),
                encrypted_content: EncryptedContentInfo {
                    content_type: DATA,
                    content_enc_alg: AlgorithmIdentifierOwned {
                        oid: AES256,
                        parameters: Some(
                            Any::encode_from(&OctetString::new(iv).map_err(der_err)?)
                                .map_err(der_err)?,
                        ),
                    },
                    encrypted_content: Some(OctetString::new(encrypted).map_err(der_err)?),
                },
                unprotected_attrs: None,
            };
            let ci = ContentInfo {
                content_type: ENVELOPED,
                content: Any::encode_from(&envelope).map_err(der_err)?,
            };
            recipients.push(ci.to_der().map_err(der_err)?);
        }
        let key = file_key(seed.as_ref(), &recipients);
        let mut filter = Dictionary::new();
        filter.set("CFM", Object::Name("AESV3".into()));
        filter.set("Length", Object::Integer(256));
        filter.set("AuthEvent", Object::Name("DocOpen".into()));
        filter.set("EncryptMetadata", Object::Boolean(true));
        filter.set(
            "Recipients",
            Object::Array(recipients.into_iter().map(Object::ByteString).collect()),
        );
        let mut cf = Dictionary::new();
        cf.set("DefaultCryptFilter", Object::Dictionary(filter));
        let mut dict = Dictionary::new();
        dict.set("Filter", Object::Name("Adobe.PubSec".into()));
        dict.set("SubFilter", Object::Name("adbe.pkcs7.s5".into()));
        dict.set("V", Object::Integer(5));
        dict.set("Length", Object::Integer(256));
        dict.set("CF", Object::Dictionary(cf));
        for field in ["StmF", "StrF", "EFF"] {
            dict.set(field, Object::Name("DefaultCryptFilter".into()));
        }
        Ok((dict, key))
    }
}
fn file_key(seed: &[u8], recipients: &[Vec<u8>]) -> EncryptionKey {
    let mut hash = Sha256::new();
    hash.update(seed);
    for recipient in recipients {
        hash.update(recipient);
    }
    EncryptionKey::new(hash.finalize().to_vec())
}

pub(crate) struct RecipientState {
    recipients: Vec<Vec<u8>>,
}
fn name<'a>(dict: &'a PdfDictionary, key: &str) -> Option<&'a str> {
    dict.get(key)?.as_name().map(|n| n.0.as_str())
}
impl RecipientState {
    pub(crate) fn parse(dict: &PdfDictionary) -> Result<Self> {
        if name(dict, "Filter") != Some("Adobe.PubSec")
            || name(dict, "SubFilter") != Some("adbe.pkcs7.s5")
            || dict.get("V").and_then(|o| o.as_integer()) != Some(5)
            || dict.get("Length").and_then(|o| o.as_integer()) != Some(256)
        {
            return Err(err("unsupported Adobe.PubSec profile"));
        }
        if [
            "Recipients",
            "P",
            "R",
            "O",
            "U",
            "OE",
            "UE",
            "Perms",
            "KDFSalt",
        ]
        .iter()
        .any(|k| dict.contains_key(k))
        {
            return Err(err("conflicting encryption dictionary entries"));
        }
        if dict
            .get("EncryptMetadata")
            .is_some_and(|v| v.as_bool() != Some(true))
        {
            return Err(err("unencrypted metadata is unsupported"));
        }
        let selected = name(dict, "StmF")
            .filter(|n| *n != "Identity")
            .ok_or_else(|| err("missing stream crypt filter"))?;
        if name(dict, "StrF") != Some(selected)
            || dict
                .get("EFF")
                .is_some_and(|_| name(dict, "EFF") != Some(selected))
        {
            return Err(err("mixed crypt filters are unsupported"));
        }
        let cf = dict
            .get("CF")
            .and_then(|v| v.as_dict())
            .ok_or_else(|| err("missing CF"))?;
        if cf.0.len() != 1 {
            return Err(err("multiple crypt filters are unsupported"));
        }
        let filter = cf
            .get(selected)
            .and_then(|v| v.as_dict())
            .ok_or_else(|| err("missing selected crypt filter"))?;
        if name(filter, "CFM") != Some("AESV3")
            || name(filter, "AuthEvent") != Some("DocOpen")
            || filter.get("Length").and_then(|v| v.as_integer()) != Some(256)
            || filter
                .get("EncryptMetadata")
                .is_some_and(|v| v.as_bool() != Some(true))
        {
            return Err(err("unsupported crypt filter"));
        }
        let array = filter
            .get("Recipients")
            .and_then(|v| v.as_array())
            .ok_or_else(|| err("Recipients must be an array"))?;
        if array.0.is_empty() || array.0.len() > MAX_RECIPIENTS {
            return Err(err("recipient count exceeds profile bounds"));
        }
        let mut recipients = Vec::new();
        let mut ids = Vec::new();
        for value in &array.0 {
            let bytes = value
                .as_string()
                .ok_or_else(|| err("recipient is not a byte string"))?
                .as_bytes();
            let envelope = envelope(bytes)?;
            for recipient in envelope.recip_infos.0.iter() {
                let RecipientInfo::Ktri(ktri) = recipient else {
                    return Err(err("only key transport recipients are supported"));
                };
                if ids.contains(&ktri.rid) {
                    return Err(err("duplicate CMS recipient identity"));
                }
                ids.push(ktri.rid.clone());
                if ids.len() > MAX_RECIPIENTS {
                    return Err(err("too many CMS recipient infos"));
                }
            }
            recipients.push(bytes.to_vec());
        }
        Ok(Self { recipients })
    }
    pub(crate) fn unlock(&self, cert_der: &[u8], key_der: &[u8]) -> Result<(EncryptionKey, u32)> {
        // Expired certificates may still decrypt archived documents. Trust, validity
        // and KeyUsage are not an authentication decision when recovering a key.
        let (cert, public) = certificate(cert_der)?;
        if key_der.is_empty() || key_der.len() > 16384 {
            return Err(err("private key size exceeds profile bounds"));
        }
        // Bound the modulus before RSA parsing performs big-integer validation.
        let pkcs8 = rsa::pkcs8::PrivateKeyInfo::from_der(key_der).map_err(der_err)?;
        let pkcs1 = rsa::pkcs1::RsaPrivateKey::from_der(pkcs8.private_key).map_err(der_err)?;
        if !(256..=512).contains(&pkcs1.modulus.as_bytes().len()) {
            return Err(err("RSA modulus must be 2048–4096 bits"));
        }
        let key = RsaPrivateKey::from_pkcs8_der(key_der)
            .map_err(|_| err("invalid unencrypted PKCS#8 RSA key"))?;
        if !(2048..=4096).contains(&key.n().bits()) {
            return Err(err("RSA modulus must be 2048–4096 bits"));
        }
        key.validate().map_err(|_| err("invalid RSA private key"))?;
        if key.to_public_key() != public {
            return Err(err("certificate and private key do not match"));
        }
        let rid = RecipientIdentifier::IssuerAndSerialNumber(identity(&cert));
        for bytes in &self.recipients {
            let envelope = envelope(bytes)?;
            for recipient in envelope.recip_infos.0.iter() {
                if let RecipientInfo::Ktri(ktri) = recipient {
                    if ktri.rid != rid {
                        continue;
                    }
                    if ktri.enc_key.as_bytes().len() != key.size() {
                        return Err(err("invalid wrapped key size"));
                    }
                    let cek = Zeroizing::new(
                        key.decrypt_blinded(
                            &mut OsRng,
                            Oaep::new::<Sha256>(),
                            ktri.enc_key.as_bytes(),
                        )
                        .map_err(|_| err("recipient key recovery failed"))?,
                    );
                    let iv = envelope
                        .encrypted_content
                        .content_enc_alg
                        .parameters
                        .as_ref()
                        .ok_or_else(|| err("missing CMS IV"))?
                        .decode_as::<OctetString>()
                        .map_err(der_err)?;
                    let data = envelope
                        .encrypted_content
                        .encrypted_content
                        .as_ref()
                        .ok_or_else(|| err("missing encrypted content"))?;
                    let mut buffer = Zeroizing::new(data.as_bytes().to_vec());
                    let payload =
                        cbc::Decryptor::<aes::Aes256>::new_from_slices(&cek, iv.as_bytes())
                            .map_err(|_| err("invalid CMS key or IV"))?
                            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
                            .map_err(|_| err("recipient key recovery failed"))?;
                    if payload.len() != 24 {
                        return Err(err("invalid CMS seed/permissions payload"));
                    }
                    let p = u32::from_be_bytes(payload[20..24].try_into().map_err(der_err)?);
                    if p & 0xfffff0c1 != 0xfffff0c1 {
                        return Err(err(
                            "unsupported public-key permissions or PDF MAC requirement",
                        ));
                    }
                    return Ok((file_key(&payload[..20], &self.recipients), p));
                }
            }
        }
        Err(err("certificate is not a document recipient"))
    }
}
fn envelope(bytes: &[u8]) -> Result<EnvelopedData> {
    if bytes.is_empty() || bytes.len() > MAX_DER {
        return Err(err("CMS size exceeds profile bounds"));
    }
    let ci = ContentInfo::from_der(bytes).map_err(der_err)?;
    if ci.content_type != ENVELOPED {
        return Err(err("recipient container is not CMS EnvelopedData"));
    }
    let value: EnvelopedData = ci.content.decode_as().map_err(der_err)?;
    if value.version != CmsVersion::V0
        || value.originator_info.is_some()
        || value.unprotected_attrs.is_some()
        || value.recip_infos.0.is_empty()
        || value.recip_infos.0.len() > MAX_RECIPIENTS
    {
        return Err(err("unsupported CMS envelope structure"));
    }
    let content = &value.encrypted_content;
    if content.content_type != DATA || content.content_enc_alg.oid != AES256 {
        return Err(err("only CMS data with AES-256-CBC is supported"));
    }
    let iv = content
        .content_enc_alg
        .parameters
        .as_ref()
        .ok_or_else(|| err("missing CMS IV"))?
        .decode_as::<OctetString>()
        .map_err(der_err)?;
    if iv.as_bytes().len() != 16
        || content
            .encrypted_content
            .as_ref()
            .is_none_or(|v| v.as_bytes().len() != 32)
    {
        return Err(err("invalid CMS ciphertext or IV size"));
    }
    let expected = RsaOaepParams::new::<Sha256>();
    for recipient in value.recip_infos.0.iter() {
        let RecipientInfo::Ktri(ktri) = recipient else {
            return Err(err("unsupported CMS recipient type"));
        };
        if ktri.version != CmsVersion::V0
            || !matches!(ktri.rid, RecipientIdentifier::IssuerAndSerialNumber(_))
            || ktri.key_enc_alg.oid != OAEP
            || !(256..=512).contains(&ktri.enc_key.as_bytes().len())
        {
            return Err(err("only RSA-OAEP issuer/serial recipients are supported"));
        }
        let params = ktri
            .key_enc_alg
            .parameters
            .as_ref()
            .ok_or_else(|| err("missing OAEP parameters"))?
            .to_der()
            .map_err(der_err)?;
        let actual = RsaOaepParams::from_der(&params).map_err(der_err)?;
        let null_or_absent = |v: Option<der::asn1::AnyRef<'_>>| v.is_none_or(|p| p.is_null());
        if actual.hash.oid != expected.hash.oid
            || !null_or_absent(actual.hash.parameters)
            || actual.mask_gen.oid != expected.mask_gen.oid
            || actual
                .mask_gen
                .parameters
                .is_none_or(|p| p.oid != expected.hash.oid || !null_or_absent(p.parameters))
            || actual.p_source != expected.p_source
        {
            return Err(err(
                "only OAEP SHA-256/MGF1-SHA-256 with empty label is supported",
            ));
        }
    }
    Ok(value)
}
