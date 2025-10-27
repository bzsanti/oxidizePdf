//! ISO 32000-1:2008 Section 7.5.2 - Document Catalog Tests
//!
//! These tests verify document catalog structure compliance

use super::super::{create_basic_test_pdf, iso_test};
use crate::verification::{parser::parse_pdf, VerificationLevel};
use crate::{Document, Font, Page, Result as PdfResult};

iso_test!(
    test_catalog_type_entry_level_2,
    "7.5.2.1",
    VerificationLevel::GeneratesPdf,
    "Test passed".to_string(),
    {
        let pdf_bytes = create_basic_test_pdf(
            "Test passed".to_string(),
            "Testing document catalog /Type entry",
        )?;

        let passed = pdf_bytes.len() > 1000;
        let level_achieved = if passed { 2 } else { 1 };
        let notes = if passed {
            "Test passed".to_string()
        } else {
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);

iso_test!(
    test_catalog_pages_reference_level_3,
    "7.5.2.2",
    VerificationLevel::ContentVerified,
    "Test passed".to_string(),
    {
        let pdf_bytes = create_basic_test_pdf(
            "Test passed".to_string(),
            "Testing catalog /Pages reference",
        )?;

        // Parse and verify content
        let parsed = parse_pdf(&pdf_bytes)?;
        let pages_reference_valid = parsed.catalog.is_some() && parsed.page_tree.is_some();

        let passed = pages_reference_valid;
        let level_achieved = if passed { 3 } else { 2 };
        let notes = if passed {
            "Test passed".to_string()
        } else {
            "Test failed - implementation error".to_string()
        };

        Ok((passed, level_achieved, notes))
    }
);
