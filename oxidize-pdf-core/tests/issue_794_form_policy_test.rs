//! Structural certification-policy fixtures; signatures are not cryptographically valid.
use oxidize_pdf::{
    parser::PdfReader,
    writer::{IncrementalFormFiller, IncrementalObjectEditor},
    PdfError,
};
use std::io::Cursor;
#[path = "common/pdf_assembler.rs"]
mod pdf_assembler;

fn fixture(permission: Option<u8>, field_params: Option<&str>, read_only: bool) -> Vec<u8> {
    pdf_assembler::assemble_pdf(&fixture_objects(permission, field_params, read_only))
}

fn fixture_objects(
    permission: Option<u8>,
    field_params: Option<&str>,
    read_only: bool,
) -> Vec<Vec<u8>> {
    let perms = if permission.is_some() {
        "/Perms << /DocMDP 7 0 R >>"
    } else {
        ""
    };
    let mut transforms = String::new();
    if let Some(p) = permission {
        transforms.push_str(&format!(
            "<< /TransformMethod /DocMDP /TransformParams << /P {p} /V /1.2 >> >> "
        ));
    }
    if let Some(params) = field_params {
        transforms.push_str(&format!(
            "<< /TransformMethod /FieldMDP /TransformParams << {params} >> >>"
        ));
    }
    vec![
        format!("<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R {perms} >>").into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Annots [5 0 R 6 0 R] >>".to_vec(),
        b"<< /Fields [5 0 R 6 0 R] /DA (/Helv 12 Tf 0 g) >>".to_vec(),
        format!("<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /V (original) /Ff {} /Rect [10 10 200 40] /P 3 0 R >>", u8::from(read_only)).into_bytes(),
        b"<< /Type /Annot /Subtype /Widget /FT /Tx /T (other) /V (untouched) /Rect [10 50 200 80] /P 3 0 R >>".to_vec(),
        format!("<< /Type /Sig /ByteRange [0 999999 1000001 1] /Contents <0000> /Reference [{transforms}] >>").into_bytes(),
    ]
}
fn value(pdf: &[u8], object: u32) -> Vec<u8> {
    let mut reader = PdfReader::new(Cursor::new(pdf)).unwrap();
    reader
        .get_object(object, 0)
        .unwrap()
        .as_dict()
        .unwrap()
        .get("V")
        .unwrap()
        .as_string()
        .unwrap()
        .as_bytes()
        .to_vec()
}
fn denied(result: Result<Vec<u8>, PdfError>) {
    let error = result
        .map(|_| ())
        .expect_err("policy must reject without output");
    assert!(matches!(error, PdfError::PermissionDenied(_)), "{error}");
}

#[test]
fn p1_rejects_single_and_batch_before_any_output() {
    let base = fixture(Some(1), None, false);
    assert!(matches!(
        IncrementalObjectEditor::new(&base),
        Err(PdfError::PermissionDenied(_))
    ));
    let filler = IncrementalFormFiller::new(&base);
    denied(filler.fill("name", "changed"));
    denied(filler.fill_many(&[("other", "changed"), ("name", "changed")]));
    assert_eq!(value(&base, 5), b"original");
    assert_eq!(value(&base, 6), b"untouched");
}

#[test]
fn p2_p3_and_approval_controls_allow_incremental_values_and_appearances() {
    for permission in [None, Some(2), Some(3)] {
        let base = fixture(permission, None, false);
        let output = IncrementalFormFiller::new(&base)
            .fill_many(&[("name", "changed"), ("other", "second")])
            .unwrap();
        assert!(output.starts_with(&base));
        assert_eq!(value(&output, 5), b"changed");
        assert_eq!(value(&output, 6), b"second");
        let mut reader = PdfReader::new(Cursor::new(output.as_slice())).unwrap();
        assert!(reader
            .get_object(5, 0)
            .unwrap()
            .as_dict()
            .unwrap()
            .get("AP")
            .is_some());
        let second = IncrementalFormFiller::new(&output)
            .fill("name", "again")
            .unwrap();
        assert!(second.starts_with(&output));
        assert_eq!(value(&second, 5), b"again");
    }
}

#[test]
fn field_locks_reject_only_applicable_targets_and_abort_mixed_batches() {
    for permission in [None, Some(2), Some(3)] {
        for params in [
            "/Action /Include /Fields [(name)]",
            "/Action /Exclude /Fields [(other)]",
        ] {
            let base = fixture(permission, Some(params), false);
            let filler = IncrementalFormFiller::new(&base);
            denied(filler.fill("name", "changed"));
            denied(filler.fill_many(&[("other", "allowed"), ("name", "blocked")]));
            denied(filler.fill_many(&[("name", "blocked"), ("other", "allowed")]));
            let output = filler.fill("other", "allowed").unwrap();
            assert!(output.starts_with(&base));
            assert_eq!(value(&output, 5), b"original");
            assert_eq!(value(&output, 6), b"allowed");
        }
        let base = fixture(permission, Some("/Action /All"), false);
        denied(IncrementalFormFiller::new(&base).fill("other", "blocked"));
    }
}

