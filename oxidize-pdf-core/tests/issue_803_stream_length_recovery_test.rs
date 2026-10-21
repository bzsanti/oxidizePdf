//! Regression contracts for bounded recovery of an overdeclared stream length.
use oxidize_pdf::parser::{lexer::Lexer, ParseOptions, PdfObject};
use std::io::Cursor;

fn input(content: &[u8], length: usize, terminator: &[u8]) -> Vec<u8> {
    let mut bytes = format!("<< /Length {length} >>\nstream\n").into_bytes();
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(terminator);
    bytes.extend_from_slice(b"\n9 0 obj\n(untouched next object)\nendobj\n");
    bytes
}

#[test]
fn overdeclared_length_recovers_partial_marker_and_preserves_next_object() {
    for consumed in 1..9 {
        for content in [b"Hello\n".as_slice(), b"Hello\r\n".as_slice()] {
            let bytes = input(content, content.len() + consumed, b"endstream\nendobj");
            let mut lexer = Lexer::new(Cursor::new(bytes));
            let object =
                PdfObject::parse_with_options(&mut lexer, &ParseOptions::tolerant()).unwrap();
            let PdfObject::Stream(stream) = object else {
                panic!("expected stream")
            };
            assert_eq!(stream.data, content);
            assert_eq!(format!("{:?}", lexer.next_token().unwrap()), "EndObj");
            assert_eq!(PdfObject::parse(&mut lexer).unwrap(), PdfObject::Integer(9));
        }
    }
}

#[test]
fn strict_rejects_overdeclared_length() {
    let mut lexer = Lexer::new(Cursor::new(input(b"Hello\n", 10, b"endstream\nendobj")));
    assert!(PdfObject::parse_with_options(&mut lexer, &ParseOptions::strict()).is_err());
}

#[test]
fn recovery_respects_backward_distance_budget() {
    let options = ParseOptions {
        max_recovery_bytes: 2,
        ..ParseOptions::tolerant()
    };
    let mut lexer = Lexer::new(Cursor::new(input(b"Hello\n", 10, b"endstream\nendobj")));
    assert!(PdfObject::parse_with_options(&mut lexer, &options).is_err());
}

#[test]
fn valid_length_preserves_embedded_marker_bytes() {
    let content = b"prefix\nendstream\nendobj\nsuffix\n";
    let mut lexer = Lexer::new(Cursor::new(input(
        content,
        content.len(),
        b"endstream\nendobj",
    )));
    let PdfObject::Stream(stream) = PdfObject::parse(&mut lexer).unwrap() else {
        panic!("expected stream")
    };
    assert_eq!(stream.data, content);
}

#[test]
fn recovery_rejects_marker_without_object_termination() {
    let mut lexer = Lexer::new(Cursor::new(input(b"Hello\n", 10, b"endstream\nnot_endobj")));
    assert!(PdfObject::parse_with_options(&mut lexer, &ParseOptions::tolerant()).is_err());
}

#[test]
fn recovery_uses_absolute_reader_offset_after_object_seek() {
    let mut bytes = vec![b' '; 97];
    bytes.extend(input(b"Hello\n", 10, b"endstream\nendobj"));
    let mut cursor = Cursor::new(bytes);
    cursor.set_position(97);
    let mut lexer = Lexer::new(cursor);
    let PdfObject::Stream(stream) =
        PdfObject::parse_with_options(&mut lexer, &ParseOptions::tolerant()).unwrap()
    else {
        panic!("expected stream")
    };
    assert_eq!(stream.data, b"Hello\n");
}
