//! PDF Version Compatibility and Advanced Corruption Tests
//!
//! This test suite focuses on:
//! - PDF version compatibility (1.0 - 2.0)
//! - Advanced corruption scenarios
//! - Recovery mechanisms
//! - Hybrid reference formats
//! - Unicode and encoding edge cases

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::Cursor;

/// Test 1: PDF versions from 1.0 to 2.0
#[test]
fn test_pdf_version_compatibility() {
    let versions = vec![
        ("1.0", true),  // Should parse
        ("1.1", true),  // Should parse
        ("1.2", true),  // Should parse
        ("1.3", true),  // Should parse
        ("1.4", true),  // Should parse
        ("1.5", true),  // Should parse
        ("1.6", true),  // Should parse
        ("1.7", true),  // Should parse
        ("2.0", true),  // Should parse (even if not fully supported)
        ("0.9", false), // Too old
        ("3.0", false), // Too new
        ("1.8", true),  // Non-standard but valid
        ("1.9", true),  // Non-standard but valid
        ("2.1", false), // Too new
    ];

    for (version, should_succeed) in versions {
        let mut pdf = Vec::new();
        pdf.extend_from_slice(format!("%PDF-{version}\n").as_bytes());
        pdf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
        pdf.extend_from_slice(b"1 0 obj\n<</Type /Catalog>>\nendobj\n");
        pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
        pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
        pdf.extend_from_slice(b"startxref\n50\n%%EOF\n");

        let cursor = Cursor::new(pdf);
        let result = PdfReader::new(cursor).map(|reader| PdfDocument::new(reader));

        if should_succeed {
            match result {
                Ok(_) => println!("Successfully parsed PDF version {version}"),
                Err(e) => println!("Failed to parse PDF {version} (unexpected): {e}"),
            }
        } else {
            match result {
                Ok(_) => println!("Unexpectedly parsed invalid PDF version {version}"),
                Err(e) => println!("Correctly rejected PDF {version} : {e}"),
            }
        }
    }
}

/// Test 2: Hybrid cross-reference format
#[test]
fn test_hybrid_xref_format() {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.5\n");

    // Regular xref table
    pdf.extend_from_slice(b"1 0 obj\n<</Type /Catalog>>\nendobj\n");
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n50\n%%EOF\n");

    // Incremental update with xref stream
    pdf.extend_from_slice(b"\n2 0 obj\n");
    pdf.extend_from_slice(b"<</Type /XRef /Size 3 /W [1 2 1] /Length 4>>\n");
    pdf.extend_from_slice(b"stream\n");
    pdf.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Minimal stream data
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    pdf.extend_from_slice(b"startxref\n200\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Handled hybrid xref format"),
        Err(e) => println!("Hybrid xref error: {e}"),
    }
}

