use oxidize_pdf::actions::UriAction;
use oxidize_pdf::annotations::{PolygonAnnotation, PolylineAnnotation};
use oxidize_pdf::geometry::Point;
use oxidize_pdf::objects::{Dictionary, Object};
use oxidize_pdf::page_labels::{PageLabel, PageLabelTree};
use oxidize_pdf::text::fonts::truetype::TrueTypeFont;

fn label_tree(index: i64, start: i64) -> Dictionary {
    let mut label = Dictionary::new();
    label.set("S", Object::Name("A".into()));
    label.set("St", Object::Integer(start));
    let mut tree = Dictionary::new();
    tree.set(
        "Nums",
        Object::Array(vec![Object::Integer(index), Object::Dictionary(label)]),
    );
    tree
}

#[test]
fn invalid_label_numbers_are_rejected_instead_of_wrapping() {
    for start in [-1, 0, i64::MAX] {
        assert!(
            PageLabelTree::from_dict(&label_tree(0, start)).is_none(),
            "St={start}"
        );
    }
    for index in [-1, i64::MAX] {
        assert!(
            PageLabelTree::from_dict(&label_tree(index, 1)).is_none(),
            "index={index}"
        );
    }
    assert_eq!(
        PageLabelTree::from_dict(&label_tree(0, 28))
            .unwrap()
            .get_label(0)
            .as_deref(),
        Some("BB")
    );
}

#[test]
fn unicode_prefix_is_pdf_utf16_and_roundtrips() {
    let prefix = "Café 第 😀";
    let label = PageLabel::letters_uppercase()
        .with_prefix(prefix)
        .starting_at(28);
    let mut expected = vec![0xfe, 0xff];
    expected.extend(prefix.encode_utf16().flat_map(u16::to_be_bytes));
    assert_eq!(
        label.to_dict().get("P"),
        Some(&Object::ByteString(expected))
    );
    let mut tree = PageLabelTree::new();
    tree.add_range(0, label);
    assert_eq!(
        PageLabelTree::from_dict(&tree.to_dict())
            .unwrap()
            .get_label(0)
            .as_deref(),
        Some("Café 第 😀BB")
    );
}

#[test]
fn recipient_cannot_inject_mailto_headers_or_fragment() {
    let address = "a+tag@example.org?bcc=evil@example.org#x%20&";
    let encoded = "a+tag@example.org%3Fbcc%3Devil@example.org%23x%2520%26";
    assert_eq!(UriAction::email(address).uri, format!("mailto:{encoded}"));
    assert_eq!(
        UriAction::email_with_subject(address, "Café").uri,
        format!("mailto:{encoded}?subject=Caf%C3%A9")
    );
    assert_eq!(
        UriAction::email("用户@example.org").uri,
        "mailto:%E7%94%A8%E6%88%B7@example.org"
    );
}

fn vertices() -> Vec<Point> {
    vec![
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        Point::new(10.0, 10.0),
    ]
}

#[test]
fn annotation_line_width_controls_pdf_border() {
    for annotation in [
        PolygonAnnotation::new(vertices())
            .with_line_width(5.0)
            .to_annotation()
            .unwrap(),
        PolylineAnnotation::new(vertices())
            .with_line_width(5.0)
            .to_annotation()
            .unwrap(),
    ] {
        let Some(Object::Dictionary(bs)) = annotation.properties.get("BS") else {
            panic!("missing BS")
        };
        assert_eq!(bs.get("W"), Some(&Object::Real(5.0)));
    }
}

#[test]
fn malformed_annotation_geometry_and_style_are_rejected() {
    assert!(PolygonAnnotation::new(vertices()).to_annotation().is_ok());
    assert!(PolylineAnnotation::new(vertices()[..2].to_vec())
        .to_annotation()
        .is_ok());
    assert!(PolygonAnnotation::new(vertices()[..2].to_vec())
        .to_annotation()
        .is_err());
    assert!(PolylineAnnotation::new(vec![]).to_annotation().is_err());
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(PolygonAnnotation::new(vertices())
            .with_opacity(value)
            .to_annotation()
            .is_err());
        assert!(PolylineAnnotation::new(vertices())
            .with_opacity(value)
            .to_annotation()
            .is_err());
        let mut points = vertices();
        points[0].x = value;
        assert!(PolygonAnnotation::new(points.clone())
            .to_annotation()
            .is_err());
        assert!(PolylineAnnotation::new(points).to_annotation().is_err());
    }
    for value in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(PolygonAnnotation::new(vertices())
            .with_line_width(value)
            .to_annotation()
            .is_err());
        assert!(PolylineAnnotation::new(vertices())
            .with_line_width(value)
            .to_annotation()
            .is_err());
    }
    for value in [-1.0, 2.0, f64::NAN, f64::INFINITY] {
        let mut p = PolygonAnnotation::new(vertices());
        p.opacity = value;
        assert!(p.to_annotation().is_err());
        let mut p = PolylineAnnotation::new(vertices());
        p.opacity = value;
        assert!(p.to_annotation().is_err());
    }
}

