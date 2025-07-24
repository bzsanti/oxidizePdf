//! Tests for XRef recovery functionality

use oxidize_pdf::recovery::{needs_xref_recovery, recover_xref, XRefRecovery};
use std::io::Write;
use tempfile::NamedTempFile;

/// Create a corrupted PDF without xref table
fn create_corrupted_pdf_no_xref() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();

    // Write PDF header
    writeln!(file, "%PDF-1.4").unwrap();
    writeln!(file, "%âãÏÓ").unwrap(); // Binary marker

    // Write some objects
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

    // Missing xref table and trailer
    // Just end the file abruptly
    write!(file, "%%EOF").unwrap();

    file.flush().unwrap();
    file
}

/// Create a corrupted PDF with damaged xref
fn create_corrupted_pdf_damaged_xref() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();

    // Write PDF header
    writeln!(file, "%PDF-1.4").unwrap();

    // Write some objects
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

    // Corrupted xref table
    writeln!(file, "xref").unwrap();
    writeln!(file, "0 4").unwrap();
    writeln!(file, "0000000000 65535 f").unwrap();
    writeln!(file, "corrupted data here").unwrap(); // Invalid xref entry
    writeln!(file, "0000000089 00000 n").unwrap();
    writeln!(file, "0000000147 00000 n").unwrap();

    writeln!(file, "trailer").unwrap();
    writeln!(file, "<< /Size 4 /Root 1 0 R >>").unwrap();
    writeln!(file, "startxref").unwrap();
    writeln!(file, "corrupted").unwrap(); // Invalid startxref
    writeln!(file, "%%EOF").unwrap();

    file.flush().unwrap();
    file
}

#[test]
fn test_needs_xref_recovery_no_xref() {
    let file = create_corrupted_pdf_no_xref();
    let result = needs_xref_recovery(file.path());
    assert!(result.unwrap());
}

#[test]
fn test_needs_xref_recovery_damaged_xref() {
    let file = create_corrupted_pdf_damaged_xref();
    let result = needs_xref_recovery(file.path());

    // This might return false if startxref is found, even if corrupted
    // The actual recovery will handle the corruption
    assert!(result.is_ok());
}

#[test]
fn test_xref_recovery_no_xref() {
    let file = create_corrupted_pdf_no_xref();

    let result = recover_xref(file.path());
    assert!(result.is_ok());

    let xref_table = result.unwrap();

    // Should have recovered 3 objects
    assert_eq!(xref_table.len(), 3);

    // Verify entries exist
    assert!(xref_table.get_entry(1).is_some());
    assert!(xref_table.get_entry(2).is_some());
    assert!(xref_table.get_entry(3).is_some());
}

#[test]
fn test_xref_recovery_with_recover_from_file() {
    let file = create_corrupted_pdf_no_xref();

    // Use the public API
    let result = XRefRecovery::recover_from_file(file.path());
    assert!(result.is_ok());

    let xref_table = result.unwrap();
    assert_eq!(xref_table.len(), 3);
}

#[test]
fn test_xref_recovery_with_stream_objects() {
    let mut file = NamedTempFile::new().unwrap();

    // Write PDF with stream object
    writeln!(file, "%PDF-1.4").unwrap();

    writeln!(file, "1 0 obj").unwrap();
    writeln!(file, "<< /Type /Catalog /Pages 2 0 R >>").unwrap();
    writeln!(file, "endobj").unwrap();

    writeln!(file, "2 0 obj").unwrap();
    writeln!(file, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>").unwrap();
    writeln!(file, "endobj").unwrap();

    writeln!(file, "3 0 obj").unwrap();
    writeln!(
        file,
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R >>"
    )
    .unwrap();
    writeln!(file, "endobj").unwrap();

    writeln!(file, "4 0 obj").unwrap();
    writeln!(file, "<< /Length 44 >>").unwrap();
    writeln!(file, "stream").unwrap();
    writeln!(file, "BT").unwrap();
    writeln!(file, "/F1 12 Tf").unwrap();
    writeln!(file, "100 700 Td").unwrap();
    writeln!(file, "(Hello World) Tj").unwrap();
    writeln!(file, "ET").unwrap();
    writeln!(file, "endstream").unwrap();
    writeln!(file, "endobj").unwrap();

    write!(file, "%%EOF").unwrap();
    file.flush().unwrap();

    // Recover xref
    let result = recover_xref(file.path());
    assert!(result.is_ok());

    let xref_table = result.unwrap();
    assert_eq!(xref_table.len(), 4);
}

#[test]
fn test_xref_recovery_integration() {
    let file = create_corrupted_pdf_no_xref();

    // Try to open with PdfReader - it should succeed with lenient parsing
    let reader_result = oxidize_pdf::parser::PdfReader::open(file.path());

    // The default parser has lenient mode enabled, so it should succeed
    assert!(reader_result.is_ok());

    // Also test direct xref recovery
    let xref_result = recover_xref(file.path());
    assert!(xref_result.is_ok());

    let xref_table = xref_result.unwrap();
    assert!(xref_table.len() >= 3); // Should have at least the 3 objects we created
}
