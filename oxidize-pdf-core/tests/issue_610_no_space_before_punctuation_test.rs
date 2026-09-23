//! Issue #610 — Avoid synthesizing space before punctuation in fragmented Tj operator runs.
//!
//! When tokens (such as IP addresses, version numbers, or decimal numbers) are split across
//! separate Tj operators with small pen advances, micro-spacing must not cause a synthetic
//! space to be inserted before punctuation like '.' or ','.

use std::io::Cursor;

use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;

#[test]
fn test_no_synthetic_space_before_period_across_tj_boundary() {
    let pdf_data = b"%PDF-1.4
1 0 obj <</Type /Catalog /Pages 2 0 R>> endobj
2 0 obj <</Type /Pages /Kids [3 0 R] /Count 1>> endobj
3 0 obj <</Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /T1 5 0 R >> >> >> endobj
4 0 obj <</Length 180>> stream
BT
/T1 1 Tf
7.7 0 0 7.7 100.0 700.0 Tm
(179.1) Tj
7.5 0 0 7.7 120.0 700.0 Tm
(91) Tj
7.7 0 0 7.7 130.5 700.0 Tm
(.) Tj
7.7 0 0 7.7 133.0 700.0 Tm
(127.102) Tj
ET
endstream endobj
5 0 obj <</Type /Font /Subtype /Type1 /BaseFont /Helvetica>> endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000266 00000 n 
0000000498 00000 n 
trailer <</Size 6 /Root 1 0 R>>
startxref
574
%%EOF";

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::lenient())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "179.191.127.102");
}

#[test]
fn test_no_synthetic_space_before_comma_across_tj_boundary() {
    let pdf_data = b"%PDF-1.4
1 0 obj <</Type /Catalog /Pages 2 0 R>> endobj
2 0 obj <</Type /Pages /Kids [3 0 R] /Count 1>> endobj
3 0 obj <</Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /T1 5 0 R >> >> >> endobj
4 0 obj <</Length 95>> stream
BT
/T1 10 Tf
100.0 700.0 Td
(Hello) Tj
28.0 0.0 Td
(,) Tj
10.0 0.0 Td
(World) Tj
ET
endstream endobj
5 0 obj <</Type /Font /Subtype /Type1 /BaseFont /Helvetica>> endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000266 00000 n 
0000000413 00000 n 
trailer <</Size 6 /Root 1 0 R>>
startxref
489
%%EOF";

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::lenient())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "Hello, World");
}

#[test]
fn test_no_synthetic_space_before_colon_across_tj_boundary() {
    let pdf_data = b"%PDF-1.4
1 0 obj <</Type /Catalog /Pages 2 0 R>> endobj
2 0 obj <</Type /Pages /Kids [3 0 R] /Count 1>> endobj
3 0 obj <</Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /T1 5 0 R >> >> >> endobj
4 0 obj <</Length 95>> stream
BT
/T1 10 Tf
100.0 700.0 Td
(Note) Tj
25.0 0.0 Td
(:) Tj
10.0 0.0 Td
(value) Tj
ET
endstream endobj
5 0 obj <</Type /Font /Subtype /Type1 /BaseFont /Helvetica>> endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000266 00000 n 
0000000413 00000 n 
trailer <</Size 6 /Root 1 0 R>>
startxref
489
%%EOF";

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::lenient())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "Note: value");
}

#[test]
fn test_no_synthetic_space_before_closing_paren_across_tj_boundary() {
    let pdf_data = b"%PDF-1.4
1 0 obj <</Type /Catalog /Pages 2 0 R>> endobj
2 0 obj <</Type /Pages /Kids [3 0 R] /Count 1>> endobj
3 0 obj <</Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /T1 5 0 R >> >> >> endobj
4 0 obj <</Length 95>> stream
BT
/T1 10 Tf
100.0 700.0 Td
(\\(inside) Tj
35.0 0.0 Td
(\\)) Tj
10.0 0.0 Td
(after) Tj
ET
endstream endobj
5 0 obj <</Type /Font /Subtype /Type1 /BaseFont /Helvetica>> endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000266 00000 n 
0000000413 00000 n 
trailer <</Size 6 /Root 1 0 R>>
startxref
489
%%EOF";

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::lenient())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "(inside) after");
}

#[test]
fn test_preserves_space_between_words_across_tj_boundary() {
    let pdf_data = b"%PDF-1.4
1 0 obj <</Type /Catalog /Pages 2 0 R>> endobj
2 0 obj <</Type /Pages /Kids [3 0 R] /Count 1>> endobj
3 0 obj <</Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /T1 5 0 R >> >> >> endobj
4 0 obj <</Length 95>> stream
BT
/T1 10 Tf
100.0 700.0 Td
(Hello) Tj
32.0 0.0 Td
(World) Tj
ET
endstream endobj
5 0 obj <</Type /Font /Subtype /Type1 /BaseFont /Helvetica>> endobj
xref
0 6
0000000000 65535 f 
0000000009 00000 n 
0000000058 00000 n 
0000000115 00000 n 
0000000266 00000 n 
0000000413 00000 n 
trailer <</Size 6 /Root 1 0 R>>
startxref
489
%%EOF";

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::lenient())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "Hello World");
}
