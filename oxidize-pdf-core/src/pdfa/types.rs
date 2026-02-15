//! Core types for PDF/A compliance

use super::error::ValidationError;
use std::fmt;
use std::str::FromStr;

/// PDF/A conformance level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PdfAConformance {
    /// Level B - Basic conformance (visual appearance)
    B,
    /// Level U - Unicode conformance (text can be reliably extracted)
    U,
    /// Level A - Accessible conformance (tagged PDF, full Unicode mapping)
    A,
}

impl fmt::Display for PdfAConformance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::B => write!(f, "B"),
            Self::U => write!(f, "U"),
            Self::A => write!(f, "A"),
        }
    }
}

impl FromStr for PdfAConformance {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "B" => Ok(Self::B),
            "U" => Ok(Self::U),
            "A" => Ok(Self::A),
            _ => Err(format!("Invalid PDF/A conformance level: {}", s)),
        }
    }
}

/// PDF/A level (part + conformance)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PdfALevel {
    /// PDF/A-1a (ISO 19005-1:2005, Level A - Accessible)
    A1a,
    /// PDF/A-1b (ISO 19005-1:2005, Level B - Basic)
    A1b,
    /// PDF/A-2a (ISO 19005-2:2011, Level A - Accessible)
    A2a,
    /// PDF/A-2b (ISO 19005-2:2011, Level B - Basic)
    A2b,
    /// PDF/A-2u (ISO 19005-2:2011, Level U - Unicode)
    A2u,
    /// PDF/A-3a (ISO 19005-3:2012, Level A - Accessible)
    A3a,
    /// PDF/A-3b (ISO 19005-3:2012, Level B - Basic)
    A3b,
    /// PDF/A-3u (ISO 19005-3:2012, Level U - Unicode)
    A3u,
}

impl PdfALevel {
    /// Get the PDF/A part number (1, 2, or 3)
    pub fn part(&self) -> u8 {
        match self {
            Self::A1a | Self::A1b => 1,
            Self::A2a | Self::A2b | Self::A2u => 2,
            Self::A3a | Self::A3b | Self::A3u => 3,
        }
    }

    /// Get the conformance level
    pub fn conformance(&self) -> PdfAConformance {
        match self {
            Self::A1a | Self::A2a | Self::A3a => PdfAConformance::A,
            Self::A1b | Self::A2b | Self::A3b => PdfAConformance::B,
            Self::A2u | Self::A3u => PdfAConformance::U,
        }
    }

    /// Get the required PDF version for this level
    pub fn required_pdf_version(&self) -> &'static str {
        match self.part() {
            1 => "1.4",
            2 | 3 => "1.7",
            _ => "1.7",
        }
    }

    /// Check if transparency is allowed
    pub fn allows_transparency(&self) -> bool {
        // Transparency is forbidden in PDF/A-1, allowed in PDF/A-2 and PDF/A-3
        self.part() >= 2
    }

    /// Check if LZW compression is allowed
    pub fn allows_lzw(&self) -> bool {
        // LZW is forbidden in PDF/A-1, allowed in PDF/A-2 and PDF/A-3
        self.part() >= 2
    }

    /// Check if embedded files are allowed
    pub fn allows_embedded_files(&self) -> bool {
        // Embedded files are forbidden in PDF/A-1 and PDF/A-2, allowed in PDF/A-3
        self.part() == 3
    }

    /// Get the ISO standard reference
    pub fn iso_reference(&self) -> &'static str {
        match self.part() {
            1 => "ISO 19005-1:2005",
            2 => "ISO 19005-2:2011",
            3 => "ISO 19005-3:2012",
            _ => "Unknown",
        }
    }
}

impl fmt::Display for PdfALevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PDF/A-{}{}", self.part(), self.conformance())
    }
}

impl FromStr for PdfALevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_uppercase().replace("PDF/A-", "").replace("PDFA", "");
        match s.as_str() {
            "1A" => Ok(Self::A1a),
            "1B" => Ok(Self::A1b),
            "2A" => Ok(Self::A2a),
            "2B" => Ok(Self::A2b),
            "2U" => Ok(Self::A2u),
            "3A" => Ok(Self::A3a),
            "3B" => Ok(Self::A3b),
            "3U" => Ok(Self::A3u),
            _ => Err(format!("Invalid PDF/A level: {}", s)),
        }
    }
}

