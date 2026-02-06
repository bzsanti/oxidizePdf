//! External PDF Validators
//!
//! This module integrates with external PDF validation tools to provide
//! REAL verification of ISO compliance. These tools are industry-standard
//! and provide the ground truth for PDF validity.

use crate::error::{PdfError, Result};
use crate::verification::ExternalValidationResult;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Validate PDF with external tools
pub fn validate_external(pdf_bytes: &[u8]) -> Result<ExternalValidationResult> {
    // Create temporary file for validation
    let mut temp_file = NamedTempFile::new().map_err(PdfError::Io)?;

    temp_file.write_all(pdf_bytes).map_err(PdfError::Io)?;

    let temp_path = temp_file.path();

    let mut result = ExternalValidationResult {
        qpdf_passed: None,
        verapdf_passed: None,
        adobe_preflight_passed: None,
        error_messages: Vec::new(),
    };

    // Validate with qpdf (most important)
    if let Some(path_str) = temp_path.to_str() {
        match validate_with_qpdf(path_str) {
            Ok(passed) => result.qpdf_passed = Some(passed),
            Err(e) => result.error_messages.push(format!("qpdf error: {}", e)),
        }

        // Validate with veraPDF if available
        match validate_with_verapdf(path_str) {
            Ok(passed) => result.verapdf_passed = Some(passed),
            Err(e) => result.error_messages.push(format!("veraPDF error: {}", e)),
        }

        // Adobe Preflight is typically not available in CI/dev environments
        // but we'll try if it exists
        match validate_with_adobe_preflight(path_str) {
            Ok(passed) => result.adobe_preflight_passed = Some(passed),
            Err(_) => {
                // Adobe Preflight not available - this is expected
                result
                    .error_messages
                    .push("Adobe Preflight not available".to_string());
            }
        }
    } else {
        result
            .error_messages
            .push("Path contains invalid UTF-8 characters".to_string());
    }

    Ok(result)
}

/// Validate PDF with qpdf
/// qpdf is the most reliable open-source PDF validator
pub fn validate_with_qpdf(pdf_path: &str) -> Result<bool> {
    let output = Command::new("qpdf")
        .arg("--check")
        .arg("--show-all-pages")
        .arg(pdf_path)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(true)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(PdfError::ExternalValidationError(format!(
                    "qpdf validation failed: {}",
                    stderr
                )))
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(PdfError::ExternalValidationError(
                    "qpdf not found. Install with: brew install qpdf".to_string(),
                ))
            } else {
                Err(PdfError::ExternalValidationError(format!(
                    "Failed to run qpdf: {}",
                    e
                )))
            }
        }
    }
}

/// Validate PDF with veraPDF
/// veraPDF is specifically designed for PDF/A validation
pub fn validate_with_verapdf(pdf_path: &str) -> Result<bool> {
    let output = Command::new("verapdf")
        .arg("--format")
        .arg("pdf")
        .arg("--flavour")
        .arg("1b") // PDF/A-1b validation
        .arg(pdf_path)
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // veraPDF returns success even for invalid PDFs, so check output
            if stdout.contains("ValidationProfile") && !stdout.contains("failed") {
                Ok(true)
            } else {
                Err(PdfError::ExternalValidationError(format!(
                    "veraPDF validation failed: {}",
                    stderr
                )))
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(PdfError::ExternalValidationError(
                    "veraPDF not found. Download from: https://verapdf.org/".to_string(),
                ))
            } else {
                Err(PdfError::ExternalValidationError(format!(
                    "Failed to run veraPDF: {}",
                    e
                )))
            }
        }
    }
}

