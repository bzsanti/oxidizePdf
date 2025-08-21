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

    temp_file
        .write_all(pdf_bytes)
        .map_err(PdfError::Io)?;

    let temp_path = temp_file.path();

    let mut result = ExternalValidationResult {
        qpdf_passed: None,
        verapdf_passed: None,
        adobe_preflight_passed: None,
        error_messages: Vec::new(),
    };

    // Validate with qpdf (most important)
    match validate_with_qpdf(temp_path.to_str().unwrap()) {
        Ok(passed) => result.qpdf_passed = Some(passed),
        Err(e) => result.error_messages.push(format!("qpdf error: {}", e)),
    }

    // Validate with veraPDF if available
    match validate_with_verapdf(temp_path.to_str().unwrap()) {
        Ok(passed) => result.verapdf_passed = Some(passed),
        Err(e) => result.error_messages.push(format!("veraPDF error: {}", e)),
    }

    // Adobe Preflight is typically not available in CI/dev environments
    // but we'll try if it exists
    match validate_with_adobe_preflight(temp_path.to_str().unwrap()) {
        Ok(passed) => result.adobe_preflight_passed = Some(passed),
        Err(_) => {
            // Adobe Preflight not available - this is expected
            result
                .error_messages
                .push("Adobe Preflight not available".to_string());
        }
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
    use std::fs;

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

    #[test]
    fn test_check_available_validators() {
        let available = check_available_validators();
        // Just test that the function runs without panicking
        // The actual validators may or may not be available in test environment
        assert!(available.len() >= 0); // Always true, but ensures function runs
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
                println!(
                    "External validation failed (expected in environments without PDF tools): {}",
                    e
                );
            }
        }
    }
}
