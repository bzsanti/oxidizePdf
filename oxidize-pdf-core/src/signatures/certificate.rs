//! Certificate validation for PDF digital signatures

use super::error::{SignatureError, SignatureResult};

/// Result of certificate validation
#[derive(Debug, Clone)]
pub struct CertificateValidationResult {
    pub subject: String,
    pub issuer: String,
    pub valid_from: String,
    pub valid_to: String,
    pub is_time_valid: bool,
    pub is_trusted: bool,
    pub is_signature_capable: bool,
    pub revocation_status: RevocationStatus,
    pub warnings: Vec<String>,
}

/// Outcome of revocation checking for the signing certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationStatus {
    CheckedValid,
    Revoked,
    Indeterminate,
    Unavailable,
}

impl CertificateValidationResult {
    pub fn is_valid(&self) -> bool {
        self.is_time_valid
            && self.is_trusted
            && self.is_signature_capable
            && self.revocation_status == RevocationStatus::CheckedValid
    }
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct TrustStore {
    roots: TrustRoots,
    crls_der: Vec<Vec<u8>>,
}

#[derive(Debug, Clone)]
enum TrustRoots {
    Mozilla,
    Custom(Vec<Vec<u8>>),
    Empty,
}

impl Default for TrustStore {
    fn default() -> Self {
        Self::mozilla_roots()
    }
}

impl TrustStore {
    pub fn mozilla_roots() -> Self {
        Self {
            roots: TrustRoots::Mozilla,
            crls_der: Vec::new(),
        }
    }
    pub fn empty() -> Self {
        Self {
            roots: TrustRoots::Empty,
            crls_der: Vec::new(),
        }
    }
    /// Build a trust store from DER-encoded X.509 trust-anchor certificates.
    pub fn from_der_certificates(roots: Vec<Vec<u8>>) -> SignatureResult<Self> {
        if roots.is_empty() {
            return Err(SignatureError::CertificateValidationFailed {
                details: "trust store must contain at least one anchor".to_string(),
            });
        }
        #[cfg(feature = "signatures")]
        for root in &roots {
            let certificate = rustls_pki_types::CertificateDer::from(root.as_slice());
            webpki::anchor_from_trusted_cert(&certificate).map_err(|e| {
                SignatureError::CertificateValidationFailed {
                    details: format!("invalid trust anchor: {e}"),
                }
            })?;
        }
        Ok(Self {
            roots: TrustRoots::Custom(roots),
            crls_der: Vec::new(),
        })
    }
    /// Attach DER-encoded X.509 certificate revocation lists.
    pub fn with_crls(mut self, crls_der: Vec<Vec<u8>>) -> SignatureResult<Self> {
        #[cfg(feature = "signatures")]
        for crl in &crls_der {
            webpki::OwnedCertRevocationList::from_der(crl).map_err(|error| {
                SignatureError::CertificateValidationFailed {
                    details: format!("invalid certificate revocation list: {error}"),
                }
            })?;
        }
        self.crls_der = crls_der;
        Ok(self)
    }
    pub fn root_count(&self) -> usize {
        match &self.roots {
            TrustRoots::Mozilla => {
                #[cfg(feature = "signatures")]
                {
                    webpki_roots::TLS_SERVER_ROOTS.len()
                }
                #[cfg(not(feature = "signatures"))]
                {
                    0
                }
            }
            TrustRoots::Custom(roots) => roots.len(),
            TrustRoots::Empty => 0,
        }
    }
    pub fn is_mozilla_bundle(&self) -> bool {
        matches!(self.roots, TrustRoots::Mozilla)
    }
}

#[cfg(feature = "signatures")]
pub fn validate_certificate(
    cert_der: &[u8],
    trust_store: &TrustStore,
) -> SignatureResult<CertificateValidationResult> {
    validate_certificate_at_time(cert_der, trust_store, None)
}

#[cfg(not(feature = "signatures"))]
pub fn validate_certificate(
    _: &[u8],
    _: &TrustStore,
) -> SignatureResult<CertificateValidationResult> {
    Err(SignatureError::CertificateValidationFailed {
        details: "signatures feature not enabled".to_string(),
    })
}

#[cfg(feature = "signatures")]
pub fn validate_certificate_at_time(
    cert_der: &[u8],
    trust_store: &TrustStore,
    validation_time: Option<time::OffsetDateTime>,
) -> SignatureResult<CertificateValidationResult> {
    use der::Decode;
    use x509_cert::Certificate;
    let cert = Certificate::from_der(cert_der).map_err(|e| {
        SignatureError::CertificateValidationFailed {
            details: format!("Failed to parse certificate: {}", e),
        }
    })?;
    let subject = extract_common_name(&cert.tbs_certificate.subject)
        .unwrap_or_else(|| format_dn(&cert.tbs_certificate.subject));
    let issuer = extract_common_name(&cert.tbs_certificate.issuer)
        .unwrap_or_else(|| format_dn(&cert.tbs_certificate.issuer));
    let validity = &cert.tbs_certificate.validity;
    let valid_from = format_x509_time(&validity.not_before);
    let valid_to = format_x509_time(&validity.not_after);
    let now = validation_time.unwrap_or_else(time::OffsetDateTime::now_utc);
    let is_time_valid = check_validity_period(&validity.not_before, &validity.not_after, now);
    let (is_trusted, revocation_status, trust_warnings) =
        validate_trust_chain(cert_der, &[], trust_store, now);
    let (is_signature_capable, usage_warnings) = check_key_usage(&cert);
    let mut warnings = Vec::new();
    warnings.extend(trust_warnings);
    warnings.extend(usage_warnings);
    Ok(CertificateValidationResult {
        subject,
        issuer,
        valid_from,
        valid_to,
        is_time_valid,
        is_trusted,
        is_signature_capable,
        revocation_status,
        warnings,
    })
}

/// Validate an end-entity signing certificate and its embedded intermediates.
#[cfg(feature = "signatures")]
pub fn validate_certificate_chain(
    cert_der: &[u8],
    certificates_der: &[Vec<u8>],
    trust_store: &TrustStore,
    validation_time: Option<time::OffsetDateTime>,
) -> SignatureResult<CertificateValidationResult> {
    use der::Decode;
    use x509_cert::Certificate;

    let cert = Certificate::from_der(cert_der).map_err(|e| {
        SignatureError::CertificateValidationFailed {
            details: format!("Failed to parse certificate: {e}"),
        }
    })?;
    let now = validation_time.unwrap_or_else(time::OffsetDateTime::now_utc);
    let intermediates = certificates_der
        .iter()
        .filter(|candidate| candidate.as_slice() != cert_der)
        .cloned()
        .collect::<Vec<_>>();
    let (is_trusted, revocation_status, trust_warnings) =
        validate_trust_chain(cert_der, &intermediates, trust_store, now);
    let (is_signature_capable, usage_warnings) = check_key_usage(&cert);
    let validity = &cert.tbs_certificate.validity;
    let mut warnings = trust_warnings;
    warnings.extend(usage_warnings);
    Ok(CertificateValidationResult {
        subject: extract_common_name(&cert.tbs_certificate.subject)
            .unwrap_or_else(|| format_dn(&cert.tbs_certificate.subject)),
        issuer: extract_common_name(&cert.tbs_certificate.issuer)
            .unwrap_or_else(|| format_dn(&cert.tbs_certificate.issuer)),
        valid_from: format_x509_time(&validity.not_before),
        valid_to: format_x509_time(&validity.not_after),
        is_time_valid: check_validity_period(&validity.not_before, &validity.not_after, now),
        is_trusted,
        is_signature_capable,
        revocation_status,
        warnings,
    })
}

#[cfg(not(feature = "signatures"))]
pub fn validate_certificate_chain(
    _: &[u8],
    _: &[Vec<u8>],
    _: &TrustStore,
    _: Option<()>,
) -> SignatureResult<CertificateValidationResult> {
    Err(SignatureError::CertificateValidationFailed {
        details: "signatures feature not enabled".to_string(),
    })
}

// Note: validate_certificate_at_time is only available when "signatures" feature is enabled
// as it requires the `time` crate for OffsetDateTime

#[cfg(feature = "signatures")]
fn extract_common_name(name: &x509_cert::name::Name) -> Option<String> {
    use der::asn1::{PrintableStringRef, Utf8StringRef};
    for rdn in name.0.iter() {
        for atv in rdn.0.iter() {
            if atv.oid.to_string() == "2.5.4.3" {
                if let Ok(utf8) = Utf8StringRef::try_from(&atv.value) {
                    return Some(utf8.as_str().to_string());
                }
                if let Ok(printable) = PrintableStringRef::try_from(&atv.value) {
                    return Some(printable.as_str().to_string());
                }
            }
        }
    }
    None
}

#[cfg(feature = "signatures")]
fn format_dn(name: &x509_cert::name::Name) -> String {
    use der::asn1::{PrintableStringRef, Utf8StringRef};
    let mut parts = Vec::new();
    for rdn in name.0.iter() {
        for atv in rdn.0.iter() {
            let oid = atv.oid.to_string();
            let value = if let Ok(utf8) = Utf8StringRef::try_from(&atv.value) {
                utf8.as_str().to_string()
            } else if let Ok(printable) = PrintableStringRef::try_from(&atv.value) {
                printable.as_str().to_string()
            } else {
                "<binary>".to_string()
            };
            parts.push(format!("{}={}", oid_to_short_name(&oid), value));
        }
    }
    parts.join(", ")
}

#[cfg(feature = "signatures")]
fn oid_to_short_name(oid: &str) -> String {
    match oid {
        "2.5.4.3" => "CN",
        "2.5.4.6" => "C",
        "2.5.4.10" => "O",
        _ => oid,
    }
    .to_string()
}

#[cfg(feature = "signatures")]
fn format_x509_time(time: &x509_cert::time::Time) -> String {
    match time {
        x509_cert::time::Time::UtcTime(ut) => ut.to_date_time().to_string(),
        x509_cert::time::Time::GeneralTime(gt) => gt.to_date_time().to_string(),
    }
}

#[cfg(feature = "signatures")]
fn check_validity_period(
    not_before: &x509_cert::time::Time,
    not_after: &x509_cert::time::Time,
    now: time::OffsetDateTime,
) -> bool {
    let nb = x509_time_to_offset_datetime(not_before);
    let na = x509_time_to_offset_datetime(not_after);
    match (nb, na) {
        (Some(nb), Some(na)) => now >= nb && now <= na,
        _ => false,
    }
}

#[cfg(feature = "signatures")]
fn x509_time_to_offset_datetime(time: &x509_cert::time::Time) -> Option<time::OffsetDateTime> {
    let dt = match time {
        x509_cert::time::Time::UtcTime(ut) => ut.to_date_time(),
        x509_cert::time::Time::GeneralTime(gt) => gt.to_date_time(),
    };
    let date = time::Date::from_calendar_date(
        dt.year() as i32,
        time::Month::try_from(dt.month()).ok()?,
        dt.day(),
    )
    .ok()?;
    let time_of_day = time::Time::from_hms(dt.hour(), dt.minutes(), dt.seconds()).ok()?;
    Some(time::OffsetDateTime::new_utc(date, time_of_day))
}

#[cfg(feature = "signatures")]
fn validate_trust_chain(
    cert_der: &[u8],
    intermediate_der: &[Vec<u8>],
    trust_store: &TrustStore,
    now: time::OffsetDateTime,
) -> (bool, RevocationStatus, Vec<String>) {
    use rustls_pki_types::{CertificateDer, UnixTime};
    use std::time::Duration;
    let mut warnings = Vec::new();
    if trust_store.root_count() == 0 {
        warnings.push("Trust store has no anchors".to_string());
        return (false, RevocationStatus::Unavailable, warnings);
    }
    let certificate = CertificateDer::from(cert_der);
    let end_entity = match webpki::EndEntityCert::try_from(&certificate) {
        Ok(cert) => cert,
        Err(e) => {
            warnings.push(format!("Failed to parse end-entity certificate: {e}"));
            return (false, RevocationStatus::Unavailable, warnings);
        }
    };
    let custom_certificates;
    let custom_anchors;
    let anchors = match &trust_store.roots {
        TrustRoots::Mozilla => webpki_roots::TLS_SERVER_ROOTS,
        TrustRoots::Custom(roots) => {
            custom_certificates = roots
                .iter()
                .map(|root| CertificateDer::from(root.as_slice()))
                .collect::<Vec<_>>();
            custom_anchors = match custom_certificates
                .iter()
                .map(webpki::anchor_from_trusted_cert)
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(anchors) => anchors,
                Err(error) => {
                    warnings.push(format!("Invalid trust anchor: {error}"));
                    return (false, RevocationStatus::Unavailable, warnings);
                }
            };
            custom_anchors.as_slice()
        }
        TrustRoots::Empty => &[],
    };
    let intermediates = intermediate_der
        .iter()
        .map(|cert| CertificateDer::from(cert.as_slice()))
        .collect::<Vec<_>>();
    let timestamp = match u64::try_from(now.unix_timestamp()) {
        Ok(seconds) => UnixTime::since_unix_epoch(Duration::from_secs(seconds)),
        Err(_) => {
            warnings.push("Validation time predates the Unix epoch".to_string());
            return (false, RevocationStatus::Unavailable, warnings);
        }
    };
    let path_result = end_entity.verify_for_usage(
        webpki::ALL_VERIFICATION_ALGS,
        anchors,
        &intermediates,
        timestamp,
        PdfSigningEku,
        None,
        None,
    );
    if let Err(error) = path_result {
        warnings.push(format!("Certificate path validation failed: {error}"));
        return (false, RevocationStatus::Unavailable, warnings);
    }
    if trust_store.crls_der.is_empty() {
        warnings.push("Certificate revocation was not checked: no CRLs provided".to_string());
        return (true, RevocationStatus::Unavailable, warnings);
    }
    let owned_crls = match trust_store
        .crls_der
        .iter()
        .map(|crl| {
            webpki::OwnedCertRevocationList::from_der(crl).map(webpki::CertRevocationList::from)
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(crls) => crls,
        Err(error) => {
            warnings.push(format!("Certificate revocation data is invalid: {error}"));
            return (true, RevocationStatus::Indeterminate, warnings);
        }
    };
    let crls = owned_crls.iter().collect::<Vec<_>>();
    let revocation = webpki::RevocationOptionsBuilder::new(&crls)
        .expect("non-empty CRL collection")
        .with_depth(webpki::RevocationCheckDepth::EndEntity)
        .with_expiration_policy(webpki::ExpirationPolicy::Enforce)
        .build();
    match end_entity.verify_for_usage(
        webpki::ALL_VERIFICATION_ALGS,
        anchors,
        &intermediates,
        timestamp,
        PdfSigningEku,
        Some(revocation),
        None,
    ) {
        Ok(_) => (true, RevocationStatus::CheckedValid, warnings),
        Err(webpki::Error::CertRevoked) => {
            warnings.push("Signing certificate is revoked".to_string());
            (true, RevocationStatus::Revoked, warnings)
        }
        Err(error) => {
            warnings.push(format!(
                "Certificate revocation check was indeterminate: {error}"
            ));
            (true, RevocationStatus::Indeterminate, warnings)
        }
    }
}

#[cfg(feature = "signatures")]
#[derive(Debug, Clone, Copy)]
struct PdfSigningEku;

#[cfg(feature = "signatures")]
impl webpki::ExtendedKeyUsageValidator for PdfSigningEku {
    fn validate(&self, purposes: webpki::KeyPurposeIdIter<'_, '_>) -> Result<(), webpki::Error> {
        const ALLOWED: &[&[usize]] = &[
            &[1, 3, 6, 1, 5, 5, 7, 3, 4],  // emailProtection
            &[1, 3, 6, 1, 5, 5, 7, 3, 3],  // codeSigning
            &[1, 3, 6, 1, 5, 5, 7, 3, 36], // documentSigning
            &[1, 2, 840, 113583, 1, 1, 5], // Adobe authentic documents
            &[2, 5, 29, 37, 0],            // anyExtendedKeyUsage
        ];
        let mut present = false;
        for purpose in purposes {
            present = true;
            let purpose = purpose?;
            let decoded = purpose.to_decoded_oid();
            if ALLOWED.iter().any(|allowed| *allowed == decoded) {
                return Ok(());
            }
        }
        if !present {
            return Ok(());
        }
        #[allow(deprecated)]
        Err(webpki::Error::RequiredEkuNotFound)
    }
}

#[cfg(feature = "signatures")]
fn check_key_usage(cert: &x509_cert::Certificate) -> (bool, Vec<String>) {
    use der::Decode;
    use x509_cert::ext::pkix::KeyUsage;
    let mut warnings = Vec::new();
    if let Some(extensions) = &cert.tbs_certificate.extensions {
        for ext in extensions.iter() {
            if ext.extn_id.to_string() == "2.5.29.15" {
                match KeyUsage::from_der(ext.extn_value.as_bytes()) {
                    Ok(usage) if usage.digital_signature() || usage.non_repudiation() => {
                        return (true, warnings)
                    }
                    Ok(_) => {
                        warnings.push("No digital signature key usage".to_string());
                        return (false, warnings);
                    }
                    Err(error) => {
                        warnings.push(format!("Malformed key usage extension: {error}"));
                        return (false, warnings);
                    }
                }
            }
        }
    }
    warnings.push("No key usage extension; signature capability is indeterminate".to_string());
    (false, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_validation_result_is_valid() {
        let result = CertificateValidationResult {
            subject: "CN=Test".to_string(),
            issuer: "CN=Test CA".to_string(),
            valid_from: "2024-01-01".to_string(),
            valid_to: "2025-01-01".to_string(),
            is_time_valid: true,
            is_trusted: true,
            is_signature_capable: true,
            revocation_status: RevocationStatus::CheckedValid,
            warnings: vec![],
        };
        assert!(result.is_valid());
    }

    #[test]
    fn test_certificate_validation_result_invalid_when_expired() {
        let result = CertificateValidationResult {
            subject: "CN=Test".to_string(),
            issuer: "CN=Test CA".to_string(),
            valid_from: "2024-01-01".to_string(),
            valid_to: "2025-01-01".to_string(),
            is_time_valid: false,
            is_trusted: true,
            is_signature_capable: true,
            revocation_status: RevocationStatus::CheckedValid,
            warnings: vec![],
        };
        assert!(!result.is_valid());
    }

    #[test]
    fn test_certificate_validation_result_invalid_when_not_trusted() {
        let result = CertificateValidationResult {
            subject: "CN=Test".to_string(),
            issuer: "CN=Test CA".to_string(),
            valid_from: "2024-01-01".to_string(),
            valid_to: "2025-01-01".to_string(),
            is_time_valid: true,
            is_trusted: false,
            is_signature_capable: true,
            revocation_status: RevocationStatus::CheckedValid,
            warnings: vec![],
        };
        assert!(!result.is_valid());
    }

    #[test]
    fn test_certificate_validation_result_has_warnings() {
        let result = CertificateValidationResult {
            subject: "CN=Test".to_string(),
            issuer: "CN=Test CA".to_string(),
            valid_from: "2024-01-01".to_string(),
            valid_to: "2025-01-01".to_string(),
            is_time_valid: true,
            is_trusted: true,
            is_signature_capable: true,
            revocation_status: RevocationStatus::CheckedValid,
            warnings: vec!["Self-signed certificate".to_string()],
        };
        assert!(result.has_warnings());
    }

    #[test]
    fn test_certificate_validation_result_no_warnings() {
        let result = CertificateValidationResult {
            subject: "CN=Test".to_string(),
            issuer: "CN=Test CA".to_string(),
            valid_from: "2024-01-01".to_string(),
            valid_to: "2025-01-01".to_string(),
            is_time_valid: true,
            is_trusted: true,
            is_signature_capable: true,
            revocation_status: RevocationStatus::CheckedValid,
            warnings: vec![],
        };
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_trust_store_mozilla_roots() {
        let store = TrustStore::mozilla_roots();
        assert!(store.is_mozilla_bundle());
    }

    #[test]
    fn test_trust_store_empty() {
        let store = TrustStore::empty();
        assert!(!store.is_mozilla_bundle());
        assert_eq!(store.root_count(), 0);
    }

    #[test]
    fn test_trust_store_default() {
        let store = TrustStore::default();
        assert!(store.is_mozilla_bundle());
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_validate_certificate_invalid_der() {
        let store = TrustStore::mozilla_roots();
        assert!(validate_certificate(&[0, 1, 2, 3], &store).is_err());
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_oid_to_short_name_cn() {
        assert_eq!(oid_to_short_name("2.5.4.3"), "CN");
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_oid_to_short_name_c() {
        assert_eq!(oid_to_short_name("2.5.4.6"), "C");
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_oid_to_short_name_o() {
        assert_eq!(oid_to_short_name("2.5.4.10"), "O");
    }

    #[cfg(feature = "signatures")]
    #[test]
    fn test_oid_to_short_name_unknown() {
        assert_eq!(oid_to_short_name("1.2.3.4"), "1.2.3.4");
    }
}
