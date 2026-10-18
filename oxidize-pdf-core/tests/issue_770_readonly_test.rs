//! Public incremental filling contracts using independently authored PDF bytes.
use oxidize_pdf::{parser::PdfReader, writer::IncrementalFormFiller, PdfError};
use std::io::Cursor;

fn fixture(parent: &str, child: &str) -> Vec<u8> {
    fixture_with_extra(parent, child, &[])
}

fn fixture_with_extra(parent: &str, child: &str, extra: &[&str]) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>".into(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".into(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [6 0 R 7 0 R] >>".into(),
        "<< /Fields [5 0 R 7 0 R] /DA (/Helv 12 Tf 0 g) >>".into(),
        format!("<< /T (group) /Kids [6 0 R] {parent} >>"),
        format!("<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /Parent 5 0 R /V (original) /Rect [10 10 200 40] /P 3 0 R {child} >>"),
        "<< /Type /Annot /Subtype /Widget /FT /Tx /T (writable) /V (untouched) /Rect [10 50 200 80] /P 3 0 R >>".into(),
    ];
    objects.extend(extra.iter().map(|value| value.to_string()));
    let size = objects.len() + 1;
    let mut bytes = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(bytes.len());
        bytes.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", i + 1).as_bytes());
    }
    let xref = bytes.len();
    bytes.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f \n").as_bytes());
    for offset in offsets {
        bytes.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    bytes.extend_from_slice(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    bytes
}

#[test]
fn direct_readonly_rejects_single_fill() {
    let base = fixture("", "/Ff 1");
    assert!(matches!(
        IncrementalFormFiller::new(&base).fill("group.name", "changed"),
        Err(PdfError::PermissionDenied(_))
    ));
}

#[test]
fn inherited_readonly_rejects_single_fill() {
    let base = fixture("/Ff 1", "");
    assert!(matches!(
        IncrementalFormFiller::new(&base).fill("group.name", "changed"),
        Err(PdfError::PermissionDenied(_))
    ));
}

#[test]
fn mixed_batches_return_only_error_in_both_orders() {
    let base = fixture("/Ff 1", "");
    for fields in [
        [("writable", "new"), ("group.name", "changed")],
        [("group.name", "changed"), ("writable", "new")],
    ] {
        assert!(matches!(
            IncrementalFormFiller::new(&base).fill_many(&fields),
            Err(PdfError::PermissionDenied(_))
        ));
    }
    let mut reader = PdfReader::new(Cursor::new(&base)).unwrap();
    assert_eq!(
        reader
            .get_object(7, 0)
            .unwrap()
            .as_dict()
            .unwrap()
            .get("V")
            .unwrap()
            .as_string()
            .unwrap()
            .as_bytes(),
        b"untouched"
    );
}

#[test]
fn writable_local_override_preserves_values_prefix_and_appearance() {
    // Ff is inherited as a whole value: explicit zero overrides parent ReadOnly.
    for (parent, child) in [("/Ff 1", "/Ff 0"), ("", ""), ("/Ff 2", "")] {
        let base = fixture(parent, child);
        let output = IncrementalFormFiller::new(&base)
            .fill("group.name", "changed")
            .unwrap();
        assert!(output.starts_with(&base));
        let mut reader = PdfReader::new(Cursor::new(&output)).unwrap();
        let field = reader.get_object(6, 0).unwrap().as_dict().unwrap().clone();
        assert_eq!(
            field.get("V").unwrap().as_string().unwrap().as_bytes(),
            b"changed"
        );
        let ap = field
            .get("AP")
            .unwrap()
            .as_dict()
            .unwrap()
            .get("N")
            .unwrap()
            .as_reference()
            .unwrap();
        let stream = reader.get_object(ap.0, ap.1).unwrap().as_stream().unwrap();
        assert!(String::from_utf8_lossy(&stream.data).contains("(changed) Tj"));
    }
}

#[test]
fn malformed_flags_do_not_grant_write_permission() {
    for flags in ["/Ff (1)", "/Ff -1", "/Ff 4294967296"] {
        let base = fixture(flags, "");
        assert!(matches!(
            IncrementalFormFiller::new(&base).fill("group.name", "changed"),
            Err(PdfError::InvalidStructure(_))
        ));
    }
}

#[test]
fn cyclic_parent_without_flags_is_rejected() {
    let base = fixture("/Parent 5 0 R", "");
    assert!(matches!(
        IncrementalFormFiller::new(&base).fill("group.name", "changed"),
        Err(PdfError::InvalidStructure(_))
    ));
}

#[test]
fn indirect_flags_and_null_inheritance_remain_effective() {
    for (parent, child, extra) in [
        ("/Ff 1", "/Ff null", vec![]),
        ("/Ff 8 0 R", "", vec!["1"]),
        ("/Ff 1", "/Ff 8 0 R", vec!["null"]),
        ("/Ff 8 0 R", "", vec!["9 0 R", "1"]),
    ] {
        let base = fixture_with_extra(parent, child, &extra);
        assert!(
            matches!(
                IncrementalFormFiller::new(&base).fill("group.name", "changed"),
                Err(PdfError::PermissionDenied(_))
            ),
            "{parent} {child}"
        );
    }
    let base = fixture_with_extra("/Ff 1", "/Ff 8 0 R", &["0"]);
    assert!(IncrementalFormFiller::new(&base)
        .fill("group.name", "changed")
        .is_ok());
}

#[test]
fn cyclic_flags_and_invalid_parent_are_rejected() {
    for (parent, extra) in [
        ("/Ff 8 0 R", vec!["8 0 R"]),
        ("/Ff 8 0 R", vec!["9 0 R", "8 0 R"]),
        ("/Parent 8 0 R", vec!["42"]),
        ("/Parent (bad)", vec![]),
    ] {
        let base = fixture_with_extra(parent, "", &extra);
        assert!(
            matches!(
                IncrementalFormFiller::new(&base).fill("group.name", "changed"),
                Err(PdfError::InvalidStructure(_))
            ),
            "{parent}"
        );
    }
}
