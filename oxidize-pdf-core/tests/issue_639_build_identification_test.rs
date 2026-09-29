use oxidize_pdf::parser::{objects::PdfDictionary, PdfReader};
use oxidize_pdf::writer::{
    IncrementalInfoPolicy, IncrementalTextNoteEditor, PdfWriter, TextNoteMutation, WriterConfig,
};
use oxidize_pdf::{BuildIdentification, Document, Page};
use std::io::Cursor;

const KEYS: [&str; 3] = [
    "oxidize-pdf-build",
    "oxidize-pdf-edition",
    "oxidize-pdf-features",
];

fn info(bytes: &[u8]) -> Option<PdfDictionary> {
    let mut reader = PdfReader::new(Cursor::new(bytes)).unwrap();
    reader
        .trailer()
        .info()
        .map(|(n, g)| reader.get_object(n, g).unwrap().as_dict().unwrap().clone())
}

fn doc() -> Document {
    let mut doc = Document::new();
    doc.set_title("Customer title");
    doc.set_author("Customer author");
    doc.set_subject("Customer subject");
    doc.set_keywords("Customer keywords");
    doc.set_creator("Customer editor");
    doc.set_producer("Customer PDF writer");
    doc.set_creation_date(chrono::DateTime::from_timestamp(1_700_000_000, 0).unwrap());
    doc.set_modification_date(chrono::DateTime::from_timestamp(1_700_000_001, 0).unwrap());
    doc.add_page(Page::a4());
    doc
}

fn generate(policy: Option<BuildIdentification>, api: usize) -> Vec<u8> {
    let mut doc = doc();
    if let Some(policy) = policy {
        doc.set_build_identification(policy);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("generated.pdf");
    match api {
        0 => doc.to_bytes().unwrap(),
        1 => {
            doc.save(&path).unwrap();
            std::fs::read(path).unwrap()
        }
        2 => doc.to_bytes_with_config(WriterConfig::modern()).unwrap(),
        3 => {
            doc.save_with_config(&path, WriterConfig::modern()).unwrap();
            std::fs::read(path).unwrap()
        }
        _ => unreachable!(),
    }
}

#[test]
fn default_and_explicit_enabled_emit_identification_in_all_output_apis() {
    assert_eq!(
        Document::default().build_identification(),
        BuildIdentification::Enabled
    );
    for api in 0..4 {
        for policy in [None, Some(BuildIdentification::Enabled)] {
            let info = info(&generate(policy, api)).unwrap();
            assert!(info
                .get(KEYS[0])
                .unwrap()
                .as_string()
                .unwrap()
                .to_text()
                .starts_with("oxpdf-"));
            assert_eq!(
                info.get(KEYS[1]).unwrap().as_string().unwrap().to_text(),
                "OpenSource"
            );
            assert_eq!(
                info.get(KEYS[2])
                    .unwrap()
                    .as_string()
                    .unwrap()
                    .as_bytes()
                    .len(),
                4
            );
        }
    }
}

#[test]
fn disabled_omits_all_three_fields_and_preserves_unrelated_metadata() {
    for api in 0..4 {
        let enabled = info(&generate(None, api)).unwrap();
        let bytes = generate(Some(BuildIdentification::Disabled), api);
        let disabled = info(&bytes).unwrap();
        for key in KEYS {
            assert!(disabled.get(key).is_none(), "{key} in API {api}");
        }
        for key in [
            "Title",
            "Author",
            "Subject",
            "Keywords",
            "Creator",
            "Producer",
            "CreationDate",
        ] {
            assert_eq!(disabled.get(key), enabled.get(key), "{key} in API {api}");
        }
        // Every write refreshes ModDate independently, regardless of this policy.
        assert!(disabled
            .get("ModDate")
            .unwrap()
            .as_string()
            .unwrap()
            .as_bytes()
            .starts_with(b"D:"));
        // The identifiers are never moved to a different object or XMP packet.
        for marker in [
            "oxidize-pdf-build",
            "oxidize-pdf-edition",
            "oxidize-pdf-features",
            "oxpdf-",
        ] {
            assert!(
                !bytes.windows(marker.len()).any(|b| b == marker.as_bytes()),
                "hidden {marker}"
            );
        }
    }
}

#[test]
fn policy_can_be_changed_between_writes_and_producer_is_independent() {
    let mut doc = Document::new();
    doc.add_page(Page::a4());
    doc.set_build_identification(BuildIdentification::Disabled);
    let disabled = info(&doc.to_bytes().unwrap()).unwrap();
    assert!(disabled.get(KEYS[0]).is_none());
    assert!(disabled
        .get("Producer")
        .unwrap()
        .as_string()
        .unwrap()
        .to_text()
        .starts_with("oxidize_pdf v"));
    doc.set_build_identification(BuildIdentification::Enabled);
    assert!(info(&doc.to_bytes().unwrap())
        .unwrap()
        .get(KEYS[0])
        .is_some());
    doc.set_build_identification(BuildIdentification::Disabled);
    assert!(info(&doc.to_bytes().unwrap())
        .unwrap()
        .get(KEYS[0])
        .is_none());
}

fn incremental(base: &[u8], method: usize, replace: bool) -> Vec<u8> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("base.pdf");
    std::fs::write(&path, base).unwrap();
    let mut output = Vec::new();
    let mut writer = PdfWriter::with_config(&mut output, WriterConfig::incremental());
    if replace {
        writer.set_incremental_info_policy(IncrementalInfoPolicy::Replace);
    }
    let mut added = Document::new();
    let mut page = Page::a4();
    page.text()
        .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
        .at(50.0, 650.0)
        .write("ReplacementMarker")
        .unwrap();
    added.add_page(page);
    added.set_title("Explicit replacement");
    added.set_build_identification(BuildIdentification::Disabled);
    match method {
        0 => writer.write_incremental_update(&path, &mut added).unwrap(),
        1 => writer
            .write_incremental_with_page_replacement(&path, &mut added)
            .unwrap(),
        2 => writer
            .write_incremental_with_overlay(&path, |page| {
                page.text()
                    .set_font(oxidize_pdf::text::Font::Helvetica, 12.0)
                    .at(50.0, 600.0)
                    .write("OverlayMarker")?;
                Ok(())
            })
            .unwrap(),
        _ => unreachable!(),
    }
    assert!(output.starts_with(base));
    output
}

