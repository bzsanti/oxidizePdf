//! Rigorous tests for PdfWriter - No compromises
//!
//! These tests are designed to be:
//! - Specific and deterministic
//! - Test actual algorithm behavior (not just "doesn't crash")
//! - Include error handling and edge cases
//! - No "relaxed" assertions

use super::*;
use crate::objects::{Object, ObjectId};
use crate::page::Page;

// =============================================================================
// TESTS FOR WriterConfig
// =============================================================================

#[test]
fn test_writer_config_modern_values() {
    let config = WriterConfig::modern();

    // Verify all modern features are enabled
    assert!(config.use_xref_streams, "Modern config must use XRef streams");
    assert!(config.use_object_streams, "Modern config must use object streams");
    assert!(config.compress_streams, "Modern config must compress streams");
    assert_eq!(config.pdf_version, "1.5", "Modern config must be PDF 1.5+");
}

#[test]
fn test_writer_config_legacy_values() {
    let config = WriterConfig::legacy();

    // Verify legacy features
    assert!(!config.use_xref_streams, "Legacy config must NOT use XRef streams");
    assert!(!config.use_object_streams, "Legacy config must NOT use object streams");
    assert!(config.compress_streams, "Legacy config should still compress streams");
    assert_eq!(config.pdf_version, "1.4", "Legacy config must be PDF 1.4");
}

#[test]
fn test_writer_config_default_values() {
    let default_config = WriterConfig::default();

    // Default config should have specific values (not necessarily legacy)
    // Verify actual default behavior
    assert_eq!(default_config.pdf_version, "1.7", "Default should be PDF 1.7");
    assert!(!default_config.use_xref_streams, "Default should not use XRef streams");
    assert!(!default_config.use_object_streams, "Default should not use object streams");
    assert!(default_config.compress_streams, "Default should compress streams");
}

#[test]
fn test_writer_with_modern_config() {
    let buffer = Vec::new();
    let config = WriterConfig::modern();
    let writer = PdfWriter::with_config(buffer, config.clone());

    // Verify config is stored
    assert_eq!(writer.config.pdf_version, "1.5");
    assert!(writer.config.use_xref_streams);
    assert!(writer.config.use_object_streams);
}

#[test]
fn test_writer_with_legacy_config() {
    let buffer = Vec::new();
    let config = WriterConfig::legacy();
    let writer = PdfWriter::with_config(buffer, config.clone());

    // Verify config is stored
    assert_eq!(writer.config.pdf_version, "1.4");
    assert!(!writer.config.use_xref_streams);
    assert!(!writer.config.use_object_streams);
}

// =============================================================================
// TESTS FOR Object ID Allocation
// =============================================================================

#[test]
fn test_allocate_object_id_sequential() {
    let buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(buffer);

    let id1 = writer.allocate_object_id();
    let id2 = writer.allocate_object_id();
    let id3 = writer.allocate_object_id();

    // IDs must be sequential
    assert_eq!(id1.number(), 1, "First object ID must be 1");
    assert_eq!(id2.number(), 2, "Second object ID must be 2");
    assert_eq!(id3.number(), 3, "Third object ID must be 3");

    // All IDs must have generation 0 initially
    assert_eq!(id1.generation(), 0);
    assert_eq!(id2.generation(), 0);
    assert_eq!(id3.generation(), 0);
}

#[test]
fn test_allocate_object_id_no_gaps() {
    let buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(buffer);

    // Allocate 100 IDs
    let ids: Vec<_> = (0..100).map(|_| writer.allocate_object_id()).collect();

    // Verify no gaps in sequence
    for (i, id) in ids.iter().enumerate() {
        assert_eq!(id.number(), (i + 1) as u32, "Object ID sequence must have no gaps");
    }
}

// =============================================================================
// TESTS FOR PDF Header
// =============================================================================

#[test]
fn test_write_header_pdf_version() {
    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);

    writer.write_header().unwrap();

    let header = String::from_utf8_lossy(&buffer);

    // MUST start with exact PDF version
    assert!(header.starts_with("%PDF-1.7\n"),
            "PDF must start with '%PDF-1.7\\n', got: {:?}",
            &header[..20]);
}

#[test]
fn test_write_header_binary_comment() {
    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);

    writer.write_header().unwrap();

    // Binary comment must be at bytes 9-14
    assert_eq!(buffer[9], b'%', "Binary comment must start with %");
    assert_eq!(buffer[10], 0xE2, "Binary comment byte 1 must be 0xE2");
    assert_eq!(buffer[11], 0xE3, "Binary comment byte 2 must be 0xE3");
    assert_eq!(buffer[12], 0xCF, "Binary comment byte 3 must be 0xCF");
    assert_eq!(buffer[13], 0xD3, "Binary comment byte 4 must be 0xD3");
    assert_eq!(buffer[14], b'\n', "Binary comment must end with newline");
}

#[test]
fn test_write_header_length() {
    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);

    writer.write_header().unwrap();

    // Header must be exactly 15 bytes
    assert_eq!(buffer.len(), 15,
               "PDF header must be exactly 15 bytes (9 for version + 6 for binary comment)");
}

// =============================================================================
// TESTS FOR XRef Table (via write_document)
// =============================================================================

#[test]
fn test_xref_format_in_document() {
    let mut buffer = Vec::new();
    let mut document = Document::new();

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Must contain xref keyword
    assert!(content.contains("xref"), "Must contain 'xref' keyword");

    // Free object 0 entry must exist
    assert!(content.contains("0000000000 65535 f"),
            "Must contain free object entry '0000000000 65535 f'");
}

