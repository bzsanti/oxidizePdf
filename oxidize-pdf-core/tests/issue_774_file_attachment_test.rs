use oxidize_pdf::annotations::{
    Annotation, FileAttachmentAnnotation, FileAttachmentIcon, FreeTextAnnotation, InkAnnotation,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::parser::{
    objects::{PdfDictionary, PdfObject},
    PdfReader,
};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

fn rect() -> Rectangle {
    Rectangle::new(Point::new(10.0, 10.0), Point::new(100.0, 60.0))
}
fn document(annotation: Annotation) -> Vec<u8> {
    let mut page = Page::new(300.0, 300.0);
    page.add_annotation(annotation);
    let mut doc = Document::new();
    doc.add_page(page);
    doc.to_bytes().unwrap()
}
fn annotation(reader: &mut PdfReader<Cursor<&[u8]>>) -> PdfDictionary {
    let pages = reader.pages().unwrap();
    let page_ref = pages.get("Kids").unwrap().as_array().unwrap().0[0]
        .as_reference()
        .unwrap();
    let page = reader
        .get_object(page_ref.0, page_ref.1)
        .unwrap()
        .as_dict()
        .unwrap();
    let annot_ref = page.get("Annots").unwrap().as_array().unwrap().0[0]
        .as_reference()
        .unwrap();
    reader
        .get_object(annot_ref.0, annot_ref.1)
        .unwrap()
        .as_dict()
        .unwrap()
        .clone()
}

#[test]
fn embedded_file_is_indirect_and_retains_exact_binary_payload() {
    for payload in [vec![], b"\0binary\xff\nendstream\nendobj\0".to_vec()] {
        let pdf = document(
            FileAttachmentAnnotation::new(rect(), "payload.bin".into(), payload.clone())
                .with_icon(FileAttachmentIcon::Tag)
                .to_annotation(),
        );
        let mut reader = PdfReader::new(Cursor::new(pdf.as_slice())).unwrap();
        let annot = annotation(&mut reader);
        assert_eq!(
            annot.get("Subtype").unwrap().as_name().unwrap().as_str(),
            "FileAttachment"
        );
        assert_eq!(
            annot.get("Name").unwrap().as_name().unwrap().as_str(),
            "Tag"
        );
        let spec = annot.get("FS").unwrap().as_dict().unwrap();
        let reference = spec
            .get("EF")
            .unwrap()
            .as_dict()
            .unwrap()
            .get("F")
            .unwrap()
            .as_reference()
            .expect("indirect EmbeddedFile stream");
        let object = reader.get_object(reference.0, reference.1).unwrap().clone();
        let stream = object.as_stream().expect("actual EmbeddedFile stream");
        assert_eq!(
            stream.dict.get("Type").unwrap().as_name().unwrap().as_str(),
            "EmbeddedFile"
        );
        assert_eq!(stream.decode(reader.options()).unwrap(), payload);
    }
}

#[test]
fn mime_and_unicode_filename_roundtrip_without_dictionary_injection() {
    for mime in [
        "text/plain",
        "application/x-test#(a)[b] <tag>%",
        "text/#00",
        "text/plain\n/Injected true",
    ] {
        let pdf = document(
            FileAttachmentAnnotation::new(rect(), "niño-東京.txt".into(), b"payload".to_vec())
                .with_mime_type(mime.into())
                .to_annotation(),
        );
        let mut reader = PdfReader::new(Cursor::new(pdf.as_slice())).unwrap();
        let annot = annotation(&mut reader);
        let spec = annot.get("FS").unwrap().as_dict().unwrap();
        assert_eq!(
            spec.get("UF").unwrap().as_string().unwrap().to_text(),
            "niño-東京.txt"
        );
        let ef = spec.get("EF").unwrap().as_dict().unwrap();
        let reference = ef.get("F").unwrap().as_reference().unwrap();
        assert_eq!(ef.get("UF").unwrap().as_reference(), Some(reference));
        let stream = reader
            .get_object(reference.0, reference.1)
            .unwrap()
            .as_stream()
            .unwrap();
        assert_eq!(
            stream
                .dict
                .get("Subtype")
                .unwrap()
                .as_name()
                .unwrap()
                .as_str(),
            mime
        );
        assert!(stream.dict.get("Injected").is_none());
    }
}

#[test]
fn free_text_and_ink_creation_controls() {
    let pdf = document(
        FreeTextAnnotation::new(rect(), "control")
            .with_justification(2)
            .to_annotation(),
    );
    let mut reader = PdfReader::new(Cursor::new(pdf.as_slice())).unwrap();
    let annot = annotation(&mut reader);
    assert_eq!(
        annot
            .get("Contents")
            .unwrap()
            .as_string()
            .unwrap()
            .as_bytes(),
        b"control"
    );
    assert_eq!(annot.get("Q").unwrap().as_integer(), Some(2));
    let pdf = document(
        InkAnnotation::new()
            .add_stroke(vec![Point::new(10.0, 20.0), Point::new(30.0, 40.0)])
            .to_annotation(),
    );
    let mut reader = PdfReader::new(Cursor::new(pdf.as_slice())).unwrap();
    let annot = annotation(&mut reader);
    let stroke = annot.get("InkList").unwrap().as_array().unwrap().0[0]
        .as_array()
        .unwrap();
    assert_eq!(
        stroke.0.iter().map(PdfObject::as_real).collect::<Vec<_>>(),
        vec![Some(10.0), Some(20.0), Some(30.0), Some(40.0)]
    );
}

#[test]
#[ignore = "requires qpdf and Poppler pdfdetach; exercised by interoperability CI"]
fn independent_tools_extract_binary_and_empty_attachments() {
    for payload in [vec![], (0..=255).collect::<Vec<u8>>()] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.pdf");
        let output = dir.path().join("extracted.bin");
        std::fs::write(
            &input,
            document(
                FileAttachmentAnnotation::new(rect(), "niño-東京.bin".into(), payload.clone())
                    .with_mime_type("application/octet-stream".into())
                    .to_annotation(),
            ),
        )
        .unwrap();
        let check = std::process::Command::new("qpdf")
            .arg("--check")
            .arg(&input)
            .output()
            .expect("qpdf required");
        assert!(
            check.status.success(),
            "{}",
            String::from_utf8_lossy(&check.stderr)
        );
        let listing = std::process::Command::new("pdfdetach")
            .arg("-list")
            .arg(&input)
            .output()
            .expect("pdfdetach required");
        assert!(listing.status.success());
        assert!(
            String::from_utf8_lossy(&listing.stdout).contains("niño-東京.bin"),
            "{}",
            String::from_utf8_lossy(&listing.stdout)
        );
        let extract = std::process::Command::new("pdfdetach")
            .args(["-save", "1", "-o"])
            .arg(&output)
            .arg(&input)
            .output()
            .unwrap();
        assert!(
            extract.status.success(),
            "{}",
            String::from_utf8_lossy(&extract.stderr)
        );
        assert_eq!(std::fs::read(output).unwrap(), payload);
    }
}

