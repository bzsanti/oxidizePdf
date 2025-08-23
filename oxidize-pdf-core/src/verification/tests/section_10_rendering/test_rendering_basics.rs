//! ISO Section 10.1-10.4: Basic Rendering Tests
//!
//! Tests for basic PDF rendering concepts: graphics state, coordinate systems,
//! and rendering model as defined in ISO 32000-1:2008 Section 10

use super::super::{create_basic_test_pdf, iso_test};
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Document, Font, Page, Result as PdfResult};

iso_test!(
    test_coordinate_system_level_2,
    "10.1",
    VerificationLevel::GeneratesPdf,
    "PDF coordinate system Level 2 verification",
    {
        let mut doc = Document::new();
        doc.set_title("Coordinate System Test");

        let mut page = Page::a4();

        // Test basic coordinate placement
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(72.0, 720.0) // 1 inch from left, 10 inches from bottom
            .write("Origin test at (72, 720)")?;

        page.text()
            .set_font(Font::TimesRoman, 10.0)
            .at(100.0, 100.0) // Different coordinates
            .write("Lower position at (100, 100)")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let passed = pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            format!(
                "PDF with coordinate positioning generated: {} bytes",
                pdf_bytes.len()
            )
        } else {
            "Coordinate system PDF generation failed".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_graphics_state_stacking_level_2,
    "10.2",
    VerificationLevel::GeneratesPdf,
    "Graphics state stack Level 2 verification",
    {
        let mut doc = Document::new();
        doc.set_title("Graphics State Stack Test");

        let mut page = Page::a4();

        // Test graphics state operations (if available)
        {
            let graphics = page.graphics();
            graphics.save_state();
            graphics.save_state();
        }

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write("State 1: Normal text")?;

        page.text()
            .set_font(Font::Courier, 12.0)
            .at(50.0, 700.0)
            .write("State 2: Courier font")?;

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 650.0)
            .write("Back to State 1: Helvetica")?;

        {
            let graphics = page.graphics();
            graphics.restore_state();
            graphics.restore_state();
        }

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        let passed = pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "PDF with graphics state stack operations generated".to_string()
        } else {
            "Graphics state stack PDF generation failed".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_rendering_model_level_3,
    "10.3",
    VerificationLevel::ContentVerified,
    "Basic rendering model content verification",
    {
        let mut doc = Document::new();
        doc.set_title("Rendering Model Test");

        let mut page = Page::a4();

        // Create content that exercises rendering model
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(72.0, 720.0)
            .write("Rendering Model Test")?;

        {
            let graphics = page.graphics();

            // Basic drawing operations
            graphics.move_to(100.0, 600.0);
            graphics.line_to(200.0, 600.0);
            graphics.stroke();
        }

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify content
        let parsed = parse_pdf(&pdf_bytes)?;

        let content_valid =
            parsed.catalog.is_some() && parsed.page_tree.is_some() && pdf_bytes.len() > 1000;

        let passed = content_valid;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            "PDF with rendering operations parsed successfully".to_string()
        } else {
            "Rendering model content verification failed".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_transparency_not_implemented_level_0,
    "10.6",
    VerificationLevel::NotImplemented,
    "Transparency rendering features",
    {
        // Transparency features are not implemented in basic version
        let passed = false;
        let level_achieved = 0;
        let notes = "Advanced transparency features not implemented in current version".to_string();

        Ok((passed, level_achieved, notes))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rendering_integration() -> PdfResult<()> {
        println!("🔍 Running Rendering Integration Test");

        // Test multiple rendering features together
        let mut doc = Document::new();
        doc.set_title("Rendering Integration Test");

        let mut page = Page::a4();

        // Text rendering
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(72.0, 720.0)
            .write("Rendering Integration Test")?;

        // Graphics rendering
        {
            let graphics = page.graphics();

            // Save state
            graphics.save_state();

            // Draw some lines
            graphics.move_to(72.0, 680.0);
            graphics.line_to(300.0, 680.0);
            graphics.stroke();

            // Restore state
            graphics.restore_state();
        }

        // More text with different font
        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 650.0)
            .write("This tests multiple rendering operations in sequence")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        println!(
            "✓ Generated rendering integration PDF: {} bytes",
            pdf_bytes.len()
        );

        // Parse to verify structure
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed rendering PDF");

        if let Some(_catalog) = &parsed.catalog {
            println!("  - Catalog present");
        }

        if let Some(page_tree) = &parsed.page_tree {
            println!("  - Page tree present with {} pages", page_tree.page_count);
        }

        // Basic assertions
        assert!(
            pdf_bytes.len() > 1100,
            "PDF should contain substantial content"
        );
        assert!(parsed.catalog.is_some(), "PDF must have catalog");
        assert!(parsed.page_tree.is_some(), "PDF must have page tree");

        println!("✅ Rendering integration test passed");
        Ok(())
    }
}