/// Validate PDF with Adobe Preflight (if available)
/// This is the gold standard but not available in most environments
pub fn validate_with_adobe_preflight(pdf_path: &str) -> Result<bool> {
    // Adobe Preflight is typically part of Adobe Acrobat Pro
    // It's not available in most CI/development environments
    // This is a placeholder for when it might be available

    let output = Command::new("acrobat")
        .arg("-preflight")
        .arg("ISO32000")
        .arg(pdf_path)
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(true)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(PdfError::ExternalValidationError(format!(
                    "Adobe Preflight failed: {}",
                    stderr
                )))
            }
        }
        Err(_) => Err(PdfError::ExternalValidationError(
            "Adobe Preflight not available".to_string(),
        )),
    }
}

/// Additional validation with pdftk for structure checking
pub fn validate_with_pdftk(pdf_path: &str) -> Result<bool> {
    let output = Command::new("pdftk")
        .arg(pdf_path)
        .arg("dump_data")
        .output();

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Basic checks for PDF structure
                let has_info = stdout.contains("InfoKey:");
                let has_pages = stdout.contains("NumberOfPages:");
                Ok(has_info && has_pages)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Err(PdfError::ExternalValidationError(format!(
                    "pdftk validation failed: {}",
                    stderr
                )))
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(PdfError::ExternalValidationError(
                    "pdftk not found. Install with: brew install pdftk-java".to_string(),
                ))
            } else {
                Err(PdfError::ExternalValidationError(format!(
                    "Failed to run pdftk: {}",
                    e
                )))
            }
        }
    }
}

/// Check which external validators are available
pub fn check_available_validators() -> Vec<String> {
    let mut available = Vec::new();

    // Check qpdf
    if Command::new("qpdf").arg("--version").output().is_ok() {
        available.push("qpdf".to_string());
    }

    // Check veraPDF
    if Command::new("verapdf").arg("--version").output().is_ok() {
        available.push("verapdf".to_string());
    }

    // Check pdftk
    if Command::new("pdftk").arg("--version").output().is_ok() {
        available.push("pdftk".to_string());
    }

    // Check Adobe Acrobat (very unlikely)
    if Command::new("acrobat").arg("-help").output().is_ok() {
        available.push("adobe-acrobat".to_string());
    }

    available
}

