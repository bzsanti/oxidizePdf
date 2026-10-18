//! #730: successful irreversible redaction must not leave a target split across text operators.
use oxidize_pdf::{
    operations::{RedactionConfig, SemanticRedactor, SemanticRedactorError},
    semantic::{BoundingBox, EntityType, SemanticEntity},
};
fn fixture(contents: &[&str]) -> Vec<u8> {
    fixture_font(
        contents,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    )
}
fn fixture_font(contents: &[&str], font: &str) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            (0..contents.len())
                .map(|i| format!("{} 0 R", 4 + i * 2))
                .collect::<Vec<_>>()
                .join(" "),
            contents.len()
        )
        .into_bytes(),
        font.as_bytes().to_vec(),
    ];
    for (i, c) in contents.iter().enumerate() {
        objects.push(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",5+i*2).into_bytes());
        objects.push(format!("<< /Length {} >>\nstream\n{}\nendstream", c.len(), c).into_bytes());
    }
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = vec![0];
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend(o);
        out.extend(b"\nendobj\n");
    }
    let x = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for n in &offsets[1..] {
        out.extend(format!("{n:010} 00000 n \n").as_bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    out
}

const TARGET: &str = "123-45-6789";
const FIRST: &str = "BT /F1 12 Tf 72 700 Td (123-45-6789) Tj ET";
fn redact(pdf: &[u8]) -> Result<Vec<u8>, SemanticRedactorError> {
    let e = SemanticEntity::new(
        "secret".into(),
        EntityType::Text,
        BoundingBox::new(72., 690., 100., 20., 1),
    )
    .with_content(TARGET);
    SemanticRedactor::redact_irreversible(
        pdf,
        &[e],
        RedactionConfig::new().with_types(vec![EntityType::Text]),
    )
    .map(|(bytes, report)| {
        assert!(report.is_irreversible());
        bytes
    })
}
fn rejects(second: &str) {
    let pdf = fixture(&[FIRST, second]);
    let error = redact(&pdf).expect_err("recoverable target must not be reported irreversible");
    assert!(
        error
            .to_string()
            .contains("forensic verification recovered entity"),
        "wrong refusal: {error}"
    );
}
#[test]
fn split_tj() {
    rejects("BT /F1 12 Tf 72 700 Td (123-45-) Tj (6789) Tj ET");
}
#[test]
fn split_tj_tj_array() {
    rejects("BT /F1 12 Tf 72 700 Td (123-45-) Tj [(67) 0 (89)] TJ ET");
}
#[test]
fn split_arrays() {
    rejects("BT /F1 12 Tf 72 700 Td [(123-) 0 (45-)] TJ [(6789)] TJ ET");
}
#[test]
fn split_array_tj() {
    rejects("BT /F1 12 Tf 72 700 Td [(123-45-)] TJ (6789) Tj ET");
}
#[test]
fn split_text_states() {
    rejects("BT /F1 12 Tf 72 700 Td (123-45-) Tj ET BT /F1 12 Tf 115 700 Td (6789) Tj ET");
}
#[test]
fn split_hex_strings() {
    rejects("BT /F1 12 Tf 72 700 Td <3132332d34352d> Tj <36373839> Tj ET");
}
#[test]
fn unsplit_duplicate() {
    rejects(FIRST);
}
#[test]
fn safe_control() {
    assert!(!redact(&fixture(&[
        FIRST,
        "BT /F1 12 Tf 72 700 Td (CONTROL_SAFE) Tj ET"
    ]))
    .unwrap()
    .is_empty());
}
#[test]
fn same_page_residual_split() {
    let pdf =
        fixture(&["BT /F1 12 Tf 72 700 Td (123-45-6789) Tj 0 -30 Td (123-45-) Tj (6789) Tj ET"]);
    assert!(redact(&pdf)
        .unwrap_err()
        .to_string()
        .contains("forensic verification recovered entity"));
}
#[test]
#[ignore = "requires installed Poppler and qpdf; run explicitly for security acceptance"]
fn independent_output_oracle() {
    use std::process::Command;
    let dir = tempfile::tempdir().unwrap();
    let check = |name: &str, bytes: &[u8]| -> String {
        let p = dir.path().join(name);
        std::fs::write(&p, bytes).unwrap();
        let q = Command::new("qpdf")
            .arg("--check")
            .arg(&p)
            .output()
            .expect("qpdf required");
        assert!(q.status.success(), "{:?}", q);
        let r = Command::new("pdftotext")
            .args(["-layout"])
            .arg(&p)
            .arg("-")
            .output()
            .expect("Poppler required");
        assert!(r.status.success(), "{:?}", r);
        String::from_utf8(r.stdout).unwrap()
    };
    let unsafe_input = fixture(&[FIRST, "BT /F1 12 Tf 72 700 Td (123-45-) Tj (6789) Tj ET"]);
    assert_eq!(check("input.pdf", &unsafe_input).matches(TARGET).count(), 2);
    assert!(redact(&unsafe_input).is_err());
    let safe = fixture(&[FIRST, "BT /F1 12 Tf 72 700 Td (CONTROL_SAFE) Tj ET"]);
    let text = check("safe-output.pdf", &redact(&safe).unwrap());
    assert!(!text.contains(TARGET));
    assert!(text.contains("CONTROL_SAFE"));
}

#[path = "common/pdf_assembler.rs"]
mod pdf_assembler;

#[test]
fn split_content_streams() {
    use pdf_assembler::{assemble_pdf, stream_obj};
    let pdf = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [4 0 R 6 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents 5 0 R >>".to_vec(),
        stream_obj("", FIRST.as_bytes()),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents [7 0 R 8 0 R] >>".to_vec(),
        stream_obj("", b"BT /F1 12 Tf 72 700 Td (123-45-) Tj ET"),
        stream_obj("", b"BT /F1 12 Tf 115 700 Td (6789) Tj ET"),
    ]);
    assert!(redact(&pdf)
        .unwrap_err()
        .to_string()
        .contains("forensic verification recovered entity"));
}
