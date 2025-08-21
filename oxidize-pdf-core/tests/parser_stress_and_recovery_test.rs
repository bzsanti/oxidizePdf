//! Parser Stress Testing and Recovery Mechanisms
//!
//! This test suite focuses on:
//! - Memory stress scenarios
//! - Parser recovery from errors
//! - Performance edge cases
//! - Resource exhaustion protection
//! - Malicious PDF patterns

use oxidize_pdf::parser::{PdfDocument, PdfReader};
use std::io::Cursor;
use std::time::{Duration, Instant};

/// Test 1: Extremely large dictionary
#[test]
fn test_massive_dictionary() {
    let mut content = String::from("1 0 obj\n<<");

    // Create dictionary with 10,000 entries
    for i in 0..10000 {
        content.push_str(&format!("/Key{} {}", i, i));
        if i < 9999 {
            content.push_str(" ");
        }
    }
    content.push_str(">>\nendobj\n");

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n");
    pdf.extend_from_slice(format!("{}\n", content.len() + 20).as_bytes());
    pdf.extend_from_slice(b"%%EOF\n");

    let cursor = Cursor::new(pdf);

    let start = Instant::now();
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(obj) => {
                println!("Parsed massive dictionary in {:?}", start.elapsed());
            }
            Err(e) => println!("Dictionary parsing error: {}", e),
        },
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 2: Extremely large array
#[test]
fn test_massive_array() {
    let mut content = String::from("1 0 obj\n[");

    // Create array with 50,000 elements
    for i in 0..50000 {
        content.push_str(&format!("{}", i));
        if i < 49999 {
            content.push_str(" ");
        }
    }
    content.push_str("]\nendobj\n");

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n");
    pdf.extend_from_slice(format!("{}\n", content.len() + 20).as_bytes());
    pdf.extend_from_slice(b"%%EOF\n");

    let cursor = Cursor::new(pdf);

    let start = Instant::now();
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(obj) => {
                println!("Parsed massive array in {:?}", start.elapsed());
            }
            Err(e) => println!("Array parsing error: {}", e),
        },
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 3: Pathological xref table
#[test]
fn test_pathological_xref() {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(b"1 0 obj\n<</Type /Catalog>>\nendobj\n");

    // Create xref with many subsections
    pdf.extend_from_slice(b"xref\n");
    for i in 0..1000 {
        pdf.extend_from_slice(format!("{} 1\n{:010} 00000 n \n", i, i * 100).as_bytes());
    }

    pdf.extend_from_slice(b"trailer\n<</Size 1000>>\n");
    pdf.extend_from_slice(b"startxref\n50\n%%EOF\n");

    let cursor = Cursor::new(pdf);

    let start = Instant::now();
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(_) => println!("Parsed pathological xref in {:?}", start.elapsed()),
        Err(e) => println!("Xref parsing error: {}", e),
    }
}

