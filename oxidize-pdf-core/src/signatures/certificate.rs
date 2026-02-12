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
    pub warnings: Vec<String>,
}

impl CertificateValidationResult {
    pub fn is_valid(&self) -> bool {
        self.is_time_valid && self.is_trusted && self.is_signature_capable
    }
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct TrustStore {
    root_count: usize,
    is_mozilla_bundle: bool,
}

impl Default for TrustStore {
    fn default() -> Self {
        Self::mozilla_roots()
    }
}

impl TrustStore {
    pub fn mozilla_roots() -> Self {
        #[cfg(feature = "signatures")]
        let root_count = webpki_roots::TLS_SERVER_ROOTS.len();
        #[cfg(not(feature = "signatures"))]
        let root_count = 0;
        Self {
            root_count,
            is_mozilla_bundle: true,
        }
    }
    pub fn empty() -> Self {
        Self {
            root_count: 0,
            is_mozilla_bundle: false,
        }
    }
    pub fn root_count(&self) -> usize {
        self.root_count
    }
    pub fn is_mozilla_bundle(&self) -> bool {
        self.is_mozilla_bundle
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
    let (is_trusted, trust_warnings) = validate_trust_chain(cert_der, trust_store, now);
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
        warnings,
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
    trust_store: &TrustStore,
    _: time::OffsetDateTime,
) -> (bool, Vec<String>) {
    use der::Decode;
    use x509_cert::Certificate;
    let mut warnings = Vec::new();
    if !trust_store.is_mozilla_bundle || trust_store.root_count == 0 {
        warnings.push("Using empty or custom trust store".to_string());
        return (false, warnings);
    }
    let cert = match Certificate::from_der(cert_der) {
        Ok(c) => c,
        Err(e) => {
            warnings.push(format!("Failed to parse: {}", e));
            return (false, warnings);
        }
    };
    let subject = format_dn(&cert.tbs_certificate.subject);
    let issuer = format_dn(&cert.tbs_certificate.issuer);
    if subject == issuer {
        warnings.push("Self-signed certificate".to_string());
        return (true, warnings);
    }
    warnings.push("CA-issued certificate (chain validation pending)".to_string());
    (true, warnings)
}

#[cfg(feature = "signatures")]
fn check_key_usage(cert: &x509_cert::Certificate) -> (bool, Vec<String>) {
    let mut warnings = Vec::new();
    if let Some(extensions) = &cert.tbs_certificate.extensions {
        for ext in extensions.iter() {
            if ext.extn_id.to_string() == "2.5.29.15" {
                let value = ext.extn_value.as_bytes();
                if value.len() >= 2 {
                    let key_usage_byte = value[1];
                    if key_usage_byte & 0x80 != 0 || key_usage_byte & 0x40 != 0 {
                        return (true, warnings);
                    } else {
                        warnings.push("No digital signature key usage".to_string());
                        return (false, warnings);
                    }
                }
            }
        }
    }
    warnings.push("No key usage extension".to_string());
    (true, warnings)
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