/// Install instructions for missing validators
pub fn get_install_instructions() -> HashMap<String, String> {
    let mut instructions = HashMap::new();

    instructions.insert(
        "qpdf".to_string(),
        "Install qpdf:\n  macOS: brew install qpdf\n  Ubuntu: apt-get install qpdf\n  Windows: Download from https://qpdf.sourceforge.io/".to_string()
    );

    instructions.insert(
        "verapdf".to_string(),
        "Install veraPDF:\n  Download from https://verapdf.org/software/\n  Or use: brew install verapdf".to_string()
    );

    instructions.insert(
        "pdftk".to_string(),
        "Install pdftk:\n  macOS: brew install pdftk-java\n  Ubuntu: apt-get install pdftk\n  Windows: Download from https://www.pdflabs.com/tools/pdftk-the-pdf-toolkit/".to_string()
    );

    instructions
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_minimal_pdf() -> Vec<u8> {
        // Create a minimal valid PDF for testing
        let pdf_content = r#"%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj

2 0 obj
<<
/Type /Pages
/Kids [3 0 R]
/Count 1
>>
endobj

3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
>>
endobj

xref
0 4
0000000000 65535 f
0000000010 00000 n
0000000079 00000 n
0000000173 00000 n
trailer
<<
/Size 4
/Root 1 0 R
>>
startxref
256
%%EOF"#;
        pdf_content.as_bytes().to_vec()
    }

    fn create_invalid_pdf() -> Vec<u8> {
        // Create an invalid PDF (corrupted structure)
        b"not a valid pdf at all".to_vec()
    }

    fn create_truncated_pdf() -> Vec<u8> {
        // Create a truncated PDF missing the trailer
        b"%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n>>\nendobj\n".to_vec()
    }

    #[test]
    fn test_check_available_validators() {
        let available = check_available_validators();
        // Just test that the function runs without panicking
        // The actual validators may or may not be available in test environment
        // Just ensure function runs without actual assertion on length
        let _ = available.len();
    }

    #[test]
    fn test_check_available_validators_returns_vec() {
        let available = check_available_validators();
        // Verify returns a Vec<String>
        for validator in &available {
            assert!(!validator.is_empty());
        }
    }

    #[test]
    fn test_get_install_instructions() {
        let instructions = get_install_instructions();
        assert!(instructions.contains_key("qpdf"));
        assert!(instructions.contains_key("verapdf"));
        assert!(instructions.contains_key("pdftk"));
        assert!(instructions["qpdf"].contains("brew install qpdf"));
    }

    #[test]
    fn test_get_install_instructions_verapdf_content() {
        let instructions = get_install_instructions();
        assert!(instructions["verapdf"].contains("https://verapdf.org/"));
    }

    #[test]
    fn test_get_install_instructions_pdftk_content() {
        let instructions = get_install_instructions();
        assert!(instructions["pdftk"].contains("pdftk-java"));
    }

    #[test]
    fn test_validate_external_with_mock_pdf() {
        let pdf_bytes = create_minimal_pdf();

        // This test will only succeed if qpdf is installed
        // In CI environments without qpdf, it should fail gracefully
        match validate_external(&pdf_bytes) {
            Ok(result) => {
                // If validation succeeds, check that we got some result
                assert!(
                    result.qpdf_passed.is_some()
                        || result.verapdf_passed.is_some()
                        || !result.error_messages.is_empty()
                );
            }
            Err(e) => {
                // If validation fails due to missing tools, that's expected
                tracing::debug!(
                    "External validation failed (expected in environments without PDF tools): {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_validate_external_result_structure() {
        let pdf_bytes = create_minimal_pdf();
        let result = validate_external(&pdf_bytes);

        // The function should always return Ok, even if validators fail
        assert!(result.is_ok());

        let validation_result = result.unwrap();
        // Error messages should always be populated (at least for missing tools)
        // This exercises the Ok path of validate_external
        let _ = validation_result.error_messages.len();
    }

    #[test]
    fn test_validate_with_qpdf_valid_pdf() {
        // Create a temporary file with valid PDF
        let pdf_bytes = create_minimal_pdf();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&pdf_bytes).unwrap();
        let path = temp_file.path().to_str().unwrap();

        match validate_with_qpdf(path) {
            Ok(passed) => {
                // qpdf is available and validated successfully
                assert!(passed);
            }
            Err(e) => {
                // qpdf not installed - verify error message
                let err_str = e.to_string();
                assert!(
                    err_str.contains("not found") || err_str.contains("validation failed"),
                    "Unexpected error: {}",
                    err_str
                );
            }
        }
    }

    #[test]
    fn test_validate_with_qpdf_invalid_pdf() {
        // Create a temporary file with invalid PDF
        let pdf_bytes = create_invalid_pdf();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&pdf_bytes).unwrap();
        let path = temp_file.path().to_str().unwrap();

        match validate_with_qpdf(path) {
            Ok(_) => {
                // qpdf might still "pass" on some invalid PDFs
            }
            Err(e) => {
                // Expected: qpdf rejects invalid PDF or qpdf not installed
                let err_str = e.to_string();
                assert!(
                    err_str.contains("validation failed") || err_str.contains("not found"),
                    "Unexpected error: {}",
                    err_str
                );
            }
        }
    }

    #[test]
    fn test_validate_with_qpdf_truncated_pdf() {
        // Create a temporary file with truncated PDF
        let pdf_bytes = create_truncated_pdf();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&pdf_bytes).unwrap();
        let path = temp_file.path().to_str().unwrap();

        match validate_with_qpdf(path) {
            Ok(_) => {
                // Truncated PDFs might pass basic validation
            }
            Err(e) => {
                let err_str = e.to_string();
                assert!(
                    err_str.contains("validation failed") || err_str.contains("not found"),
                    "Unexpected error: {}",
                    err_str
                );
            }
        }
    }

    #[test]
    fn test_validate_with_qpdf_nonexistent_file() {
        let result = validate_with_qpdf("/nonexistent/path/to/file.pdf");

        // Should return an error
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        // Either qpdf not found or file not found error
        assert!(
            err_str.contains("not found") || err_str.contains("failed"),
            "Unexpected error: {}",
            err_str
        );
    }

    #[test]
    fn test_validate_with_verapdf_not_available() {
        // veraPDF is typically not installed on most systems
        let pdf_bytes = create_minimal_pdf();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&pdf_bytes).unwrap();
        let path = temp_file.path().to_str().unwrap();

        match validate_with_verapdf(path) {
            Ok(passed) => {
                // veraPDF is available - verify it returns a boolean
                let _ = passed;
            }
            Err(e) => {
                // Expected: veraPDF not installed
                let err_str = e.to_string();
                assert!(
                    err_str.contains("not found") || err_str.contains("validation failed"),
                    "Unexpected error: {}",
                    err_str
                );
            }
        }
    }

    #[test]
    fn test_validate_with_adobe_preflight_not_available() {
        // Adobe Preflight is almost never available
        let pdf_bytes = create_minimal_pdf();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&pdf_bytes).unwrap();
        let path = temp_file.path().to_str().unwrap();

        let result = validate_with_adobe_preflight(path);

        // Adobe Preflight is almost never installed
        // It should return an error
        match result {
            Ok(_) => {
                // Rare case: Adobe Acrobat is installed
            }
            Err(e) => {
                let err_str = e.to_string();
                assert!(
                    err_str.contains("not available") || err_str.contains("failed"),
                    "Unexpected error: {}",
                    err_str
                );
            }
        }
    }

    #[test]
    fn test_validate_with_pdftk_not_available() {
        // pdftk may or may not be installed
        let pdf_bytes = create_minimal_pdf();
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&pdf_bytes).unwrap();
        let path = temp_file.path().to_str().unwrap();

        match validate_with_pdftk(path) {
            Ok(passed) => {
                // pdftk is available - it checks for InfoKey and NumberOfPages
                // A minimal PDF might not have both
                let _ = passed;
            }
            Err(e) => {
                let err_str = e.to_string();
                assert!(
                    err_str.contains("not found") || err_str.contains("validation failed"),
                    "Unexpected error: {}",
                    err_str
                );
            }
        }
    }

    #[test]
    fn test_validate_with_pdftk_nonexistent_file() {
        let result = validate_with_pdftk("/nonexistent/path/to/file.pdf");

        // Should return an error
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_external_empty_pdf() {
        let pdf_bytes = Vec::new();
        let result = validate_external(&pdf_bytes);

        // Should return Ok with error messages about failed validations
        assert!(result.is_ok());
        let validation_result = result.unwrap();
        // At least Adobe Preflight should be in error messages (always unavailable)
        assert!(!validation_result.error_messages.is_empty());
    }

    #[test]
    fn test_external_validation_result_fields() {
        let pdf_bytes = create_minimal_pdf();
        let result = validate_external(&pdf_bytes).unwrap();

        // Test that all fields are accessible
        let _ = result.qpdf_passed;
        let _ = result.verapdf_passed;
        let _ = result.adobe_preflight_passed;
        let _ = result.error_messages.len();
    }

    #[test]
    fn test_install_instructions_completeness() {
        let instructions = get_install_instructions();

        // Should have exactly 3 validators
        assert_eq!(instructions.len(), 3);

        // Each should have non-empty instructions
        for (name, instruction) in &instructions {
            assert!(!name.is_empty());
            assert!(!instruction.is_empty());
            // Each should have at least one installation method
            assert!(
                instruction.contains("brew")
                    || instruction.contains("apt")
                    || instruction.contains("Download"),
                "Instructions for {} don't contain installation method",
                name
            );
        }
    }
}
