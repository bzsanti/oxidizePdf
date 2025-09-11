use super::{LicenseType, ProLicense};
use crate::error::Result;
use chrono::Utc;

pub struct LicenseValidator {
    strict_validation: bool,
    allow_expired: bool,
    max_clock_skew_minutes: i64,
}

impl Default for LicenseValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl LicenseValidator {
    pub fn new() -> Self {
        Self {
            strict_validation: true,
            allow_expired: false,
            max_clock_skew_minutes: 5,
        }
    }

    pub fn with_strict_validation(mut self, strict: bool) -> Self {
        self.strict_validation = strict;
        self
    }

    pub fn with_expired_allowed(mut self, allow: bool) -> Self {
        self.allow_expired = allow;
        self
    }

    pub fn with_clock_skew_tolerance(mut self, minutes: i64) -> Self {
        self.max_clock_skew_minutes = minutes;
        self
    }

    pub fn validate_license(&self, license: &ProLicense) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Validate basic structure
        self.validate_structure(license, &mut result)?;

        // Validate temporal constraints
        self.validate_dates(license, &mut result)?;

        // Validate features
        self.validate_features(license, &mut result)?;

        // Validate usage limits
        self.validate_usage_limits(license, &mut result)?;

        // Validate license type constraints
        self.validate_license_type(license, &mut result)?;

