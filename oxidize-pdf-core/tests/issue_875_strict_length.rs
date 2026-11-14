use oxidize_pdf::parser::{ParseOptions, PdfObject, PdfReader};
use std::io::Cursor;

fn fixture(length: &str, value: &str, data: &[u8], ending: &str, header: &str) -> Vec<u8> {
    fixture_with_eol(length, value, data, ending, header, "\n")
}

fn fixture_with_eol(
    length: &str,
    value: &str,
    data: &[u8],
    ending: &str,
    header: &str,
    eol: &str,
) -> Vec<u8> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0];
    for (n, body) in [(1, "<< /Type /Catalog /Pages 2 0 R >>"), (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"), (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R >>")] {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{n} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("4 0 obj\n<< /Length {length} >>\nstream{eol}").as_bytes());
    pdf.extend_from_slice(data);
    pdf.extend_from_slice(ending.as_bytes());
    offsets.push(pdf.len());
    pdf.extend_from_slice(format!("{header} obj\n{value}\nendobj\n").as_bytes());
    let xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 6\n0000000000 65535 f \n");
    for offset in &offsets[1..] {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!("trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    pdf
}

#[test]
fn strict_indirect_length_with_and_without_preload() {
    for preload in [false, true] {
        let bytes = fixture("5 0 R", "6", b"BT ET\n", "endstream\nendobj\n", "5 0");
        let mut reader =
            PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
        if preload {
            assert_eq!(
                reader
                    .resolve_stream_length(&PdfObject::Reference(5, 0))
                    .unwrap(),
                Some(6)
            );
        }
        let PdfObject::Stream(stream) = reader.get_object(4, 0).expect("valid indirect Length")
        else {
            panic!("not a stream")
        };
        assert_eq!(stream.data, b"BT ET\n");
        assert_eq!(stream.dict.get("Length"), Some(&PdfObject::Reference(5, 0)));
        assert!(reader.get_object(3, 0).is_ok());
    }
}

#[test]
fn strict_length_does_not_scan_payload_markers() {
    let data = b"abc\nendstream\nendobj\nxyz\n";
    let bytes = fixture(
        "5 0 R",
        &data.len().to_string(),
        data,
        "endstream\nendobj\n",
        "5 0",
    );
    let mut reader =
        PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
    let PdfObject::Stream(stream) = reader.get_object(4, 0).unwrap() else {
        panic!("not a stream")
    };
    assert_eq!(stream.data, data);
}

#[test]
fn strict_invalid_lengths_fail_and_reader_recovers() {
    for (length, value, ending, header) in [
        ("5 0 R", "-1", "endstream\nendobj\n", "5 0"),
        ("5 0 R", "/Bad", "endstream\nendobj\n", "5 0"),
        ("5 0 R", "5 0 R", "endstream\nendobj\n", "5 0"),
        ("4 0 R", "6", "endstream\nendobj\n", "5 0"),
        ("5 1 R", "6", "endstream\nendobj\n", "5 0"),
        ("5 0 R", "6", "endstream\nendobj\n", "5 1"),
        ("5 0 R", "6", "endstream\nendobj\n", "5 65536"),
        ("5 0 R", "9223372036854775807", "endstream\nendobj\n", "5 0"),
        ("5 0 R", "2", "endstream\nendobj\n", "5 0"),
        ("5 0 R", "8", "endstream\nendobj\n", "5 0"),
        ("5 0 R", "6", "endstreaX\nendobj\n", "5 0"),
        ("5 0 R", "6", "endstream\nwrong\n", "5 0"),
        ("99 0 R", "6", "endstream\nendobj\n", "5 0"),
    ] {
        let bytes = fixture(length, value, b"BT ET\n", ending, header);
        let mut reader =
            PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
        assert!(
            reader.get_object(4, 0).is_err(),
            "accepted {length}, {value}, {ending}, {header}"
        );
        assert!(
            reader.get_object(4, 0).is_err(),
            "cached failed stream: {length}, {value}, {header}"
        );
        assert!(
            reader.get_object(3, 0).is_ok(),
            "reader poisoned after failure"
        );
    }
}

#[test]
fn strict_zero_length_and_crlf_streams() {
    for data in [b"".as_slice(), b"BT ET\r\n"] {
        let bytes = fixture_with_eol(
            "5 0 R",
            &data.len().to_string(),
            data,
            "\r\nendstream\r\nendobj\r\n",
            "5 0",
            "\r\n",
        );
        let mut reader =
            PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
        let PdfObject::Stream(stream) = reader.get_object(4, 0).unwrap() else {
            panic!("not a stream")
        };
        assert_eq!(stream.data, data);
    }
}

fn compressed_length_fixture(generation: u16, cycle: bool, object_stream_length: &str) -> Vec<u8> {
    let mut pdf = fixture(
        &format!("5 {generation} R"),
        "6",
        b"BT ET\n",
        "endstream\nendobj\n",
        "5 0",
    );
    let offset = pdf.windows(7).position(|w| w == b"5 0 obj").unwrap();
    pdf.truncate(offset);
    let length = if cycle { "5 0 R" } else { object_stream_length };
    pdf.extend_from_slice(format!("6 0 obj\n<< /Type /ObjStm /N 1 /First 4 /Length {length} >>\nstream\n5 0 6\nendstream\nendobj\n").as_bytes());
    let xref_offset = pdf.len();
    let mut xref = Vec::new();
    for id in 0..8 {
        let (kind, field, last) = match id {
            0 => (0u8, 0u32, 65535u16),
            5 => (2, 6, 0),
            7 => (1, xref_offset as u32, 0),
            _ => {
                let header = format!("{id} 0 obj");
                (
                    1,
                    pdf.windows(header.len())
                        .position(|w| w == header.as_bytes())
                        .unwrap() as u32,
                    0,
                )
            }
        };
        xref.push(kind);
        xref.extend(field.to_be_bytes());
        xref.extend(last.to_be_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "7 0 obj\n<< /Type /XRef /Size 8 /Root 1 0 R /W [1 4 2] /Length {} >>\nstream\n",
            xref.len()
        )
        .as_bytes(),
    );
    pdf.extend(xref);
    pdf.extend_from_slice(
        format!("\nendstream\nendobj\nstartxref\n{xref_offset}\n%%EOF\n").as_bytes(),
    );
    pdf
}

#[test]
fn strict_compressed_length_validates_generation_and_cycles() {
    for (generation, cycle) in [(0, false), (1, false), (0, true)] {
        let bytes = compressed_length_fixture(generation, cycle, "6");
        let mut reader =
            PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
        for _ in 0..2 {
            let result = reader.get_object(4, 0);
            if generation == 0 && !cycle {
                let PdfObject::Stream(stream) = result.unwrap() else {
                    panic!("not a stream")
                };
                assert_eq!(stream.data, b"BT ET\n");
            } else {
                assert!(result.is_err(), "generation={generation}, cycle={cycle}");
            }
        }
    }
}

#[test]
fn indirect_length_target_stream_with_oversized_body_fails_closed() {
    for value in [
        "<< /Length 9223372036854775807 >>\nstream\nx\nendstream",
        "[<< /Length 9223372036854775807 >>\nstream\nx\nendstream\n]",
        "<< /Nested << /Length 9223372036854775807 >>\nstream\nx\nendstream\n >>",
    ] {
        let bytes = fixture("5 0 R", value, b"BT ET\n", "endstream\nendobj\n", "5 0");
        let mut reader =
            PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
        assert!(reader.get_object(4, 0).is_err());
        assert!(reader.get_object(4, 0).is_err());
        assert!(reader.get_object(3, 0).is_ok());
    }
}

#[test]
fn strict_direct_lengths_share_bounds_and_negative_validation() {
    for length in ["-1", "-2", "9223372036854775807"] {
        let bytes = fixture(length, "6", b"BT ET\n", "endstream\nendobj\n", "5 0");
        let mut reader =
            PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
        assert!(reader.get_object(4, 0).is_err());
    }
}

#[test]
fn compressed_length_container_rejects_oversized_stream() {
    let bytes = compressed_length_fixture(0, false, "9223372036854775807");
    let mut reader =
        PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::strict()).unwrap();
    assert!(reader.get_object(4, 0).is_err());
    assert!(reader.get_object(4, 0).is_err());
}
