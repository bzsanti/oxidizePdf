//! Regression coverage for issue #538: mixed page-tree changes are one
//! lossless, validated incremental revision.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::operations::{
    mutate_pdf_pages_lossless, plan_pdf_page_mutations, PageMutation, PageMutationBatch,
};
use oxidize_pdf::parser::{PdfObject, PdfReader};
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Document, Page};
use std::fs;
use std::io::Cursor;
use std::process::Command;
use tempfile::TempDir;

fn base_pdf(catalog_extra: &str) -> Vec<u8> {
    assemble_pdf(&[
        format!("<< /Type /Catalog /Pages 2 0 R {catalog_extra} >>").into_bytes(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R] /Count 3 /MediaBox [0 0 200 300] /Resources 9 0 R >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 6 0 R /Annots [10 0 R] >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 7 0 R /Rotate 90 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /Contents 8 0 R >>".to_vec(),
        stream_obj("", b"first"),
        stream_obj("", b"second"),
        stream_obj("", b"third"),
        b"<< /ProcSet [/PDF] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Text /P 3 0 R /Rect [0 0 10 10] /Contents (note) >>".to_vec(),
        b"<< /Dests << /third [5 0 R /Fit] >> >>".to_vec(),
    ])
}

fn page_refs(bytes: &[u8]) -> Vec<(u32, u16)> {
    let document = PdfReader::new(Cursor::new(bytes)).unwrap().into_document();
    (0..document.page_count().unwrap())
        .map(|index| document.get_page(index).unwrap().obj_ref)
        .collect()
}

#[test]
fn applies_mixed_move_rotate_delete_and_duplicate_atomically() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("input.pdf");
    let output = directory.path().join("output.pdf");
    let source = base_pdf("");
    fs::write(&input, &source).unwrap();
    let batch = PageMutationBatch {
        operations: vec![
            PageMutation::Move { from: 2, to: 0 },
            PageMutation::Rotate {
                page: 1,
                degrees: 90,
            },
            PageMutation::Duplicate { page: 1, at: 2 },
            PageMutation::Delete { page: 3 },
        ],
    };

    let planned = plan_pdf_page_mutations(&input, &batch).unwrap();
    assert_eq!(planned.page_count, 3);
    assert_eq!(planned.unreachable_objects, vec![(4, 0), (7, 0)]);
    let report = mutate_pdf_pages_lossless(&input, &output, &batch).unwrap();
    assert_eq!(report, planned);

    let bytes = fs::read(output).unwrap();
    assert!(bytes.starts_with(&source));
    let refs = page_refs(&bytes);
    assert_eq!(refs[0], (5, 0));
    assert_eq!(refs[1], (3, 0));
    assert_ne!(refs[2], (3, 0));
    let mut reader = PdfReader::new(Cursor::new(bytes)).unwrap();
    let original = reader.get_object(3, 0).unwrap().as_dict().unwrap();
    assert_eq!(
        original.get("Rotate").and_then(PdfObject::as_integer),
        Some(90)
    );
    let duplicate = reader
        .get_object(refs[2].0, refs[2].1)
        .unwrap()
        .as_dict()
        .unwrap();
    assert_eq!(
        duplicate.get("Rotate").and_then(PdfObject::as_integer),
        Some(90)
    );
    assert_ne!(
        duplicate
            .get("Annots")
            .and_then(PdfObject::as_array)
            .unwrap()
            .0[0]
            .as_reference(),
        Some((10, 0)),
        "duplicate annotations must have independent identities"
    );
}

#[test]
fn imports_a_page_and_its_reachable_object_graph() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("input.pdf");
    let imported = directory.path().join("imported.pdf");
    let output = directory.path().join("output.pdf");
    let source = base_pdf("");
    fs::write(&input, &source).unwrap();
    fs::write(&imported, base_pdf("")).unwrap();
    let batch = PageMutationBatch {
        operations: vec![PageMutation::Insert {
            source: imported,
            page: 1,
            at: 1,
        }],
    };

    let report = mutate_pdf_pages_lossless(&input, &output, &batch).unwrap();
    assert_eq!(report.page_count, 4);
    assert!(!report.added_objects.is_empty());
    let bytes = fs::read(output).unwrap();
    assert!(bytes.starts_with(&source));
    let refs = page_refs(&bytes);
    assert_eq!(&refs[0..1], &[(3, 0)]);
    assert_ne!(refs[1], (4, 0));
    let mut reader = PdfReader::new(Cursor::new(bytes)).unwrap();
    let page = reader
        .get_object(refs[1].0, refs[1].1)
        .unwrap()
        .as_dict()
        .unwrap();
    assert_eq!(page.get("Rotate").and_then(PdfObject::as_integer), Some(90));
    let contents = page
        .get("Contents")
        .and_then(PdfObject::as_reference)
        .unwrap();
    assert!(reader
        .get_object(contents.0, contents.1)
        .unwrap()
        .as_stream()
        .is_some());
}

