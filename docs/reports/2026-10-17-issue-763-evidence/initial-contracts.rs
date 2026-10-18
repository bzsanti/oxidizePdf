use oxidize_pdf::pdfa::{PdfAConformance, PdfAError, XmpMetadata, XmpPdfAIdentifier};

#[test]
fn all_supported_fields_preserve_unicode_entities_and_lists() {
    let value = " café & 日本語 <tag> \"quoted\" 'text'\r\n ";
    let original = XmpMetadata {
        title: Some(value.into()),
        creator: Some(vec![value.into(), "second".into()]),
        description: Some(value.into()),
        keywords: Some(vec!["café".into(), "日本語 & <tag>".into()]),
        create_date: Some(value.into()),
        modify_date: Some(value.into()),
        creator_tool: Some(value.into()),
        pdfa_id: Some(XmpPdfAIdentifier {
            part: 2,
            conformance: PdfAConformance::B,
            amd: Some(value.into()),
            corr: Some(value.into()),
        }),
        document_id: Some(value.into()),
        instance_id: Some(value.into()),
    };
    let parsed = XmpMetadata::parse(&original.to_xml()).unwrap();
    assert_eq!(parsed.title, original.title);
    assert_eq!(parsed.creator, original.creator);
    assert_eq!(parsed.description, original.description);
    assert_eq!(parsed.keywords, original.keywords);
    assert_eq!(parsed.create_date, original.create_date);
    assert_eq!(parsed.modify_date, original.modify_date);
    assert_eq!(parsed.creator_tool, original.creator_tool);
    assert_eq!(parsed.pdfa_id, original.pdfa_id);
    assert_eq!(parsed.document_id, original.document_id);
    assert_eq!(parsed.instance_id, original.instance_id);
}
#[test]
fn keywords_survive_serialization() {
    let model = XmpMetadata {
        keywords: Some(vec!["café".into(), "日本語".into()]),
        ..Default::default()
    };
    assert_eq!(
        XmpMetadata::parse(&model.to_xml()).unwrap().keywords,
        model.keywords
    );
}
#[test]
fn namespace_aliases_and_wrong_uri_controls() {
    let original = XmpMetadata {
        title: Some("Namespace title".into()),
        creator: Some(vec!["Author".into()]),
        ..Default::default()
    };
    let xml = original
        .to_xml()
        .replace("xmlns:dc=", "xmlns:d=")
        .replace("dc:", "d:");
    let parsed = XmpMetadata::parse(&xml).unwrap();
    assert_eq!(parsed.title, original.title);
    assert_eq!(parsed.creator, original.creator);
    let wrong = XmpMetadata::parse(
        &original
            .to_xml()
            .replace("http://purl.org/dc/elements/1.1/", "urn:unrelated"),
    )
    .unwrap();
    assert_eq!(wrong.title, None);
    assert_eq!(wrong.creator, None);
}
#[test]
fn empty_values_and_list_entries_remain_present() {
    let model = XmpMetadata {
        title: Some("".into()),
        creator: Some(vec!["".into(), "  ".into()]),
        keywords: Some(vec![]),
        ..Default::default()
    };
    let parsed = XmpMetadata::parse(&model.to_xml()).unwrap();
    assert_eq!(parsed.title, model.title);
    assert_eq!(parsed.creator, model.creator);
    assert_eq!(parsed.keywords, model.keywords);
}
#[test]
fn amendment_cannot_inject_properties() {
    let attack = "</pdfaid:amd><dc:title>injected</dc:title><pdfaid:amd>";
    let original = XmpMetadata {
        pdfa_id: Some(XmpPdfAIdentifier {
            part: 2,
            conformance: PdfAConformance::B,
            amd: Some(attack.into()),
            corr: None,
        }),
        ..Default::default()
    };
    let xml = original.to_xml();
    assert!(xml.contains("&lt;/pdfaid:amd&gt;"));
    let parsed = XmpMetadata::parse(&xml).unwrap();
    assert_eq!(parsed.title, None);
    assert_eq!(parsed.pdfa_id, original.pdfa_id);
}
#[test]
fn malformed_xml_is_a_typed_error() {
    assert!(matches!(
        XmpMetadata::parse("<x:xmpmeta><broken>"),
        Err(PdfAError::XmpParseError(_))
    ));
}
#[test]
fn ordinary_unicode_identifier_control() {
    let model = XmpMetadata {
        title: Some("café 日本語".into()),
        pdfa_id: Some(XmpPdfAIdentifier::new(2, PdfAConformance::B)),
        ..Default::default()
    };
    let parsed = XmpMetadata::parse(&model.to_xml()).unwrap();
    assert_eq!(parsed.title, model.title);
    assert_eq!(parsed.pdfa_id, model.pdfa_id);
}
