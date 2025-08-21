//! Comprehensive integration tests for the annotations module

use oxidize_pdf::annotations::*;
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::{Object, ObjectReference};
use oxidize_pdf::text::Font;

#[test]
fn test_complete_annotation_workflow() {
    let mut manager = AnnotationManager::new();
    let page_ref = ObjectReference::new(1, 0);

    // Create various annotation types
    let text_annot = TextAnnotation::new(Point::new(100.0, 700.0))
        .with_contents("Important note")
        .with_icon(Icon::Comment)
        .open()
        .with_state("Review", "Accepted")
        .to_annotation();

    let link_rect = Rectangle::new(Point::new(200.0, 600.0), Point::new(300.0, 620.0));
    let link_annot = LinkAnnotation::to_uri(link_rect, "https://example.com")
        .with_highlight_mode(HighlightMode::Push)
        .to_annotation();

    let markup_rect = Rectangle::new(Point::new(100.0, 500.0), Point::new(400.0, 515.0));
    let markup_annot = MarkupAnnotation::highlight(markup_rect)
        .with_author("John Doe")
        .with_subject("Key point")
        .with_contents("This is the main argument")
        .to_annotation();

    // Add all annotations to the manager
    let text_ref = manager.add_annotation(page_ref, text_annot);
    let link_ref = manager.add_annotation(page_ref, link_annot);
    let markup_ref = manager.add_annotation(page_ref, markup_annot);

    // Verify references are sequential
    assert_eq!(text_ref.number(), 1);
    assert_eq!(link_ref.number(), 2);
    assert_eq!(markup_ref.number(), 3);

    // Get annotations for the page
    let page_annotations = manager.get_page_annotations(&page_ref).unwrap();
    assert_eq!(page_annotations.len(), 3);

    // Verify annotation types
    assert_eq!(page_annotations[0].annotation_type, AnnotationType::Text);
    assert_eq!(page_annotations[1].annotation_type, AnnotationType::Link);
    assert_eq!(
        page_annotations[2].annotation_type,
        AnnotationType::Highlight
    );
}

#[test]
fn test_annotation_manager_multiple_pages() {
    let mut manager = AnnotationManager::new();
    let mut total_annotations = 0;

    // Create annotations for 5 different pages
    for page_num in 1..=5 {
        let page_ref = ObjectReference::new(page_num, 0);

        // Add different number of annotations to each page
        for annot_num in 0..page_num {
            let y_pos = 700.0 - (annot_num as f64 * 50.0);
            let rect = Rectangle::new(Point::new(100.0, y_pos), Point::new(200.0, y_pos + 20.0));

            let annotation = match annot_num % 3 {
                0 => TextAnnotation::new(Point::new(100.0, y_pos))
                    .with_contents(format!("Note {annot_num} on page {page_num}"))
                    .to_annotation(),
                1 => LinkAnnotation::to_page(rect, ObjectReference::new(1, 0)).to_annotation(),
                _ => MarkupAnnotation::underline(rect)
                    .with_contents(format!("Underline {annot_num} on page {page_num}"))
                    .to_annotation(),
            };

            manager.add_annotation(page_ref, annotation);
            total_annotations += 1;
        }
    }

    // Verify annotations per page
    for page_num in 1..=5 {
        let page_ref = ObjectReference::new(page_num, 0);
        let annotations = manager.get_page_annotations(&page_ref).unwrap();
        assert_eq!(annotations.len(), page_num as usize);
    }

    // Verify total annotations
    let all_annotations = manager.all_annotations();
    let total_count: usize = all_annotations.values().map(|v| v.len()).sum();
    assert_eq!(total_count, total_annotations as usize);
}

