//! Regression coverage for issue #531: page reordering must retain the source
//! object graph and bytes instead of rebuilding pages through `Document`.

mod common;

use common::pdf_assembler::{assemble_pdf, stream_obj};
use oxidize_pdf::operations::reorder_pdf_pages_lossless;
use oxidize_pdf::parser::{PdfObject, PdfReader};
use oxidize_pdf::writer::WriterConfig;
use oxidize_pdf::{Document, Page};
use std::fs;
use std::io::Cursor;
use tempfile::TempDir;

fn nested_page_tree_pdf(extra_object: Option<Vec<u8>>) -> Vec<u8> {
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R /Metadata 11 0 R /Outlines 12 0 R \
          /Names 13 0 R /AcroForm 14 0 R /StructTreeRoot 15 0 R >>"
            .to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>".to_vec(),
        b"<< /Type /Pages /Parent 2 0 R /Kids [5 0 R] /Count 1 \
          /Resources 7 0 R /MediaBox [0 0 300 400] /CropBox [10 20 290 380] >>"
            .to_vec(),
        b"<< /Type /Pages /Parent 2 0 R /Kids [6 0 R] /Count 1 \
          /Resources 8 0 R /MediaBox [0 0 500 600] /Rotate 90 >>"
            .to_vec(),
        b"<< /Type /Page /Parent 3 0 R /Contents 9 0 R /Annots [16 0 R 17 0 R] \
          /StructParents 0 >>"
            .to_vec(),
        b"<< /Type /Page /Parent 4 0 R /Contents 10 0 R >>".to_vec(),
        b"<< /ProcSet [/PDF] /Marker (left) >>".to_vec(),
        b"<< /ProcSet [/PDF] /Marker (right) >>".to_vec(),
        stream_obj("", b""),
        stream_obj("", b""),
        stream_obj("/Type /Metadata /Subtype /XML", b"<keep-me/>"),
        b"<< /Type /Outlines /Count 0 >>".to_vec(),
        b"<< /Dests << /First [5 0 R /Fit] >> \
          /EmbeddedFiles << /Names [(attachment.txt) 19 0 R] >> >>"
            .to_vec(),
        b"<< /Fields [17 0 R] >>".to_vec(),
        b"<< /Type /StructTreeRoot /K [] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Text /Rect [0 0 10 10] >>".to_vec(),
        b"<< /Type /Annot /Subtype /Widget /FT /Tx /T (field) /P 5 0 R \
          /Rect [10 10 20 20] >>"
            .to_vec(),
        stream_obj("/Type /EmbeddedFile", b"attachment payload"),
        b"<< /Type /Filespec /F (attachment.txt) /EF << /F 18 0 R >> >>".to_vec(),
    ];
    if let Some(object) = extra_object {
        objects.push(object);
    }
    assemble_pdf(&objects)
}

fn page_references(bytes: &[u8]) -> Vec<(u32, u16)> {
    let document = PdfReader::new(Cursor::new(bytes))
        .expect("PDF should parse")
        .into_document();
    (0..document.page_count().expect("page count"))
        .map(|index| document.get_page(index).expect("page").obj_ref)
        .collect()
}

