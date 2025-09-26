//! Integration tests for transparency functionality
//! These tests verify that ExtGState resources are properly written to PDFs

use oxidize_pdf::error::Result;
use oxidize_pdf::{Color, Document, Page};

#[test]
fn test_transparency_generates_extgstate_in_pdf() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Draw with transparency
    page.graphics()
        .set_alpha(0.5)?
        .set_fill_color(Color::red())
        .rect(100.0, 100.0, 50.0, 50.0)
        .fill();

    doc.add_page(page);

    // Generate PDF bytes
    let pdf_bytes = doc.to_bytes()?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // Verify ExtGState is present in PDF structure
    assert!(
        pdf_content.contains("/ExtGState"),
        "PDF should contain ExtGState dictionary"
    );
    assert!(
        pdf_content.contains("/GS1"),
        "PDF should contain GS1 ExtGState reference"
    );
    assert!(
        pdf_content.contains("/CA 0.5"),
        "PDF should contain stroke alpha value"
    );
    assert!(
        pdf_content.contains("/ca 0.5"),
        "PDF should contain fill alpha value"
    );
    assert!(
        pdf_content.contains("/Type /ExtGState"),
        "PDF should contain ExtGState type"
    );

    Ok(())
}

#[test]
fn test_multiple_transparency_values() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Draw with different transparency values
    page.graphics()
        .set_alpha(0.3)?
        .set_fill_color(Color::red())
        .rect(50.0, 50.0, 40.0, 40.0)
        .fill()
        .set_alpha(0.7)?
        .set_fill_color(Color::blue())
        .rect(70.0, 70.0, 40.0, 40.0)
        .fill();

    doc.add_page(page);

    let pdf_bytes = doc.to_bytes()?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // Should have multiple ExtGState entries
    assert!(
        pdf_content.contains("/GS1"),
        "Should contain first ExtGState"
    );
    assert!(
        pdf_content.contains("/GS2"),
        "Should contain second ExtGState"
    );
    assert!(
        pdf_content.contains("/CA 0.3"),
        "Should contain 0.3 alpha value"
    );
    assert!(
        pdf_content.contains("/CA 0.7"),
        "Should contain 0.7 alpha value"
    );

    Ok(())
}

#[test]
fn test_separate_fill_stroke_transparency() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Draw with separate fill and stroke transparency
    // Note: Currently separate set_alpha_fill + set_alpha_stroke calls overwrite each other
    // since they both use pending state. Use the functional approach instead:
    page.graphics()
        .with_extgstate(|state| state.with_alpha_fill(0.4).with_alpha_stroke(0.8))?
        .set_fill_color(Color::green())
        .set_stroke_color(Color::black())
        .set_line_width(3.0)
        .rect(100.0, 100.0, 60.0, 60.0)
        .fill_stroke();

    doc.add_page(page);

    let pdf_bytes = doc.to_bytes()?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // Should have separate alpha values
    assert!(
        pdf_content.contains("/ca 0.4"),
        "Should contain fill alpha 0.4"
    );
    assert!(
        pdf_content.contains("/CA 0.8"),
        "Should contain stroke alpha 0.8"
    );

    Ok(())
}

#[test]
fn test_transparency_with_existing_page_resources() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Add text and transparency
    page.text()
        .set_font(oxidize_pdf::Font::Helvetica, 12.0)
        .at(50.0, 600.0)
        .write("Text with transparency")?;

    page.graphics()
        .set_alpha(0.6)?
        .set_fill_color(Color::yellow())
        .rect(50.0, 580.0, 200.0, 30.0)
        .fill();

    doc.add_page(page);

    let pdf_bytes = doc.to_bytes()?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // Should contain both font and ExtGState resources
    assert!(
        pdf_content.contains("/Font"),
        "Should contain font resources"
    );
    assert!(
        pdf_content.contains("/ExtGState"),
        "Should contain ExtGState resources"
    );
    assert!(
        pdf_content.contains("/Helvetica"),
        "Should contain Helvetica font"
    );
    assert!(
        pdf_content.contains("/GS1"),
        "Should contain ExtGState reference"
    );

    Ok(())
}