#[test]
fn test_all_annotation_types_creation() {
    let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 100.0));

    // Test each annotation type
    let types_and_annotations = vec![
        (
            AnnotationType::Text,
            TextAnnotation::new(Point::new(50.0, 50.0)).to_annotation(),
        ),
        (
            AnnotationType::Link,
            LinkAnnotation::to_uri(rect, "https://test.com").to_annotation(),
        ),
        (
            AnnotationType::FreeText,
            FreeTextAnnotation::new(rect, "Free text").to_annotation(),
        ),
        (
            AnnotationType::Line,
            LineAnnotation::new(Point::new(50.0, 50.0), Point::new(150.0, 100.0)).to_annotation(),
        ),
        (
            AnnotationType::Square,
            SquareAnnotation::new(rect).to_annotation(),
        ),
        (
            AnnotationType::Highlight,
            HighlightAnnotation::new(rect).to_annotation(),
        ),
        (
            AnnotationType::Stamp,
            StampAnnotation::new(rect, StampName::Approved).to_annotation(),
        ),
        (
            AnnotationType::Ink,
            InkAnnotation::new()
                .add_stroke(vec![
                    Point::new(50.0, 50.0),
                    Point::new(100.0, 75.0),
                    Point::new(150.0, 100.0),
                ])
                .to_annotation(),
        ),
    ];

    for (expected_type, annotation) in types_and_annotations {
        assert_eq!(annotation.annotation_type, expected_type);

        // Verify PDF dictionary has required fields
        let dict = annotation.to_dict();
        assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
        assert!(dict.get("Subtype").is_some());
        assert!(dict.get("Rect").is_some());
    }
}

#[test]
fn test_complex_free_text_annotation() {
    let rect = Rectangle::new(Point::new(100.0, 200.0), Point::new(400.0, 300.0));

    let free_text = FreeTextAnnotation::new(rect, "Multi-line\ntext content\nwith formatting")
        .with_font(Font::Courier, 10.0, Color::Rgb(0.2, 0.3, 0.8))
        .with_justification(2) // right justified
        .to_annotation();

    let dict = free_text.to_dict();

    // Verify all properties are set
    assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
    assert_eq!(
        dict.get("Subtype"),
        Some(&Object::Name("FreeText".to_string()))
    );
    assert!(dict.get("DA").is_some()); // Default appearance
    assert_eq!(dict.get("Q"), Some(&Object::Integer(2))); // Quadding

    // Verify contents
    assert_eq!(
        dict.get("Contents"),
        Some(&Object::String(
            "Multi-line\ntext content\nwith formatting".to_string()
        ))
    );
}

#[test]
fn test_complex_line_annotation() {
    let start = Point::new(50.0, 100.0);
    let end = Point::new(250.0, 300.0);

    let line = LineAnnotation::new(start, end)
        .with_endings(LineEndingStyle::ClosedArrow, LineEndingStyle::Diamond)
        .with_interior_color(Color::Rgb(1.0, 0.5, 0.0))
        .to_annotation();

    let dict = line.to_dict();

    // Check line coordinates
    if let Some(Object::Array(coords)) = dict.get("L") {
        assert_eq!(coords.len(), 4);
        assert_eq!(coords[0], Object::Real(50.0));
        assert_eq!(coords[1], Object::Real(100.0));
        assert_eq!(coords[2], Object::Real(250.0));
        assert_eq!(coords[3], Object::Real(300.0));
    } else {
        panic!("Line coordinates not found");
    }

    // Check line endings
    if let Some(Object::Array(endings)) = dict.get("LE") {
        assert_eq!(endings.len(), 2);
        assert_eq!(endings[0], Object::Name("ClosedArrow".to_string()));
        assert_eq!(endings[1], Object::Name("Diamond".to_string()));
    } else {
        panic!("Line endings not found");
    }

    // Check interior color
    assert!(dict.get("IC").is_some());
}

#[test]
fn test_square_with_cloudy_border() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 200.0));

    let square = SquareAnnotation::new(rect)
        .with_interior_color(Color::Rgb(0.9, 0.9, 1.0))
        .with_cloudy_border(1.5)
        .to_annotation();

    let dict = square.to_dict();

    // Check border effect
    if let Some(Object::Dictionary(be_dict)) = dict.get("BE") {
        assert_eq!(be_dict.get("S"), Some(&Object::Name("C".to_string())));
        assert_eq!(be_dict.get("I"), Some(&Object::Real(1.5)));
    } else {
        panic!("Border effect not found");
    }

    // Check interior color
    if let Some(Object::Array(color)) = dict.get("IC") {
        assert_eq!(color.len(), 3);
        assert_eq!(color[0], Object::Real(0.9));
        assert_eq!(color[1], Object::Real(0.9));
        assert_eq!(color[2], Object::Real(1.0));
    }
}