        Ok(result)
    }

    fn validate_structure(
        &self,
        license: &ProLicense,
        result: &mut ValidationResult,
    ) -> Result<()> {
        if license.license_key.is_empty() {
            result.add_error("License key cannot be empty".to_string());
        }

        if license.customer_id.is_empty() {
            result.add_error("Customer ID cannot be empty".to_string());
        }

        if license.features.is_empty() {
            result.add_warning("No features specified in license".to_string());
        }

        // Validate license key format
        if self.strict_validation {
            if license.license_key.len() < 16 {
                result.add_error("License key too short for security".to_string());
            }

            if !license
                .customer_id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                result.add_error("Customer ID contains invalid characters".to_string());
            }
        }

        Ok(())
    }

    fn validate_dates(&self, license: &ProLicense, result: &mut ValidationResult) -> Result<()> {
        let now = Utc::now();

        // Check if license was issued in the future
        let skew_tolerance = chrono::Duration::minutes(self.max_clock_skew_minutes);
        if license.issued_date > now + skew_tolerance {
            result.add_error("License issued date is in the future".to_string());
        }

        // Check expiry
        if let Some(expiry) = license.expiry_date {
            if expiry <= license.issued_date {
                result.add_error("Expiry date must be after issued date".to_string());
            }

            if !self.allow_expired && expiry < now {
                result.add_error("License has expired".to_string());
            } else if expiry < now {
                result.add_warning(
                    "License has expired but validation allows expired licenses".to_string(),
                );
            }

            // Warn if expiring soon
            let days_until_expiry = (expiry - now).num_days();
            if days_until_expiry <= 30 && days_until_expiry > 0 {
                result.add_warning(format!("License expires in {} days", days_until_expiry));
            }
        }

        Ok(())
    }

    fn validate_features(&self, license: &ProLicense, result: &mut ValidationResult) -> Result<()> {
        let known_features = [
            "xmp",
            "extraction",
            "templates",
            "license-validation",
            "advanced-ocr",
            "streaming-api",
            "dashboard-templates",
            "enterprise-support",
            "multi-tenant",
            "*",
        ];

        for feature in &license.features {
            if feature != "*" && !known_features.contains(&feature.as_str()) {
                result.add_warning(format!("Unknown feature: {}", feature));
            }
        }

        // Validate feature combinations
        if license.features.contains(&"*".to_string()) && license.features.len() > 1 {
            result.add_warning("Wildcard feature '*' should be used alone".to_string());
        }

        // Check license type vs features compatibility
        match license.license_type {
            LicenseType::Trial => {
                let restricted_features = ["enterprise-support", "multi-tenant", "unlimited-usage"];
                for restricted in &restricted_features {
                    if license.features.contains(&restricted.to_string()) {
                        result.add_error(format!(
                            "Feature '{}' not allowed in trial license",
                            restricted
                        ));
                    }
                }
            }
            LicenseType::Development => {
                if !license.features.contains(&"*".to_string()) {
                    result.add_info(
                        "Development licenses typically include all features".to_string(),
                    );
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn validate_usage_limits(
        &self,
        license: &ProLicense,
        result: &mut ValidationResult,
    ) -> Result<()> {
        if let Some(limits) = &license.usage_limits {
            if let Some(max_docs) = limits.max_documents_per_month {
                if max_docs == 0 {
                    result.add_error("Maximum documents per month cannot be zero".to_string());
                }
                if max_docs > 1_000_000 {
                    result.add_warning(
                        "Very high document limit may indicate misconfiguration".to_string(),
                    );
                }
            }

            if let Some(max_pages) = limits.max_pages_per_document {
                if max_pages == 0 {
                    result.add_error("Maximum pages per document cannot be zero".to_string());
                }
                if max_pages > 10_000 {
                    result.add_warning("Very high page limit may impact performance".to_string());
                }
            }

            if let Some(max_entities) = limits.max_entities_per_document {
                if max_entities == 0 {
                    result.add_error("Maximum entities per document cannot be zero".to_string());
                }
            }

            if let Some(max_concurrent) = limits.max_concurrent_processes {
                if max_concurrent == 0 {
                    result.add_error("Maximum concurrent processes cannot be zero".to_string());
                }
                if max_concurrent > 100 {
                    result.add_warning(
                        "Very high concurrency limit may strain system resources".to_string(),
                    );
                }
            }
        } else {
            match license.license_type {
                LicenseType::Trial => {
                    result.add_warning("Trial license should have usage limits".to_string());
                }
                LicenseType::Development => {
                    result.add_info("Development license has no usage limits".to_string());
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn validate_license_type(
        &self,
        license: &ProLicense,
        result: &mut ValidationResult,
    ) -> Result<()> {
        match license.license_type {
            LicenseType::Trial => {
                // Trial licenses should have expiry dates
                if license.expiry_date.is_none() {
                    result.add_error("Trial license must have expiry date".to_string());
                }

                // Trial should have usage limits
                if license.usage_limits.is_none() {
                    result.add_warning("Trial license should have usage limits".to_string());
                }

                // Check trial duration
                if let Some(expiry) = license.expiry_date {
                    let duration = expiry - license.issued_date;
                    if duration.num_days() > 90 {
                        result.add_warning("Trial period longer than typical 90 days".to_string());
                    }
                }
            }
            LicenseType::Development => {
                // Development licenses should not be used in production
                if !license.customer_id.contains("dev") && !license.customer_id.contains("test") {
                    result.add_warning(
                        "Development license should not be used in production".to_string(),
                    );
                }
            }
            LicenseType::Professional => {
                // Professional licenses should have reasonable limits
                if let Some(limits) = &license.usage_limits {
                    if let Some(max_docs) = limits.max_documents_per_month {
                        if max_docs < 100 {
                            result.add_info(
                                "Professional license has low document limit".to_string(),
                            );
                        }
                    }
                }
            }
            LicenseType::Enterprise => {
                // Enterprise licenses typically don't have strict limits
                if license.usage_limits.is_some() {
                    result.add_info(
                        "Enterprise license has usage limits (unusual but valid)".to_string(),
                    );
                }
            }
        }

        Ok(())
    }

    pub fn validate_license_key_format(&self, license_key: &str) -> Result<KeyValidationResult> {
        let mut result = KeyValidationResult::new();

        if license_key.is_empty() {
            result.add_error("License key is empty".to_string());
            return Ok(result);
        }

        // Check for development keys
        if matches!(license_key, "OXIDIZE_PRO_DEV" | "OXIDIZE_PRO_TRIAL") {
            result.key_type = Some("development".to_string());
            result.add_info("Development/testing license key detected".to_string());
            return Ok(result);
        }

        // Check length
        if license_key.len() < 16 {
            result.add_error("License key too short".to_string());
        } else if license_key.len() < 32 {
            result.add_warning("License key may be too short for security".to_string());
        }

        // Check character set
        let valid_chars = license_key
            .chars()
            .all(|c| c.is_alphanumeric() || "+-/=_".contains(c));
        if !valid_chars {
            result.add_error("License key contains invalid characters".to_string());
        }

        // Try to determine key type
        if license_key.starts_with("eyJ") || license_key.contains('.') {
            result.key_type = Some("jwt".to_string());
            result.add_info("JWT-style license key detected".to_string());
        } else if license_key.len() % 4 == 0
            && license_key
                .chars()
                .all(|c| c.is_alphanumeric() || "+-/=".contains(c))
        {
            result.key_type = Some("base64".to_string());
            result.add_info("Base64-encoded license key detected".to_string());
        } else {
            result.key_type = Some("custom".to_string());
        }

        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    fn add_info(&mut self, info: String) {
        self.info.push(info);
    }

    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }

    pub fn summary(&self) -> String {
        format!(
            "Validation result: {} (Errors: {}, Warnings: {}, Info: {})",
            if self.is_valid { "VALID" } else { "INVALID" },
            self.errors.len(),
            self.warnings.len(),
            self.info.len()
        )
    }
}

#[derive(Debug, Clone)]
pub struct KeyValidationResult {
    pub is_valid_format: bool,
    pub key_type: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

impl KeyValidationResult {
    fn new() -> Self {
        Self {
            is_valid_format: true,
            key_type: None,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    fn add_error(&mut self, error: String) {
        self.is_valid_format = false;
        self.errors.push(error);
    }

    fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    fn add_info(&mut self, info: String) {
        self.info.push(info);
    }
}
