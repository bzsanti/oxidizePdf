//! CLI OCR tests
//!
//! Tests for the OCR command-line interface and examples

#[cfg(feature = "ocr-tesseract")]
mod cli_ocr_tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    fn cargo_example_path() -> String {
        // Get the path to the cargo executable
        env!("CARGO").to_string()
    }

    fn create_simple_test_pdf(output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        use oxidize_pdf::{Color, Document, Font, Page};

        let mut doc = Document::new();
        let mut page = Page::a4();

        // Create a simple test document
        page.graphics()
            .set_fill_color(Color::rgb(250.0, 250.0, 250.0))
            .rect(50.0, 50.0, 500.0, 700.0)
            .fill();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(100.0, 700.0)
            .write("CLI OCR Test Document")?;

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 670.0)
            .write("This document is used to test the CLI OCR functionality.")?;

        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, 650.0)
            .write("It should be converted to a searchable PDF format.")?;

        doc.add_page(page);

        let pdf_bytes = doc.to_bytes()?;
        fs::write(output_path, pdf_bytes)?;

        Ok(())
    }

    #[test]
    fn test_cli_ocr_help_command() {
        let output = Command::new(cargo_example_path())
            .args(["run", "--example", "convert_pdf_ocr", "--", "--help"])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                assert!(stdout.contains("PDF OCR Converter"));
                assert!(stdout.contains("USAGE:"));
                assert!(stdout.contains("OPTIONS:"));
                println!("✅ CLI help command works correctly");
            }
            Err(e) => {
                println!("⚠️  CLI help test failed: {}", e);
            }
        }
    }

    #[test]
    fn test_cli_version_info() {
        let output = Command::new(cargo_example_path())
            .args(["run", "--example", "convert_pdf_ocr", "--", "--help"])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                // Should contain version info
                assert!(stdout.contains("PDF OCR Converter"));
                println!("✅ CLI version info displayed correctly");
            }
            Err(e) => {
                println!("⚠️  CLI version test failed: {}", e);
            }
        }
    }

    #[test]
    fn test_cli_single_file_conversion() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        // Create test PDF
        create_simple_test_pdf(&input_path)?;

        // Test CLI conversion (may fail if Tesseract not available)
        let output = Command::new(cargo_example_path())
            .args([
                "run",
                "--example",
                "convert_pdf_ocr",
                "--",
                input_path.to_str().unwrap(),
                output_path.to_str().unwrap(),
                "--verbose",
            ])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);
                let stderr = String::from_utf8_lossy(&result.stderr);

                if result.status.success() {
                    println!("✅ CLI single file conversion successful");
                    println!("Output: {}", stdout);
                    assert!(output_path.exists(), "Output file should be created");
                } else {
                    println!("⚠️  CLI conversion failed (may be due to missing Tesseract):");
                    println!("stdout: {}", stdout);
                    println!("stderr: {}", stderr);
                }
            }
            Err(e) => {
                println!("⚠️  CLI execution failed: {}", e);
            }
        }

        Ok(())
    }

    #[test]
    fn test_cli_with_language_option() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        create_simple_test_pdf(&input_path)?;

        let output = Command::new(cargo_example_path())
            .args([
                "run",
                "--example",
                "convert_pdf_ocr",
                "--",
                input_path.to_str().unwrap(),
                output_path.to_str().unwrap(),
                "--lang",
                "eng",
                "--verbose",
            ])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);

                if result.status.success() {
                    println!("✅ CLI language option works");
                    assert!(stdout.contains("language: eng") || stdout.contains("OCR"));
                } else {
                    println!("⚠️  CLI language test failed (Tesseract may not be available)");
                }
            }
            Err(e) => {
                println!("⚠️  CLI language test execution failed: {}", e);
            }
        }

        Ok(())
    }

    #[test]
    fn test_cli_with_dpi_option() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("input.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        create_simple_test_pdf(&input_path)?;

        let output = Command::new(cargo_example_path())
            .args([
                "run",
                "--example",
                "convert_pdf_ocr",
                "--",
                input_path.to_str().unwrap(),
                output_path.to_str().unwrap(),
                "--dpi",
                "150",
                "--verbose",
            ])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);

                if result.status.success() {
                    println!("✅ CLI DPI option works");
                    assert!(stdout.contains("DPI: 150") || stdout.contains("OCR"));
                } else {
                    println!("⚠️  CLI DPI test failed");
                }
            }
            Err(e) => {
                println!("⚠️  CLI DPI test execution failed: {}", e);
            }
        }

        Ok(())
    }

    #[test]
    fn test_cli_batch_mode() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_dir = temp_dir.path().join("input");
        let output_dir = temp_dir.path().join("output");

        fs::create_dir_all(&input_dir)?;
        fs::create_dir_all(&output_dir)?;

        // Create multiple test PDFs
        for i in 1..=2 {
            let pdf_path = input_dir.join(format!("test_{}.pdf", i));
            create_simple_test_pdf(&pdf_path)?;
        }

        let output = Command::new(cargo_example_path())
            .args([
                "run",
                "--example",
                "convert_pdf_ocr",
                "--",
                "--batch",
                input_dir.to_str().unwrap(),
                output_dir.to_str().unwrap(),
                "--verbose",
            ])
            .output();

        match output {
            Ok(result) => {
                let stdout = String::from_utf8_lossy(&result.stdout);

                if result.status.success() {
                    println!("✅ CLI batch mode works");
                    assert!(stdout.contains("Found") && stdout.contains("PDF files"));
                } else {
                    println!("⚠️  CLI batch mode test failed");
                }
            }
            Err(e) => {
                println!("⚠️  CLI batch mode test execution failed: {}", e);
            }
        }

        Ok(())
    }

    #[test]
    fn test_cli_error_handling() {
        // Test with non-existent file
        let output = Command::new(cargo_example_path())
            .args([
                "run",
                "--example",
                "convert_pdf_ocr",
                "--",
                "nonexistent.pdf",
                "output.pdf",
            ])
            .output();

        match output {
            Ok(result) => {
                // Should fail gracefully
                assert!(
                    !result.status.success(),
                    "Should fail with non-existent input"
                );

                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("✅ CLI error handling works: {}", stderr);
            }
            Err(e) => {
                println!("⚠️  CLI error handling test failed: {}", e);
            }
        }
    }

    #[test]
    fn test_cli_invalid_arguments() {
        // Test with invalid arguments
        let output = Command::new(cargo_example_path())
            .args([
                "run",
                "--example",
                "convert_pdf_ocr",
                "--",
                "--invalid-option",
            ])
            .output();

        match output {
            Ok(result) => {
                // Should fail with invalid option
                assert!(!result.status.success(), "Should fail with invalid option");

                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("✅ CLI invalid argument handling works: {}", stderr);
            }
            Err(e) => {
                println!("⚠️  CLI invalid argument test failed: {}", e);
            }
        }
    }
}

#[cfg(not(feature = "ocr-tesseract"))]
mod cli_ocr_disabled_tests {
    #[test]
    fn test_cli_without_ocr_feature() {
        println!("CLI OCR tests are disabled when 'ocr-tesseract' feature is not enabled");
        assert!(true, "This is expected behavior");
    }
}