#[test]
fn test_ink_annotation_with_multiple_strokes() {
    let ink = InkAnnotation::new()
        .add_stroke(vec![
            Point::new(100.0, 100.0),
            Point::new(120.0, 110.0),
            Point::new(140.0, 105.0),
        ])
        .add_stroke(vec![
            Point::new(100.0, 120.0),
            Point::new(120.0, 130.0),
            Point::new(140.0, 125.0),
            Point::new(160.0, 130.0),
        ])
        .add_stroke(vec![Point::new(100.0, 140.0), Point::new(150.0, 140.0)])
        .to_annotation();

    let dict = ink.to_dict();

    // Verify InkList structure
    if let Some(Object::Array(ink_list)) = dict.get("InkList") {
        assert_eq!(ink_list.len(), 3); // 3 strokes

        // Check first stroke (3 points = 6 coordinates)
        if let Object::Array(stroke1) = &ink_list[0] {
            assert_eq!(stroke1.len(), 6);
        }

        // Check second stroke (4 points = 8 coordinates)
        if let Object::Array(stroke2) = &ink_list[1] {
            assert_eq!(stroke2.len(), 8);
        }

        // Check third stroke (2 points = 4 coordinates)
        if let Object::Array(stroke3) = &ink_list[2] {
            assert_eq!(stroke3.len(), 4);
        }
    } else {
        panic!("InkList not found");
    }

    // Verify bounding box was calculated correctly
    if let Some(Object::Array(rect)) = dict.get("Rect") {
        assert_eq!(rect[0], Object::Real(100.0)); // min x
        assert_eq!(rect[1], Object::Real(100.0)); // min y
        assert_eq!(rect[2], Object::Real(160.0)); // max x
        assert_eq!(rect[3], Object::Real(140.0)); // max y
    }
}

#[test]
fn test_markup_with_multiple_quad_points() {
    // Simulate highlighting multiple lines of text
    let rects = vec![
        Rectangle::new(Point::new(100.0, 700.0), Point::new(500.0, 715.0)),
        Rectangle::new(Point::new(100.0, 680.0), Point::new(480.0, 695.0)),
        Rectangle::new(Point::new(100.0, 660.0), Point::new(450.0, 675.0)),
    ];

    let bounding_rect = Rectangle::new(Point::new(100.0, 660.0), Point::new(500.0, 715.0));
    let quad_points = QuadPoints::from_rects(&rects);

    let markup = MarkupAnnotation::new(MarkupType::Highlight, bounding_rect, quad_points)
        .with_author("Reviewer")
        .with_subject("Multi-line highlight")
        .with_contents("Important section across multiple lines")
        .to_annotation();

    let dict = markup.to_dict();

    // Check QuadPoints array
    if let Some(Object::Array(points)) = dict.get("QuadPoints") {
        // 3 rectangles * 8 coordinates each = 24 total
        assert_eq!(points.len(), 24);

        // Verify first rectangle coordinates
        assert_eq!(points[0], Object::Real(100.0)); // x1
        assert_eq!(points[1], Object::Real(700.0)); // y1
        assert_eq!(points[2], Object::Real(500.0)); // x2
        assert_eq!(points[3], Object::Real(700.0)); // y2
    } else {
        panic!("QuadPoints not found");
    }

    // Check metadata
    assert_eq!(dict.get("T"), Some(&Object::String("Reviewer".to_string())));
    assert_eq!(
        dict.get("Subj"),
        Some(&Object::String("Multi-line highlight".to_string()))
    );
}

