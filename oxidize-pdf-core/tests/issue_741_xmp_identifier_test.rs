use oxidize_pdf::pdfa::{PdfAConformance, PdfAError, XmpMetadata, XmpPdfAIdentifier};

fn packet(attributes: &str, properties: &str) -> String {
    format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><r:RDF xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><r:Description xmlns:p="http://www.aiim.org/pdfa/ns/id/" {attributes}>{properties}</r:Description></r:RDF></x:xmpmeta>"#
    )
}
fn identifier(xml: &str) -> Option<XmpPdfAIdentifier> {
    XmpMetadata::parse(xml).unwrap().pdfa_id
}
fn rejects(xml: &str) {
    assert!(
        matches!(XmpMetadata::parse(xml), Err(PdfAError::XmpParseError(_))),
        "{xml:.200}"
    );
}
#[test]
fn equivalent_attribute_element_and_mixed_forms() {
    for (attributes, properties) in [
        (r#"p:part="2" p:conformance="B""#, ""),
        ("", "<p:part>2</p:part><p:conformance>B</p:conformance>"),
        (r#"p:part="2""#, "<p:conformance>B</p:conformance>"),
    ] {
        assert_eq!(
            identifier(&packet(attributes, properties)),
            Some(XmpPdfAIdentifier::new(2, PdfAConformance::B))
        );
    }
}
#[test]
fn namespace_identity_and_default_namespace() {
    let xml = packet(
        "",
        r#"<part xmlns="http://www.aiim.org/pdfa/ns/id/">3</part><conformance xmlns="http://www.aiim.org/pdfa/ns/id/">U</conformance>"#,
    );
    assert_eq!(
        identifier(&xml),
        Some(XmpPdfAIdentifier::new(3, PdfAConformance::U))
    );
    for xml in [
        packet(r#"p:part="2" p:conformance="B""#, "")
            .replace("http://www.aiim.org/pdfa/ns/id/", "urn:unrelated"),
        packet(r#"part="2" conformance="B""#, ""),
        packet(
            "",
            r#"<p:part xmlns:p="urn:unrelated">2</p:part><p:conformance xmlns:p="urn:unrelated">B</p:conformance>"#,
        ),
    ] {
        assert_eq!(identifier(&xml), None);
    }
}
#[test]
fn missing_is_distinct_from_partial_invalid_and_unsupported() {
    assert_eq!(identifier(&packet("", "")), None);
    for attributes in [
        r#"p:part="2""#,
        r#"p:conformance="B""#,
        r#"p:part="two" p:conformance="B""#,
        r#"p:part="4" p:conformance="B""#,
        r#"p:part="1" p:conformance="U""#,
        r#"p:part="2" p:conformance="Z""#,
        r#"p:amd="amd1""#,
    ] {
        rejects(&packet(attributes, ""));
    }
}
#[test]
fn entity_and_cdata_values_are_decoded() {
    let id = identifier(&packet(
        r#"p:part="&#50;" p:conformance="&#x42;" p:amd="a&amp;b""#,
        "<p:corr><![CDATA[c<d]]></p:corr>",
    ))
    .unwrap();
    assert_eq!(
        id,
        XmpPdfAIdentifier {
            part: 2,
            conformance: PdfAConformance::B,
            amd: Some("a&b".into()),
            corr: Some("c<d".into())
        }
    );
}
#[test]
fn malformed_xml_and_dtds_are_typed_errors() {
    for xml in [
        "",
        "<x:xmpmeta><broken>",
        "<a/><b/>",
        "<a>&unknown;</a>",
        "<a>\0</a>",
        "<a x='1' x='2'/>",
        "<!DOCTYPE a SYSTEM 'file:///etc/passwd'><a/>",
    ] {
        rejects(xml);
    }
}
#[test]
fn conflicting_and_nonliteral_properties_are_rejected() {
    for properties in [
        "<p:part>3</p:part>",
        "<p:part><r:Bag/></p:part>",
        "<p:part r:resource='2'/>",
    ] {
        rejects(&packet(r#"p:part="2" p:conformance="B""#, properties));
    }
}
#[test]
fn serialized_identifier_roundtrips() {
    let metadata = XmpMetadata {
        pdfa_id: Some(XmpPdfAIdentifier::new(1, PdfAConformance::B)),
        ..Default::default()
    };
    assert_eq!(identifier(&metadata.to_xml()), metadata.pdfa_id);
}
#[test]
fn descriptions_must_describe_the_same_subject() {
    let xml = r#"<r:RDF xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:p="http://www.aiim.org/pdfa/ns/id/"><r:Description r:about="urn:a" p:part="2"/><r:Description r:about="urn:a" p:conformance="B"/></r:RDF>"#;
    assert_eq!(
        identifier(xml),
        Some(XmpPdfAIdentifier::new(2, PdfAConformance::B))
    );
    rejects(&xml.replace(
        "r:about=\"urn:a\" p:conformance",
        "r:about=\"urn:b\" p:conformance",
    ));
    for subject in ["r:nodeID='node'", "r:ID='node'"] {
        rejects(&packet(
            subject,
            "<p:part>2</p:part><p:conformance>B</p:conformance>",
        ));
    }
}
#[test]
fn xml_base_subject_resolution_is_not_silently_ignored() {
    rejects(
        r#"<r:RDF xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#" xmlns:p="http://www.aiim.org/pdfa/ns/id/"><r:Description xml:base="https://a.example/" r:about="doc" p:part="2"/><r:Description xml:base="https://b.example/" r:about="doc" p:conformance="B"/></r:RDF>"#,
    );
}
#[test]
fn input_size_and_node_limits() {
    let xml = format!("<a>{}</a>", " ".repeat(16 * 1024 * 1024 - 7));
    assert!(XmpMetadata::parse(&xml).is_ok());
    rejects(&(xml + " "));
    assert!(XmpMetadata::parse(&format!("<a>{}</a>", "<b/>".repeat(99_998))).is_ok());
    rejects(&format!("<a>{}</a>", "<b/>".repeat(100_000)));
}
#[test]
fn nesting_is_bounded_before_tree_recursion() {
    assert!(XmpMetadata::parse(&format!("{}{}", "<a>".repeat(128), "</a>".repeat(128))).is_ok());
    rejects(&format!("{}{}", "<a>".repeat(129), "</a>".repeat(129)));
    rejects(&format!("{}<b/>{}", "<a>".repeat(128), "</a>".repeat(128)));
}
#[test]
#[ignore = "requires python3; explicitly run in interoperability CI"]
fn independent_xml_reader_agrees_on_identifier_forms() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    for xml in [
        packet(r#"p:part="2" p:conformance="B""#, ""),
        packet("", "<p:part>2</p:part><p:conformance>B</p:conformance>"),
    ] {
        let mut child = Command::new("python3").args(["-c", r#"import sys,xml.etree.ElementTree as E
root=E.fromstring(sys.stdin.read())
d=root.find('.//{http://www.w3.org/1999/02/22-rdf-syntax-ns#}Description')
ns='{http://www.aiim.org/pdfa/ns/id/}'
values=[d.get(ns+k) if d.get(ns+k) is not None else d.findtext(ns+k) for k in ['part','conformance']]
assert values == ['2','B'], values
print(*values)
"#]).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(xml.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"2 B\n");
        assert_eq!(
            identifier(&xml),
            Some(XmpPdfAIdentifier::new(2, PdfAConformance::B))
        );
    }
}
