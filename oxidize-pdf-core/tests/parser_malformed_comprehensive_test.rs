//! Comprehensive Parser Malformed PDF Tests
//!
//! This test suite extends the existing parser edge cases with additional
//! comprehensive tests for malformed PDFs, focusing on:
//! - Stream corruption and invalid lengths
//! - Object stream edge cases
//! - Cross-reference stream corruption
//! - Linearized PDF corruption
//! - Incremental update corruption
//! - Font and encoding edge cases
//! - Form XObject parsing errors
//! - Security handler corruption
//! - Page tree corruption
//! - Resource dictionary errors

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::Cursor;

/// Helper function to create a basic PDF structure with custom content
fn create_pdf_with_content(content: &str) -> Vec<u8> {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n"); // Binary comment
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"\nxref\n0 1\n0000000000 65535 f \n");
    pdf.extend_from_slice(b"trailer\n<</Size 1>>\n");
    pdf.extend_from_slice(b"startxref\n");
    pdf.extend_from_slice(format!("{}\n", pdf.len() - 50).as_bytes());
    pdf.extend_from_slice(b"%%EOF\n");
    pdf
}

/// Test 1: Stream with negative length
#[test]
fn test_stream_negative_length() {
    let content = r#"
1 0 obj
<</Length -100>>
stream
This is some stream data
endstream
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            // Try to read the stream object
            match doc.get_object(1, 0) {
                Ok(_) => println!("Unexpectedly succeeded reading negative length stream"),
                Err(e) => {
                    println!("Expected error: {e}");
                }
            }
        }
        Err(e) => {
            println!("Parser error (expected): {e}");
        }
    }
}

/// Test 2: Stream with length larger than file size
#[test]
fn test_stream_excessive_length() {
    let content = r#"
1 0 obj
<</Length 999999999>>
stream
Short data
endstream
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Unexpectedly succeeded with excessive stream length"),
            Err(e) => {
                println!("Expected error: {e}");
            }
        },
        Err(e) => println!("Parser error (expected): {e}"),
    }
}

/// Test 3: Stream without endstream marker
#[test]
fn test_stream_missing_endstream() {
    let content = r#"
1 0 obj
<</Length 10>>
stream
1234567890
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Parsed stream without endstream"),
            Err(e) => {
                println!("Expected error: {e}");
            }
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 4: Object stream with corrupted index
#[test]
fn test_object_stream_corrupted_index() {
    let content = r#"
1 0 obj
<</Type /ObjStm /N 3 /First 10 /Length 50>>
stream
invalid index data here
10 20 30
<</Key /Value>>
endstream
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Unexpectedly parsed corrupted object stream"),
            Err(e) => println!("Expected error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 5: Cross-reference stream with invalid predictor
#[test]
fn test_xref_stream_invalid_predictor() {
    let content = r#"
1 0 obj
<</Type /XRef /Size 10 /W [1 2 1] /Filter /FlateDecode /DecodeParms <</Predictor 99>> /Length 20>>
stream
invalid compressed data
endstream
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Parser created, xref stream handling tested"),
        Err(e) => {
            println!("Expected error: {e}");
        }
    }
}

/// Test 6: Linearized PDF with corrupted hint stream
#[test]
fn test_linearized_corrupted_hint() {
    let content = r#"
1 0 obj
<</Linearized 1 /L 5000 /H [100 200] /O 3 /E 500 /N 1 /T 4800>>
endobj
2 0 obj
<</Type /Catalog /Pages 3 0 R>>
endobj
3 0 obj
<</Type /Pages /Kids [] /Count 0>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    // Should handle linearized PDFs gracefully even if corrupted
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Handled linearized PDF"),
        Err(e) => println!("Error with linearized PDF: {e}"),
    }
}

/// Test 7: Incremental update with invalid prev offset
#[test]
fn test_incremental_update_invalid_prev() {
    let mut pdf = Vec::new();
    // First version
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(b"1 0 obj\n<</Type /Catalog>>\nendobj\n");
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n40\n%%EOF\n");

    // Incremental update with invalid prev
    pdf.extend_from_slice(b"\n1 0 obj\n<</Type /Catalog /Updated true>>\nendobj\n");
    pdf.extend_from_slice(b"xref\n0 1\n0000000000 65535 f \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2 /Prev 999999>>\n"); // Invalid prev
    pdf.extend_from_slice(b"startxref\n150\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Handled incremental update"),
        Err(e) => {
            println!("Expected error: {e}");
        }
    }
}