#[test]
fn test_all_stamp_types() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 150.0));

    let standard_stamps = vec![
        StampName::Approved,
        StampName::Experimental,
        StampName::NotApproved,
        StampName::AsIs,
        StampName::Expired,
        StampName::NotForPublicRelease,
        StampName::Confidential,
        StampName::Final,
        StampName::Sold,
        StampName::Departmental,
        StampName::ForComment,
        StampName::TopSecret,
        StampName::Draft,
        StampName::ForPublicRelease,
        StampName::Custom("CustomStamp".to_string()),
    ];

    for stamp_name in standard_stamps {
        let expected_name = stamp_name.pdf_name();
        let stamp = StampAnnotation::new(rect, stamp_name).to_annotation();
        let dict = stamp.to_dict();

        assert_eq!(
            dict.get("Subtype"),
            Some(&Object::Name("Stamp".to_string()))
        );
        assert_eq!(dict.get("Name"), Some(&Object::Name(expected_name)));
    }
}

#[test]
fn test_annotation_flags_combinations() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));

    // Test various flag combinations
    let flag_combinations = vec![
        (
            AnnotationFlags {
                invisible: true,
                hidden: true,
                ..Default::default()
            },
            3u32, // bits 0 and 1
        ),
        (
            AnnotationFlags {
                print: true,
                no_zoom: true,
                no_rotate: true,
                ..Default::default()
            },
            28u32, // bits 2, 3, and 4
        ),
        (
            AnnotationFlags {
                read_only: true,
                locked: true,
                locked_contents: true,
                ..Default::default()
            },
            704u32, // bits 6, 7, and 9
        ),
    ];

    for (flags, expected_value) in flag_combinations {
        let annotation = Annotation::new(AnnotationType::Text, rect).with_flags(flags);
        let dict = annotation.to_dict();

        assert_eq!(dict.get("F"), Some(&Object::Integer(expected_value as i64)));
    }
}

#[test]
fn test_link_destinations_comprehensive() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));

    // Test all destination types
    let destinations = vec![
        LinkDestination::XYZ {
            page: ObjectReference::new(5, 0),
            left: Some(100.0),
            top: Some(700.0),
            zoom: Some(2.0),
        },
        LinkDestination::Fit {
            page: ObjectReference::new(3, 0),
        },
        LinkDestination::FitH {
            page: ObjectReference::new(2, 0),
            top: Some(500.0),
        },
        LinkDestination::FitV {
            page: ObjectReference::new(4, 0),
            left: Some(150.0),
        },
        LinkDestination::FitR {
            page: ObjectReference::new(1, 0),
            rect: Rectangle::new(Point::new(50.0, 50.0), Point::new(550.0, 750.0)),
        },
        LinkDestination::Named("TableOfContents".to_string()),
    ];

    for dest in destinations {
        let link = LinkAnnotation::new(rect, LinkAction::GoTo(dest.clone()));
        let annotation = link.to_annotation();
        let dict = annotation.to_dict();

        // Verify action dictionary exists
        assert!(dict.get("A").is_some());

        if let Some(Object::Dictionary(action_dict)) = dict.get("A") {
            assert_eq!(
                action_dict.get("S"),
                Some(&Object::Name("GoTo".to_string()))
            );

            // Verify destination format
            match &dest {
                LinkDestination::Named(name) => {
                    assert_eq!(action_dict.get("D"), Some(&Object::String(name.clone())));
                }
                _ => {
                    assert!(matches!(action_dict.get("D"), Some(Object::Array(_))));
                }
            }
        }
    }
}

#[test]
fn test_annotation_color_representations() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(50.0, 50.0));

    let color_tests = vec![
        (Color::Gray(0.5), 1),
        (Color::Rgb(1.0, 0.5, 0.0), 3),
        (Color::Cmyk(0.1, 0.2, 0.3, 0.4), 4),
    ];

    for (color, expected_components) in color_tests {
        let annotation = Annotation::new(AnnotationType::Square, rect).with_color(color);
        let dict = annotation.to_dict();

        if let Some(Object::Array(color_array)) = dict.get("C") {
            assert_eq!(color_array.len(), expected_components);

            // Verify all components are Real objects
            for component in color_array {
                assert!(matches!(component, Object::Real(_)));
            }
        } else {
            panic!("Color array not found");
        }
    }
}

#[test]
fn test_annotation_modified_date_format() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 150.0));

    let mut annotation = Annotation::new(AnnotationType::Text, rect).with_contents("Test note");

    // Test various date formats
    let date_formats = vec![
        "D:20231225120000Z",
        "D:20231225120000+05'00'",
        "D:20231225120000-08'00'",
        "D:20231225",
    ];

    for date in date_formats {
        annotation.modified = Some(date.to_string());
        let dict = annotation.to_dict();

        assert_eq!(dict.get("M"), Some(&Object::String(date.to_string())));
    }
}

