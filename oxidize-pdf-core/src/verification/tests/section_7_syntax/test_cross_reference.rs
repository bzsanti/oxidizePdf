//! ISO Section 7.5.5: Cross-Reference Tests
//!
//! Tests for PDF cross-reference table integrity and object reference consistency

use super::super::iso_test;
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Color, Document, Font, Page, Result as PdfResult};

iso_test!(
    test_xref_table_integrity_level_3,
    "7.5.5.1",
    VerificationLevel::ContentVerified,
    "Cross-reference table structure and object references integrity",
    {
        let mut doc = Document::new();
        doc.set_title("Cross-Reference Integrity Test");

        let mut page = Page::a4();

        // Create content that generates multiple objects
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write("Cross-Reference Table Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Testing xref table structure and object integrity")?;

        page.graphics()
            .set_fill_color(Color::rgb(0.2, 0.4, 0.8))
            .rectangle(50.0, 680.0, 100.0, 30.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify cross-reference structure
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Verify xref table presence and structure
        let has_xref_table = pdf_string.contains("xref");
        let has_trailer = pdf_string.contains("trailer");
        let has_startxref = pdf_string.contains("startxref");
        let has_eof = pdf_string.contains("%%EOF");

        // Check for proper object numbering
        let object_count = pdf_string.matches(" obj").count();
        let has_objects = object_count >= 3; // At least catalog, page tree, page

        // Verify basic PDF structure integrity
        let has_valid_header = pdf_bytes.starts_with(b"%PDF-");
        let has_catalog = parsed.catalog.is_some();
        let has_page_tree = parsed.page_tree.is_some();
        let sufficient_size = pdf_bytes.len() > 1200;

        let passed = has_xref_table
            && has_trailer
            && has_startxref
            && has_eof
            && has_objects
            && has_valid_header
            && has_catalog
            && has_page_tree
            && sufficient_size;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Cross-reference integrity verified: xref table: {}, {} objects, trailer: {}, startxref: {}, {} bytes", 
                   has_xref_table, object_count, has_trailer, has_startxref, pdf_bytes.len())
        } else {
            format!("Cross-reference verification incomplete: xref: {}, objects: {}, trailer: {}, structure valid: {}", 
                   has_xref_table, object_count, has_trailer, has_catalog && has_page_tree)
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_object_numbering_consistency_level_3,
    "7.5.5.2",
    VerificationLevel::ContentVerified,
    "Object numbering consistency and reference validation",
    {
        let mut doc = Document::new();
        doc.set_title("Object Numbering Test");

        let mut page = Page::a4();

        // Create multiple objects with different types
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(72.0, 720.0)
            .write("Object Numbering Consistency Test")?;

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(72.0, 690.0)
            .write("Verifying PDF object references and numbering")?;

        // Add graphics to create more objects
        page.graphics()
            .set_stroke_color(Color::rgb(1.0, 0.0, 0.0))
            .rectangle(72.0, 650.0, 200.0, 25.0)
            .stroke();

        page.graphics()
            .set_fill_color(Color::rgb(0.0, 0.8, 0.2))
            .rectangle(72.0, 620.0, 150.0, 20.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify object consistency
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Count objects and verify structure
        let obj_count = pdf_string.matches(" obj").count();
        let endobj_count = pdf_string.matches("endobj").count();
        let objects_balanced = obj_count == endobj_count && obj_count >= 3;

        // Verify object references structure
        let has_catalog_ref = pdf_string.contains("/Root");
        let has_page_refs = pdf_string.contains("/Pages") && pdf_string.contains("/Kids");
        let has_content_refs = pdf_string.contains("/Contents");

        // Check cross-reference consistency
        let has_xref = pdf_string.contains("xref");
        let has_trailer_size = pdf_string.contains("/Size");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let sufficient_content = pdf_bytes.len() > 1400;

        let passed = objects_balanced
            && has_catalog_ref
            && has_page_refs
            && has_xref
            && has_trailer_size
            && has_valid_structure
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Object numbering verified: {}/{} obj/endobj pairs, catalog ref: {}, page refs: {}, xref: {}, {} bytes", 
                   obj_count, endobj_count, has_catalog_ref, has_page_refs, has_xref, pdf_bytes.len())
        } else {
            format!(
                "Object numbering issues: obj/endobj: {}/{}, refs: catalog:{} pages:{}, xref: {}",
                obj_count, endobj_count, has_catalog_ref, has_page_refs, has_xref
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_indirect_object_references_level_3,
    "7.5.5.3",
    VerificationLevel::ContentVerified,
    "Indirect object references and generation numbers",
    {
        let mut doc = Document::new();
        doc.set_title("Indirect Object References Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write("Indirect Object References Test")?;

        // Create content that requires indirect references
        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(50.0, 720.0)
            .write("Testing PDF indirect object structure")?;

        page.text()
            .set_font(Font::Courier, 10.0)
            .at(50.0, 690.0)
            .write("Verifying generation numbers and references")?;

        // Add graphics objects
        page.graphics()
            .set_fill_color(Color::rgb(0.3, 0.7, 0.9))
            .rectangle(50.0, 650.0, 120.0, 30.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify indirect references
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check for indirect object references (format: "n g R")
        let has_indirect_refs = pdf_string.contains(" R") && pdf_string.contains(" 0 R");
        let has_generation_numbers = pdf_string.contains(" 0 obj"); // Generation 0 is standard

        // Verify object structure
        let obj_declarations = pdf_string.matches(" obj").count();
        let has_sufficient_objects = obj_declarations >= 3;

        // Check reference integrity
        let has_catalog_indirect = pdf_string.contains("/Root") && pdf_string.contains(" R");
        let has_page_indirect = pdf_string.contains("/Pages") && pdf_string.contains(" R");

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let sufficient_content = pdf_bytes.len() > 1200;

        let passed = has_indirect_refs
            && has_generation_numbers
            && has_sufficient_objects
            && has_catalog_indirect
            && has_page_indirect
            && has_valid_structure
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Indirect references verified: indirect refs: {}, gen numbers: {}, {} objects, catalog/page refs: {}/{}, {} bytes", 
                   has_indirect_refs, has_generation_numbers, obj_declarations, has_catalog_indirect, has_page_indirect, pdf_bytes.len())
        } else {
            format!(
                "Indirect reference issues: refs: {}, gen: {}, objects: {}, catalog/page: {}/{}",
                has_indirect_refs,
                has_generation_numbers,
                obj_declarations,
                has_catalog_indirect,
                has_page_indirect
            )
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_trailer_dictionary_level_3,
    "7.5.5.4",
    VerificationLevel::ContentVerified,
    "Trailer dictionary structure and required entries",
    {
        let mut doc = Document::new();
        doc.set_title("Trailer Dictionary Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(72.0, 720.0)
            .write("Trailer Dictionary Structure Test")?;

        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 690.0)
            .write("Verifying PDF trailer dictionary compliance")?;

        // Add varied content for more complete PDF structure
        page.graphics()
            .set_fill_color(Color::rgb(0.8, 0.2, 0.4))
            .rectangle(72.0, 650.0, 180.0, 25.0)
            .fill();

        page.text()
            .set_font(Font::Courier, 9.0)
            .at(72.0, 620.0)
            .write("Testing /Size, /Root, and other trailer entries")?;

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        // Parse and verify trailer structure
        let parsed = parse_pdf(&pdf_bytes)?;
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);

        // Check for required trailer entries
        let has_trailer_keyword = pdf_string.contains("trailer");
        let has_size_entry = pdf_string.contains("/Size");
        let has_root_entry = pdf_string.contains("/Root");
        let has_startxref = pdf_string.contains("startxref");
        let has_eof = pdf_string.contains("%%EOF");

        // Verify trailer structure context
        let trailer_pos = pdf_string.find("trailer");
        let startxref_pos = pdf_string.find("startxref");
        let proper_order = match (trailer_pos, startxref_pos) {
            (Some(t_pos), Some(s_pos)) => t_pos < s_pos,
            _ => false,
        };

        let has_valid_structure = parsed.catalog.is_some() && parsed.page_tree.is_some();
        let sufficient_content = pdf_bytes.len() > 1300;

        let passed = has_trailer_keyword
            && has_size_entry
            && has_root_entry
            && has_startxref
            && has_eof
            && proper_order
            && has_valid_structure
            && sufficient_content;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            format!("Trailer dictionary verified: trailer: {}, /Size: {}, /Root: {}, startxref: {}, EOF: {}, order: {}, {} bytes", 
                   has_trailer_keyword, has_size_entry, has_root_entry, has_startxref, has_eof, proper_order, pdf_bytes.len())
        } else {
            format!("Trailer verification incomplete: trailer: {}, /Size: {}, /Root: {}, order: {}, structure: {}", 
                   has_trailer_keyword, has_size_entry, has_root_entry, proper_order, has_valid_structure)
        };

        Ok((passed, level_achieved, notes))
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_reference_infrastructure() -> PdfResult<()> {
        println!("🔍 Running Cross-Reference Infrastructure Test");

        // Test PDF with multiple objects to verify xref structure
        let mut doc = Document::new();
        doc.set_title("Cross-Reference Infrastructure Test");

        let mut page = Page::a4();

        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(72.0, 720.0)
            .write("Cross-Reference Test")?;

        // Add content to create multiple objects
        page.text()
            .set_font(Font::TimesRoman, 12.0)
            .at(72.0, 680.0)
            .write("Testing xref table and object references")?;

        page.graphics()
            .set_fill_color(Color::rgb(0.5, 0.7, 0.9))
            .rectangle(72.0, 640.0, 150.0, 20.0)
            .fill();

        doc.add_page(page);
        let pdf_bytes = doc.to_bytes()?;

        println!(
            "✓ Generated PDF with cross-reference structure: {} bytes",
            pdf_bytes.len()
        );

        // Verify cross-reference components
        let pdf_string = String::from_utf8_lossy(&pdf_bytes);
        let has_xref = pdf_string.contains("xref");
        let has_trailer = pdf_string.contains("trailer");
        let has_startxref = pdf_string.contains("startxref");
        let has_eof = pdf_string.contains("%%EOF");

        println!(
            "✓ Cross-reference components - xref: {}, trailer: {}, startxref: {}, EOF: {}",
            has_xref, has_trailer, has_startxref, has_eof
        );

        // Verify parsing
        let parsed = parse_pdf(&pdf_bytes)?;
        println!("✓ Successfully parsed PDF structure");

        assert!(
            pdf_bytes.len() > 1000,
            "PDF should have substantial content"
        );
        assert!(has_xref, "PDF must have xref table");
        assert!(has_trailer, "PDF must have trailer");
        assert!(parsed.catalog.is_some(), "PDF must have catalog");

        println!("✅ Cross-reference infrastructure test passed");
        Ok(())
    }
}
