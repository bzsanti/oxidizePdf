use oxidize_pdf::metadata::xmp::{XmpMetadata, XmpNamespace, XmpValue};

fn check(input: &str, valid: bool) {
    let mut xmp = XmpMetadata::new();
    xmp.set_date(XmpNamespace::XmpBasic, "CreateDate", input);
    let value = &xmp.properties()[0].value;
    match (valid, value) {
        (true, XmpValue::Date(s)) | (false, XmpValue::Text(s)) => assert_eq!(s, input),
        _ => panic!("wrong classification for {input:?}: {value:?}"),
    }
}

#[test]
fn unicode_falls_back_without_panicking() {
    for s in [
        "202é",
        "2024-é1",
        "2024-01-é",
        "2024-01-01Té:00",
        "💥",
        "２０２４",
    ] {
        check(s, false);
    }
}

#[test]
fn invalid_calendar_dates_fall_back() {
    for s in [
        "2023-02-29",
        "1900-02-29",
        "2100-02-29",
        "2024-04-31",
        "2024-00-01",
        "2024-13",
        "2024-01-00",
        "2024-02-30",
        "2024-01-32",
    ] {
        check(s, false);
    }
}

#[test]
fn invalid_time_and_timezone_fall_back() {
    for suffix in [
        "x:",
        "24:00",
        "12:60",
        "12:00:60",
        "1:00",
        "12:0",
        "12:00:",
        "12:00:0",
        "12:00:00.",
        "12:00.5",
        "12:00Zjunk",
        "12:00+24:00",
        "12:00-01:60",
        "12:00+0100",
        "12:00+1:00",
        "12:00z",
        "12:00:00.1.2",
        "12:00:00.é",
        "12:00:00Z ",
    ] {
        check(&format!("2024-01-01T{suffix}"), false);
    }
}

#[test]
fn reduced_precision_and_leap_year_controls() {
    for s in [
        "1000",
        "9999",
        "2024",
        "2024-02",
        "2024-02-29",
        "2000-02-29",
        "2400-02-29",
        "2023-02-28",
        "2024-04-30",
        "2024-01-31",
    ] {
        check(s, true);
    }
}

#[test]
fn time_precision_and_timezone_controls() {
    for time in [
        "00:00",
        "23:59",
        "12:34:56",
        "23:59:59.0",
        "12:34:56.12345678901234567890",
    ] {
        for zone in ["", "Z", "+00:00", "-00:00", "+05:30", "-08:00", "+23:59"] {
            check(&format!("2024-02-29T{time}{zone}"), true);
        }
    }
}

#[test]
fn malformed_structure_and_truncations_fall_back() {
    for s in [
        "",
        "999",
        "0999",
        "10000",
        "2024/01/01",
        "2024-1",
        "2024-01-1",
        "2024-01-01 ",
        "2024-01-01Z",
        "2024-01-01T",
        "2024-01-01t12:00Z",
        "2024-+1-01",
        "2024-01-+1",
    ] {
        check(s, false);
    }
}

proptest::proptest! {
    #[test]
    fn arbitrary_unicode_preserves_input_without_panics(s in proptest::collection::vec(proptest::char::any(), 0..128)) {
        let input: String = s.into_iter().collect();
        let mut xmp = XmpMetadata::new();
        xmp.set_date(XmpNamespace::XmpBasic, "CreateDate", &input);
        match &xmp.properties()[0].value {
            XmpValue::Date(s) | XmpValue::Text(s) => proptest::prop_assert_eq!(s, &input),
            other => proptest::prop_assert!(false, "unexpected variant: {other:?}"),
        }
    }
}

#[test]
fn packet_import_validates_date_candidates() {
    for (input, valid) in [
        ("2023-02-29", false),
        ("2024-01-01Tx:", false),
        ("2024-02-29T12:00Z", true),
    ] {
        let xml = format!(
            r#"<?xpacket begin=""?><x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:xmp="http://ns.adobe.com/xap/1.0/"><xmp:CreateDate>{input}</xmp:CreateDate></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end="w"?>"#
        );
        let xmp = XmpMetadata::from_xmp_packet(&xml).unwrap();
        match (valid, &xmp.properties()[0].value) {
            (true, XmpValue::Date(s)) | (false, XmpValue::Text(s)) => assert_eq!(s, input),
            (_, other) => panic!("wrong imported classification for {input}: {other:?}"),
        }
    }
}

#[test]
fn invalid_date_text_is_escaped_on_serialization() {
    let mut xmp = XmpMetadata::new();
    xmp.set_date(XmpNamespace::XmpBasic, "CreateDate", "2024-01-01T<&:");
    assert!(matches!(&xmp.properties()[0].value, XmpValue::Text(s) if s == "2024-01-01T<&:"));
    assert!(xmp.to_xmp_packet().contains("2024-01-01T&lt;&amp;:"));
}