#[test]
fn test_annotation_border_styles_comprehensive() {
    let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 100.0));

    let border_tests = vec![
        (
            BorderStyle {
                width: 3.0,
                style: BorderStyleType::Solid,
                dash_pattern: None,
            },
            "S",
            false,
        ),
        (
            BorderStyle {
                width: 2.0,
                style: BorderStyleType::Dashed,
                dash_pattern: Some(vec![5.0, 3.0]),
            },
            "D",
            true,
        ),
        (
            BorderStyle {
                width: 1.5,
                style: BorderStyleType::Beveled,
                dash_pattern: None,
            },
            "B",
            false,
        ),
    ];

    for (border, expected_style, has_dash) in border_tests {
        let annotation = Annotation::new(AnnotationType::Square, rect).with_border(border);
        let dict = annotation.to_dict();

        if let Some(Object::Dictionary(bs_dict)) = dict.get("BS") {
            assert_eq!(
                bs_dict.get("S"),
                Some(&Object::Name(expected_style.to_string()))
            );

            if has_dash {
                assert!(bs_dict.get("D").is_some());
            } else {
                assert!(bs_dict.get("D").is_none());
            }
        } else {
            panic!("Border style dictionary not found");
        }
    }
}

#[test]
fn test_annotation_properties_merging() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 150.0));

    let mut annotation =
        Annotation::new(AnnotationType::Widget, rect).with_contents("Widget annotation");

    // Add custom properties
    annotation
        .properties
        .set("FieldType", Object::Name("Tx".to_string()));
    annotation
        .properties
        .set("FieldName", Object::String("username".to_string()));
    annotation
        .properties
        .set("DefaultValue", Object::String("".to_string()));
    annotation.properties.set("MaxLen", Object::Integer(50));

    let dict = annotation.to_dict();

    // Verify standard fields are present
    assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
    assert_eq!(
        dict.get("Subtype"),
        Some(&Object::Name("Widget".to_string()))
    );

    // Verify custom properties are merged
    assert_eq!(dict.get("FieldType"), Some(&Object::Name("Tx".to_string())));
    assert_eq!(
        dict.get("FieldName"),
        Some(&Object::String("username".to_string()))
    );
    assert_eq!(
        dict.get("DefaultValue"),
        Some(&Object::String("".to_string()))
    );
    assert_eq!(dict.get("MaxLen"), Some(&Object::Integer(50)));
}

// Error handling and edge case tests

#[test]
fn test_annotation_with_extreme_coordinates() {
    let extreme_coords = vec![
        (f64::MIN, f64::MIN, f64::MAX, f64::MAX),
        (-1e308, -1e308, 1e308, 1e308),
        (0.0, 0.0, 0.0, 0.0), // Zero-size rectangle
    ];

    for (x1, y1, x2, y2) in extreme_coords {
        let rect = Rectangle::new(Point::new(x1, y1), Point::new(x2, y2));
        let annotation = Annotation::new(AnnotationType::Text, rect);
        let dict = annotation.to_dict();

        // Should still produce valid dictionary
        assert!(dict.contains_key("Type"));
        assert!(dict.contains_key("Subtype"));
        assert!(dict.contains_key("Rect"));

        if let Some(Object::Array(rect_array)) = dict.get("Rect") {
            assert_eq!(rect_array.len(), 4);
        }
    }
}

#[test]
fn test_link_with_malformed_uris() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));

    let malformed_uris = vec![
        "", // Empty URI
        "not a valid uri",
        "javascript:alert('xss')",
        "file:///etc/passwd",
        "data:text/html,<script>alert('xss')</script>",
        "\n\r\t",                    // Whitespace only
        "http://example.com/\0null", // Null character
    ];

    for uri in malformed_uris {
        let link = LinkAnnotation::to_uri(rect, uri);
        let annotation = link.to_annotation();
        let dict = annotation.to_dict();

        // Should still create valid annotation
        assert_eq!(dict.get("Subtype"), Some(&Object::Name("Link".to_string())));

        if let Some(Object::Dictionary(action_dict)) = dict.get("A") {
            assert_eq!(action_dict.get("S"), Some(&Object::Name("URI".to_string())));
            assert_eq!(
                action_dict.get("URI"),
                Some(&Object::String(uri.to_string()))
            );
        }
    }
}