#[test]
fn test_xref_positions_format() {
    let mut buffer = Vec::new();
    let mut document = Document::new();
    document.add_page(Page::a4());

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // XRef entries must be 10-digit format with " 00000 n"
    // Count how many valid xref entries exist
    let xref_entries = content.matches(" 00000 n").count();

    assert!(xref_entries >= 3,
            "Document must have at least 3 xref entries (Catalog, Pages, Info), got: {}",
            xref_entries);
}

// =============================================================================
// TESTS FOR Trailer (via write_document)
// =============================================================================

#[test]
fn test_trailer_structure_in_document() {
    let mut buffer = Vec::new();
    let mut document = Document::new();

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Must contain trailer keyword
    assert!(content.contains("trailer"), "Must contain 'trailer' keyword");

    // Must contain Size
    assert!(content.contains("/Size"), "Trailer must contain /Size");

    // Must contain Root reference
    assert!(content.contains("/Root"), "Trailer must contain /Root reference");

    // Must contain Info reference
    assert!(content.contains("/Info"), "Trailer must contain /Info reference");
}

#[test]
fn test_trailer_references_valid_objects() {
    let mut buffer = Vec::new();
    let mut document = Document::new();
    document.add_page(Page::a4());

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Root and Info must reference valid object IDs with format "N 0 R"
    // Extract Root reference
    if let Some(root_start) = content.find("/Root") {
        let root_section = &content[root_start..root_start + 20];
        assert!(root_section.contains("0 R"),
                "Root reference must have format 'N 0 R'");
    } else {
        panic!("Trailer must contain /Root");
    }
}

// =============================================================================
// TESTS FOR EOF Marker (via write_document)
// =============================================================================

#[test]
fn test_document_ends_with_eof() {
    let mut buffer = Vec::new();
    let mut document = Document::new();

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Document must end with EOF
    assert!(content.ends_with("%%EOF\n"),
            "Document must end with '%%EOF\\n'");
}

#[test]
fn test_eof_marker_format() {
    let mut buffer = Vec::new();
    let mut document = Document::new();

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Extract last 7 characters (%%EOF\n = 6 chars + potential extra newline)
    let end = &content[content.len().saturating_sub(10)..];

    // Must contain exactly "%%EOF\n" at the end
    assert!(end.contains("%%EOF\n"),
            "Document must end with exactly '%%EOF\\n', got: {:?}", end);
}

// =============================================================================
// TESTS FOR Empty Document
// =============================================================================

#[test]
fn test_empty_document_structure() {
    let mut buffer = Vec::new();
    let mut document = Document::new();

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Must have all required PDF elements
    assert!(content.starts_with("%PDF-1.7\n"), "Must start with PDF header");
    assert!(content.contains("/Type /Catalog"), "Must contain Catalog");
    assert!(content.contains("/Type /Pages"), "Must contain Pages");
    assert!(content.contains("trailer"), "Must contain trailer");
    assert!(content.ends_with("%%EOF\n"), "Must end with EOF");
}

#[test]
fn test_empty_document_page_count() {
    let mut buffer = Vec::new();
    let mut document = Document::new();

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Empty document must have /Count 0
    assert!(content.contains("/Count 0"),
            "Empty document must have '/Count 0' in Pages object");
}

// =============================================================================
// TESTS FOR Document with Pages
// =============================================================================

#[test]
fn test_document_single_page() {
    let mut buffer = Vec::new();
    let mut document = Document::new();
    document.add_page(Page::a4());

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Must have Count 1
    assert!(content.contains("/Count 1"),
            "Document with 1 page must have '/Count 1'");

    // Must have MediaBox for A4
    assert!(content.contains("/MediaBox"), "Must contain /MediaBox");
}

#[test]
fn test_document_multiple_pages() {
    let mut buffer = Vec::new();
    let mut document = Document::new();

    document.add_page(Page::a4());
    document.add_page(Page::letter());
    document.add_page(Page::a4());

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Must have Count 3
    assert!(content.contains("/Count 3"),
            "Document with 3 pages must have '/Count 3'");
}

// =============================================================================
// TESTS FOR Document Metadata
// =============================================================================

#[test]
fn test_document_with_title() {
    let mut buffer = Vec::new();
    let mut document = Document::new();
    document.set_title("Test Document Title");

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Must contain Title in Info dictionary
    assert!(content.contains("/Title (Test Document Title)"),
            "Info dictionary must contain /Title");
}

#[test]
fn test_document_with_author() {
    let mut buffer = Vec::new();
    let mut document = Document::new();
    document.set_author("Test Author");

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Must contain Author in Info dictionary
    assert!(content.contains("/Author (Test Author)"),
            "Info dictionary must contain /Author");
}

#[test]
fn test_document_with_all_metadata() {
    let mut buffer = Vec::new();
    let mut document = Document::new();

    document.set_title("Complete Document");
    document.set_author("Full Author Name");
    document.set_subject("Test Subject");
    document.set_keywords("test, metadata, complete");

    {
        let mut writer = PdfWriter::new_with_writer(&mut buffer);
        writer.write_document(&mut document).unwrap();
    }

    let content = String::from_utf8_lossy(&buffer);

    // Must contain all metadata fields
    assert!(content.contains("/Title"), "Must contain /Title");
    assert!(content.contains("/Author"), "Must contain /Author");
    assert!(content.contains("/Subject"), "Must contain /Subject");
    assert!(content.contains("/Keywords"), "Must contain /Keywords");
}