#[test]
fn nested_tree_reorder_preserves_prefix_ids_and_effective_inheritance() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("input.pdf");
    let output = directory.path().join("output.pdf");
    let source = nested_page_tree_pdf(None);
    fs::write(&input, &source).unwrap();

    reorder_pdf_pages_lossless(&input, &output, &[1, 0]).expect("lossless reorder");
    let reordered = fs::read(&output).unwrap();
    assert!(
        reordered.starts_with(&source),
        "source must be an exact prefix"
    );
    assert_eq!(page_references(&reordered), vec![(6, 0), (5, 0)]);

    let mut reader = PdfReader::new(Cursor::new(&reordered)).unwrap();
    let page_6 = reader.get_object(6, 0).unwrap().as_dict().unwrap().clone();
    assert_eq!(
        page_6.get("Parent").and_then(PdfObject::as_reference),
        Some((2, 0))
    );
    assert_eq!(
        page_6.get("Resources").and_then(PdfObject::as_reference),
        Some((8, 0))
    );
    assert_eq!(
        page_6.get("Rotate").and_then(PdfObject::as_integer),
        Some(90)
    );
    assert_eq!(
        page_6.get("MediaBox"),
        Some(&PdfObject::Array(oxidize_pdf::PdfArray(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(500),
            PdfObject::Integer(600),
        ])))
    );

    let page_5 = reader.get_object(5, 0).unwrap().as_dict().unwrap();
    assert_eq!(
        page_5.get("Parent").and_then(PdfObject::as_reference),
        Some((2, 0))
    );
    assert_eq!(
        page_5.get("Resources").and_then(PdfObject::as_reference),
        Some((7, 0))
    );
    assert_eq!(
        page_5.get("CropBox"),
        Some(&PdfObject::Array(oxidize_pdf::PdfArray(vec![
            PdfObject::Integer(10),
            PdfObject::Integer(20),
            PdfObject::Integer(290),
            PdfObject::Integer(380),
        ])))
    );
    assert_eq!(
        page_5
            .get("Annots")
            .and_then(PdfObject::as_array)
            .and_then(|array| array.0.first())
            .and_then(PdfObject::as_reference),
        Some((16, 0))
    );
    let catalog = reader.catalog().unwrap();
    for (key, reference) in [
        ("Metadata", (11, 0)),
        ("Outlines", (12, 0)),
        ("Names", (13, 0)),
        ("AcroForm", (14, 0)),
        ("StructTreeRoot", (15, 0)),
    ] {
        assert_eq!(
            catalog.get(key).and_then(PdfObject::as_reference),
            Some(reference)
        );
    }
    assert!(reordered
        .windows(b"<keep-me/>".len())
        .any(|window| window == b"<keep-me/>"));
    let form = reader.get_object(14, 0).unwrap().as_dict().unwrap();
    assert_eq!(
        form.get("Fields")
            .and_then(PdfObject::as_array)
            .and_then(|fields| fields.0.first())
            .and_then(PdfObject::as_reference),
        Some((17, 0))
    );
    let names = reader.get_object(13, 0).unwrap().as_dict().unwrap();
    assert!(names.contains_key("EmbeddedFiles"));
    assert!(
        reader.get_object(17, 0).is_ok(),
        "widget must remain reachable"
    );
    assert!(
        reader.get_object(18, 0).is_ok(),
        "attachment stream must remain reachable"
    );
    assert!(
        reader.get_object(19, 0).is_ok(),
        "file specification must remain reachable"
    );
}

#[test]
fn flat_tree_rewrites_only_the_page_tree_root() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("flat.pdf");
    let output = directory.path().join("output.pdf");
    let source = assemble_pdf(&[
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 \
          /Resources 5 0 R /MediaBox [0 0 100 100] >>"
            .to_vec(),
        b"<< /Type /Page /Parent 2 0 R >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R >>".to_vec(),
        b"<< /ProcSet [/PDF] >>".to_vec(),
    ]);
    fs::write(&input, &source).unwrap();

    reorder_pdf_pages_lossless(&input, &output, &[1, 0]).expect("flat reorder");
    let reordered = fs::read(&output).unwrap();
    let revision = &reordered[source.len()..];
    assert!(revision.windows(7).any(|window| window == b"2 0 obj"));
    assert!(!revision.windows(7).any(|window| window == b"3 0 obj"));
    assert!(!revision.windows(7).any(|window| window == b"4 0 obj"));
    assert_eq!(page_references(&reordered), vec![(4, 0), (3, 0)]);
}

#[test]
fn accepts_a_source_with_an_existing_incremental_revision() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("input.pdf");
    let first = directory.path().join("first.pdf");
    let second = directory.path().join("second.pdf");
    let source = nested_page_tree_pdf(None);
    fs::write(&input, &source).unwrap();

    reorder_pdf_pages_lossless(&input, &first, &[1, 0]).expect("first revision");
    let first_revision = fs::read(&first).unwrap();
    reorder_pdf_pages_lossless(&first, &second, &[1, 0]).expect("second revision");
    let second_revision = fs::read(&second).unwrap();

    assert!(second_revision.starts_with(&first_revision));
    assert_eq!(page_references(&second_revision), vec![(5, 0), (6, 0)]);
}