#[test]
fn rejects_deletion_referenced_by_catalog_semantics_without_touching_destination() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("input.pdf");
    let output = directory.path().join("output.pdf");
    fs::write(&input, base_pdf("/Names 11 0 R")).unwrap();
    fs::write(&output, b"keep destination").unwrap();
    let batch = PageMutationBatch {
        operations: vec![PageMutation::Delete { page: 2 }],
    };

    let error = mutate_pdf_pages_lossless(&input, &output, &batch)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("catalog-level structure references it"),
        "{error}"
    );
    assert_eq!(fs::read(output).unwrap(), b"keep destination");
}

#[test]
fn rejects_dangling_page_links_and_unsupported_widget_duplication() {
    let directory = TempDir::new().unwrap();
    let output = directory.path().join("output.pdf");
    let linked = directory.path().join("linked.pdf");
    fs::write(
        &linked,
        assemble_pdf(&[
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 10 10] >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /Annots [5 0 R] >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R >>".to_vec(),
            b"<< /Type /Annot /Subtype /Link /Rect [0 0 1 1] /Dest [4 0 R /Fit] >>".to_vec(),
        ]),
    )
    .unwrap();
    let error = plan_pdf_page_mutations(
        &linked,
        &PageMutationBatch {
            operations: vec![PageMutation::Delete { page: 1 }],
        },
    )
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("retained page structure references it"),
        "{error}"
    );

    let widget = directory.path().join("widget.pdf");
    fs::write(
        &widget,
        assemble_pdf(&[
            b"<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R >>".to_vec(),
            b"<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 10 10] >>".to_vec(),
            b"<< /Type /Page /Parent 2 0 R /Annots [4 0 R] >>".to_vec(),
            b"<< /Type /Annot /Subtype /Widget /FT /Tx /P 3 0 R /Rect [0 0 1 1] >>".to_vec(),
            b"<< /Fields [4 0 R] >>".to_vec(),
        ]),
    )
    .unwrap();
    let error = mutate_pdf_pages_lossless(
        &widget,
        &output,
        &PageMutationBatch {
            operations: vec![PageMutation::Duplicate { page: 0, at: 1 }],
        },
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("AcroForm field tree"), "{error}");
    assert!(!output.exists());
}

#[test]
fn supports_xref_stream_input_and_rejects_invalid_batches() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("modern.pdf");
    let output = directory.path().join("output.pdf");
    let mut document = Document::new();
    document.add_page(Page::a4());
    document.add_page(Page::letter());
    let source = document
        .to_bytes_with_config(WriterConfig::modern())
        .unwrap();
    fs::write(&input, &source).unwrap();
    let batch = PageMutationBatch {
        operations: vec![PageMutation::Rotate {
            page: 0,
            degrees: -90,
        }],
    };
    mutate_pdf_pages_lossless(&input, &output, &batch).unwrap();
    let bytes = fs::read(&output).unwrap();
    assert!(bytes.starts_with(&source));
    assert!(bytes
        .windows(b"/Type /XRef".len())
        .any(|window| window == b"/Type /XRef"));

    for operations in [
        vec![],
        vec![PageMutation::Rotate {
            page: 0,
            degrees: 45,
        }],
        vec![
            PageMutation::Delete { page: 0 },
            PageMutation::Delete { page: 0 },
        ],
    ] {
        assert!(plan_pdf_page_mutations(&input, &PageMutationBatch { operations }).is_err());
    }
}

#[test]
#[ignore = "requires qpdf and Poppler command-line tools"]
fn qpdf_and_poppler_accept_classic_and_xref_stream_page_mutations() {
    for (name, config) in [
        ("classic", WriterConfig::default()),
        ("xref-stream", WriterConfig::modern()),
    ] {
        let directory = TempDir::new().unwrap();
        let input = directory.path().join(format!("{name}-input.pdf"));
        let output = directory.path().join(format!("{name}-output.pdf"));
        let mut document = Document::new();
        document.add_page(Page::a4());
        document.add_page(Page::letter());
        let source = document.to_bytes_with_config(config).unwrap();
        fs::write(&input, source).unwrap();
        mutate_pdf_pages_lossless(
            &input,
            &output,
            &PageMutationBatch {
                operations: vec![
                    PageMutation::Duplicate { page: 0, at: 1 },
                    PageMutation::Rotate {
                        page: 1,
                        degrees: 90,
                    },
                    PageMutation::Delete { page: 2 },
                ],
            },
        )
        .unwrap();

        let check = Command::new("qpdf")
            .arg("--check")
            .arg(&output)
            .output()
            .unwrap();
        assert!(
            check.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&check.stderr)
        );
        let render_prefix = directory.path().join(format!("{name}-render"));
        let render = Command::new("pdftoppm")
            .args(["-f", "1", "-singlefile", "-png"])
            .arg(&output)
            .arg(&render_prefix)
            .output()
            .unwrap();
        assert!(
            render.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&render.stderr)
        );
        assert!(render_prefix.with_extension("png").is_file());
    }
}