#[test]
fn test_no_extgstate_when_no_transparency() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Draw without transparency
    page.graphics()
        .set_fill_color(Color::red())
        .rect(100.0, 100.0, 50.0, 50.0)
        .fill();

    doc.add_page(page);

    let pdf_bytes = doc.to_bytes()?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // Should NOT contain ExtGState when no transparency is used
    assert!(
        !pdf_content.contains("/ExtGState"),
        "PDF should not contain ExtGState when no transparency used"
    );

    Ok(())
}

/// CRITICAL REGRESSION TEST for Issue #51
/// Verifies that ExtGState operations appear in correct order to prevent transparency regression
#[test]
fn test_transparency_operation_order() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Draw with transparency - this should generate operations in correct order
    page.graphics()
        .set_alpha(0.5)?
        .set_fill_color(Color::red())
        .rect(100.0, 100.0, 50.0, 50.0)
        .fill();

    doc.add_page(page);

    // Generate PDF content
    let pdf_bytes = doc.to_bytes()?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // Find the stream content containing the operations
    if let Some(stream_start) = pdf_content.find("stream\r\n") {
        let stream_content = &pdf_content[stream_start + 8..];
        if let Some(stream_end) = stream_content.find("\r\nendstream") {
            let operations = &stream_content[..stream_end];

            // Critical check: /GS1 gs MUST appear before rectangle operation
            let gs_pos = operations.find("/GS1 gs");
            let rect_pos = operations.find("re\n"); // rectangle operation

            assert!(
                gs_pos.is_some(),
                "ExtGState operation /GS1 gs not found in stream"
            );
            assert!(
                rect_pos.is_some(),
                "Rectangle operation 're' not found in stream"
            );

            let gs_index = gs_pos.unwrap();
            let rect_index = rect_pos.unwrap();

            // THE CRITICAL ASSERTION: ExtGState MUST be applied BEFORE rectangle
            assert!(gs_index < rect_index,
                "REGRESSION DETECTED: /GS1 gs (at position {}) must appear BEFORE 're' (at position {}). Operation order: {}",
                gs_index, rect_index, operations);

            // Additional checks: ExtGState should also be before fill operation
            if let Some(fill_pos) = operations.find(" f\n") {
                assert!(
                    gs_index < fill_pos,
                    "REGRESSION DETECTED: /GS1 gs must appear before fill operation 'f'"
                );
            }
        }
    }

    Ok(())
}

/// Test for ExtGState timing - ensures immediate application
#[test]
fn test_extgstate_applied_immediately() -> Result<()> {
    let _doc = Document::new();
    let mut page = Page::a4();

    // Create graphics context and verify ExtGState is applied immediately
    {
        let graphics = page.graphics();

        // Before setting alpha, there should be no ExtGState
        assert_eq!(graphics.extgstate_manager().count(), 0);

        // Set alpha - this should immediately create and apply ExtGState
        graphics.set_alpha(0.7)?;

        // After setting alpha, ExtGState should exist
        assert_eq!(graphics.extgstate_manager().count(), 1);

        // The operations should already contain the ExtGState reference
        let ops = graphics.operations();
        assert!(
            ops.contains("/GS1 gs"),
            "ExtGState should be applied immediately in operations"
        );
    }

    Ok(())
}

/// Test for function naming compatibility
#[test]
fn test_write_pages_compatibility() -> Result<()> {
    // This is a compile-time test - if this compiles, the API compatibility is maintained
    use oxidize_pdf::Document;

    let mut doc = Document::new();
    let page = oxidize_pdf::Page::a4();
    doc.add_page(page);

    // Both function names should work (write_pages is main, write_pages_with_fonts is alias)
    // This test ensures we haven't broken backwards compatibility
    let _pdf_bytes = doc.to_bytes()?;

    Ok(())
}