#[test]
fn rejects_non_permutations_without_replacing_existing_destination() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("input.pdf");
    let output = directory.path().join("output.pdf");
    fs::write(&input, nested_page_tree_pdf(None)).unwrap();
    fs::write(&output, b"existing destination").unwrap();

    for order in [&[0][..], &[0, 0][..], &[0, 2][..]] {
        let error = reorder_pdf_pages_lossless(&input, &output, order)
            .expect_err("invalid order must fail")
            .to_string();
        assert!(
            error.contains("page order")
                || error.contains("duplicated")
                || error.contains("out of bounds")
        );
        assert_eq!(fs::read(&output).unwrap(), b"existing destination");
    }
}

#[test]
fn permits_ordinary_approval_signatures() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("signed.pdf");
    let output = directory.path().join("output.pdf");
    let signature = b"<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> >>".to_vec();
    let source = nested_page_tree_pdf(Some(signature));
    fs::write(&input, &source).unwrap();

    reorder_pdf_pages_lossless(&input, &output, &[1, 0])
        .expect("approval signature must not establish a DocMDP policy");
    assert!(fs::read(output).unwrap().starts_with(&source));
}

#[test]
fn supports_xref_stream_sources_and_emits_an_incremental_xref_stream() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("xref-stream.pdf");
    let output = directory.path().join("output.pdf");
    let mut document = Document::new();
    document.add_page(Page::a4());
    document.add_page(Page::a4());
    let source = document
        .to_bytes_with_config(WriterConfig::modern())
        .unwrap();
    fs::write(&input, &source).unwrap();
    let original_refs = page_references(&source);

    reorder_pdf_pages_lossless(&input, &output, &[1, 0]).expect("xref-stream reorder");
    let reordered = fs::read(&output).unwrap();
    assert!(reordered.starts_with(&source));
    assert_eq!(
        page_references(&reordered),
        vec![original_refs[1], original_refs[0]]
    );
    assert!(reordered
        .windows(b"/Type /XRef".len())
        .any(|window| window == b"/Type /XRef"));
}

#[test]
fn atomically_replaces_the_input_when_paths_are_identical() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("in-place.pdf");
    let source = nested_page_tree_pdf(None);
    fs::write(&path, &source).unwrap();

    reorder_pdf_pages_lossless(&path, &path, &[1, 0]).expect("in-place reorder");
    let reordered = fs::read(&path).unwrap();
    assert!(reordered.starts_with(&source));
    assert_eq!(page_references(&reordered), vec![(6, 0), (5, 0)]);
}

#[test]
fn rejects_malformed_parent_links_and_page_counts() {
    let directory = TempDir::new().unwrap();
    let output = directory.path().join("output.pdf");

    for (name, objects) in [
        (
            "bad-parent",
            vec![
                b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
                b"<< /Type /Page /Parent 99 0 R /MediaBox [0 0 10 10] >>".to_vec(),
            ],
        ),
        (
            "bad-count",
            vec![
                b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
                b"<< /Type /Pages /Kids [3 0 R] /Count 2 >>".to_vec(),
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>".to_vec(),
            ],
        ),
        (
            "cycle",
            vec![
                b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
                b"<< /Type /Pages /Kids [3 0 R 3 0 R] /Count 2 >>".to_vec(),
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 10 10] >>".to_vec(),
            ],
        ),
    ] {
        let input = directory.path().join(format!("{name}.pdf"));
        fs::write(&input, assemble_pdf(&objects)).unwrap();
        let error = reorder_pdf_pages_lossless(&input, &output, &[0])
            .expect_err("malformed tree must fail")
            .to_string();
        assert!(error.contains("/Parent") || error.contains("/Count") || error.contains("cycle"));
        assert!(!output.exists());
    }
}

#[test]
fn rejects_encrypted_documents() {
    let directory = TempDir::new().unwrap();
    let input = directory.path().join("encrypted.pdf");
    let output = directory.path().join("output.pdf");
    let mut document = Document::new();
    document.add_page(Page::a4());
    document.add_page(Page::a4());
    document.encrypt_with_passwords("user", "owner");
    fs::write(&input, document.to_bytes().unwrap()).unwrap();

    let error = reorder_pdf_pages_lossless(&input, &output, &[1, 0])
        .expect_err("encrypted input must be rejected")
        .to_string();
    assert!(error.contains("encrypted PDFs"));
    assert!(!output.exists());
}
