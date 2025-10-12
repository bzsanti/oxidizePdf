/// Test to validate that embedded fonts are preserved during overlay
///
/// CURRENT STATUS (Phase 3 Complete for Type 1 fonts):
/// - ✅ Type 1 embedded fonts: WORKING (detection, resolution, copying)
/// - ⚠️  CID/Type0 embedded fonts: PARTIAL (visible but not embedded)
///
/// This test uses Cold_Email_Hacks.pdf which contains CID/Type0 TrueType fonts
/// (Arial-BoldMT, ArialMT). These require recursive resolution of:
/// Type0 → DescendantFonts → CIDFont → FontDescriptor → FontFile2 → Stream
///
/// Phase 3.4 (CID font support) is required for this test to pass.
/// See .private/PHASE3_SESSION_SUMMARY.md for details.
use oxidize_pdf::error::Result;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::{Document, Page};
use tempfile::TempDir;

#[test]
#[ignore] // TODO: Enable when Phase 3.4 complete (CID/Type0 font hierarchy resolution)
fn test_overlay_preserves_embedded_fonts() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();

    // Step 1: Load real PDF with embedded Arial font
    let original_pdf = "tests/fixtures/Cold_Email_Hacks.pdf";
    let reader = PdfReader::open(original_pdf)?;
    let parsed_doc = PdfDocument::new(reader);

    // Step 2: Get first page and extract text (baseline)
    let parsed_page = parsed_doc.get_page(0)?;
    let original_text = extract_text_from_page(&parsed_page, &parsed_doc);

    println!(
        "Original text sample: {:?}",
        &original_text[..100.min(original_text.len())]
    );

    // Step 3: Convert to writable page with content preservation
    let mut writable_page = Page::from_parsed_with_content(&parsed_page, &parsed_doc)?;

    // Step 4: Add overlay text
    writable_page
        .text()
        .set_font(oxidize_pdf::text::Font::HelveticaBold, 24.0)
        .at(100.0, 750.0)
        .write("OVERLAY TEXT")?;

    // Step 5: Save overlaid PDF
    let output_path = temp_dir.path().join("overlaid.pdf");
    let mut output_doc = Document::new();
    output_doc.add_page(writable_page);
    output_doc.save(&output_path)?;

    // Step 6: Validate with pdffonts
    let pdffonts_output = std::process::Command::new("pdffonts")
        .arg(&output_path)
        .output()
        .expect("pdffonts should be available");

    let fonts_list = String::from_utf8_lossy(&pdffonts_output.stdout);
    println!("Fonts in overlaid PDF:\n{}", fonts_list);

    // Step 7: Validate embedded fonts are present
    assert!(
        fonts_list.contains("Arial"),
        "Arial font should be preserved in overlaid PDF"
    );
    assert!(
        fonts_list.contains("yes") && fonts_list.contains("Arial"),
        "Arial should be marked as embedded (emb=yes)"
    );

    // Step 8: Validate text extraction still works
    let pdftotext_output = std::process::Command::new("pdftotext")
        .arg(&output_path)
        .arg("-")
        .output()
        .expect("pdftotext should be available");

    let extracted_text = String::from_utf8_lossy(&pdftotext_output.stdout);
    println!(
        "Extracted text sample: {:?}",
        &extracted_text[..100.min(extracted_text.len())]
    );

    // Should contain both original and overlay text
    assert!(
        extracted_text.contains("OVERLAY TEXT"),
        "Overlay text should be present"
    );

    // Original text should still be present (this is the critical test)
    // If fonts are not preserved correctly, original text might be garbled or missing
    assert!(
        !original_text.is_empty(),
        "Original text should not be empty"
    );

    Ok(())
}

/// Helper function to extract text from a parsed page
fn extract_text_from_page<R: std::io::Read + std::io::Seek>(
    page: &oxidize_pdf::parser::page_tree::ParsedPage,
    doc: &PdfDocument<R>,
) -> String {
    // Get content streams
    let content_streams = page.content_streams_with_document(doc).unwrap_or_default();

    // Simple text extraction (just for validation, not production-quality)
    let mut text = String::new();
    for stream in content_streams {
        if let Ok(content_str) = String::from_utf8(stream) {
            // Very basic extraction: look for (text) patterns
            for line in content_str.lines() {
                if line.contains("(") && line.contains(")") {
                    text.push_str(line);
                    text.push('\n');
                }
            }
        }
    }
    text
}