#[test]
fn malformed_policy_cannot_authorize_a_fill() {
    for params in [
        "/Action /Include",
        "/Action /Exclude",
        "/Action /Unknown",
        "/Action /Include /Fields [7]",
        "/Action /All /Fields [(name)]",
    ] {
        let base = fixture(Some(2), Some(params), false);
        let result = IncrementalFormFiller::new(&base).fill("name", "changed");
        assert!(
            matches!(result, Err(PdfError::InvalidStructure(_))),
            "{params}"
        );
    }
    let base = fixture(Some(4), None, false);
    assert!(matches!(
        IncrementalFormFiller::new(&base).fill("name", "changed"),
        Err(PdfError::InvalidStructure(_))
    ));
}

#[test]
fn readonly_and_missing_field_controls_remain_errors() {
    let base = fixture(Some(2), None, true);
    denied(IncrementalFormFiller::new(&base).fill("name", "changed"));
    let base = fixture(Some(3), None, false);
    assert!(matches!(
        IncrementalFormFiller::new(&base).fill_many(&[("name", "changed"), ("missing", "bad")]),
        Err(PdfError::FieldNotFound(_))
    ));
}

#[test]
fn locks_cover_aliases_of_the_same_field_object() {
    let mut objects = fixture_objects(
        Some(2),
        Some("/Action /Include /Fields [(locked.name)]"),
        false,
    );
    objects[3] = b"<< /Fields [8 0 R 9 0 R] /DA (/Helv 12 Tf 0 g) >>".to_vec();
    objects.push(b"<< /T (locked) /Kids [5 0 R] >>".to_vec());
    objects.push(b"<< /T (alias) /Kids [5 0 R] >>".to_vec());
    let base = pdf_assembler::assemble_pdf(&objects);
    denied(IncrementalFormFiller::new(&base).fill("alias.name", "blocked"));
}

#[test]
fn indirect_transform_and_pdf20_names_use_the_field_resolution_encoding() {
    let mut objects = fixture_objects(None, None, false);
    objects[0] = b"<< /Type /Catalog /Version /2.0 /Pages 2 0 R /AcroForm 4 0 R >>".to_vec();
    objects[4] = b"<< /FT /Tx /T <EFBBBF636166C3A9> /V (original) >>".to_vec();
    // /Type is optional: ByteRange + Contents still identifies a signature.
    objects[6] =
        b"<< /ByteRange [0 999999 1000001 1] /Contents <0000> /Reference 8 0 R >>".to_vec();
    objects.push(b"[9 0 R]".to_vec());
    objects.push(b"<< /TransformMethod /FieldMDP /TransformParams 10 0 R >>".to_vec());
    objects.push(b"<< /Action /Include /Fields 11 0 R >>".to_vec());
    objects.push(b"[<EFBBBF636166C3A9>]".to_vec());
    let base = pdf_assembler::assemble_pdf(&objects);
    denied(IncrementalFormFiller::new(&base).fill("café", "blocked"));
    let output = IncrementalFormFiller::new(&base)
        .fill("other", "allowed")
        .unwrap();
    assert!(output.starts_with(&base));
    assert_eq!(value(&output, 6), b"allowed");
}

#[test]
fn inline_or_unsigned_field_policy_definitions_fail_closed() {
    for signature in [
        "<< /Nested << /Type /Sig /ByteRange [0 999999 1000001 1] /Contents <0000> /Reference [<< /TransformMethod /FieldMDP /TransformParams << /Action /All >> >>] >> >>",
        "<< /Type /Sig /ByteRange [0 1 2 1] /Contents <0000> /Reference [<< /TransformMethod /FieldMDP /TransformParams << /Action /All >> >>] >>",
    ] {
        let mut objects = fixture_objects(None, None, false);
        objects[6] = signature.as_bytes().to_vec();
        let base = pdf_assembler::assemble_pdf(&objects);
        assert!(matches!(IncrementalFormFiller::new(&base).fill("name", "blocked"), Err(PdfError::InvalidStructure(_))));
    }
}

#[test]
fn unlocked_aliases_remain_fillable() {
    let mut objects = fixture_objects(Some(2), None, false);
    objects[3] = b"<< /Fields [8 0 R 9 0 R] /DA (/Helv 12 Tf 0 g) >>".to_vec();
    objects.push(b"<< /T (first) /Kids [5 0 R] >>".to_vec());
    objects.push(b"<< /T (alias) /Kids [5 0 R] >>".to_vec());
    let base = pdf_assembler::assemble_pdf(&objects);
    let output = IncrementalFormFiller::new(&base)
        .fill("alias.name", "allowed")
        .unwrap();
    assert!(output.starts_with(&base));
    assert_eq!(value(&output, 5), b"allowed");
}

#[test]
fn actual_field_cycles_still_fail_closed() {
    let mut objects = fixture_objects(None, None, false);
    objects[3] = b"<< /Fields [8 0 R] >>".to_vec();
    objects.push(b"<< /T (cycle) /Kids [8 0 R] >>".to_vec());
    let base = pdf_assembler::assemble_pdf(&objects);
    let error = IncrementalObjectEditor::new(&base).err().unwrap();
    assert!(matches!(error, PdfError::InvalidStructure(_)), "{error}");
    assert!(
        error
            .to_string()
            .contains("cyclic signature field reference"),
        "{error}"
    );
}