#[test]
fn incremental_writers_preserve_the_source_info_by_default() {
    for policy in [BuildIdentification::Enabled, BuildIdentification::Disabled] {
        let base = generate(Some(policy), 0);
        let expected = info(&base);
        for method in 0..3 {
            let output = incremental(&base, method, false);
            assert_eq!(
                info(&output),
                expected,
                "method {method}, policy {policy:?}"
            );
            let mut reader = PdfReader::new(Cursor::new(&output)).unwrap();
            let base_reader = PdfReader::new(Cursor::new(&base)).unwrap();
            assert_eq!(reader.trailer().info(), base_reader.trailer().info());
            assert_eq!(
                reader.page_count().unwrap(),
                if method == 0 { 2 } else { 1 }
            );
        }
    }
}

#[test]
fn incremental_replacement_is_explicit_and_uses_document_policy() {
    let base = generate(None, 0);
    for method in 0..2 {
        let result = incremental(&base, method, true);
        let metadata = info(&result).unwrap();
        assert_eq!(
            metadata
                .get("Title")
                .unwrap()
                .as_string()
                .unwrap()
                .to_text(),
            "Explicit replacement"
        );
        for key in KEYS {
            assert!(metadata.get(key).is_none());
        }
        // Old identification bytes remain in the unchanged source revision.
        assert!(result.starts_with(&base));
    }
}

#[path = "common/pdf_assembler.rs"]
mod pdf_assembler;
#[test]
fn incremental_writers_do_not_add_info_when_source_has_none() {
    let base = pdf_assembler::assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Resources << >> >>".to_vec(),
    ]);
    for method in 0..3 {
        assert!(info(&incremental(&base, method, false)).is_none());
    }
}

#[test]
fn modern_incremental_editor_preserves_info_without_injecting_identification() {
    let base = generate(Some(BuildIdentification::Disabled), 0);
    let result = IncrementalTextNoteEditor::new(&base)
        .apply(&[TextNoteMutation::Add {
            page_index: 0,
            position: oxidize_pdf::geometry::Point::new(10.0, 20.0),
            contents: "Note".into(),
        }])
        .unwrap();
    assert!(result.pdf_bytes.starts_with(&base));
    assert_eq!(info(&result.pdf_bytes), info(&base));
    assert_eq!(
        IncrementalTextNoteEditor::new(&result.pdf_bytes)
            .notes()
            .unwrap()[0]
            .contents,
        "Note"
    );
}

const INDEPENDENT: &[(&str, &[u8])] = &[
    (
        "flat",
        include_bytes!("fixtures/incremental_metadata/flat.pdf"),
    ),
    (
        "nested",
        include_bytes!("fixtures/incremental_metadata/nested.pdf"),
    ),
    (
        "generation",
        include_bytes!("fixtures/incremental_metadata/generation.pdf"),
    ),
    (
        "compressed",
        include_bytes!("fixtures/incremental_metadata/compressed.pdf"),
    ),
];

