//! Public consumers for nonzero page-tree generations (issue #836).

use oxidize_pdf::parser::{PdfObject, PdfReader};
use oxidize_pdf::signatures::{
    prepare_document_timestamp, prepare_incremental_signature, DocumentTimestampOptions,
    PreparedSignature, SignaturePreparationOptions,
};
use oxidize_pdf::writer::IncrementalObjectEditor;
use std::io::Cursor;

const FLAT_TABLE: &[u8] = include_bytes!("fixtures/page_generations/flat-table.pdf");
const FLAT_STREAM: &[u8] = include_bytes!("fixtures/page_generations/flat-stream.pdf");
const NESTED_TABLE: &[u8] = include_bytes!("fixtures/page_generations/nested-table.pdf");
const NESTED_STREAM: &[u8] = include_bytes!("fixtures/page_generations/nested-stream.pdf");
const FIXTURES: [&[u8]; 4] = [FLAT_TABLE, FLAT_STREAM, NESTED_TABLE, NESTED_STREAM];

fn assert_prepared(base: &[u8], prepared: &PreparedSignature, timestamp: bool) {
    let pdf = prepared.prepared_pdf();
    assert!(pdf.starts_with(base));
    let ranges = prepared.byte_range().ranges();
    assert_eq!(ranges.len(), 2);
    let (first, first_len) = ranges[0];
    let (second, second_len) = ranges[1];
    assert_eq!(first, 0);
    assert_eq!(second + second_len, pdf.len() as u64);
    let gap = &pdf[first_len as usize..second as usize];
    assert_eq!(gap.first(), Some(&b'<'));
    assert_eq!(gap.last(), Some(&b'>'));
    assert!(gap[1..gap.len() - 1].iter().all(u8::is_ascii_hexdigit));
    let mut expected_digest = pdf[..first_len as usize].to_vec();
    expected_digest.extend_from_slice(&pdf[second as usize..]);
    assert_eq!(prepared.bytes_to_digest(), expected_digest);

    let mut reader = PdfReader::new(Cursor::new(pdf)).unwrap();
    assert_eq!(reader.trailer().root().unwrap(), (1, 7));
    let root = reader.get_object(1, 7).unwrap().as_dict().unwrap();
    assert_eq!(root.get("Pages").unwrap().as_reference(), Some((2, 4)));
    let document = reader.into_document();
    assert_eq!(document.page_count().unwrap(), 1);
    let page = document.get_page(0).unwrap();
    assert_eq!(page.obj_ref, (3, 2));
    assert_eq!(page.media_box, [0.0, 0.0, 595.0, 842.0]);
    let annotation = page.annotations.unwrap().0[0].as_reference().unwrap();
    let mut reader = PdfReader::new(Cursor::new(pdf)).unwrap();
    let field = reader
        .get_object(annotation.0, annotation.1)
        .unwrap()
        .as_dict()
        .unwrap();
    assert_eq!(field.get("P").unwrap().as_reference(), Some((3, 2)));
    let value = field.get("V").unwrap().as_reference().unwrap();
    let signature = reader
        .get_object(value.0, value.1)
        .unwrap()
        .as_dict()
        .unwrap();
    assert_eq!(
        signature.get("Type").unwrap().as_name().unwrap().0,
        if timestamp { "DocTimeStamp" } else { "Sig" }
    );
    assert_eq!(
        signature.get("SubFilter").unwrap().as_name().unwrap().0,
        if timestamp {
            "ETSI.RFC3161"
        } else {
            "adbe.pkcs7.detached"
        }
    );
}

#[test]
fn timestamp_after_catalog_update_preserves_nonzero_generations() {
    for source in FIXTURES {
        let mut editor = IncrementalObjectEditor::new(source).unwrap();
        let id = editor.allocate_id().unwrap();
        editor
            .replace(id, PdfObject::Dictionary(Default::default()))
            .unwrap();
        editor
            .set_catalog_entry("DSS", PdfObject::Reference(id.0, id.1))
            .unwrap();
        let updated = editor.finish().unwrap();
        assert!(updated.starts_with(source));
        let prepared = prepare_document_timestamp(
            &updated,
            &DocumentTimestampOptions::invisible("ArchiveTimestamp"),
        )
        .unwrap();
        assert_prepared(&updated, &prepared, true);
    }
}

