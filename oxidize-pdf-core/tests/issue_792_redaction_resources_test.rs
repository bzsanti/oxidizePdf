//! Independently serialized resources must not bypass irreversible redaction.
use oxidize_pdf::{
    operations::{RedactionConfig, SemanticRedactor, SemanticRedactorError},
    semantic::{BoundingBox, EntityType, SemanticEntity},
};
#[path = "common/pdf_assembler.rs"]
mod pdf_assembler;

fn fixture(extra: &str, indirect: bool, unused: bool) -> Vec<u8> {
    let font = format!("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica {extra} >>");
    fixture_font(&font, indirect, unused)
}

fn fixture_font(font: &str, indirect: bool, unused: bool) -> Vec<u8> {
    let fonts = if unused {
        format!("/F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> /Unused {font}")
    } else if indirect {
        "/F1 5 0 R".to_string()
    } else {
        format!("/F1 {font}")
    };
    pdf_assembler::assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << {fonts} >> >> /Contents 4 0 R >>").into_bytes(),
        pdf_assembler::stream_obj("", b"BT /F1 12 Tf 20 80 Td (SECRET) Tj 0 -30 Td (SAFE) Tj ET\n"),
        font.as_bytes().to_vec(),
        b"<< /Private (SECRET) >>".to_vec(),
    ])
}
fn redact(pdf: &[u8]) -> Result<Vec<u8>, SemanticRedactorError> {
    redact_target(pdf, "SECRET")
}

fn redact_target(pdf: &[u8], target: &str) -> Result<Vec<u8>, SemanticRedactorError> {
    let entity = SemanticEntity::new(
        "secret".into(),
        EntityType::Text,
        BoundingBox::new(10., 60., 150., 30., 1),
    )
    .with_content(target);
    SemanticRedactor::redact_irreversible(
        pdf,
        &[entity],
        RedactionConfig::new().with_types(vec![EntityType::Text]),
    )
    .map(|(bytes, report)| {
        assert!(report.is_irreversible());
        bytes
    })
}

#[test]
fn rejects_unverified_font_fields_in_direct_indirect_and_unused_fonts() {
    for field in [
        "/Private (SECRET)",
        "/Private <534543524554>",
        "/Private << /Nested [(SECRET)] >>",
        "/Private 6 0 R",
        "/FontDescriptor 6 0 R",
        "/ToUnicode 6 0 R",
        "/Name /SECRET",
    ] {
        for (indirect, unused) in [(false, false), (true, false), (false, true)] {
            let error = redact(&fixture(field, indirect, unused))
                .map(|_| ())
                .expect_err("retained font payload must fail without output");
            assert!(matches!(error, SemanticRedactorError::ParseFailed(_)));
            assert!(
                error.to_string().contains("unsupported font field"),
                "{field}: {error}"
            );
        }
    }
}

#[test]
fn ordinary_standard14_fonts_remain_supported() {
    for encoding in [
        "",
        "/Encoding /StandardEncoding",
        "/Encoding /WinAnsiEncoding",
        "/Encoding /MacRomanEncoding",
    ] {
        for indirect in [false, true] {
            assert!(
                redact(&fixture(encoding, indirect, false)).is_ok(),
                "{encoding}"
            );
        }
    }
}

#[test]
fn rejects_payload_in_type_field() {
    let input = fixture_font(
        "<< /Type (SECRET) /Subtype /Type1 /BaseFont /Helvetica >>",
        true,
        false,
    );
    let error = redact(&input)
        .map(|_| ())
        .expect_err("Type payload must fail");
    assert!(error.to_string().contains("invalid /Type"), "{error}");
}

#[test]
fn rejects_font_payload_on_page_without_selected_entity() {
    let input = pdf_assembler::assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 2 /Kids [3 0 R 5 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents 4 0 R >>".to_vec(),
        pdf_assembler::stream_obj("", b"BT /F1 12 Tf 20 80 Td (SECRET) Tj ET"),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font 7 0 R >> /Contents 6 0 R >>".to_vec(),
        pdf_assembler::stream_obj("", b""),
        b"<< /Unused << /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Private (SECRET) >> >>".to_vec(),
    ]);
    let error = redact(&input)
        .map(|_| ())
        .expect_err("unselected page resources must be checked");
    assert!(
        error
            .to_string()
            .contains("page 2 font /Unused contains unsupported font field /Private"),
        "{error}"
    );
}

#[test]
fn rejects_target_in_retained_font_names() {
    for name in ["SECRET", "#53ECRET"] {
        let input = pdf_assembler::assemble_pdf(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /{name} << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents 4 0 R >>").into_bytes(),
            pdf_assembler::stream_obj("", format!("BT /{name} 12 Tf 20 80 Td (SECRET) Tj ET").as_bytes()),
        ]);
        let error = redact(&input)
            .map(|_| ())
            .expect_err("resource name exposes target");
        assert!(error.to_string().contains("in font resources"), "{error}");
    }
}

#[test]
fn checks_names_added_by_writer_in_serialized_output() {
    // Helvetica-Bold is not present in the input font dictionary. The writer
    // inserts standard font resources, which must pass the same output check.
    let input = pdf_assembler::assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Count 1 /Kids [3 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents 4 0 R >>".to_vec(),
        pdf_assembler::stream_obj("", b"BT /F1 12 Tf 20 80 Td (Helvetica-Bold) Tj ET"),
    ]);
    let error = redact_target(&input, "Helvetica-Bold")
        .map(|_| ())
        .expect_err("writer-added names must be checked");
    assert!(error.to_string().contains("in font resources"), "{error}");
}

fn command_ok(program: &str, args: &[&std::ffi::OsStr]) -> std::process::Output {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .expect(program);
    assert!(
        output.status.success(),
        "{program}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
#[ignore = "requires qpdf and Poppler; executed by interoperability CI"]
fn independent_resource_oracle() {
    let dir = tempfile::tempdir().unwrap();
    for (label, extra) in [("unsafe", "/Private (SECRET)"), ("safe", "")] {
        let input = fixture(extra, true, false);
        let input_path = dir.path().join(format!("{label}-input.pdf"));
        let output_path = dir.path().join(format!("{label}-output.pdf"));
        let qdf_path = dir.path().join(format!("{label}.qdf"));
        std::fs::write(&input_path, &input).unwrap();
        command_ok("qpdf", &["--check".as_ref(), input_path.as_os_str()]);
        match redact(&input) {
            Ok(output) => {
                std::fs::write(&output_path, output).unwrap();
                command_ok("qpdf", &["--check".as_ref(), output_path.as_os_str()]);
                command_ok(
                    "qpdf",
                    &[
                        "--qdf".as_ref(),
                        "--object-streams=disable".as_ref(),
                        output_path.as_os_str(),
                        qdf_path.as_os_str(),
                    ],
                );
                let qdf = std::fs::read(&qdf_path).unwrap();
                let text = command_ok("pdftotext", &[output_path.as_os_str(), "-".as_ref()]);
                assert!(!String::from_utf8_lossy(&text.stdout).contains("SECRET"));
                assert!(String::from_utf8_lossy(&text.stdout).contains("SAFE"));
                assert!(
                    !String::from_utf8_lossy(&qdf).contains("SECRET"),
                    "qpdf recovered SECRET from a supposedly irreversible {label} output"
                );
                assert_eq!(label, "safe", "unsupported resource unexpectedly accepted");
            }
            Err(error) => {
                assert_eq!(label, "unsafe", "ordinary font rejected: {error}");
                assert!(
                    error.to_string().contains("unsupported font field"),
                    "{error}"
                );
            }
        }
    }
}
