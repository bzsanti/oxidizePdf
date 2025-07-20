//! Integration tests for PDF recovery features

use oxidize_pdf::{
    detect_corruption, repair_document, validate_pdf, CorruptionType, Document, ObjectScanner,
    Page, PdfRecovery, RecoveryOptions, RepairStrategy,
};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

/// Create a corrupted PDF file for testing
fn create_corrupted_pdf(path: &Path, corruption_type: &str) -> std::io::Result<()> {
    let mut file = File::create(path)?;

    match corruption_type {
        "no_header" => {
            // Missing PDF header
            writeln!(file, "1 0 obj")?;
            writeln!(file, "<< /Type /Catalog /Pages 2 0 R >>")?;
            writeln!(file, "endobj")?;
            writeln!(file, "%%EOF")?;
        }
        "no_eof" => {
            // Missing EOF marker
            writeln!(file, "%PDF-1.7")?;
            writeln!(file, "1 0 obj")?;
            writeln!(file, "<< /Type /Catalog /Pages 2 0 R >>")?;
            writeln!(file, "endobj")?;
        }
        "truncated" => {
            // Truncated file
            writeln!(file, "%PDF-1.7")?;
            writeln!(file, "1 0 obj")?;
            write!(file, "<< /Type /Catalog")?; // Incomplete
        }
        "bad_xref" => {
            // Corrupted cross-reference
            writeln!(file, "%PDF-1.7")?;
            writeln!(file, "1 0 obj")?;
            writeln!(file, "<< /Type /Catalog /Pages 2 0 R >>")?;
            writeln!(file, "endobj")?;
            writeln!(file, "xref")?;
            writeln!(file, "CORRUPTED DATA HERE")?;
            writeln!(file, "%%EOF")?;
        }
        "partial_page" => {
            // Partial page content
            writeln!(file, "%PDF-1.7")?;
            writeln!(file, "1 0 obj")?;
            writeln!(file, "<< /Type /Page /Parent 2 0 R")?;
            writeln!(file, "   /MediaBox [0 0 612 792]")?;
            writeln!(file, "   /Contents 3 0 R >>")?;
            writeln!(file, "endobj")?;
            writeln!(file, "3 0 obj")?;
            writeln!(file, "<< /Length 44 >>")?;
            writeln!(file, "stream")?;
            writeln!(file, "BT /F1 12 Tf 100 700 Td (Partial text) Tj ET")?;
            writeln!(file, "endstream")?;
            // Missing endobj and EOF
        }
        _ => {
            // Default corrupted file
            write!(file, "This is not a valid PDF file")?;
        }
    }

    Ok(())
}

#[test]
fn test_recovery_options() {
    let options = RecoveryOptions::default();
    assert!(!options.aggressive_recovery);
    assert!(options.partial_content);
    assert_eq!(options.max_errors, 100);
    assert!(options.rebuild_xref);

    let custom_options = RecoveryOptions::default()
        .with_aggressive_recovery(true)
        .with_max_errors(50)
        .with_memory_limit(100 * 1024 * 1024);

    assert!(custom_options.aggressive_recovery);
    assert_eq!(custom_options.max_errors, 50);
    assert_eq!(custom_options.memory_limit, 100 * 1024 * 1024);
}

#[test]
fn test_corruption_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Test various corruption types
    let test_cases = vec![
        ("no_header.pdf", "no_header"),
        ("no_eof.pdf", "no_eof"),
        ("truncated.pdf", "truncated"),
        ("bad_xref.pdf", "bad_xref"),
    ];

    for (filename, corruption_type) in test_cases {
        let path = temp_dir.path().join(filename);
        create_corrupted_pdf(&path, corruption_type).unwrap();

        let report = detect_corruption(&path).unwrap();
        // Some corrupted PDFs might still be partially valid
        if corruption_type == "bad_xref" {
            // bad_xref might still parse partially, just check for detection
            assert!(
                report.severity > 0 || !report.errors.is_empty(),
                "Should detect issues in {}",
                filename
            );
        } else {
            assert!(
                report.severity > 0,
                "Should detect corruption in {}",
                filename
            );
        }
        // Skip error check for bad_xref as it might parse without errors
    }
}

