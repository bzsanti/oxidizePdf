//! Issue #609 — Literal string escaped line break (`\` + EOL) not stripped per ISO 32000-1 §7.3.4.2.
//!
//! ISO 32000-1:2008 §7.3.4.2 (Literal Strings):
//! "An end-of-line marker preceded by a reverse solidus shall not be considered a part of the string."
//! An end-of-line marker may be CR, LF, or CRLF.

use std::io::Cursor;

use oxidize_pdf::parser::lexer::{Lexer, Token};
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use oxidize_pdf::text::TextExtractor;

#[test]
fn test_lexer_escaped_line_feed() {
    let input = b"(Hello\\\nWorld)";
    let mut lexer = Lexer::new(Cursor::new(input));
    let token = lexer.next_token().unwrap();
    assert_eq!(token, Token::String(b"HelloWorld".to_vec()));
}

#[test]
fn test_lexer_escaped_carriage_return() {
    let input = b"(Hello\\\rWorld)";
    let mut lexer = Lexer::new(Cursor::new(input));
    let token = lexer.next_token().unwrap();
    assert_eq!(token, Token::String(b"HelloWorld".to_vec()));
}

#[test]
fn test_lexer_escaped_crlf() {
    let input = b"(Hello\\\r\nWorld)";
    let mut lexer = Lexer::new(Cursor::new(input));
    let token = lexer.next_token().unwrap();
    assert_eq!(token, Token::String(b"HelloWorld".to_vec()));
}

#[test]
fn test_lexer_escaped_multiple_line_breaks_iso_example() {
    // ISO 32000-1 §7.3.4.2 Example: (These \ \n two strings \ \n are the same.)
    let input = b"(These \\\ntwo strings \\\nare the same.)";
    let mut lexer = Lexer::new(Cursor::new(input));
    let token = lexer.next_token().unwrap();
    assert_eq!(
        token,
        Token::String(b"These two strings are the same.".to_vec())
    );
}

#[test]
fn test_pdf_extraction_escaped_crlf() {
    let pdf_data = b"%PDF-1.4
1 0 obj <</Type /Catalog /Pages 2 0 R>> endobj
2 0 obj <</Type /Pages /Kids [3 0 R] /Count 1>> endobj
3 0 obj <</Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj
4 0 obj <</Length 55>> stream
BT
/F1 12 Tf
100 700 Td
(Docu\\\r\nment) Tj
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
0000000373 00000 n 
trailer <</Size 6 /Root 1 0 R>>
startxref
449
%%EOF";

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::lenient())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "Document");
}

#[test]
fn test_pdf_extraction_escaped_lf() {
    let pdf_data = b"%PDF-1.4
1 0 obj <</Type /Catalog /Pages 2 0 R>> endobj
2 0 obj <</Type /Pages /Kids [3 0 R] /Count 1>> endobj
3 0 obj <</Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj
4 0 obj <</Length 53>> stream
BT
/F1 12 Tf
100 700 Td
(Docu\\\nment) Tj
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
0000000373 00000 n 
trailer <</Size 6 /Root 1 0 R>>
startxref
447
%%EOF";

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::lenient())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "Document");
}
