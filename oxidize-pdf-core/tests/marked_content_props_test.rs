//! Parser-level tests for the typed marked-content properties carrier
//! (issue #269 Phase 1).

use oxidize_pdf::parser::content::{
    ContentOperation, ContentParser, MarkedContentProps, MarkedContentValue,
};

/// `BDC <</ActualText <FEFF00660069>>>` MUST preserve the raw 6 bytes
/// `FE FF 00 66 00 69` (UTF-16BE BOM + "fi"). The old `HashMap<String,String>`
/// carrier ran the bytes through `String::from_utf8_lossy`, mangling the BOM
/// and producing `\u{FFFD}\u{FFFD}\0f\0i`.
#[test]
fn utf16be_actualtext_preserved_as_raw_bytes() {
    let stream = b"/Span <</ActualText <FEFF00660069>>> BDC EMC";
    let ops = ContentParser::parse_content(stream).expect("parse");

    let (tag, props) = match &ops[0] {
        ContentOperation::BeginMarkedContentWithProps(t, p) => (t, p),
        other => panic!("expected BeginMarkedContentWithProps, got {:?}", other),
    };
    assert_eq!(tag, "Span");

    let inline = match props {
        MarkedContentProps::Inline(map) => map,
        MarkedContentProps::ResourceRef(name) => {
            panic!("expected Inline, got ResourceRef({})", name)
        }
    };
    let actual = inline.get("ActualText").expect("/ActualText key present");
    let bytes = match actual {
        MarkedContentValue::String(b) => b,
        other => panic!("expected MarkedContentValue::String, got {:?}", other),
    };
    assert_eq!(
        bytes.as_slice(),
        &[0xFE, 0xFF, 0x00, 0x66, 0x00, 0x69],
        "UTF-16BE bytes must be preserved verbatim"
    );
}

/// `BDC /PropsName` (single name operand) produces `ResourceRef("PropsName")`,
/// not an `Inline` map with a `__resource_ref` magic key.
#[test]
fn resource_ref_props_parsed_as_resource_ref_variant() {
    let stream = b"/P /PropsName BDC EMC";
    let ops = ContentParser::parse_content(stream).expect("parse");

    let props = match &ops[0] {
        ContentOperation::BeginMarkedContentWithProps(_, p) => p,
        other => panic!("expected BeginMarkedContentWithProps, got {:?}", other),
    };
    match props {
        MarkedContentProps::ResourceRef(name) => assert_eq!(name, "PropsName"),
        MarkedContentProps::Inline(_) => panic!("expected ResourceRef, got Inline"),
    }
}

/// `BDC <</MCID 0>>` produces `MarkedContentValue::Integer(0)` for the
/// `MCID` key — never `String` or `Name`. MCID is the *only* required
/// integer-typed key for tagged PDFs.
#[test]
fn mcid_integer_value_preserved_as_integer_variant() {
    let stream = b"/P <</MCID 42>> BDC EMC";
    let ops = ContentParser::parse_content(stream).expect("parse");

    let inline = match &ops[0] {
        ContentOperation::BeginMarkedContentWithProps(_, MarkedContentProps::Inline(m)) => m,
        other => panic!("expected Inline props, got {:?}", other),
    };
    let mcid = inline.get("MCID").expect("/MCID key present");
    match mcid {
        MarkedContentValue::Integer(n) => assert_eq!(*n, 42),
        other => panic!("expected Integer(42), got {:?}", other),
    }
}
