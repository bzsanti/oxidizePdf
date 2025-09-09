use super::{check_feature_access, record_usage, UsageType};
use crate::error::{ProError, Result};

pub struct FeatureGate;

impl FeatureGate {
    /// Check if XMP embedding features are available
    pub fn check_xmp_features() -> Result<()> {
        check_feature_access("xmp")
    }

    /// Check if entity extraction features are available
    pub fn check_extraction_features() -> Result<()> {
        check_feature_access("extraction")
    }

    /// Check if Pro templates are available
    pub fn check_template_features() -> Result<()> {
        check_feature_access("templates")
    }

    /// Check if advanced OCR features are available
    pub fn check_advanced_ocr() -> Result<()> {
        check_feature_access("advanced-ocr")
    }

    /// Check if streaming API features are available
    pub fn check_streaming_api() -> Result<()> {
        check_feature_access("streaming-api")
    }

    /// Check if dashboard templates are available
    pub fn check_dashboard_templates() -> Result<()> {
        check_feature_access("dashboard-templates")
    }

    /// Check if enterprise support features are available
    pub fn check_enterprise_support() -> Result<()> {
        check_feature_access("enterprise-support")
    }

    /// Check if multi-tenant features are available
    pub fn check_multi_tenant() -> Result<()> {
        check_feature_access("multi-tenant")
    }

    /// Record document processing usage
    pub fn record_document_processed(count: u32) -> Result<()> {
        record_usage(UsageType::DocumentProcessed, count)
    }

    /// Record page processing usage
    pub fn record_pages_processed(count: u32) -> Result<()> {
        record_usage(UsageType::PageProcessed, count)
    }

    /// Record entity extraction usage
    pub fn record_entities_extracted(count: u32) -> Result<()> {
        record_usage(UsageType::EntityExtracted, count)
    }

    /// Update concurrent process count
    pub fn update_concurrent_processes(count: u32) -> Result<()> {
        record_usage(UsageType::ConcurrentProcess, count)
    }
}

/// Macro to automatically check feature access and record usage
#[macro_export]
macro_rules! with_feature {
    ($feature:literal, $usage_type:expr, $count:expr, $body:block) => {{
        use $crate::license::FeatureGate;

        // Check feature access
        match $feature {
            "xmp" => FeatureGate::check_xmp_features()?,
            "extraction" => FeatureGate::check_extraction_features()?,
            "templates" => FeatureGate::check_template_features()?,
            "advanced-ocr" => FeatureGate::check_advanced_ocr()?,
            "streaming-api" => FeatureGate::check_streaming_api()?,
            "dashboard-templates" => FeatureGate::check_dashboard_templates()?,
            "enterprise-support" => FeatureGate::check_enterprise_support()?,
            "multi-tenant" => FeatureGate::check_multi_tenant()?,
            _ => $crate::license::check_feature_access($feature)?,
        }

        // Record usage before execution
        match $usage_type {
            Some($crate::license::UsageType::DocumentProcessed) => {
                FeatureGate::record_document_processed($count)?;
            }
            Some($crate::license::UsageType::PageProcessed) => {
                FeatureGate::record_pages_processed($count)?;
            }
            Some($crate::license::UsageType::EntityExtracted) => {
                FeatureGate::record_entities_extracted($count)?;
            }
            Some($crate::license::UsageType::ConcurrentProcess) => {
                FeatureGate::update_concurrent_processes($count)?;
            }
            None => {}
        }

        // Execute the protected code
        let result = $body;

        result
    }};

    ($feature:literal, $body:block) => {
        with_feature!($feature, None::<$crate::license::UsageType>, 0, $body)
    };
}

/// Attribute macro for feature gating functions (conceptual - would need proc_macro)
///
/// Usage example:
/// ```rust
/// #[feature_gate("xmp")]
/// pub fn embed_xmp_metadata(doc: &mut Document) -> Result<()> {
///     // Implementation
/// }
/// ```
///
/// This would be implemented as a procedural macro in a separate crate.

/// Helper trait for feature-gated operations
pub trait FeatureGated {
    /// Check if the required features are available for this operation
    fn check_features(&self) -> Result<()>;

    /// Get the features required for this operation
    fn required_features(&self) -> Vec<&'static str>;

