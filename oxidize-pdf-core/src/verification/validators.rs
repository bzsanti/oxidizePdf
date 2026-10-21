//! External PDF Validators
//!
//! This module integrates with external PDF validation tools to provide
//! distinct structural (qpdf) and PDF/A-1b (veraPDF) checks. Passing these checks
//! is not a certificate of full ISO 32000 conformance.

use crate::error::{PdfError, Result};
use crate::verification::ExternalValidationResult;
use std::io::Write;
use std::process::Command;
#[cfg(test)]
use tempfile::NamedTempFile;

/// Validate PDF with external tools
pub fn validate_external(pdf_bytes: &[u8]) -> Result<ExternalValidationResult> {
    // Create temporary file for validation
    let mut temp_file = tempfile::Builder::new()
        .suffix(".pdf")
        .tempfile()
        .map_err(PdfError::Io)?;

    temp_file.write_all(pdf_bytes).map_err(PdfError::Io)?;

    // Close the file before launching tools (including Windows file sharing).
    let temp_path = temp_file.into_temp_path();

    let mut result = ExternalValidationResult {
        qpdf_passed: None,
        verapdf_passed: None,
        adobe_preflight_passed: None,
        error_messages: Vec::new(),
    };

    // Validate with qpdf (most important)
    if let Some(path_str) = temp_path.to_str() {
        match validate_with_qpdf(path_str) {
            Ok(passed) => {
                result.qpdf_passed = Some(passed);
                if !passed {
                    result.error_messages.push(
                        "qpdf structural check did not pass (input errors or warnings)".into(),
                    );
                }
            }
            Err(e) => result.error_messages.push(format!("qpdf error: {}", e)),
        }

        // Validate with veraPDF if available
        match validate_with_verapdf(path_str) {
            Ok(passed) => {
                result.verapdf_passed = Some(passed);
                if !passed {
                    result
                        .error_messages
                        .push("veraPDF PDF/A-1b validation: noncompliant".into());
                }
            }
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

/// Check PDF structure with qpdf, not PDF/A or full ISO conformance.
/// Returns true only for exit 0; errors/warnings (2/3) return false. Missing
/// tools, inaccessible input, abnormal exits and execution failures return Err
/// with distinct diagnostics. A false result does not identify the cause of a
/// qpdf input-processing error; consult qpdf directly for detailed diagnostics.
pub fn validate_with_qpdf(pdf_path: &str) -> Result<bool> {
    run_qpdf(std::ffi::OsStr::new("qpdf"), std::path::Path::new(pdf_path))
}

fn input_path(path: &std::path::Path) -> Result<std::path::PathBuf> {
    // Absolute paths also prevent leading '-' or '@' from being interpreted as
    // CLI options/response files. No shell interpolation is used.
    let path = path
        .canonicalize()
        .map_err(|e| PdfError::ExternalValidationError(format!("validator input error: {e}")))?;
    let file = std::fs::File::open(&path)
        .map_err(|e| PdfError::ExternalValidationError(format!("validator input error: {e}")))?;
    if !file.metadata().map_err(PdfError::Io)?.is_file() {
        return Err(PdfError::ExternalValidationError(
            "validator input error: not a regular file".into(),
        ));
    }
    Ok(path)
}

fn execute(command: &mut Command, tool: &str) -> Result<std::process::Output> {
    command.output().map_err(|e| {
        let kind = if e.kind() == std::io::ErrorKind::NotFound {
            "unavailable (not found)"
        } else {
            "execution error"
        };
        PdfError::ExternalValidationError(format!("{tool} {kind}: {e}"))
    })
}

fn execution_error(tool: &str, output: &std::process::Output) -> PdfError {
    PdfError::ExternalValidationError(format!(
        "{tool} execution error ({}): {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn run_qpdf(executable: &std::ffi::OsStr, path: &std::path::Path) -> Result<bool> {
    let path = input_path(path)?;
    let output = execute(Command::new(executable).arg("--check").arg(path), "qpdf")?;
    match output.status.code() {
        Some(0) => Ok(true),
        // qpdf uses 2 for input processing errors and 3 for warnings. Neither
        // establishes a clean structural check. Other statuses are execution errors.
        Some(2 | 3) => Ok(false),
        _ => Err(execution_error("qpdf", &output)),
    }
}

/// Check PDF/A-1b with veraPDF 1.24+ XML reports.
/// Returns true/false for a completed, coherent compliance report. Unavailable
/// tools, execution failures and malformed/incomplete reports return Err with
/// distinct diagnostics. This is not a full ISO 32000 conformance certificate.
pub fn validate_with_verapdf(pdf_path: &str) -> Result<bool> {
    run_verapdf(
        std::ffi::OsStr::new("verapdf"),
        std::path::Path::new(pdf_path),
    )
}

fn run_verapdf(executable: &std::ffi::OsStr, path: &std::path::Path) -> Result<bool> {
    let path = input_path(path)?;
    let output = execute(
        Command::new(executable)
            .args(["--format", "xml", "--flavour", "1b", "--nonpdfext"])
            .arg(path),
        "veraPDF",
    )?;
    // veraPDF CLI: 0 = conforming, 1 = nonconforming; other codes are
    // execution/processing failures (CliConstants.ExitCodes, veraPDF 1.30.3).
    let exit_compliant = match output.status.code() {
        Some(0) => true,
        Some(1) => false,
        _ => return Err(execution_error("veraPDF", &output)),
    };
    let report_compliant = super::verapdf_report::parse_report(&output.stdout)?;
    if report_compliant != exit_compliant {
        return Err(PdfError::ExternalValidationError(
            "veraPDF execution error: exit status and report disagree".into(),
        ));
    }
    Ok(report_compliant)
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
    if Command::new("qpdf")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        available.push("qpdf".to_string());
    }

    // Check veraPDF
    if Command::new("verapdf")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
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
        include_bytes!("../../tests/fixtures/issue_729/non-pdfa.pdf").to_vec()
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
                    err_str.contains("unavailable (not found)"),
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
            Ok(passed) => assert!(!passed),
            Err(e) => {
                // Expected: qpdf rejects invalid PDF or qpdf not installed
                let err_str = e.to_string();
                assert!(
                    err_str.contains("unavailable (not found)"),
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
            Ok(passed) => assert!(!passed),
            Err(e) => {
                let err_str = e.to_string();
                assert!(
                    err_str.contains("unavailable (not found)"),
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
            err_str.contains("not found") || err_str.contains("input error"),
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
                // Structurally valid fixture lacks PDF/A metadata.
                assert!(!passed);
            }
            Err(e) => {
                // Expected: veraPDF not installed
                let err_str = e.to_string();
                assert!(
                    err_str.contains("unavailable (not found)"),
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

#[cfg(test)]
mod process_contracts {
    use super::*;

    fn input() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(include_bytes!(
            "../../tests/fixtures/issue_729/non-pdfa.pdf"
        ))
        .unwrap();
        file
    }

    #[test]
    fn missing_tool_and_missing_input_are_distinct_errors() {
        let file = input();
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("absent-validator");
        let error = run_verapdf(missing.as_os_str(), file.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains("unavailable (not found)"), "{error}");
        let error = run_qpdf(missing.as_os_str(), file.path())
            .unwrap_err()
            .to_string();
        assert!(error.contains("unavailable (not found)"), "{error}");
        let error = run_qpdf(missing.as_os_str(), &dir.path().join("absent.pdf"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("input error"), "{error}");
        assert!(!error.contains("unavailable"));
    }

    #[cfg(unix)]
    fn script(dir: &std::path::Path, content: &str) -> tempfile::TempPath {
        use std::os::unix::fs::PermissionsExt;
        // Each case owns a distinct executable inode. Rewriting a previous
        // executable can race outstanding handles and cause ETXTBSY on Linux.
        let mut file = NamedTempFile::new_in(dir).unwrap();
        writeln!(file, "#!/bin/sh\n{content}").unwrap();
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o700))
            .unwrap();
        // Close the writable handle before exec; retain automatic path cleanup.
        file.into_temp_path()
    }

    #[test]
    #[cfg(unix)]
    fn validator_fixture_remains_executable_while_previous_case_has_a_writer() {
        let file = input();
        let dir = tempfile::tempdir().unwrap();
        let first = script(dir.path(), "exit 2");
        let previous_writer = std::fs::OpenOptions::new()
            .write(true)
            .open(&first)
            .unwrap();
        let next = script(dir.path(), "exit 0");
        assert!(run_qpdf(next.as_os_str(), file.path()).unwrap());
        assert_eq!(
            std::fs::read_to_string(&first).unwrap(),
            "#!/bin/sh\nexit 2\n"
        );
        drop(previous_writer);
        assert!(!run_qpdf(first.as_os_str(), file.path()).unwrap());
    }

    #[test]
    #[cfg(unix)]
    fn verapdf_checks_arguments_exit_and_report_separately() {
        let file = input();
        let dir = tempfile::tempdir().unwrap();
        let xml = include_str!("../../tests/fixtures/issue_729/valid.xml");
        let invalid = include_str!("../../tests/fixtures/issue_729/invalid.xml");
        for (status, report, expected) in [
            (0, xml, Some(true)),
            (1, invalid, Some(false)),
            (1, xml, None),
            (7, xml, None),
            (0, "ValidationProfile", None),
        ] {
            let script = script(dir.path(), &format!(
                "[ \"$#\" -eq 6 ] && [ \"$1\" = --format ] && [ \"$2\" = xml ] && [ \"$3\" = --flavour ] && [ \"$4\" = 1b ] && [ \"$5\" = --nonpdfext ] || exit 9\ncat <<'REPORT'\n{report}\nREPORT\nexit {status}"
            ));
            let result = run_verapdf(script.as_os_str(), file.path());
            match expected {
                Some(passed) => assert_eq!(result.unwrap(), passed),
                None => {
                    let message = result.unwrap_err().to_string();
                    assert!(
                        message.contains(if status == 0 {
                            "report error"
                        } else {
                            "execution error"
                        }),
                        "{message}"
                    );
                }
            }
        }
    }

    #[test]
    #[cfg(unix)]
    fn qpdf_statuses_and_arguments_are_structural_only() {
        let file = input();
        let dir = tempfile::tempdir().unwrap();
        for (status, expected) in [
            (0, Some(true)),
            (2, Some(false)),
            (3, Some(false)),
            (9, None),
        ] {
            let script = script(dir.path(), &format!(
                "[ \"$#\" -eq 2 ] && [ \"$1\" = --check ] && [ \"${{2#/}}\" != \"$2\" ] || exit 9\nexit {status}"
            ));
            let result = run_qpdf(script.as_os_str(), file.path());
            if let Some(passed) = expected {
                assert_eq!(result.unwrap(), passed);
            } else {
                assert!(result.unwrap_err().to_string().contains("execution error"));
            }
        }
    }

    #[test]
    #[cfg(unix)]
    fn nonexecutable_tool_is_execution_error_not_unavailable() {
        let file = input();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-executable");
        std::fs::write(&path, "not executable").unwrap();
        let message = run_verapdf(path.as_os_str(), file.path())
            .unwrap_err()
            .to_string();
        assert!(message.contains("execution error"), "{message}");
        assert!(!message.contains("unavailable"));
    }
}