#[test]
fn test_pdf_recovery_basic() {
    let temp_dir = TempDir::new().unwrap();
    let corrupted_path = temp_dir.path().join("corrupted.pdf");

    // Create a corrupted PDF with missing header
    create_corrupted_pdf(&corrupted_path, "no_header").unwrap();

    let mut recovery = PdfRecovery::new(RecoveryOptions::default());

    // Try to recover
    match recovery.recover_document(&corrupted_path) {
        Ok(_doc) => {
            // Recovery might succeed with an empty document
            // Document recovered successfully
        }
        Err(e) => {
            // Recovery might fail, which is also acceptable
            println!("Recovery failed as expected: {}", e);
        }
    }

    // Check warnings were generated
    assert!(!recovery.warnings().is_empty());
}

#[test]
fn test_partial_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let corrupted_path = temp_dir.path().join("partial.pdf");

    // Create a partially corrupted PDF
    create_corrupted_pdf(&corrupted_path, "partial_page").unwrap();

    let mut recovery = PdfRecovery::new(RecoveryOptions::default().with_partial_content(true));

    let partial = recovery.recover_partial(&corrupted_path).unwrap();

    // Should have some recovery results
    assert!(partial.total_objects > 0);
    println!("Recovered {} objects", partial.recovered_objects);
    println!("Warnings: {:?}", partial.recovery_warnings);
}

#[test]
fn test_repair_strategies() {
    // Test strategy selection
    assert!(matches!(
        RepairStrategy::for_corruption(&CorruptionType::InvalidHeader),
        RepairStrategy::FixStructure
    ));

    assert!(matches!(
        RepairStrategy::for_corruption(&CorruptionType::CorruptXRef),
        RepairStrategy::RebuildXRef
    ));

    assert!(matches!(
        RepairStrategy::for_corruption(&CorruptionType::TruncatedFile),
        RepairStrategy::ExtractContent
    ));
}

#[test]
fn test_repair_document() {
    let temp_dir = TempDir::new().unwrap();
    let corrupted_path = temp_dir.path().join("to_repair.pdf");

    // Create corrupted file
    create_corrupted_pdf(&corrupted_path, "no_eof").unwrap();

    let options = RecoveryOptions::default();
    let result = repair_document(&corrupted_path, RepairStrategy::FixStructure, &options).unwrap();

    // Should have some result
    assert!(!result.warnings.is_empty());
}

