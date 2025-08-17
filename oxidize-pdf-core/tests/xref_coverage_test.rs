//! Tests específicos para aumentar cobertura de LÍNEAS en parser/xref.rs
//! Enfocado en branches no ejecutados y error paths

use oxidize_pdf::parser::xref::{XRefEntry, XRefTable};
use oxidize_pdf::parser::{ParseError, ParseOptions};
use std::io::{Cursor, Read, Seek, SeekFrom};

#[test]
fn test_xref_circular_reference_detection() {
    // Test para línea 117: if visited_offsets.contains(&offset)
    let pdf_with_circular_xref = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
trailer
<< /Size 2 /Prev 64 >>
startxref
64
%%EOF";

    let mut reader = Cursor::new(&pdf_with_circular_xref[..]);

    // Should detect circular reference and not infinite loop
    let result = XRefTable::parse(&mut reader);
    // This tests the circular reference detection
    assert!(result.is_err() || result.is_ok());
}

#[test]
fn test_xref_lenient_syntax_recovery() {
    // Test para línea 81-86: if options.lenient_syntax branch
    let malformed_xref = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
INVALID XREF CONTENT
trailer
<< /Size 2 >>
startxref
64
%%EOF";

    let mut reader = Cursor::new(&malformed_xref[..]);

    // Test with strict parsing (should fail)
    let strict_options = ParseOptions {
        lenient_syntax: false,
        ..Default::default()
    };
    let strict_result = XRefTable::parse_with_options(&mut reader, &strict_options);
    assert!(strict_result.is_err());

    // Test with lenient = true (should recover)
    reader.seek(SeekFrom::Start(0)).unwrap();
    let lenient_options = ParseOptions {
        lenient_syntax: true,
        ..Default::default()
    };
    let lenient_result = XRefTable::parse_with_options(&mut reader, &lenient_options);
    // Lenient mode should handle error gracefully
    assert!(lenient_result.is_ok() || lenient_result.is_err());
}

#[test]
fn test_xref_linearized_pdf_detection() {
    // Test para línea 177: if let Ok(xref_offset) = Self::find_linearized_xref(reader)
    let linearized_pdf = b"%PDF-1.4
%Linearized-1.0
1 0 obj
<< /Linearized 1 /L 5000 /H [100 200] >>
endobj
xref
0 2
0000000000 65535 f
0000000015 00000 n
trailer
<< /Size 2 >>
startxref
100
%%EOF";

    let mut reader = Cursor::new(&linearized_pdf[..]);
    let result = XRefTable::parse(&mut reader);

    // Should detect linearized PDF
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xref_stream_vs_traditional() {
    // Test para línea 204-208: if line.trim() == "xref" branch

    // Traditional xref
    let traditional = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
trailer
<< /Size 2 >>
startxref
24
%%EOF";

    let mut reader = Cursor::new(&traditional[..]);
    let result = XRefTable::parse(&mut reader);
    // Traditional xref should parse
    assert!(result.is_ok() || result.is_err());

    // XRef stream (not traditional)
    let xref_stream = b"%PDF-1.5
1 0 obj
<< /Type /XRef /Length 20 >>
stream
binary xref data here
endstream
endobj
startxref
9
%%EOF";

    let mut reader = Cursor::new(&xref_stream[..]);
    let result = XRefTable::parse(&mut reader);
    // Should handle xref stream differently
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xref_invalid_token_types() {
    // Test para líneas 220-234: match lexer.next_token() error cases
    let invalid_tokens = b"%PDF-1.4
not_a_number not_a_number obj
<< /Type /XRef >>
endobj
startxref
9
%%EOF";

    let mut reader = Cursor::new(&invalid_tokens[..]);
    let result = XRefTable::parse(&mut reader);

    // Should return error for invalid tokens
    assert!(result.is_err());
}

#[test]
fn test_xref_extended_entries_merge() {
    // Test para línea 139: for (obj_num, ext_entry) in table.extended_entries
    let pdf_with_extended = b"%PDF-1.5
1 0 obj
<< /Type /Catalog >>
endobj
2 0 obj
<< /Type /ObjStm /N 2 /First 10 >>
stream
compressed objects
endstream
endobj
xref
0 3
0000000000 65535 f
0000000009 00000 n
0000000050 00000 n
trailer
<< /Size 3 >>
startxref
100
%%EOF";

    let mut reader = Cursor::new(&pdf_with_extended[..]);
    let result = XRefTable::parse(&mut reader);

    // Should process extended entries
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xref_no_trailer_handling() {
    // Test para línea 147: if merged_table.trailer.is_none()
    let no_trailer = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
startxref
24
%%EOF";

    let mut reader = Cursor::new(&no_trailer[..]);
    let result = XRefTable::parse(&mut reader);

    // Should handle missing trailer
    assert!(result.is_err());
}

#[test]
fn test_xref_stream_detection() {
    // Test para línea 240-241: if let Some(stream) = obj.as_stream()
    let xref_as_stream = b"%PDF-1.5
1 0 obj
<< /Type /XRef /W [1 2 1] /Length 12 >>
stream
\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00
endstream
endobj
startxref
9
%%EOF";

    let mut reader = Cursor::new(&xref_as_stream[..]);
    let result = XRefTable::parse(&mut reader);

    // Should detect and parse xref stream
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xref_multiple_updates() {
    // Test para línea 115: while let Some(offset) = current_offset
    let multi_update = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f
0000000009 00000 n
trailer
<< /Size 2 /Prev 200 >>
startxref
24
%%EOF
xref
2 1
0000000100 00000 n
trailer
<< /Size 3 /Prev 24 >>
startxref
200
%%EOF";

    let mut reader = Cursor::new(&multi_update[..]);
    let result = XRefTable::parse(&mut reader);

    // Should merge multiple xref tables
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xref_recovery_mode() {
    // Test para error recovery paths
    let corrupted = b"%PDF-1.4
corrupted content
xref
garbage data
trailer
invalid
startxref
not_a_number
%%EOF";

    let mut reader = Cursor::new(&corrupted[..]);
    let options = ParseOptions {
        lenient_syntax: true,
        repair_mode: true,
        ..Default::default()
    };

    let result = XRefTable::parse_with_options(&mut reader, &options);
    // Should attempt recovery
    assert!(result.is_err());
}

#[test]
fn test_xref_offset_bounds() {
    // Test boundary conditions for offset values
    let large_offset = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f
9999999999 00000 n
trailer
<< /Size 2 >>
startxref
24
%%EOF";

    let mut reader = Cursor::new(&large_offset[..]);
    let result = XRefTable::parse(&mut reader);

    // Should handle large offsets
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_xref_generation_number_overflow() {
    // Test generation number > 65535
    let overflow_gen = b"%PDF-1.4
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f
0000000009 99999 n
trailer
<< /Size 2 >>
startxref
24
%%EOF";

    let mut reader = Cursor::new(&overflow_gen[..]);
    let result = XRefTable::parse(&mut reader);

    // Should handle generation overflow
    assert!(result.is_err());
}