    /// Get the usage type for recording metrics
    fn usage_type(&self) -> Option<UsageType> {
        None
    }

    /// Get the usage count for recording metrics
    fn usage_count(&self) -> u32 {
        1
    }
}

/// Standard feature gates as constants for consistency
pub mod features {
    pub const XMP_EMBEDDING: &str = "xmp";
    pub const ENTITY_EXTRACTION: &str = "extraction";
    pub const PRO_TEMPLATES: &str = "templates";
    pub const ADVANCED_OCR: &str = "advanced-ocr";
    pub const STREAMING_API: &str = "streaming-api";
    pub const DASHBOARD_TEMPLATES: &str = "dashboard-templates";
    pub const ENTERPRISE_SUPPORT: &str = "enterprise-support";
    pub const MULTI_TENANT: &str = "multi-tenant";
    pub const ALL_FEATURES: &str = "*";
}

/// License tier information
pub mod tiers {
    use super::features::*;

    pub fn community_features() -> Vec<&'static str> {
        vec![] // Community edition has no Pro features
    }

    pub fn professional_features() -> Vec<&'static str> {
        vec![
            XMP_EMBEDDING,
            ENTITY_EXTRACTION,
            PRO_TEMPLATES,
            ADVANCED_OCR,
            STREAMING_API,
            DASHBOARD_TEMPLATES,
        ]
    }

    pub fn enterprise_features() -> Vec<&'static str> {
        vec![
            ALL_FEATURES, // Enterprise gets everything
        ]
    }

    pub fn trial_features() -> Vec<&'static str> {
        vec![XMP_EMBEDDING, ENTITY_EXTRACTION, PRO_TEMPLATES]
    }

    pub fn development_features() -> Vec<&'static str> {
        vec![
            ALL_FEATURES, // Development gets everything for testing
        ]
    }
}

/// Feature compatibility matrix
pub mod compatibility {
    use super::super::LicenseType;
    use super::features::*;

    pub fn is_feature_available(license_type: &LicenseType, feature: &str) -> bool {
        match license_type {
            LicenseType::Professional => super::tiers::professional_features().contains(&feature),
            LicenseType::Enterprise => {
                feature == ALL_FEATURES || super::tiers::enterprise_features().contains(&feature)
            }
            LicenseType::Trial => super::tiers::trial_features().contains(&feature),
            LicenseType::Development => {
                true // Development licenses get everything
            }
        }
    }

    pub fn get_missing_features<'a>(
        license_type: &LicenseType,
        requested: &'a [&str],
    ) -> Vec<&'a str> {
        requested
            .iter()
            .filter(|&feature| !is_feature_available(license_type, feature))
            .copied()
            .collect()
    }

    pub fn recommend_license_tier(requested_features: &[&str]) -> LicenseType {
        let enterprise_only = [ENTERPRISE_SUPPORT, MULTI_TENANT];
        let professional_features = super::tiers::professional_features();

        if requested_features
            .iter()
            .any(|f| enterprise_only.contains(f))
        {
            LicenseType::Enterprise
        } else if requested_features
            .iter()
            .any(|f| professional_features.contains(f))
        {
            LicenseType::Professional
        } else {
            LicenseType::Trial // Could also recommend Community for no Pro features
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compatibility::*;
    use super::*;

    #[test]
    fn test_feature_availability() {
        assert!(is_feature_available(
            &LicenseType::Enterprise,
            features::XMP_EMBEDDING
        ));
        assert!(is_feature_available(
            &LicenseType::Professional,
            features::XMP_EMBEDDING
        ));
        assert!(!is_feature_available(
            &LicenseType::Trial,
            features::ENTERPRISE_SUPPORT
        ));
        assert!(is_feature_available(
            &LicenseType::Development,
            features::ENTERPRISE_SUPPORT
        ));
    }

    #[test]
    fn test_license_recommendations() {
        let basic_features = vec![features::XMP_EMBEDDING];
        assert_eq!(
            recommend_license_tier(&basic_features),
            LicenseType::Professional
        );

        let enterprise_features = vec![features::MULTI_TENANT];
        assert_eq!(
            recommend_license_tier(&enterprise_features),
            LicenseType::Enterprise
        );
    }
}