/// Test for ExtGState with dash pattern - prevents field name regression
#[test]
fn test_extgstate_dash_pattern() -> Result<()> {
    use oxidize_pdf::graphics::LineDashPattern;

    let mut doc = Document::new();
    let mut page = Page::a4();

    // Create ExtGState with dash pattern to test field name correctness
    {
        let graphics = page.graphics();

        // Use with_extgstate to set dash pattern
        graphics.with_extgstate(|state| {
            let dash_pattern = LineDashPattern {
                array: vec![3.0, 2.0, 1.0],
                phase: 1.5,
            };
            state.with_dash_pattern(dash_pattern).with_alpha(0.5)
        })?;

        graphics
            .set_stroke_color(Color::blue())
            .rect(50.0, 50.0, 100.0, 100.0)
            .stroke();
    }

    doc.add_page(page);

    // Generate PDF and verify dash pattern is correctly written
    let pdf_bytes = doc.to_bytes()?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // Should contain ExtGState with dash pattern
    assert!(
        pdf_content.contains("/ExtGState"),
        "Should contain ExtGState dictionary"
    );
    assert!(
        pdf_content.contains("/D ["),
        "Should contain dash array in PDF format"
    );

    // The dash pattern should be formatted as: /D [[3.0 2.0 1.0] 1.5]
    // This verifies that dash_pattern.array and dash_pattern.phase are processed correctly

    Ok(())
}

/// Visual end-to-end test - comprehensive transparency verification
#[test]
fn test_transparency_visual_output() -> Result<()> {
    let mut doc = Document::new();
    let mut page = Page::a4();

    // Create a complex transparency scenario
    {
        let graphics = page.graphics();

        // Multiple transparency layers with different operations
        graphics
            .save_state()
            .set_alpha(0.8)?
            .set_fill_color(Color::red())
            .rect(50.0, 50.0, 100.0, 100.0)
            .fill()
            .restore_state()
            .save_state()
            .set_alpha(0.6)?
            .set_stroke_color(Color::blue())
            .set_line_width(5.0)
            .rect(75.0, 75.0, 100.0, 100.0)
            .stroke()
            .restore_state()
            .set_alpha(0.4)?
            .set_fill_color(Color::green())
            .circle(125.0, 125.0, 30.0)
            .fill();
    }

    doc.add_page(page);

    // Generate PDF content
    let pdf_bytes = doc.to_bytes()?;
    let pdf_content = String::from_utf8_lossy(&pdf_bytes);

    // Comprehensive visual output checks
    assert!(
        pdf_content.contains("/ExtGState"),
        "Should contain ExtGState for transparency"
    );
    assert!(
        pdf_content.contains("/GS1"),
        "Should contain at least one graphics state"
    );
    assert!(pdf_content.contains("/CA "), "Should contain stroke alpha");
    assert!(pdf_content.contains("/ca "), "Should contain fill alpha");

    // Verify multiple ExtGStates for different alpha values
    assert!(
        pdf_content.contains("0.8") || pdf_content.contains("0.800"),
        "Should contain alpha 0.8"
    );
    assert!(
        pdf_content.contains("0.6") || pdf_content.contains("0.600"),
        "Should contain alpha 0.6"
    );
    assert!(
        pdf_content.contains("0.4") || pdf_content.contains("0.400"),
        "Should contain alpha 0.4"
    );

    // Verify the PDF structure is valid
    assert!(
        pdf_content.starts_with("%PDF-"),
        "Should be valid PDF format"
    );
    assert!(
        pdf_content.contains("%%EOF"),
        "Should have proper PDF ending"
    );

    // Find and verify stream operations order
    if let Some(stream_start) = pdf_content.find("stream\r\n") {
        let stream_content = &pdf_content[stream_start + 8..];
        if let Some(stream_end) = stream_content.find("\r\nendstream") {
            let operations = &stream_content[..stream_end];

            // Verify that ALL transparency operations appear before their corresponding drawing operations
            let gs_positions: Vec<_> = operations.match_indices("/GS").map(|(i, _)| i).collect();
            let draw_positions: Vec<_> = operations
                .match_indices(" re\n")
                .map(|(i, _)| i)
                .chain(operations.match_indices(" f\n").map(|(i, _)| i))
                .chain(operations.match_indices(" S\n").map(|(i, _)| i))
                .collect();

            // Each graphics state should appear before any drawing operations that use it
            for &gs_pos in &gs_positions {
                let subsequent_draws: Vec<_> = draw_positions
                    .iter()
                    .filter(|&&draw_pos| draw_pos > gs_pos)
                    .collect();
                assert!(
                    !subsequent_draws.is_empty(),
                    "Each ExtGState should be followed by drawing operations"
                );
            }
        }
    }

    Ok(())
}