fn font_with_count(count: u16) -> Vec<u8> {
    let mut data = include_bytes!("fixtures/text_contracts/symbolic/mac-subset.ttf").to_vec();
    let tables = u16::from_be_bytes(data[4..6].try_into().unwrap()) as usize;
    for i in 0..tables {
        let pos = 12 + i * 16;
        if &data[pos..pos + 4] == b"maxp" {
            let offset = u32::from_be_bytes(data[pos + 8..pos + 12].try_into().unwrap()) as usize;
            data[offset + 4..offset + 6].copy_from_slice(&count.to_be_bytes());
            return data;
        }
    }
    panic!("missing maxp")
}

#[test]
fn parallel_font_parser_respects_glyph_inventory() {
    for count in [1, 3, 4] {
        let font = oxidize_pdf::fonts::Font::from_bytes("test", font_with_count(count)).unwrap();
        assert_eq!(
            font.glyph_mapping.char_to_glyph('A'),
            if count == 4 { Some(3) } else { None },
            "count={count}"
        );
        if count < 4 {
            assert_eq!(font.glyph_mapping.glyph_to_char(3), None);
        }
    }
}

#[test]
fn explicit_subset_rejects_out_of_inventory_glyphs() {
    let data = font_with_count(4);
    let font = TrueTypeFont::parse(data).unwrap();
    assert!(font.create_subset(&[0, 3].into()).is_ok());
    for gid in [4, u16::MAX] {
        assert!(font.create_subset(&[gid].into()).is_err(), "GID={gid}");
    }
}

#[test]
fn label_limits_are_checked_before_expansion_and_arithmetic() {
    use oxidize_pdf::page_labels::{PageLabelStyle, DEFAULT_LABEL_BYTE_LIMIT};
    let style = PageLabelStyle::UppercaseLetters;
    assert!(style.try_format(u32::MAX).is_err());
    assert!(PageLabelStyle::LowercaseRoman.try_format(u32::MAX).is_err());
    assert!(style.try_format(0).is_err());
    assert_eq!(style.try_format_with_limit(28, 2).unwrap(), "BB");
    assert!(style.try_format_with_limit(28, 1).is_err());
    let boundary = (DEFAULT_LABEL_BYTE_LIMIT * 26) as u32;
    assert_eq!(
        style.try_format(boundary).unwrap(),
        "Z".repeat(DEFAULT_LABEL_BYTE_LIMIT)
    );
    assert!(style.try_format(boundary + 1).is_err());
    assert_eq!(
        style
            .try_format_with_limit(boundary + 1, DEFAULT_LABEL_BYTE_LIMIT + 1)
            .unwrap(),
        "A".repeat(DEFAULT_LABEL_BYTE_LIMIT + 1)
    );
    let label = PageLabel::decimal().starting_at(u32::MAX);
    assert_eq!(label.try_format_label(0).unwrap(), "4294967295");
    assert!(label.try_format_label(1).is_err());
    assert!(PageLabel::decimal().starting_at(0).try_to_dict().is_err());
    let label = PageLabel::letters_uppercase()
        .with_prefix("é")
        .starting_at(28);
    assert_eq!(label.try_format_label_with_limit(0, 4).unwrap(), "éBB");
    assert!(label.try_format_label_with_limit(0, 3).is_err());
    assert_eq!(
        PageLabel::roman_uppercase()
            .starting_at(1994)
            .try_format_label_with_limit(0, 7)
            .unwrap(),
        "MCMXCIV"
    );
    assert!(PageLabel::roman_uppercase()
        .starting_at(1994)
        .try_format_label_with_limit(0, 6)
        .is_err());
    let mut tree = PageLabelTree::new();
    tree.add_range(0, PageLabel::letters_uppercase().starting_at(u32::MAX));
    assert!(tree.try_get_label(0).is_err());
    assert!(tree.try_get_all_labels(1, 1024).is_err());
    assert!(PageLabelTree::new()
        .try_get_all_labels(u32::MAX, 1024)
        .is_err());
    assert_eq!(
        PageLabelTree::new().try_get_all_labels(2, 1024).unwrap(),
        ["1", "2"]
    );
}