#[test]
fn test_annotation_manager_with_invalid_references() {
    let mut manager = AnnotationManager::new();

    // Test with very large reference numbers
    let huge_ref = ObjectReference::new(u32::MAX, u16::MAX);
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    let annotation = Annotation::new(AnnotationType::Text, rect);

    let annot_ref = manager.add_annotation(huge_ref, annotation);
    assert_eq!(annot_ref.number(), 1); // Manager assigns its own IDs

    // Verify retrieval works
    let annotations = manager.get_page_annotations(&huge_ref);
    assert!(annotations.is_some());
    assert_eq!(annotations.unwrap().len(), 1);

    // Test with zero reference
    let zero_ref = ObjectReference::new(0, 0);
    let annotation2 = Annotation::new(AnnotationType::Link, rect);
    manager.add_annotation(zero_ref, annotation2);

    assert!(manager.get_page_annotations(&zero_ref).is_some());
}

#[test]
fn test_markup_with_empty_quad_points() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));
    let empty_quad = QuadPoints { points: vec![] };

    let markup = MarkupAnnotation::new(MarkupType::Highlight, rect, empty_quad);
    let annotation = markup.to_annotation();
    let dict = annotation.to_dict();

    // Should still create valid annotation
    assert_eq!(
        dict.get("Subtype"),
        Some(&Object::Name("Highlight".to_string()))
    );

    if let Some(Object::Array(points)) = dict.get("QuadPoints") {
        assert_eq!(points.len(), 0);
    }
}

#[test]
fn test_free_text_with_invalid_quadding() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 200.0));

    // Test negative quadding (should clamp to 0)
    let negative_quad = FreeTextAnnotation::new(rect, "Test").with_justification(-100);
    assert_eq!(negative_quad.quadding, 0);

    // Test large quadding (should clamp to 2)
    let large_quad = FreeTextAnnotation::new(rect, "Test").with_justification(9999);
    assert_eq!(large_quad.quadding, 2);
}

#[test]
fn test_ink_annotation_with_invalid_points() {
    let mut ink = InkAnnotation::new();

    // Add stroke with NaN coordinates
    ink = ink.add_stroke(vec![
        Point::new(f64::NAN, f64::NAN),
        Point::new(100.0, 100.0),
        Point::new(f64::INFINITY, f64::NEG_INFINITY),
    ]);

    let annotation = ink.to_annotation();
    let dict = annotation.to_dict();

    // Should still create annotation
    assert_eq!(dict.get("Subtype"), Some(&Object::Name("Ink".to_string())));

    if let Some(Object::Array(ink_list)) = dict.get("InkList") {
        assert_eq!(ink_list.len(), 1);
        if let Object::Array(stroke) = &ink_list[0] {
            assert_eq!(stroke.len(), 6);
            // NaN and Infinity values should be preserved
            assert!(matches!(stroke[0], Object::Real(v) if v.is_nan()));
            assert!(matches!(stroke[4], Object::Real(v) if v.is_infinite()));
        }
    }
}

#[test]
fn test_annotations_with_very_long_strings() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));

    // Create very long string (1MB)
    let long_string = "a".repeat(1_000_000);

    // Test with text annotation
    let text_annot = TextAnnotation::new(Point::new(0.0, 0.0))
        .with_contents(&long_string)
        .to_annotation();

    assert_eq!(text_annot.contents, Some(long_string.clone()));

    // Test with free text annotation
    let free_text = FreeTextAnnotation::new(rect, &long_string).to_annotation();

    assert_eq!(free_text.contents, Some(long_string));
}