#[test]
#[ignore = "requires qpdf and Poppler pdfdetach; exercised by interoperability CI"]
fn encrypted_attachment_is_not_plaintext_and_decrypts_independently() {
    use oxidize_pdf::document::{DocumentEncryption, EncryptionStrength};
    use oxidize_pdf::encryption::Permissions;
    let payload = b"unique confidential attachment bytes\0\xff";
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_annotation(
        FileAttachmentAnnotation::new(rect(), "secret.bin".into(), payload.to_vec())
            .with_mime_type("application/octet-stream".into())
            .to_annotation(),
    );
    doc.add_page(page);
    doc.set_encryption(DocumentEncryption::new(
        "user",
        "owner",
        Permissions::all(),
        EncryptionStrength::Aes128,
    ));
    let pdf = doc.to_bytes().unwrap();
    assert!(!pdf.windows(payload.len()).any(|window| window == payload));
    let dir = tempfile::tempdir().unwrap();
    let encrypted = dir.path().join("encrypted.pdf");
    let decrypted = dir.path().join("decrypted.pdf");
    let extracted = dir.path().join("attachment.bin");
    std::fs::write(&encrypted, pdf).unwrap();
    let decrypt = std::process::Command::new("qpdf")
        .args(["--password=user", "--decrypt"])
        .arg(&encrypted)
        .arg(&decrypted)
        .output()
        .unwrap();
    assert!(
        decrypt.status.success(),
        "{}",
        String::from_utf8_lossy(&decrypt.stderr)
    );
    let extract = std::process::Command::new("pdfdetach")
        .args(["-save", "1", "-o"])
        .arg(&extracted)
        .arg(&decrypted)
        .output()
        .unwrap();
    assert!(
        extract.status.success(),
        "{}",
        String::from_utf8_lossy(&extract.stderr)
    );
    assert_eq!(std::fs::read(extracted).unwrap(), payload);
}

#[test]
fn compressed_object_path_preserves_attachment_mapping() {
    use oxidize_pdf::writer::WriterConfig;
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_annotation(
        FileAttachmentAnnotation::new(
            rect(),
            "stream.bin".into(),
            b"object stream payload".to_vec(),
        )
        .with_mime_type("application/octet-stream".into())
        .to_annotation(),
    );
    doc.add_page(page);
    let pdf = doc.to_bytes_with_config(WriterConfig::modern()).unwrap();
    let mut reader = PdfReader::new(Cursor::new(pdf.as_slice())).unwrap();
    let annot = annotation(&mut reader);
    let spec = annot.get("FS").unwrap().as_dict().unwrap();
    let reference = spec
        .get("EF")
        .unwrap()
        .as_dict()
        .unwrap()
        .get("F")
        .unwrap()
        .as_reference()
        .unwrap();
    let stream = reader
        .get_object(reference.0, reference.1)
        .unwrap()
        .as_stream()
        .unwrap()
        .clone();
    assert_eq!(
        stream.decode(reader.options()).unwrap(),
        b"object stream payload"
    );
    assert_eq!(
        stream
            .dict
            .get("Subtype")
            .unwrap()
            .as_name()
            .unwrap()
            .as_str(),
        "application/octet-stream"
    );
}

#[test]
fn nul_mime_name_is_rejected_before_emitting_an_invalid_pdf_name() {
    let mut doc = Document::new();
    let mut page = Page::a4();
    page.add_annotation(
        FileAttachmentAnnotation::new(rect(), "payload.bin".into(), b"payload".to_vec())
            .with_mime_type("text/\0plain".into())
            .to_annotation(),
    );
    doc.add_page(page);
    assert!(matches!(
        doc.to_bytes(),
        Err(oxidize_pdf::PdfError::InvalidOperation(_))
    ));
}
