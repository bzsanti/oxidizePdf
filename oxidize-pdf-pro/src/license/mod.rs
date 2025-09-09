use crate::error::{ProError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

pub mod features;
pub mod validator;

pub use features::FeatureGate;
pub use validator::LicenseValidator;

static LICENSE_STATE: OnceLock<Arc<Mutex<LicenseState>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProLicense {
    pub license_key: String,
    pub license_type: LicenseType,
    pub customer_id: String,
    pub issued_date: DateTime<Utc>,
    pub expiry_date: Option<DateTime<Utc>>,
    pub features: Vec<String>,
    pub usage_limits: Option<UsageLimits>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LicenseType {
    Professional,
    Enterprise,
    Trial,
    Development,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLimits {
    pub max_documents_per_month: Option<u32>,
    pub max_pages_per_document: Option<u32>,
    pub max_entities_per_document: Option<u32>,
    pub max_concurrent_processes: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct LicenseInfo {
    pub is_valid: bool,
    pub license_type: Option<LicenseType>,
    pub expires_at: Option<DateTime<Utc>>,
    pub features: Vec<String>,
    pub usage_stats: UsageStats,
    pub days_until_expiry: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct UsageStats {
    pub documents_processed_this_month: u32,
    pub total_pages_processed: u64,
    pub entities_extracted_this_month: u32,
    pub current_concurrent_processes: u32,
}

#[derive(Debug, Clone)]
struct LicenseState {
    current_license: Option<ProLicense>,
    validation_cache: HashMap<String, (DateTime<Utc>, bool)>,
    usage_stats: UsageStats,
    last_validation: Option<DateTime<Utc>>,
}

impl LicenseState {
    fn new() -> Self {
        Self {
            current_license: None,
            validation_cache: HashMap::new(),
            usage_stats: UsageStats::default(),
            last_validation: None,
        }
    }
}

pub fn validate_license(license_key: Option<&str>) -> Result<()> {
    let state = LICENSE_STATE.get_or_init(|| Arc::new(Mutex::new(LicenseState::new())));
    let mut state_guard = state
        .lock()
        .map_err(|_| ProError::LicenseValidation("Lock poisoned".to_string()))?;

    match license_key {
        Some(key) => {
            let license = validate_license_key(key)?;
            state_guard.current_license = Some(license);
            state_guard.last_validation = Some(Utc::now());
            Ok(())
        }
        None => {
            // Check for environment variable
            if let Ok(env_key) = std::env::var("OXIDIZE_PDF_PRO_LICENSE") {
                let license = validate_license_key(&env_key)?;
                state_guard.current_license = Some(license);
                state_guard.last_validation = Some(Utc::now());
                Ok(())
            } else {
                tracing::warn!("No license key provided. Pro features will be disabled.");
                state_guard.current_license = None;
                Ok(())
            }
        }
    }
}

pub fn is_valid_license() -> bool {
    let state = LICENSE_STATE.get_or_init(|| Arc::new(Mutex::new(LicenseState::new())));
    let state_guard = match state.lock() {
        Ok(guard) => guard,
        Err(_) => return false,
    };

    match &state_guard.current_license {
        Some(license) => {
            // Check expiry
            if let Some(expiry) = license.expiry_date {
                if Utc::now() > expiry {
                    return false;
                }
            }

            // Check if validation is recent (within 24 hours for online validation)
            if let Some(last_validation) = state_guard.last_validation {
                let hours_since_validation = (Utc::now() - last_validation).num_hours();
                hours_since_validation < 24
            } else {
                false
            }
        }
        None => false,
    }
}

pub fn get_license_info() -> LicenseInfo {
    let state = LICENSE_STATE.get_or_init(|| Arc::new(Mutex::new(LicenseState::new())));
    let state_guard = match state.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return LicenseInfo {
                is_valid: false,
                license_type: None,
                expires_at: None,
                features: Vec::new(),
                usage_stats: UsageStats::default(),
                days_until_expiry: None,
            }
        }
    };

    match &state_guard.current_license {
        Some(license) => {
            let days_until_expiry = license
                .expiry_date
                .map(|expiry| (expiry - Utc::now()).num_days());

            LicenseInfo {
                is_valid: is_valid_license(),
                license_type: Some(license.license_type.clone()),
                expires_at: license.expiry_date,
                features: license.features.clone(),
                usage_stats: state_guard.usage_stats.clone(),
                days_until_expiry,
            }
        }
        None => LicenseInfo {
            is_valid: false,
            license_type: None,
            expires_at: None,
            features: Vec::new(),
            usage_stats: state_guard.usage_stats.clone(),
            days_until_expiry: None,
        },
    }
}

pub fn check_feature_access(feature: &str) -> Result<()> {
    if !is_valid_license() {
        return Err(ProError::LicenseValidation(
            "No valid license found".to_string(),
        ));
    }

    let state = LICENSE_STATE.get_or_init(|| Arc::new(Mutex::new(LicenseState::new())));
    let state_guard = state
        .lock()
        .map_err(|_| ProError::LicenseValidation("Lock poisoned".to_string()))?;

    match &state_guard.current_license {
        Some(license) => {
            if license.features.contains(&feature.to_string())
                || license.features.contains(&"*".to_string())
            {
                Ok(())
            } else {
                Err(ProError::FeatureNotLicensed(feature.to_string()))
            }
        }
        None => Err(ProError::LicenseValidation("No license loaded".to_string())),
    }
}

pub fn record_usage(usage_type: UsageType, count: u32) -> Result<()> {
    let state = LICENSE_STATE.get_or_init(|| Arc::new(Mutex::new(LicenseState::new())));
    let mut state_guard = state
        .lock()
        .map_err(|_| ProError::LicenseValidation("Lock poisoned".to_string()))?;

    match usage_type {
        UsageType::DocumentProcessed => {
            state_guard.usage_stats.documents_processed_this_month += count;
        }
        UsageType::PageProcessed => {
            state_guard.usage_stats.total_pages_processed += count as u64;
        }
        UsageType::EntityExtracted => {
            state_guard.usage_stats.entities_extracted_this_month += count;
        }
        UsageType::ConcurrentProcess => {
            state_guard.usage_stats.current_concurrent_processes = count;
        }
    }

    // Check usage limits
    if let Some(license) = &state_guard.current_license {
        if let Some(limits) = &license.usage_limits {
            check_usage_limits(&state_guard.usage_stats, limits)?;
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub enum UsageType {
    DocumentProcessed,
    PageProcessed,
    EntityExtracted,
    ConcurrentProcess,
}

fn validate_license_key(license_key: &str) -> Result<ProLicense> {
    // First try offline validation (for development/testing)
    if let Ok(license) = validate_offline_license(license_key) {
        return Ok(license);
    }

    // Try online validation if available
    #[cfg(feature = "license-validation")]
    {
        if let Ok(license) = validate_online_license(license_key) {
            return Ok(license);
        }
    }

    Err(ProError::LicenseValidation(
        "Invalid license key".to_string(),
    ))
}

fn validate_offline_license(license_key: &str) -> Result<ProLicense> {
    // Development/testing license keys
    match license_key {
        "OXIDIZE_PRO_DEV" => Ok(ProLicense {
            license_key: license_key.to_string(),
            license_type: LicenseType::Development,
            customer_id: "dev".to_string(),
            issued_date: Utc::now(),
            expiry_date: Some(Utc::now() + chrono::Duration::days(365)),
            features: vec!["*".to_string()], // All features
            usage_limits: None,
            metadata: HashMap::new(),
        }),
        "OXIDIZE_PRO_TRIAL" => Ok(ProLicense {
            license_key: license_key.to_string(),
            license_type: LicenseType::Trial,
            customer_id: "trial".to_string(),
            issued_date: Utc::now(),
            expiry_date: Some(Utc::now() + chrono::Duration::days(30)),
            features: vec!["xmp".to_string(), "extraction".to_string()],
            usage_limits: Some(UsageLimits {
                max_documents_per_month: Some(100),
                max_pages_per_document: Some(50),
                max_entities_per_document: Some(100),
                max_concurrent_processes: Some(2),
            }),
            metadata: HashMap::new(),
        }),
        _ => {
            // Try to decode as base64 JWT-like token
            if license_key.len() > 50 {
                decode_license_token(license_key)
            } else {
                Err(ProError::LicenseValidation(
                    "Unknown license key format".to_string(),
                ))
            }
        }
    }
}

#[cfg(feature = "license-validation")]
async fn validate_online_license(license_key: &str) -> Result<ProLicense> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.oxidizepdf.dev/v1/license/validate")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "license_key": license_key,
            "product": "oxidize-pdf-pro",
            "version": env!("CARGO_PKG_VERSION")
        }))
        .send()
        .await?;

    if response.status().is_success() {
        let license: ProLicense = response.json().await?;
        Ok(license)
    } else {
        Err(ProError::LicenseValidation(format!(
            "Server returned status: {}",
            response.status()
        )))
    }
}

#[cfg(not(feature = "license-validation"))]
fn validate_online_license(license_key: &str) -> Result<ProLicense> {
    Err(ProError::LicenseValidation(
        "Online validation not enabled".to_string(),
    ))
}

fn decode_license_token(token: &str) -> Result<ProLicense> {
    // Simple base64 decoding for demo purposes
    // In production, this would be a proper JWT validation
    let decoded = base64::decode(token)?;
    let json_str = String::from_utf8(decoded)
        .map_err(|_| ProError::LicenseValidation("Invalid token encoding".to_string()))?;

    let license: ProLicense = serde_json::from_str(&json_str)?;

    // Validate signature/checksum in production
    Ok(license)
}

fn check_usage_limits(stats: &UsageStats, limits: &UsageLimits) -> Result<()> {
    if let Some(max_docs) = limits.max_documents_per_month {
        if stats.documents_processed_this_month >= max_docs {
            return Err(ProError::LicenseValidation(format!(
                "Monthly document limit exceeded: {}/{}",
                stats.documents_processed_this_month, max_docs
            )));
        }
    }

    if let Some(max_concurrent) = limits.max_concurrent_processes {
        if stats.current_concurrent_processes >= max_concurrent {
            return Err(ProError::LicenseValidation(format!(
                "Concurrent process limit exceeded: {}/{}",
                stats.current_concurrent_processes, max_concurrent
            )));
        }
    }

    if let Some(max_entities) = limits.max_entities_per_document {
        if stats.entities_extracted_this_month >= max_entities {
            return Err(ProError::LicenseValidation(format!(
                "Monthly entity extraction limit exceeded: {}/{}",
                stats.entities_extracted_this_month, max_entities
            )));
        }
    }

    Ok(())
}