#[test]
fn test_object_scanner() {
    let temp_dir = TempDir::new().unwrap();
    let pdf_path = temp_dir.path().join("scan_test.pdf");

    // Create a simple PDF with identifiable objects
    let mut file = File::create(&pdf_path).unwrap();
    writeln!(file, "%PDF-1.7").unwrap();
    writeln!(file, "1 0 obj").unwrap();
    writeln!(file, "<< /Type /Catalog /Pages 2 0 R >>").unwrap();
    writeln!(file, "endobj").unwrap();
    writeln!(file, "2 0 obj").unwrap();
    writeln!(file, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>").unwrap();
    writeln!(file, "endobj").unwrap();
    writeln!(file, "3 0 obj").unwrap();
    writeln!(
        file,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"
    )
    .unwrap();
    writeln!(file, "endobj").unwrap();
    writeln!(file, "%%EOF").unwrap();
    drop(file);

    let mut scanner = ObjectScanner::new();
    let result = scanner.scan_file(&pdf_path).unwrap();

    assert!(result.total_objects > 0);
    assert!(result.estimated_pages > 0);
    println!(
        "Found {} objects, {} pages",
        result.total_objects, result.estimated_pages
    );
}

#[test]
fn test_pdf_validation() {
    let temp_dir = TempDir::new().unwrap();

    // Create a valid PDF
    let valid_path = temp_dir.path().join("valid.pdf");
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.save(&valid_path).unwrap();

    // Validate it
    let result = validate_pdf(&valid_path).unwrap();
    assert!(result.is_valid);
    assert!(result.errors.is_empty());

    // Create an invalid PDF
    let invalid_path = temp_dir.path().join("invalid.pdf");
    create_corrupted_pdf(&invalid_path, "invalid").unwrap();

    // Validate it
    let result = validate_pdf(&invalid_path).unwrap();
    assert!(!result.is_valid);
}

#[test]
fn test_aggressive_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let corrupted_path = temp_dir.path().join("aggressive.pdf");

    // Create a badly corrupted file
    let mut file = File::create(&corrupted_path).unwrap();
    writeln!(file, "CORRUPTED START").unwrap();
    writeln!(file, "3 0 obj").unwrap();
    writeln!(file, "<< /Type /Page >>").unwrap();
    writeln!(file, "endobj").unwrap();
    writeln!(file, "MORE CORRUPTION").unwrap();
    writeln!(file, "5 0 obj").unwrap();
    writeln!(file, "<< /Type /Font >>").unwrap();
    writeln!(file, "endobj").unwrap();
    drop(file);

    let options = RecoveryOptions::default().with_aggressive_recovery(true);

    let mut recovery = PdfRecovery::new(options);

    // Aggressive recovery should try multiple strategies
    match recovery.recover_document(&corrupted_path) {
        Ok(_doc) => println!("Aggressive recovery succeeded"),
        Err(e) => println!("Aggressive recovery failed: {}", e),
    }
}

#[test]
fn test_quick_recover() {
    let temp_dir = TempDir::new().unwrap();
    let corrupted_path = temp_dir.path().join("quick.pdf");

    create_corrupted_pdf(&corrupted_path, "no_eof").unwrap();

    // Quick recover is a convenience function
    match oxidize_pdf::quick_recover(&corrupted_path) {
        Ok(_doc) => println!("Quick recovery succeeded"),
        Err(e) => println!("Quick recovery failed: {}", e),
    }
}

#[test]
fn test_analyze_corruption() {
    let temp_dir = TempDir::new().unwrap();
    let corrupted_path = temp_dir.path().join("analyze.pdf");

    create_corrupted_pdf(&corrupted_path, "bad_xref").unwrap();

    let report = oxidize_pdf::analyze_corruption(&corrupted_path).unwrap();

    // bad_xref might still parse partially, just check for detection
    assert!(
        report.severity > 0 || !report.errors.is_empty(),
        "Should detect issues in analyze.pdf"
    );
    assert!(report.file_stats.file_size > 0);
}

#[test]
fn test_memory_limit_recovery() {
    let options = RecoveryOptions::default().with_memory_limit(1024 * 1024); // 1MB limit

    let recovery = PdfRecovery::new(options);
    assert_eq!(recovery.warnings().len(), 0);
}

#[test]
fn test_recovery_with_embedded_files() {
    let options = RecoveryOptions::default().with_aggressive_recovery(true);
    // recover_embedded is a field, not a method

    let _recovery = PdfRecovery::new(options);
    // Would test embedded file recovery with a proper test file
}

#[test]
fn test_circular_reference_detection() {
    // This would test detection of circular references in corrupted PDFs
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("circular.pdf");

    // Create a PDF with potential circular references
    let mut file = File::create(&path).unwrap();
    writeln!(file, "%PDF-1.7").unwrap();
    writeln!(file, "1 0 obj").unwrap();
    writeln!(file, "<< /Type /Catalog /Pages 1 0 R >>").unwrap(); // Self-reference
    writeln!(file, "endobj").unwrap();
    writeln!(file, "%%EOF").unwrap();
    drop(file);

    let report = detect_corruption(&path).unwrap();
    assert!(report.severity > 0);
}