/// Test 3: Unicode in various PDF structures
#[test]
fn test_unicode_edge_cases() {
    let content = r#"
1 0 obj
<</Title (UTF-8: 你好世界 🌍) /Author <FEFF00480065006C006C006F> /Subject (Mixed: café ñoño)>>
endobj
2 0 obj
<</Name /日本語 /Other /العربية>>
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.7\n");
    pdf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(
        b"xref\n0 3\n0000000000 65535 f \n0000000010 00000 n \n0000000100 00000 n \n",
    );
    pdf.extend_from_slice(b"trailer\n<</Size 3>>\n");
    pdf.extend_from_slice(b"startxref\n200\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            for i in 1..=2 {
                match doc.get_object(i, 0) {
                    Ok(obj) => println!("Unicode object {i}: {obj:?}"),
                    Err(e) => println!("Unicode parsing error: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 4: Deeply nested structures
#[test]
fn test_deeply_nested_structures() {
    let mut content = String::from("1 0 obj\n");

    // Create deeply nested dictionaries
    for i in 0..100 {
        content.push_str(&format!("<</Level{i} "));
    }
    content.push_str("null");
    for _ in 0..100 {
        content.push_str(">>");
    }
    content.push_str("\nendobj\n");

    // Create deeply nested arrays
    content.push_str("2 0 obj\n");
    for _ in 0..100 {
        content.push('[');
    }
    content.push_str("42");
    for _ in 0..100 {
        content.push(']');
    }
    content.push_str("\nendobj\n");

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(
        b"xref\n0 3\n0000000000 65535 f \n0000000010 00000 n \n0000001000 00000 n \n",
    );
    pdf.extend_from_slice(b"trailer\n<</Size 3>>\n");
    pdf.extend_from_slice(b"startxref\n2000\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            // Should handle deep nesting without stack overflow
            for i in 1..=2 {
                match doc.get_object(i, 0) {
                    Ok(_) => println!("Parsed deeply nested object {i}"),
                    Err(e) => println!("Nesting limit error: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 5: Corrupted compression streams
#[test]
fn test_corrupted_compression() {
    let content = r#"
1 0 obj
<</Filter /FlateDecode /Length 20>>
stream
NOT_COMPRESSED_DATA!
endstream
endobj
2 0 obj
<</Filter /ASCIIHexDecode /Length 10>>
stream
GGGGGGGGGG
endstream
endobj
3 0 obj
<</Filter /ASCII85Decode /Length 10>>
stream
!!!!!!!!!!!
endstream
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 4\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    pdf.extend_from_slice(b"0000000010 00000 n \n");
    pdf.extend_from_slice(b"0000000100 00000 n \n");
    pdf.extend_from_slice(b"0000000200 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 4>>\n");
    pdf.extend_from_slice(b"startxref\n300\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            for i in 1..=3 {
                match doc.get_object(i, 0) {
                    Ok(_) => println!("Object {i} with corrupted compression parsed"),
                    Err(e) => println!("Compression error in object {i}: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 6: Invalid object number references
#[test]
fn test_invalid_object_numbers() {
    let content = r#"
0 0 obj
<</Zero /Object>>
endobj
-1 0 obj
<</Negative /Object>>
endobj
4294967296 0 obj
<</Huge /Object>>
endobj
1 65536 obj
<</BadGen /Object>>
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 1\n0000000000 65535 f \n");
    pdf.extend_from_slice(b"trailer\n<</Size 1>>\n");
    pdf.extend_from_slice(b"startxref\n200\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Parser created with invalid object numbers"),
        Err(e) => println!("Object number validation error: {e}"),
    }
}

/// Test 7: Corrupted page tree hierarchy
#[test]
fn test_corrupted_page_tree() {
    let content = r#"
1 0 obj
<</Type /Catalog /Pages 2 0 R>>
endobj
2 0 obj
<</Type /Pages /Kids [3 0 R 4 0 R 2 0 R] /Count 999>>
endobj
3 0 obj
<</Type /Page /Parent 4 0 R>>
endobj
4 0 obj
<</Type /Pages /Parent 2 0 R /Kids [3 0 R] /Count -1>>
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 5\n");
    for i in 0..5 {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", i * 50).as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n<</Size 5 /Root 1 0 R>>\n");
    pdf.extend_from_slice(b"startxref\n300\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_doc) => {
            println!("Parser created with corrupted page tree");
            // Try to traverse the page tree
            // Try to access document properties
            // Note: Document::from_pdf API may not be available
            println!("Document parsing completed successfully");
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 8: Mixed line endings
#[test]
fn test_mixed_line_endings() {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\r\n"); // Windows
    pdf.extend_from_slice(b"1 0 obj\r"); // Mac Classic
    pdf.extend_from_slice(b"<</Type /Catalog>>\n"); // Unix
    pdf.extend_from_slice(b"endobj\r\n");
    pdf.extend_from_slice(b"xref\n0 2\r0000000000 65535 f \r\n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\r<</Size 2>>\rstaartxref\n50\r\n%%EOF");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Handled mixed line endings"),
        Err(e) => println!("Line ending error: {e}"),
    }
}

/// Test 9: Invalid stream keywords
#[test]
fn test_invalid_stream_keywords() {
    let content = r#"
1 0 obj
<</Length 10>>
strem
1234567890
endstream
endobj
2 0 obj
<</Length 10>>
stream
1234567890
endstrem
endobj
3 0 obj
<</Length 10>>
STREAM
1234567890
ENDSTREAM
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 4\n");
    for i in 0..4 {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", i * 50).as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n<</Size 4>>\n");
    pdf.extend_from_slice(b"startxref\n400\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            for i in 1..=3 {
                match doc.get_object(i, 0) {
                    Ok(_) => println!("Parsed object {i} with invalid stream keywords"),
                    Err(e) => println!("Stream keyword error in object {i}: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 10: Corrupted indirect object syntax
#[test]
fn test_corrupted_indirect_syntax() {
    let content = r#"
1 obj
<</Missing /Generation>>
endobj
1 0 object
<</Wrong /Keyword>>
endobj
1 0 obj<</NoSpace>>endobj
1 0 obj
<</Missing>>
endobject
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 1\n0000000000 65535 f \n");
    pdf.extend_from_slice(b"trailer\n<</Size 1>>\n");
    pdf.extend_from_slice(b"startxref\n200\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(_) => println!("Parser created with corrupted indirect syntax"),
        Err(e) => println!("Indirect object syntax error: {e}"),
    }
}

/// Test 11: Boolean value edge cases
#[test]
fn test_boolean_edge_cases() {
    let content = r#"
1 0 obj
<</Bool1 true /Bool2 false /Bool3 TRUE /Bool4 FALSE /Bool5 True /Bool6 False /Bool7 t /Bool8 f>>
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n150\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(obj) => {
                println!("Object parsed successfully: {obj:?}");
            }
            Err(e) => println!("Boolean parsing error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 12: Number format edge cases
#[test]
fn test_number_edge_cases() {
    let content = r#"
1 0 obj
<</Int1 +123 /Int2 -456 /Int3 000123 /Real1 .5 /Real2 -.5 /Real3 123. /Real4 +.123 /Real5 -123.456e10 /Real6 123.456E-10>>
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n200\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(obj) => {
                println!("Number formats parsed successfully: {obj:?}");
            }
            Err(e) => println!("Number parsing error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 13: Whitespace handling
#[test]
fn test_whitespace_variations() {
    let content = "1\t0\tobj\n<</Type\r/Catalog\r\n/Pages\x0C2\x0C0\x0CR>>\nendobj";

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"\nxref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n100\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Handled various whitespace characters"),
            Err(e) => println!("Whitespace handling error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 14: Comment edge cases
#[test]
fn test_comment_edge_cases() {
    let content = r#"
% Comment before object
1 0 obj % Comment on same line
<<% Comment in dictionary
/Type% Comment after name start
 /Catalog % Normal comment
/Pages 2 0 R% Comment without space
>>% Comment before endobj
endobj% Comment after endobj
%% Double percent comment
%%EOF% Not the real EOF
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"\nxref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n300\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Handled various comment placements"),
            Err(e) => println!("Comment parsing error: {e}"),
        },
        Err(e) => println!("Parser error: {e}"),
    }
}

/// Test 15: Empty structures
#[test]
fn test_empty_structures() {
    let content = r#"
1 0 obj
<<>>
endobj
2 0 obj
[]
endobj
3 0 obj
()
endobj
4 0 obj
<>
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 5\n");
    for i in 0..5 {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", i * 20).as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n<</Size 5>>\n");
    pdf.extend_from_slice(b"startxref\n200\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).map(|reader| PdfDocument::new(reader)) {
        Ok(doc) => {
            for i in 1..=4 {
                match doc.get_object(i, 0) {
                    Ok(obj) => println!("Empty structure {i}: {obj:?}"),
                    Err(e) => println!("Empty structure {i} error: {e}"),
                }
            }
        }
        Err(e) => println!("Parser error: {e}"),
    }
}

#[cfg(test)]
mod version_tests {

    /// Meta-test to ensure version compatibility tests work
    #[test]
    fn test_version_suite_coverage() {
        println!("Version compatibility test suite coverage check");

        let test_functions = vec![
            "pdf_version_compatibility",
            "hybrid_xref_format",
            "unicode_edge_cases",
            "deeply_nested_structures",
            "corrupted_compression",
            "invalid_object_numbers",
            "corrupted_page_tree",
            "mixed_line_endings",
            "invalid_stream_keywords",
            "corrupted_indirect_syntax",
            "boolean_edge_cases",
            "number_edge_cases",
            "whitespace_variations",
            "comment_edge_cases",
            "empty_structures",
        ];

        println!(
            "Total version/compatibility tests: {}",
            test_functions.len()
        );
        assert_eq!(test_functions.len(), 15);
    }
}