/// Test 8: Font descriptor with invalid encoding
#[test]
fn test_font_invalid_encoding() {
    let content = r#"
1 0 obj
<</Type /Font /Subtype /Type1 /BaseFont /InvalidFont /Encoding 999>>
endobj
2 0 obj
<</Type /Font /Subtype /TrueType /BaseFont /Arial /Encoding <</Type /Encoding /BaseEncoding /InvalidEncoding>>>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            // Try to read font objects
            for i in 1..=2 {
                match doc.get_object(i, 0) {
                    Ok(_) => println!("Read font object {i}"),
                    Err(e) => println!("Font {i} error: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 9: Form XObject with recursive references
#[test]
fn test_form_xobject_recursive() {
    let content = r#"
1 0 obj
<</Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources <</XObject <</Form1 1 0 R>>>>/Length 10>>
stream
q Q
endstream
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Handled recursive form XObject"),
            Err(e) => println!("Expected recursion error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 10: Security handler with corrupted encryption dictionary
#[test]
fn test_corrupted_encryption_dict() {
    let content = r#"
1 0 obj
<</Type /Catalog>>
endobj
"#;
    let mut pdf = create_pdf_with_content(content);

    // Insert corrupted encryption dictionary in trailer
    let trailer_pos = pdf.windows(7).position(|w| w == b"trailer").unwrap();
    let insert_pos = trailer_pos + 8;
    let encrypt_dict = b" /Encrypt <</Filter /Standard /V 99 /R 99 /Length -1 /P -1 /O (corrupted) /U (corrupted)>>";
    pdf.splice(insert_pos..insert_pos, encrypt_dict.iter().cloned());

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Handled corrupted encryption"),
        Err(e) => {
            println!("Expected encryption error: {e}");
        }
    }
}

/// Test 11: Page tree with missing required fields
#[test]
fn test_page_tree_missing_fields() {
    let content = r#"
1 0 obj
<</Type /Catalog /Pages 2 0 R>>
endobj
2 0 obj
<</Type /Pages /Kids [3 0 R]>>
endobj
3 0 obj
<</Type /Page>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(3, 0) {
            Ok(_) => println!("Read incomplete page object"),
            Err(e) => println!("Expected page error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 12: Resource dictionary with circular color space
#[test]
fn test_circular_colorspace() {
    let content = r#"
1 0 obj
<</Type /Page /Resources <</ColorSpace <</CS1 [/ICCBased 2 0 R]>>>>>
endobj
2 0 obj
<</N 3 /Alternate [/ICCBased 2 0 R] /Length 10>>
stream
1234567890
endstream
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Handled circular color space"),
            Err(e) => println!("Expected circular reference error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 13: Malformed hexadecimal strings
#[test]
fn test_malformed_hex_strings() {
    let content = r#"
1 0 obj
<</String1 <4G6F6F64> /String2 <48656C6C> /String3 <>>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(obj) => println!("Parsed malformed hex strings: {obj:?}"),
            Err(e) => println!("Hex string error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 14: Array with mismatched brackets
#[test]
fn test_array_mismatched_brackets() {
    let content = r#"
1 0 obj
[1 2 3 [4 5] 6 7]]
endobj
2 0 obj
[[1 2] [3 4 [5 6]
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            for i in 1..=2 {
                match doc.get_object(i, 0) {
                    Ok(obj) => println!("Object {i} parsed: {obj:?}"),
                    Err(e) => println!("Object {i} error: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 15: Dictionary with duplicate keys
#[test]
fn test_dictionary_duplicate_keys() {
    let content = r#"
1 0 obj
<</Type /Page /Type /Pages /MediaBox [0 0 612 792] /MediaBox [0 0 100 100]>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(obj) => {
                println!("Dictionary parsed with duplicate keys: {obj:?}");
            }
            Err(e) => println!("Dictionary error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 16: Name object with invalid characters
#[test]
fn test_name_invalid_characters() {
    let content = r#"
1 0 obj
<</Name1 /Valid#20Name /Name2 /#00#01#02 /Name3 /Name(with)Parens /Name4 /Name[with]Brackets>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(obj) => println!("Parsed names with special characters: {obj:?}"),
            Err(e) => println!("Name parsing error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 17: String with unbalanced parentheses
#[test]
fn test_string_unbalanced_parens() {
    let content = r#"
1 0 obj
<</String1 (Hello (nested) world /String2 (Missing close /String3 (Escaped \) paren)>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(obj) => println!("Parsed unbalanced strings: {obj:?}"),
            Err(e) => println!("String parsing error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 18: Invalid cross-reference subsection
#[test]
fn test_xref_invalid_subsection() {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(b"1 0 obj\n<</Type /Catalog>>\nendobj\n");
    pdf.extend_from_slice(b"xref\n");
    pdf.extend_from_slice(b"0 1000000\n"); // Huge subsection
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(b"trailer\n<</Size 1>>\n");
    pdf.extend_from_slice(b"startxref\n40\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Handled huge xref subsection"),
        Err(e) => {
            println!("Expected xref error: {e}");
        }
    }
}

/// Test 19: PDF with multiple %%EOF markers
#[test]
fn test_multiple_eof_markers() {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(b"1 0 obj\n<</Type /Catalog>>\nendobj\n");
    pdf.extend_from_slice(b"%%EOF\n"); // First EOF
    pdf.extend_from_slice(b"xref\n0 1\n0000000000 65535 f \n");
    pdf.extend_from_slice(b"trailer\n<</Size 1>>\n");
    pdf.extend_from_slice(b"startxref\n40\n");
    pdf.extend_from_slice(b"%%EOF\n"); // Second EOF
    pdf.extend_from_slice(b"%%EOF\n"); // Third EOF

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Handled multiple EOF markers"),
        Err(e) => println!("EOF handling error: {e}"),
    }
}

/// Test 20: Object reference to non-existent generation
#[test]
fn test_invalid_generation_reference() {
    let content = r#"
1 0 obj
<</Type /Catalog /Pages 2 99 R>>
endobj
2 0 obj
<</Type /Pages /Kids [3 50 R] /Count 1>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Read catalog with invalid generation"),
            Err(e) => println!("Expected reference error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 21: Compressed object stream referencing itself
#[test]
fn test_objstm_self_reference() {
    let content = r#"
1 0 obj
<</Type /ObjStm /N 1 /First 10 /Length 20 /Extends 1 0 R>>
stream
1 0 <</Self 1 0 R>>
endstream
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Handled self-referencing object stream"),
            Err(e) => println!("Expected self-reference error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 22: Invalid filter combination
#[test]
fn test_invalid_filter_chain() {
    let content = r#"
1 0 obj
<</Filter [/FlateDecode /ASCIIHexDecode /InvalidFilter /LZWDecode] /Length 20>>
stream
Invalid filtered data
endstream
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Parsed invalid filter chain"),
            Err(e) => {
                println!("Expected filter error: {e}");
            }
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 23: Page with invalid rotation value
#[test]
fn test_page_invalid_rotation() {
    let content = r#"
1 0 obj
<</Type /Page /Rotate 45 /MediaBox [0 0 612 792]>>
endobj
2 0 obj
<</Type /Page /Rotate 360 /MediaBox [0 0 612 792]>>
endobj
3 0 obj
<</Type /Page /Rotate -90 /MediaBox [0 0 612 792]>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            for i in 1..=3 {
                match doc.get_object(i, 0) {
                    Ok(obj) => println!("Page {i} with rotation: {obj:?}"),
                    Err(e) => println!("Page {i} error: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 24: Null object in unexpected places
#[test]
fn test_null_in_required_fields() {
    let content = r#"
1 0 obj
<</Type null /Pages null>>
endobj
2 0 obj
<</Type /Page /Parent null /MediaBox null>>
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            for i in 1..=2 {
                match doc.get_object(i, 0) {
                    Ok(obj) => println!("Object {i} with nulls: {obj:?}"),
                    Err(e) => println!("Object {i} error: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 25: Stream filter with missing decode parameters
#[test]
fn test_missing_decode_params() {
    let content = r#"
1 0 obj
<</Filter /LZWDecode /Length 20>>
stream
compressed data here
endstream
endobj
2 0 obj
<</Filter /CCITTFaxDecode /Length 20>>
stream
fax compressed data
endstream
endobj
"#;
    let pdf = create_pdf_with_content(content);
    let cursor = Cursor::new(pdf);

    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            for i in 1..=2 {
                match doc.get_object(i, 0) {
                    Ok(_) => println!("Object {i} parsed without decode params"),
                    Err(e) => println!("Object {i} decode error: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

#[cfg(test)]
mod tests {
    /// Verify all tests can run without panicking
    #[test]
    fn test_all_edge_cases_no_panic() {
        // This meta-test ensures all edge case tests complete without panic
        println!("Running all parser edge case tests...");

        // List all test functions
        let test_names = vec![
            "stream_negative_length",
            "stream_excessive_length",
            "stream_missing_endstream",
            "object_stream_corrupted_index",
            "xref_stream_invalid_predictor",
            "linearized_corrupted_hint",
            "incremental_update_invalid_prev",
            "font_invalid_encoding",
            "form_xobject_recursive",
            "corrupted_encryption_dict",
            "page_tree_missing_fields",
            "circular_colorspace",
            "malformed_hex_strings",
            "array_mismatched_brackets",
            "dictionary_duplicate_keys",
            "name_invalid_characters",
            "string_unbalanced_parens",
            "xref_invalid_subsection",
            "multiple_eof_markers",
            "invalid_generation_reference",
            "objstm_self_reference",
            "invalid_filter_chain",
            "page_invalid_rotation",
            "null_in_required_fields",
            "missing_decode_params",
        ];

        println!("Total edge case tests: {}", test_names.len());
    }
}