#[test]
fn test_border_style_with_invalid_dash_patterns() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));

    let invalid_patterns = vec![
        vec![],              // Empty pattern
        vec![0.0],           // Zero dash
        vec![-1.0, 2.0],     // Negative dash
        vec![f64::NAN, 2.0], // NaN in pattern
        vec![f64::INFINITY], // Infinity in pattern
    ];

    for pattern in invalid_patterns {
        let border = BorderStyle {
            width: 1.0,
            style: BorderStyleType::Dashed,
            dash_pattern: Some(pattern.clone()),
        };

        let annotation = Annotation::new(AnnotationType::Square, rect).with_border(border);

        let dict = annotation.to_dict();

        if let Some(Object::Dictionary(bs_dict)) = dict.get("BS") {
            if let Some(Object::Array(dash_array)) = bs_dict.get("D") {
                assert_eq!(dash_array.len(), pattern.len());
            }
        }
    }
}

#[test]
fn test_color_edge_cases() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));

    let edge_colors = vec![
        Color::Gray(-0.5),                 // Negative gray
        Color::Gray(1.5),                  // Out of range gray
        Color::Rgb(-1.0, 2.0, 0.5),        // Out of range RGB
        Color::Cmyk(1.5, -0.5, 2.0, -1.0), // Out of range CMYK
    ];

    for color in edge_colors {
        let annotation = Annotation::new(AnnotationType::Square, rect).with_color(color);

        let dict = annotation.to_dict();

        // Color should still be included even if out of range
        assert!(dict.contains_key("C"));

        if let Some(Object::Array(color_array)) = dict.get("C") {
            match color {
                Color::Gray(_) => assert_eq!(color_array.len(), 1),
                Color::Rgb(_, _, _) => assert_eq!(color_array.len(), 3),
                Color::Cmyk(_, _, _, _) => assert_eq!(color_array.len(), 4),
            }
        }
    }
}

#[test]
fn test_annotation_flags_edge_cases() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));

    // Test with all flags set to opposite of defaults
    let unusual_flags = AnnotationFlags {
        invisible: true,
        hidden: true,
        print: false, // Usually true by default
        no_zoom: true,
        no_rotate: true,
        no_view: true,
        read_only: true,
        locked: true,
        locked_contents: true,
    };

    let annotation = Annotation::new(AnnotationType::Text, rect).with_flags(unusual_flags);

    let dict = annotation.to_dict();

    if let Some(Object::Integer(flags_value)) = dict.get("F") {
        // All bits except print (bit 2) should be set
        let expected = 1 + 2 + 8 + 16 + 32 + 64 + 128 + 512; // bits 0,1,3,4,5,6,7,9
        assert_eq!(*flags_value, expected as i64);
    }
}

#[test]
fn test_link_destination_with_null_page_reference() {
    // Create destination with null object reference (0,0)
    let null_ref = ObjectReference::new(0, 0);
    let dest = LinkDestination::Fit { page: null_ref };

    if let Object::Array(arr) = dest.to_array() {
        assert_eq!(arr[0], Object::Reference(null_ref));
        assert_eq!(arr[1], Object::Name("Fit".to_string()));
    }
}

#[test]
fn test_concurrent_annotation_manager_operations() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let manager = Arc::new(Mutex::new(AnnotationManager::new()));
    let mut handles = vec![];

    // Simulate concurrent additions from multiple threads
    for thread_id in 0..10 {
        let manager_clone = Arc::clone(&manager);
        let handle = thread::spawn(move || {
            let page_ref = ObjectReference::new(thread_id, 0);
            let rect = Rectangle::new(
                Point::new(thread_id as f64 * 100.0, 0.0),
                Point::new((thread_id + 1) as f64 * 100.0, 100.0),
            );

            for i in 0..10 {
                let annotation = Annotation::new(AnnotationType::Text, rect)
                    .with_contents(format!("Thread {thread_id} annotation {i}"));

                let mut manager = manager_clone.lock().unwrap();
                manager.add_annotation(page_ref, annotation);
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all annotations were added
    let manager = manager.lock().unwrap();
    let all_annotations = manager.all_annotations();

    assert_eq!(all_annotations.len(), 10); // 10 pages
    for (_, annotations) in all_annotations.iter() {
        assert_eq!(annotations.len(), 10); // 10 annotations per page
    }
}