#[test]
fn ordinary_signature_preserves_nonzero_generations() {
    for source in FIXTURES {
        let prepared = prepare_incremental_signature(
            source,
            &SignaturePreparationOptions::invisible("Approval"),
        )
        .unwrap();
        assert_prepared(source, &prepared, false);
    }
}

#[test]
fn public_page_access_preserves_generation_and_inheritance() {
    for source in FIXTURES {
        let document = PdfReader::new(Cursor::new(source)).unwrap().into_document();
        assert_eq!(document.page_count().unwrap(), 1);
        let page = document.get_page(0).unwrap();
        assert_eq!(page.obj_ref, (3, 2));
        assert_eq!(page.media_box, [0.0, 0.0, 595.0, 842.0]);
    }
}

#[test]
#[cfg(feature = "signatures")]
fn timestamp_preserves_existing_signature_on_nonzero_generation_tree() {
    let source = include_bytes!("fixtures/page_generations/signed-nested-stream.pdf");
    let mut original = PdfReader::new(Cursor::new(source.as_slice())).unwrap();
    let signatures_before = original.signatures().unwrap();
    let verified_before = original.verify_signatures().unwrap();
    assert_eq!(verified_before.len(), 1);
    assert!(verified_before[0].hash_valid);
    assert!(verified_before[0].signature_valid);
    // The ephemeral fixture certificate is not a trusted public root.
    let prepared = prepare_document_timestamp(
        source,
        &DocumentTimestampOptions::invisible("ArchiveTimestamp"),
    )
    .unwrap();
    assert!(prepared.prepared_pdf().starts_with(source));
    let mut reader = PdfReader::new(Cursor::new(prepared.prepared_pdf())).unwrap();
    let signatures_after = reader.signatures().unwrap();
    assert_eq!(signatures_after.len(), 2);
    let retained = signatures_after
        .iter()
        .find(|sig| sig.name == signatures_before[0].name)
        .unwrap();
    assert_eq!(retained.contents, signatures_before[0].contents);
    assert_eq!(
        retained.byte_range.ranges(),
        signatures_before[0].byte_range.ranges()
    );
    let verified_after = reader.verify_signatures().unwrap();
    assert!(verified_after
        .iter()
        .any(|sig| sig.hash_valid && sig.signature_valid && sig.has_modifications_after_signing));
    assert_eq!(reader.into_document().get_page(0).unwrap().obj_ref, (3, 2));
}

#[test]
fn a_missing_generation_is_not_replaced_with_an_existing_one() {
    let mut source = FLAT_TABLE.to_vec();
    let position = source
        .windows(b"/Pages 2 4 R".len())
        .position(|w| w == b"/Pages 2 4 R")
        .unwrap();
    source[position + 9] = b'5';
    // Keep original offsets: replacing the single digit does not move objects.
    let document = PdfReader::new(Cursor::new(source.as_slice()))
        .unwrap()
        .into_document();
    assert!(matches!(
        document.get_page(0),
        Err(oxidize_pdf::parser::ParseError::InvalidReference(2, 5))
    ));
    assert!(
        prepare_document_timestamp(&source, &DocumentTimestampOptions::invisible("Rejected"))
            .is_err()
    );
    assert!(prepare_incremental_signature(
        &source,
        &SignaturePreparationOptions::invisible("Rejected")
    )
    .is_err());
}

#[test]
fn indirect_pages_reference_retains_the_resolved_generation() {
    let mut editor = IncrementalObjectEditor::new(NESTED_TABLE).unwrap();
    editor.replace((2, 4), PdfObject::Reference(4, 9)).unwrap();
    let source = editor.finish().unwrap();
    let document = PdfReader::new(Cursor::new(&source))
        .unwrap()
        .into_document();
    assert_eq!(document.page_count().unwrap(), 1);
    assert_eq!(document.get_page(0).unwrap().obj_ref, (3, 2));
    let prepared =
        prepare_incremental_signature(&source, &SignaturePreparationOptions::invisible("Approval"))
            .unwrap();
    assert_prepared(&source, &prepared, false);
}
