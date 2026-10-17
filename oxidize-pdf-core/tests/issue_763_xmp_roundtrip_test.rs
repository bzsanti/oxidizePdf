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

fn packet(attributes: &str, properties: &str) -> String {
    format!(
        r#"<r:RDF xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><r:Description xmlns:d="http://purl.org/dc/elements/1.1/" xmlns:t="http://ns.adobe.com/xap/1.0/" {attributes}>{properties}</r:Description></r:RDF>"#
    )
}
#[test]
fn attributes_default_namespaces_cdata_and_language_alternatives() {
    let parsed = XmpMetadata::parse(&packet(
        r#"d:title="café &amp; 日本語" t:CreatorTool="tool &amp; editor""#,
        "",
    ))
    .unwrap();
    assert_eq!(parsed.title.as_deref(), Some("café & 日本語"));
    assert_eq!(parsed.creator_tool.as_deref(), Some("tool & editor"));
    let parsed = XmpMetadata::parse(&packet("", r#"<title xmlns="http://purl.org/dc/elements/1.1/"><r:Alt><r:li xml:lang="es">Español</r:li><r:li xml:lang="x-default"><![CDATA[Default & <text>]]></r:li></r:Alt></title>"#)).unwrap();
    assert_eq!(parsed.title.as_deref(), Some("Default & <text>"));
}
#[test]
fn conflicting_nonliteral_and_cross_subject_properties_are_errors() {
    for xml in [
        packet("d:title='one'", "<d:title>two</d:title>"),
        packet("", "<d:title r:resource='urn:title'/>"),
        packet("", "<d:creator><r:Seq><other/></r:Seq></d:creator>"),
        packet("", "<d:title><r:Alt/></d:title>"),
        r#"<r:RDF xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:d="http://purl.org/dc/elements/1.1/" xmlns:p="http://www.aiim.org/pdfa/ns/id/"><r:Description r:about="urn:a" d:title="Title"/><r:Description r:about="urn:b" p:part="2" p:conformance="B"/></r:RDF>"#.into(),
    ] {
        assert!(matches!(XmpMetadata::parse(&xml), Err(PdfAError::XmpParseError(_))), "{xml}");
    }
}
#[test]
fn xml_forbidden_characters_are_rejected_without_silent_text_loss() {
    let model = XmpMetadata {
        title: Some("before\0after".into()),
        ..Default::default()
    };
    assert!(matches!(
        XmpMetadata::parse(&model.to_xml()),
        Err(PdfAError::XmpParseError(_))
    ));
}
#[test]
#[ignore = "requires python3; explicitly run in interoperability CI"]
fn independent_xml_reader_checks_every_serialized_property() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let value = "café & <x> 日本語\r\n";
    let model = XmpMetadata {
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
    let mut child = Command::new("python3").args(["-c", r#"import sys, xml.etree.ElementTree as E
root=E.fromstring(sys.stdin.read())
ns={'d':'http://purl.org/dc/elements/1.1/','r':'http://www.w3.org/1999/02/22-rdf-syntax-ns#','t':'http://ns.adobe.com/xap/1.0/','m':'http://ns.adobe.com/xap/1.0/mm/','p':'http://www.aiim.org/pdfa/ns/id/'}
value='café & <x> 日本語\r\n'
for path in ['d:title/r:Alt/r:li','d:description/r:Alt/r:li','t:CreateDate','t:ModifyDate','t:CreatorTool','m:DocumentID','m:InstanceID','p:amd','p:corr']:
    nodes=root.findall('.//'+path, ns)
    assert len(nodes)==1 and nodes[0].text==value, (path, [n.text for n in nodes])
assert [n.text for n in root.findall('.//d:creator/r:Seq/r:li',ns)]==[value,'second']
assert [n.text for n in root.findall('.//d:subject/r:Bag/r:li',ns)]==['café','日本語 & <tag>']
assert root.findtext('.//p:part',namespaces=ns)=='2'
assert root.findtext('.//p:conformance',namespaces=ns)=='B'
print('all properties verified')
"#]).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(model.to_xml().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"all properties verified\n");
}