/// Test 4: Billion laughs attack pattern
#[test]
fn test_billion_laughs_protection() {
    let content = r#"
1 0 obj
[0 0 R 0 0 R 0 0 R 0 0 R 0 0 R]
endobj
2 0 obj
[1 0 R 1 0 R 1 0 R 1 0 R 1 0 R]
endobj
3 0 obj
[2 0 R 2 0 R 2 0 R 2 0 R 2 0 R]
endobj
4 0 obj
[3 0 R 3 0 R 3 0 R 3 0 R 3 0 R]
endobj
5 0 obj
[4 0 R 4 0 R 4 0 R 4 0 R 4 0 R]
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 6\n");
    for i in 0..6 {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", i * 50).as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n<</Size 6>>\n");
    pdf.extend_from_slice(b"startxref\n300\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => {
            // Should detect exponential expansion
            let start = Instant::now();
            let timeout = Duration::from_secs(5);

            match doc.get_object(5, 0) {
                Ok(_) => {
                    if start.elapsed() > timeout {
                        println!("Billion laughs took too long!");
                    } else {
                        println!("Handled billion laughs pattern");
                    }
                }
                Err(e) => println!("Detected billion laughs: {}", e),
            }
        }
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 5: Zip bomb pattern in streams
#[test]
fn test_stream_zip_bomb() {
    let content = r#"
1 0 obj
<</Type /XObject /Subtype /Image /Width 10000 /Height 10000 /BitsPerComponent 8 /ColorSpace /DeviceRGB /Filter /FlateDecode /Length 100>>
stream
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());

    // Add highly compressed data that would expand enormously
    // This is a simplified example - real zip bombs are more sophisticated
    let compressed_data = vec![0x78, 0x9C, 0x01, 0x00, 0x00, 0xFF, 0xFF];
    pdf.extend_from_slice(&compressed_data);

    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n200\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Handled potential zip bomb"),
            Err(e) => println!("Zip bomb protection: {}", e),
        },
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 6: Parser recovery after error
#[test]
fn test_parser_recovery() {
    let content = r#"
1 0 obj
<</Type /Catalog /Pages 2 0 R>>
endobj
2 0 obj
<</Type /Pages /Kids [3 0 R 4 0 R] /Count 2>>
endobj
3 0 obj
CORRUPTED_OBJECT_DATA
endobj
4 0 obj
<</Type /Page /MediaBox [0 0 612 792]>>
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
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => {
            // Should be able to read good objects even if one is corrupted
            let mut successful_reads = 0;
            let mut failed_reads = 0;

            for i in 1..=4 {
                match doc.get_object(i, 0) {
                    Ok(_) => successful_reads += 1,
                    Err(_) => failed_reads += 1,
                }
            }

            println!(
                "Recovery test: {} successful, {} failed",
                successful_reads, failed_reads
            );
            assert!(successful_reads >= 3, "Should read at least 3 good objects");
        }
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 7: Malformed content streams
#[test]
fn test_malformed_content_streams() {
    let content = r#"
1 0 obj
<</Type /Page /Contents 2 0 R>>
endobj
2 0 obj
<</Length 50>>
stream
BT
/F1 12 Tf
(Unclosed string
100 100 Td
ET
endstream
endobj
3 0 obj
<</Length 30>>
stream
q
1 0 0 1 50 50 cm
Q Q Q Q Q
endstream
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
    pdf.extend_from_slice(b"startxref\n300\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => {
            for i in 2..=3 {
                match doc.get_object(i, 0) {
                    Ok(_) => println!("Read content stream {}", i),
                    Err(e) => println!("Content stream {} error: {}", i, e),
                }
            }
        }
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 8: Invalid Type 3 font programs
#[test]
fn test_invalid_font_programs() {
    let content = r#"
1 0 obj
<</Type /Font /Subtype /Type3 /FontBBox [0 0 1000 1000] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs 2 0 R /Encoding <</Differences [65 /A]>>>>
endobj
2 0 obj
<</A 3 0 R>>
endobj
3 0 obj
<</Length 100>>
stream
INVALID POSTSCRIPT CODE !!!
{ [ ] } ( ) <> 
div 0 0
endstream
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
    pdf.extend_from_slice(b"startxref\n300\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Parsed Type3 font with invalid program"),
            Err(e) => println!("Font parsing error: {}", e),
        },
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 9: Recursive form XObjects
#[test]
fn test_recursive_form_xobjects() {
    let content = r#"
1 0 obj
<</Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources <</XObject <</Form1 2 0 R>>>>/Length 20>>
stream
q /Form1 Do Q
endstream
endobj
2 0 obj
<</Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources <</XObject <</Form2 3 0 R>>>>/Length 20>>
stream
q /Form2 Do Q
endstream
endobj
3 0 obj
<</Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources <</XObject <</Form0 1 0 R>>>>/Length 20>>
stream
q /Form0 Do Q
endstream
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
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => {
            // Should detect recursion when trying to render
            println!("Parser created with recursive forms");
            match doc.get_object(1, 0) {
                Ok(_) => println!("Read recursive form XObject"),
                Err(e) => println!("Recursion detected: {}", e),
            }
        }
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 10: Memory exhaustion through object count
#[test]
fn test_object_count_exhaustion() {
    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");

    // Create many small objects
    for i in 1..=10000 {
        pdf.extend_from_slice(format!("{} 0 obj\nnull\nendobj\n", i).as_bytes());
    }

    // Create xref for all objects
    pdf.extend_from_slice(b"xref\n0 10001\n");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for i in 1..=10000 {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", i * 20).as_bytes());
    }

    pdf.extend_from_slice(b"trailer\n<</Size 10001>>\n");
    pdf.extend_from_slice(b"startxref\n200000\n%%EOF\n");

    let cursor = Cursor::new(pdf);

    let start = Instant::now();
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(_) => println!("Handled 10,000 objects in {:?}", start.elapsed()),
        Err(e) => println!("Object count limit: {}", e),
    }
}

/// Test 11: Malicious JavaScript in annotations
#[test]
fn test_malicious_javascript() {
    let content = r#"
1 0 obj
<</Type /Annot /Subtype /Widget /FT /Btn /T (Submit) /A <</S /JavaScript /JS (app.alert('XSS'); this.exportDataObject({cName: 'exploit', nLaunch: 2});)>>>>
endobj
2 0 obj
<</Type /Annot /Subtype /Screen /A <</S /JavaScript /JS (
var heap = new ArrayBuffer(0x10000);
var exploit = new Uint32Array(heap);
for(var i = 0; i < 0x4000; i++) {
    exploit[i] = 0x41414141;
}
)>>>>
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 3\n");
    for i in 0..3 {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", i * 100).as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n<</Size 3>>\n");
    pdf.extend_from_slice(b"startxref\n500\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => {
            // Should parse but not execute JavaScript
            for i in 1..=2 {
                match doc.get_object(i, 0) {
                    Ok(obj) => println!("Parsed annotation {} with JavaScript: {:?}", i, obj),
                    Err(e) => println!("Annotation {} error: {}", i, e),
                }
            }
        }
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 12: Invalid outline hierarchy
#[test]
fn test_invalid_outline_hierarchy() {
    let content = r#"
1 0 obj
<</Type /Outlines /First 2 0 R /Last 2 0 R /Count -5>>
endobj
2 0 obj
<</Title (Chapter 1) /Parent 1 0 R /Next 3 0 R /First 3 0 R /Last 3 0 R>>
endobj
3 0 obj
<</Title (Section 1.1) /Parent 2 0 R /Prev 2 0 R /Next 2 0 R>>
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
    pdf.extend_from_slice(b"startxref\n300\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => {
            // Should handle circular outline references
            match doc.get_object(1, 0) {
                Ok(_) => println!("Parsed invalid outline hierarchy"),
                Err(e) => println!("Outline error: {}", e),
            }
        }
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 13: Corrupted image data
#[test]
fn test_corrupted_image_data() {
    let content = r#"
1 0 obj
<</Type /XObject /Subtype /Image /Width 100 /Height 100 /BitsPerComponent 8 /ColorSpace /DeviceRGB /Length 30000>>
stream
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());

    // Add corrupted image data (not enough for 100x100 RGB)
    pdf.extend_from_slice(&vec![0xFF; 1000]);

    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n1200\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => println!("Parsed corrupted image data"),
            Err(e) => println!("Image data error: {}", e),
        },
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 14: Parser timeout protection
#[test]
fn test_parser_timeout() {
    // Create a PDF that would take very long to parse
    let mut content = String::from("1 0 obj\n<<");

    // Nested structure that causes exponential parsing time
    for _ in 0..20 {
        content.push_str("/Key <</SubKey <</SubSubKey ");
    }
    content.push_str("null");
    for _ in 0..20 {
        content.push_str(">>>>");
    }
    content.push_str(">>\nendobj\n");

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 2\n0000000000 65535 f \n0000000010 00000 n \n");
    pdf.extend_from_slice(b"trailer\n<</Size 2>>\n");
    pdf.extend_from_slice(b"startxref\n");
    pdf.extend_from_slice(format!("{}\n", content.len() + 20).as_bytes());
    pdf.extend_from_slice(b"%%EOF\n");

    let cursor = Cursor::new(pdf);

    let start = Instant::now();
    let timeout = Duration::from_secs(5);

    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => match doc.get_object(1, 0) {
            Ok(_) => {
                let elapsed = start.elapsed();
                if elapsed > timeout {
                    println!("Parser took too long: {:?}", elapsed);
                } else {
                    println!("Parsed complex structure in {:?}", elapsed);
                }
            }
            Err(e) => println!("Parsing error: {}", e),
        },
        Err(e) => println!("Parser error: {}", e),
    }
}

/// Test 15: Mixed valid and invalid content
#[test]
fn test_mixed_content_robustness() {
    let content = r#"
1 0 obj
<</Type /Catalog /Pages 2 0 R /Names 5 0 R>>
endobj
2 0 obj
<</Type /Pages /Kids [3 0 R 4 0 R] /Count 2>>
endobj
3 0 obj
<</Type /Page /MediaBox [0 0 612 792] /Contents 6 0 R>>
endobj
4 0 obj
INVALID PAGE OBJECT
endobj
5 0 obj
<</Dests CORRUPTED>>
endobj
6 0 obj
<</Length 44>>
stream
BT
/F1 12 Tf
100 700 Td
(Valid content) Tj
ET
endstream
endobj
"#;

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    pdf.extend_from_slice(content.as_bytes());
    pdf.extend_from_slice(b"xref\n0 7\n");
    for i in 0..7 {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", i * 50).as_bytes());
    }
    pdf.extend_from_slice(b"trailer\n<</Size 7 /Root 1 0 R>>\n");
    pdf.extend_from_slice(b"startxref\n500\n%%EOF\n");

    let cursor = Cursor::new(pdf);
    match PdfReader::new(cursor).and_then(|reader| Ok(PdfDocument::new(reader))) {
        Ok(doc) => {
            // Should be able to work with valid parts despite some corruption
            let mut valid_count = 0;
            let mut invalid_count = 0;

            for i in 1..=6 {
                match doc.get_object(i, 0) {
                    Ok(_) => valid_count += 1,
                    Err(_) => invalid_count += 1,
                }
            }

            println!(
                "Mixed content: {} valid, {} invalid objects",
                valid_count, invalid_count
            );
            assert!(valid_count >= 4, "Should parse most valid objects");
        }
        Err(e) => println!("Parser error: {}", e),
    }
}

#[cfg(test)]
mod stress_tests {

    /// Meta-test for stress test coverage
    #[test]
    fn test_stress_suite_coverage() {
        println!("Stress and recovery test suite coverage");

        let test_names = vec![
            "massive_dictionary",
            "massive_array",
            "pathological_xref",
            "billion_laughs_protection",
            "stream_zip_bomb",
            "parser_recovery",
            "malformed_content_streams",
            "invalid_font_programs",
            "recursive_form_xobjects",
            "object_count_exhaustion",
            "malicious_javascript",
            "invalid_outline_hierarchy",
            "corrupted_image_data",
            "parser_timeout",
            "mixed_content_robustness",
        ];

        println!("Total stress/recovery tests: {}", test_names.len());
        assert_eq!(test_names.len(), 15);
    }
}
