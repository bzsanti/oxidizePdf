//! Issue #609 — Literal string escaped line break (`\` + EOL) not stripped per ISO 32000-1 §7.3.4.2.
//!
//! ISO 32000-1:2008 §7.3.4.2 (Literal Strings):
//! "An end-of-line marker preceded by a reverse solidus shall not be considered a part of the string."
//! An end-of-line marker may be CR, LF, or CRLF.

mod common;
use common::pdf_assembler::{assemble_pdf, stream_obj};

fn text_pdf(content: &[u8]) -> Vec<u8> {
    assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R /T1 5 0 R >> >> >>".to_vec(),
        stream_obj("", content),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
    ])
}

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
    let pdf_data = text_pdf(
        b"BT
/F1 12 Tf
100 700 Td
(Docu\\\r\nment) Tj
ET",
    );

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::strict())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "Document");
}

#[test]
fn test_pdf_extraction_escaped_lf() {
    let pdf_data = text_pdf(
        b"BT
/F1 12 Tf
100 700 Td
(Docu\\\nment) Tj
ET",
    );

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::strict())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "Document");
}

#[test]
fn test_pdf_extraction_escaped_cr() {
    let pdf_data = text_pdf(
        b"BT
/F1 12 Tf
100 700 Td
(Docu\\\rment) Tj
ET",
    );

    let doc = PdfReader::new_with_options(Cursor::new(pdf_data), ParseOptions::strict())
        .unwrap()
        .into_document();
    let mut extractor = TextExtractor::new();
    let text = extractor.extract_from_page(&doc, 0).unwrap().text;
    assert_eq!(text.trim(), "Document");
}