#[test]
fn border_setter_order_and_invalid_dashes_are_checked() {
    use oxidize_pdf::annotations::PolygonBorderStyle;
    let style = PolygonBorderStyle {
        width: 7.0,
        ..Default::default()
    };
    let p = PolygonAnnotation::new(vertices())
        .with_line_width(3.0)
        .with_border_style(style.clone());
    assert_eq!(p.line_width, 7.0);
    assert_eq!(p.calculate_rect().lower_left.x, -7.0);
    let p = PolygonAnnotation::new(vertices())
        .with_border_style(style.clone())
        .with_line_width(3.0);
    assert_eq!(p.border_style.width, 3.0);
    let p = PolylineAnnotation::new(vertices())
        .with_line_width(3.0)
        .with_border_style(style.clone());
    assert_eq!(p.line_width, 7.0);
    for dash in [
        vec![-1.0],
        vec![0.0, 0.0],
        vec![f64::NAN],
        vec![f64::INFINITY],
    ] {
        let style = PolygonBorderStyle {
            dash_pattern: Some(dash),
            ..Default::default()
        };
        assert!(PolygonAnnotation::new(vertices())
            .with_border_style(style.clone())
            .to_annotation()
            .is_err());
        assert!(PolylineAnnotation::new(vertices())
            .with_border_style(style)
            .to_annotation()
            .is_err());
    }
    for dash in [vec![], vec![0.0, 2.0], vec![3.0, 2.0]] {
        let style = PolygonBorderStyle {
            dash_pattern: Some(dash),
            ..Default::default()
        };
        assert!(PolygonAnnotation::new(vertices())
            .with_border_style(style)
            .to_annotation()
            .is_ok());
    }
}

#[test]
fn document_writer_returns_error_for_invalid_label_start() {
    let mut doc = oxidize_pdf::Document::new();
    doc.add_page(oxidize_pdf::Page::new(100.0, 100.0));
    let mut labels = PageLabelTree::new();
    labels.add_range(0, PageLabel::decimal().starting_at(0));
    doc.set_page_labels(labels);
    assert!(doc.to_bytes().is_err());
}

#[test]
fn document_label_getters_propagate_errors_and_bound_aggregate_output() {
    let mut doc = oxidize_pdf::Document::new();
    doc.add_page(oxidize_pdf::Page::new(100.0, 100.0));
    assert_eq!(doc.try_get_page_label(u32::MAX).unwrap(), "4294967296");
    assert_eq!(doc.try_get_all_page_labels(1024).unwrap(), ["1"]);
    assert!(doc.try_get_all_page_labels(0).is_err());
    let mut tree = PageLabelTree::new();
    tree.add_range(0, PageLabel::letters_uppercase().starting_at(u32::MAX));
    doc.set_page_labels(tree);
    assert!(doc.try_get_page_label(0).is_err());
    assert!(doc.try_get_all_page_labels(1024).is_err());
}

#[test]
fn missing_or_truncated_maxp_is_an_explicit_mapping_error() {
    for missing in [false, true] {
        let mut data = font_with_count(4);
        let tables = u16::from_be_bytes(data[4..6].try_into().unwrap()) as usize;
        let pos = (0..tables)
            .map(|i| 12 + i * 16)
            .find(|&p| &data[p..p + 4] == b"maxp")
            .unwrap();
        if missing {
            data[pos..pos + 4].copy_from_slice(b"xxxx");
        } else {
            data[pos + 12..pos + 16].copy_from_slice(&5u32.to_be_bytes());
        }
        let parser = oxidize_pdf::fonts::TtfParser::new(&data).unwrap();
        let err = parser.extract_glyph_mapping().unwrap_err();
        assert!(err.to_string().contains("maxp"), "{err}");
    }
}

#[test]
fn mailto_recipient_list_is_preserved_but_semicolons_are_data() {
    assert_eq!(
        UriAction::email("a@example.org,b@example.org;x").uri,
        "mailto:a@example.org,b@example.org%3Bx"
    );
}