#[test]
fn independent_metadata_survives_all_incremental_apis() {
    for &(name, base) in INDEPENDENT {
        for method in 0..3 {
            let out = incremental(base, method, false);
            let mut before = PdfReader::new(Cursor::new(base)).unwrap();
            let mut after = PdfReader::new(Cursor::new(&out)).unwrap();
            assert_eq!(before.trailer().info(), after.trailer().info());
            assert_eq!(info(base), info(&out));
            for key in ["Lang", "Metadata"] {
                assert_eq!(
                    before.catalog().unwrap().get(key),
                    after.catalog().unwrap().get(key),
                    "{name} API {method} {key}"
                );
            }
            assert_eq!(
                before.get_object(8, 0).unwrap(),
                after.get_object(8, 0).unwrap()
            );
            let parsed = oxidize_pdf::parser::PdfDocument::new(after);
            for i in 0..2 {
                assert_eq!(
                    parsed.get_page(i).unwrap().dict.get("Metadata"),
                    Some(&oxidize_pdf::parser::objects::PdfObject::Reference(8, 0)),
                    "{name} API {method} page {i}"
                );
            }
        }
    }
}

#[test]
fn issue_653_retains_leaf_pages_resources_and_parent_links() {
    for &(name, base) in INDEPENDENT {
        for method in 0..3 {
            let out = incremental(base, method, false);
            let mut reader = PdfReader::new(Cursor::new(&out)).unwrap();
            let root = reader.catalog().unwrap().get("Pages").unwrap().clone();
            let original =
                oxidize_pdf::parser::PdfDocument::new(PdfReader::new(Cursor::new(base)).unwrap());
            let parsed = oxidize_pdf::parser::PdfDocument::new(reader);
            assert_eq!(
                parsed.page_count().unwrap(),
                if method == 0 { 3 } else { 2 },
                "{name} API {method}"
            );
            let expected_new_page = if method == 0 { 2 } else { 0 };
            assert!(parsed
                .extract_text_from_page(expected_new_page)
                .unwrap()
                .text
                .contains(if method == 2 {
                    "OverlayMarker"
                } else {
                    "ReplacementMarker"
                }));
            for i in 0..2 {
                let page = parsed.get_page(i).unwrap();
                assert_eq!(
                    page.obj_ref,
                    original.get_page(i).unwrap().obj_ref,
                    "stable destinations {name} API {method}"
                );
                assert_eq!(page.dict.get("Parent"), Some(&root));
                if method == 1 && i == 0 {
                    continue;
                }
                assert_eq!(page.media_box, [0., 0., 600., 800.]);
                assert!(page.get_resources().unwrap().get("Font").is_some());
                let text = parsed.extract_text_from_page(i).unwrap().text;
                assert!(
                    text.contains(if i == 0 { "OriginalOne" } else { "OriginalTwo" }),
                    "{name} API {method} page {i}: {text:?}"
                );
            }
        }
    }
}

#[test]
fn issue_654_encrypted_inputs_are_rejected_before_any_output() {
    let base = include_bytes!("fixtures/incremental_metadata/encrypted.pdf");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("encrypted.pdf");
    std::fs::write(&path, base).unwrap();
    for method in 0..3 {
        for policy in [
            IncrementalInfoPolicy::Preserve,
            IncrementalInfoPolicy::Replace,
        ] {
            let mut out = Vec::new();
            let mut writer = PdfWriter::with_config(&mut out, WriterConfig::incremental());
            writer.set_incremental_info_policy(policy);
            let result = match method {
                0 => writer.write_incremental_update(&path, &mut doc()),
                1 => writer.write_incremental_with_page_replacement(&path, &mut doc()),
                _ => writer.write_incremental_with_overlay(&path, |_| {
                    panic!("callback must not see encrypted input")
                }),
            };
            let error = result.expect_err("encrypted input must be rejected");
            assert!(
                error.to_string().contains("Encrypted"),
                "API {method} {policy:?}: {error}"
            );
            assert!(
                out.is_empty(),
                "API {method} {policy:?} wrote {} bytes",
                out.len()
            );
        }
    }
}

#[test]
fn fingerprint_reports_effective_writer_configuration() {
    for compress in [false, true] {
        for xref in [false, true] {
            let config = WriterConfig {
                compress_streams: compress,
                use_xref_streams: xref,
                ..WriterConfig::default()
            };
            let bytes = doc().to_bytes_with_config(config).unwrap();
            let metadata = info(&bytes).unwrap();
            let value = metadata
                .get("oxidize-pdf-features")
                .unwrap()
                .as_string()
                .unwrap()
                .to_text();
            let bits = u16::from_str_radix(&value, 16).unwrap();
            assert_eq!(bits & 0x0200 != 0, compress);
            assert_eq!(bits & 0x0400 != 0, xref);
        }
    }
}