/// Warnings during PDF/A validation (informational, don't affect compliance)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationWarning {
    /// Font is subset but may cause issues
    FontSubsetWarning {
        /// Font name
        font_name: String,
        /// Warning details
        details: String,
    },
    /// Optional metadata field is missing
    OptionalMetadataMissing {
        /// Field name
        field: String,
    },
    /// Color profile warning
    ColorProfileWarning {
        /// Warning details
        details: String,
    },
    /// File size warning
    LargeFileWarning {
        /// File size in bytes
        size_bytes: u64,
    },
}

impl fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FontSubsetWarning { font_name, details } => {
                write!(f, "Font '{}' subset warning: {}", font_name, details)
            }
            Self::OptionalMetadataMissing { field } => {
                write!(f, "Optional metadata field '{}' is missing", field)
            }
            Self::ColorProfileWarning { details } => {
                write!(f, "Color profile warning: {}", details)
            }
            Self::LargeFileWarning { size_bytes } => {
                write!(
                    f,
                    "Large file ({:.2} MB) may cause performance issues",
                    *size_bytes as f64 / 1_048_576.0
                )
            }
        }
    }
}

/// Result of PDF/A validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// The PDF/A level that was validated against
    level: PdfALevel,
    /// List of validation errors (empty if compliant)
    errors: Vec<ValidationError>,
    /// List of warnings (informational, don't affect compliance)
    warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Creates a new validation result
    pub fn new(level: PdfALevel) -> Self {
        Self {
            level,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Creates a validation result with errors
    pub fn with_errors(level: PdfALevel, errors: Vec<ValidationError>) -> Self {
        Self {
            level,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Creates a validation result with errors and warnings
    pub fn with_errors_and_warnings(
        level: PdfALevel,
        errors: Vec<ValidationError>,
        warnings: Vec<ValidationWarning>,
    ) -> Self {
        Self {
            level,
            errors,
            warnings,
        }
    }

    /// Returns true if the document is compliant (no errors)
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns the PDF/A level that was validated against
    pub fn level(&self) -> PdfALevel {
        self.level
    }

    /// Returns the list of validation errors
    pub fn errors(&self) -> &[ValidationError] {
        &self.errors
    }

    /// Returns the list of warnings
    pub fn warnings(&self) -> &[ValidationWarning] {
        &self.warnings
    }

    /// Returns the number of errors
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Returns the number of warnings
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Adds an error to the result
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
    }

    /// Adds a warning to the result
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(f, "{} compliant", self.level)?;
        } else {
            write!(
                f,
                "{} validation failed: {} error(s)",
                self.level,
                self.errors.len()
            )?;
        }
        if !self.warnings.is_empty() {
            write!(f, ", {} warning(s)", self.warnings.len())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdfa_level_part() {
        assert_eq!(PdfALevel::A1a.part(), 1);
        assert_eq!(PdfALevel::A1b.part(), 1);
        assert_eq!(PdfALevel::A2a.part(), 2);
        assert_eq!(PdfALevel::A2b.part(), 2);
        assert_eq!(PdfALevel::A2u.part(), 2);
        assert_eq!(PdfALevel::A3a.part(), 3);
        assert_eq!(PdfALevel::A3b.part(), 3);
        assert_eq!(PdfALevel::A3u.part(), 3);
    }

    #[test]
    fn test_pdfa_level_conformance() {
        assert_eq!(PdfALevel::A1a.conformance(), PdfAConformance::A);
        assert_eq!(PdfALevel::A1b.conformance(), PdfAConformance::B);
        assert_eq!(PdfALevel::A2a.conformance(), PdfAConformance::A);
        assert_eq!(PdfALevel::A2b.conformance(), PdfAConformance::B);
        assert_eq!(PdfALevel::A2u.conformance(), PdfAConformance::U);
        assert_eq!(PdfALevel::A3a.conformance(), PdfAConformance::A);
        assert_eq!(PdfALevel::A3b.conformance(), PdfAConformance::B);
        assert_eq!(PdfALevel::A3u.conformance(), PdfAConformance::U);
    }

    #[test]
    fn test_pdfa_level_required_version() {
        assert_eq!(PdfALevel::A1b.required_pdf_version(), "1.4");
        assert_eq!(PdfALevel::A2b.required_pdf_version(), "1.7");
        assert_eq!(PdfALevel::A3b.required_pdf_version(), "1.7");
    }

    #[test]
    fn test_pdfa_level_transparency() {
        assert!(!PdfALevel::A1b.allows_transparency());
        assert!(PdfALevel::A2b.allows_transparency());
        assert!(PdfALevel::A3b.allows_transparency());
    }

    #[test]
    fn test_pdfa_level_lzw() {
        assert!(!PdfALevel::A1b.allows_lzw());
        assert!(PdfALevel::A2b.allows_lzw());
        assert!(PdfALevel::A3b.allows_lzw());
    }

    #[test]
    fn test_pdfa_level_embedded_files() {
        assert!(!PdfALevel::A1b.allows_embedded_files());
        assert!(!PdfALevel::A2b.allows_embedded_files());
        assert!(PdfALevel::A3b.allows_embedded_files());
    }

    #[test]
    fn test_pdfa_level_display() {
        assert_eq!(PdfALevel::A1b.to_string(), "PDF/A-1B");
        assert_eq!(PdfALevel::A2u.to_string(), "PDF/A-2U");
        assert_eq!(PdfALevel::A3a.to_string(), "PDF/A-3A");
    }

    #[test]
    fn test_pdfa_level_from_str() {
        assert_eq!("1B".parse::<PdfALevel>().unwrap(), PdfALevel::A1b);
        assert_eq!("PDF/A-2U".parse::<PdfALevel>().unwrap(), PdfALevel::A2u);
        assert_eq!("3a".parse::<PdfALevel>().unwrap(), PdfALevel::A3a);
    }

    #[test]
    fn test_pdfa_level_from_str_invalid() {
        assert!("4B".parse::<PdfALevel>().is_err());
        assert!("invalid".parse::<PdfALevel>().is_err());
    }

    #[test]
    fn test_pdfa_conformance_display() {
        assert_eq!(PdfAConformance::A.to_string(), "A");
        assert_eq!(PdfAConformance::B.to_string(), "B");
        assert_eq!(PdfAConformance::U.to_string(), "U");
    }

    #[test]
    fn test_pdfa_conformance_from_str() {
        assert_eq!("A".parse::<PdfAConformance>().unwrap(), PdfAConformance::A);
        assert_eq!("b".parse::<PdfAConformance>().unwrap(), PdfAConformance::B);
        assert_eq!("U".parse::<PdfAConformance>().unwrap(), PdfAConformance::U);
    }

    #[test]
    fn test_validation_result_new() {
        let result = ValidationResult::new(PdfALevel::A1b);
        assert!(result.is_valid());
        assert_eq!(result.level(), PdfALevel::A1b);
        assert_eq!(result.error_count(), 0);
        assert_eq!(result.warning_count(), 0);
    }

    #[test]
    fn test_validation_result_with_errors() {
        let errors = vec![ValidationError::EncryptionForbidden];
        let result = ValidationResult::with_errors(PdfALevel::A2b, errors);
        assert!(!result.is_valid());
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut result = ValidationResult::new(PdfALevel::A1b);
        assert!(result.is_valid());
        result.add_error(ValidationError::XmpMetadataMissing);
        assert!(!result.is_valid());
        assert_eq!(result.error_count(), 1);
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::new(PdfALevel::A1b);
        result.add_warning(ValidationWarning::OptionalMetadataMissing {
            field: "Title".to_string(),
        });
        assert!(result.is_valid()); // Warnings don't affect validity
        assert_eq!(result.warning_count(), 1);
    }

    #[test]
    fn test_validation_result_display_valid() {
        let result = ValidationResult::new(PdfALevel::A1b);
        assert!(result.to_string().contains("compliant"));
    }

    #[test]
    fn test_validation_result_display_invalid() {
        let errors = vec![ValidationError::EncryptionForbidden];
        let result = ValidationResult::with_errors(PdfALevel::A2b, errors);
        assert!(result.to_string().contains("failed"));
        assert!(result.to_string().contains("1 error"));
    }

    #[test]
    fn test_pdfa_level_iso_reference() {
        assert_eq!(PdfALevel::A1b.iso_reference(), "ISO 19005-1:2005");
        assert_eq!(PdfALevel::A2b.iso_reference(), "ISO 19005-2:2011");
        assert_eq!(PdfALevel::A3b.iso_reference(), "ISO 19005-3:2012");
    }

    #[test]
    fn test_validation_warning_display() {
        let warning = ValidationWarning::LargeFileWarning {
            size_bytes: 10_485_760,
        };
        assert!(warning.to_string().contains("10.00 MB"));
    }

    #[test]
    fn test_pdfa_level_clone_eq() {
        let level1 = PdfALevel::A1b;
        let level2 = level1;
        assert_eq!(level1, level2);
    }

    #[test]
    fn test_pdfa_conformance_clone_eq() {
        let conf1 = PdfAConformance::A;
        let conf2 = conf1;
        assert_eq!(conf1, conf2);
    }
}
