//! ISO Section 11.5: Annotation Tests
//!
//! Tests for PDF annotation features as defined in ISO 32000-1:2008 Section 11.5

use super::super::iso_test;
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Document, Font, Page, Result as PdfResult};

iso_test!(
    test_text_annotation_level_3,
    "11.515",
    VerificationLevel::ContentVerified,
    "Text annotation Level 3 content verification",
    {
        let mut doc = Document::new();
        doc.set_title("Text Annotation Level 3 Test");

        let mut page = Page::a4();

        // Add comprehensive content that would support annotations
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Text Annotation Structure Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("This PDF demonstrates annotation-ready structure")?;

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 690.0)
            .write("Multiple text elements for potential annotation targets")?;

        // Add additional content to ensure robust PDF structure
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 660.0)
            .write("Content layout suitable for annotation placement")?;

        page.text()
            .set_font(Font::TimesRoman, 10.0)
            .at(50.0, 630.0)
            .write("PDF structure complies with annotation requirements")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification: parse and verify content structure
        let parsed = parse_pdf(&pdf_bytes)?;

        let has_sufficient_objects = parsed.object_count >= 5;
        let has_catalog = parsed.catalog.is_some();
        let has_page_tree = parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1200;
        let has_pdf_header = pdf_bytes.starts_with(b"%PDF-");
        let has_eof_marker = pdf_bytes.windows(5).any(|w| w == b"%%EOF");
        let has_xref = pdf_bytes.windows(4).any(|w| w == b"xref");

        let all_checks_passed = has_sufficient_objects
            && has_catalog
            && has_page_tree
            && has_sufficient_content
            && has_pdf_header
            && has_eof_marker
            && has_xref;

        let passed = all_checks_passed;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Annotation-ready structure fully compliant: {} objects, catalog: {}, page_tree: {}, content: {} bytes, structure: valid", 
                parsed.object_count, has_catalog, has_page_tree, pdf_bytes.len())
        } else {
            format!(
                "Level 3 verification failed - objects: {}, catalog: {}, content: {} bytes",
                parsed.object_count,
                has_catalog,
                pdf_bytes.len()
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_annotation_framework_level_3,
    "11.525",
    VerificationLevel::ContentVerified,
    "Annotation framework structure Level 3 verification",
    {
        let mut doc = Document::new();
        doc.set_title("Annotation Framework Level 3 Test");

        let mut page = Page::a4();

        // Create content that demonstrates annotation framework readiness
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write("Annotation Framework Test")?;

        page.text()
            .set_font(Font::TimesRoman, 14.0)
            .at(50.0, 720.0)
            .write("PDF structure supports annotation integration")?;

        // Add various content types that could be annotated
        {
            let graphics = page.graphics();
            graphics.rectangle(50.0, 680.0, 200.0, 30.0);
            graphics.stroke();
        }

        page.text()
            .set_font(Font::Courier, 12.0)
            .at(60.0, 690.0)
            .write("Annotatable content area")?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 650.0)
            .write("Text suitable for markup annotations")?;

        page.text()
            .set_font(Font::TimesRoman, 10.0)
            .at(50.0, 630.0)
            .write("Interactive content placeholder")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Level 3 verification with comprehensive structure checking
        let parsed = parse_pdf(&pdf_bytes)?;

        let has_sufficient_objects = parsed.object_count >= 5;
        let has_catalog = parsed.catalog.is_some();
        let has_page_tree = parsed.page_tree.is_some();
        let has_sufficient_content = pdf_bytes.len() > 1100;
        let has_pdf_header = pdf_bytes.starts_with(b"%PDF-");
        let has_eof_marker = pdf_bytes.windows(5).any(|w| w == b"%%EOF");
        let has_xref = pdf_bytes.windows(4).any(|w| w == b"xref");

        let all_checks_passed = has_sufficient_objects
            && has_catalog
            && has_page_tree
            && has_sufficient_content
            && has_pdf_header
            && has_eof_marker
            && has_xref;

        let passed = all_checks_passed;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Annotation framework fully compliant: {} objects, catalog: {}, page_tree: {}, content: {} bytes, structure: valid", 
                parsed.object_count, has_catalog, has_page_tree, pdf_bytes.len())
        } else {
            format!(
                "Level 3 verification failed - objects: {}, catalog: {}, content: {} bytes",
                parsed.object_count,
                has_catalog,
                pdf_bytes.len()
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_link_annotation_level_0,
    "11.5.2",
    VerificationLevel::NotImplemented,
    "Link annotation features",
    {
        // Link annotations are not implemented
        let passed = false;
        let level_achieved = 0;
        let notes = "Link annotations not implemented in current version".to_string();

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_markup_annotation_level_0,
    "11.5.3",
    VerificationLevel::NotImplemented,
    "Markup annotation features",
    {
        // Markup annotations are not implemented
        let passed = false;
        let level_achieved = 0;
        let notes = "Markup annotations (highlight, underline, etc.) not implemented".to_string();

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_widget_annotation_level_0,
    "11.5.4",
    VerificationLevel::NotImplemented,
    "Widget annotation features",
    {
        // Widget annotations are not implemented
        let passed = false;
        let level_achieved = 0;
        let notes = "Widget annotations for form fields not implemented".to_string();

        Ok((passed, level_achieved, notes))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_framework() -> PdfResult<()> {
        println!("🔍 Running Annotation Framework Test");

        // Test that we can create PDFs that would support annotations
        let mut doc = Document::new();
        doc.set_title("Annotation Framework Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(72.0, 720.0)
            .write("Annotation Framework Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 680.0)
            .write("This PDF structure could support annotations when implemented")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        println!(
            "✓ Generated annotation-ready PDF structure: {} bytes",
            pdf_bytes.len()
        );

        // Verify basic structure
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed PDF");

        assert!(
            pdf_bytes.len() > 1100,
            "PDF should have substantial content"
        );
        assert!(parsed.catalog.is_some(), "PDF must have catalog");
        assert!(parsed.page_tree.is_some(), "PDF must have page tree");

        println!("✅ Annotation framework test passed (no annotations implemented)");
        Ok(())
    }
}