#[test]
fn overlays_preserve_binary_metadata_geometry_annotations_and_destinations() {
    let base = include_bytes!("fixtures/incremental_metadata/rich.pdf");
    for method in 0..3 {
        let out = incremental(base, method, false);
        let mut reader = PdfReader::new(Cursor::new(&out)).unwrap();
        let catalog = reader.catalog().unwrap();
        assert_eq!(
            catalog
                .get("Customer Field")
                .unwrap()
                .as_name()
                .unwrap()
                .as_str(),
            "Value#name"
        );
        assert_eq!(
            catalog.get("Rawÿ").unwrap().as_name().unwrap().as_str(),
            "Nÿ"
        );
        assert_eq!(
            catalog
                .get("CustomerCatalog")
                .unwrap()
                .as_string()
                .unwrap()
                .as_bytes(),
            b"\xfe\xff\x00E\x00s"
        );
        assert_eq!(
            catalog.get("OpenAction").unwrap().as_array().unwrap().0[0],
            oxidize_pdf::parser::objects::PdfObject::Reference(3, 0)
        );
        let parsed = oxidize_pdf::parser::PdfDocument::new(reader);
        let page = parsed.get_page(0).unwrap();
        assert_eq!(page.obj_ref, (3, 0));
        assert_eq!(
            page.dict
                .get("CustomerPage")
                .unwrap()
                .as_string()
                .unwrap()
                .as_bytes(),
            b"\x00\xff\x80"
        );
        let annots = parsed.resolve(page.dict.get("Annots").unwrap()).unwrap();
        assert_eq!(
            annots.as_array().unwrap().0,
            vec![oxidize_pdf::parser::objects::PdfObject::Reference(12, 0)]
        );
        if method != 1 {
            assert_eq!(page.media_box, [10., 20., 610., 820.]);
            assert_eq!(page.crop_box, Some([20., 30., 600., 800.]));
            assert_eq!(page.rotation, 90);
            assert!(parsed
                .extract_text_from_page(0)
                .unwrap()
                .text
                .contains("OriginalOne"));
        }
    }
}

#[test]
fn exhausted_object_numbers_reject_without_output() {
    let base = include_bytes!("fixtures/incremental_metadata/maxsize.pdf");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("maxsize.pdf");
    std::fs::write(&path, base).unwrap();
    for method in 0..3 {
        let mut output = Vec::new();
        let mut writer = PdfWriter::with_config(&mut output, WriterConfig::incremental());
        let result = match method {
            0 => writer.write_incremental_update(&path, &mut doc()),
            1 => writer.write_incremental_with_page_replacement(&path, &mut doc()),
            _ => writer.write_incremental_with_overlay(&path, |_| Ok(())),
        };
        assert!(result.is_err());
        assert!(output.is_empty());
    }
}

#[test]
fn repeated_updates_preserve_info_and_handle_eof_without_newline() {
    let base = include_bytes!("fixtures/incremental_metadata/flat.pdf");
    let base = base.strip_suffix(b"\n").unwrap();
    let first = incremental(base, 0, false);
    let second = incremental(&first, 2, false);
    assert!(second.starts_with(&first));
    assert_eq!(info(base), info(&second));
    let parsed =
        oxidize_pdf::parser::PdfDocument::new(PdfReader::new(Cursor::new(&second)).unwrap());
    assert_eq!(parsed.page_count().unwrap(), 3);
    for (i, text) in ["OriginalOne", "OriginalTwo", "ReplacementMarker"]
        .iter()
        .enumerate()
    {
        let extracted = parsed.extract_text_from_page(i as u32).unwrap().text;
        assert!(extracted.contains(text));
        assert!(extracted.contains("OverlayMarker"));
    }
}

#[test]
fn incremental_rejects_incompatible_writer_config_before_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("base.pdf");
    std::fs::write(&path, INDEPENDENT[0].1).unwrap();
    for method in 0..3 {
        let mut output = Vec::new();
        let mut writer = PdfWriter::with_config(&mut output, WriterConfig::modern());
        let result = match method {
            0 => writer.write_incremental_update(&path, &mut doc()),
            1 => writer.write_incremental_with_page_replacement(&path, &mut doc()),
            _ => writer.write_incremental_with_overlay(&path, |_| {
                panic!("invalid config reached callback")
            }),
        };
        assert!(result.unwrap_err().to_string().contains("classic xref"));
        assert!(output.is_empty());
    }
}
