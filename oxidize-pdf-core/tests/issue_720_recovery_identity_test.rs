//! Independent fixtures: changing an indirect object's identity must not invent semantics.
use oxidize_pdf::parser::{ParseOptions, PdfReader};
use std::io::Cursor;
#[path = "common/pdf_assembler.rs"]
mod pdf_assembler;

fn fixture(id: u32, body: &[u8], indexed: bool) -> Vec<u8> {
    let mut objects = vec![b"null".to_vec(); id as usize];
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    objects[1] = b"<< /Type /Pages /Kids [] /Count 0 >>".to_vec();
    objects[id as usize - 1] = body.to_vec();
    if indexed {
        return pdf_assembler::assemble_pdf(&objects);
    }
    // The target exists in the bytes but is omitted from the xref subsection.
    let mut bytes = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (n, body) in [
        (1, &objects[0]),
        (2, &objects[1]),
        (id, &objects[id as usize - 1]),
    ] {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        bytes.extend_from_slice(body);
        bytes.extend_from_slice(b"\nendobj\n");
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 3\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<< /Size 3 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets[0], offsets[1]).as_bytes());
    bytes
}

#[test]
fn strict_rejects_malformed_dictionary_under_every_identity() {
    for id in [3, 100, 102, 113, 114, 200] {
        let mut reader = PdfReader::new_with_options(
            Cursor::new(fixture(id, b"<< /Type /Unrelated /Broken [ >>", true)),
            ParseOptions::strict(),
        )
        .unwrap();
        assert!(reader.get_object(id, 0).is_err(), "fabricated object {id}");
    }
}
#[test]
fn lenient_rejects_unrepairable_dictionary_under_every_identity() {
    for id in [3, 100, 102, 113, 114, 200] {
        let mut reader = PdfReader::new_with_options(
            Cursor::new(fixture(id, b"<< /Type /Unrelated /Broken [ >>", true)),
            ParseOptions::lenient(),
        )
        .unwrap();
        assert!(reader.get_object(id, 0).is_err(), "fabricated object {id}");
    }
}
#[test]
fn valid_dictionary_preserves_parent_fractional_geometry_and_unknown_keys() {
    for options in [ParseOptions::strict(), ParseOptions::lenient()] {
        for id in [3, 113, 200] {
            let mut reader = PdfReader::new_with_options(Cursor::new(fixture(id, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 12.5 20.5] /Vendor (keep >> this) >>", true)), options.clone()).unwrap();
            let dict = reader.get_object(id, 0).unwrap().as_dict().unwrap();
            assert_eq!(dict.get("Parent").unwrap().as_reference(), Some((2, 0)));
            assert_eq!(
                dict.get("MediaBox").unwrap().as_array().unwrap().0[2].as_real(),
                Some(12.5)
            );
            assert_eq!(
                dict.get("Vendor").unwrap().as_string().unwrap().as_bytes(),
                b"keep >> this"
            );
        }
    }
}
#[test]
fn lenient_recovers_real_unindexed_dictionary_independently_of_identity() {
    for id in [3, 113, 200] {
        let mut reader = PdfReader::new_with_options(
            Cursor::new(fixture(
                id,
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 12.5 20.5] /Vendor (keep >> this) >>",
                false,
            )),
            ParseOptions::lenient(),
        )
        .unwrap();
        let dict = reader
            .get_object(id, 0)
            .unwrap()
            .as_dict()
            .expect("recover real dictionary");
        assert_eq!(dict.get("Parent").unwrap().as_reference(), Some((2, 0)));
        assert_eq!(
            dict.get("MediaBox").unwrap().as_array().unwrap().0[2].as_real(),
            Some(12.5)
        );
        assert_eq!(
            dict.get("Vendor").unwrap().as_string().unwrap().as_bytes(),
            b"keep >> this"
        );
    }
}
#[test]
fn strict_does_not_repair_unindexed_objects() {
    for id in [3, 113, 200] {
        let mut reader = PdfReader::new_with_options(
            Cursor::new(fixture(id, b"<< /Type /Pages /Count 7 /Kids [] >>", false)),
            ParseOptions::strict(),
        )
        .unwrap();
        assert!(reader.get_object(id, 0).is_err(), "strict repaired {id}");
    }
}

#[test]
fn malformed_stream_dictionary_is_not_replaced_with_an_empty_one() {
    for id in [4, 113, 200] {
        let bytes = fixture(
            id,
            b"<< /Length 3 /Filter /DCTDecode /BadKey >>\nstream\nabc\nendstream",
            true,
        );
        let mut reader =
            PdfReader::new_with_options(Cursor::new(bytes), ParseOptions::lenient()).unwrap();
        assert!(
            reader.get_object(id, 0).is_err(),
            "discarded dictionary of {id}"
        );
    }
}

#[test]
fn unrelated_n_entry_cannot_become_a_page_count() {
    for id in [100, 200] {
        let mut objects = vec![b"null".to_vec(); id];
        objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
        objects[id - 1] = b"<< /N 44 >>".to_vec();
        let mut reader = PdfReader::new_with_options(
            Cursor::new(pdf_assembler::assemble_pdf(&objects)),
            ParseOptions::lenient(),
        )
        .unwrap();
        assert_eq!(
            reader.page_count().unwrap(),
            0,
            "unrelated /N in object {id}"
        );
    }
}

#[test]
fn strict_page_count_propagates_malformed_tree() {
    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Broken [ >>".to_vec(),
    ];
    let mut reader = PdfReader::new_with_options(
        Cursor::new(pdf_assembler::assemble_pdf(&objects)),
        ParseOptions::strict(),
    )
    .unwrap();
    assert!(reader.page_count().is_err());
}
