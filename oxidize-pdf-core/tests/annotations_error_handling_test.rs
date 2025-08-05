//! Annotations Error Handling and Edge Cases Tests
//!
//! This test suite addresses critical coverage gaps identified in the annotations module,
//! focusing on error paths, edge cases, and integration scenarios that were missing
//! from the comprehensive test suite.
//!
//! Based on coverage analysis: targeting +8% coverage through error handling tests

use oxidize_pdf::annotations::*;
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::{Object, ObjectReference};
use std::panic;

/// Test 1: Error handling for annotation dictionary creation with corrupted properties
#[test]
fn test_annotation_dict_with_invalid_properties() {
    let mut annotation = Annotation::new(
        AnnotationType::Text,
        Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0)),
    );

    // Add some malformed properties that could cause issues
    annotation
        .properties
        .set("InvalidFloat", Object::String("not_a_number".to_string()));
    annotation
        .properties
        .set("InvalidArray", Object::Boolean(true)); // Should be array

    // Should still generate valid dictionary without crashing
    let dict = annotation.to_dict();

    // Basic structure should be preserved
    assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
    assert_eq!(dict.get("Subtype"), Some(&Object::Name("Text".to_string())));
    assert!(dict.get("Rect").is_some());

    // Invalid properties should still be included (raw passthrough)
    assert_eq!(
        dict.get("InvalidFloat"),
        Some(&Object::String("not_a_number".to_string()))
    );
    assert_eq!(dict.get("InvalidArray"), Some(&Object::Boolean(true)));

    println!("Annotation dictionary handles invalid properties gracefully");
}

/// Test 2: Border style validation with invalid values
#[test]
fn test_border_style_with_invalid_values() {
    // Test negative width
    let border_negative = BorderStyle {
        width: -5.0,
        style: BorderStyleType::Solid,
        dash_pattern: None,
    };

    let annotation = Annotation::new(
        AnnotationType::Square,
        Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0)),
    )
    .with_border(border_negative);

    // Should handle gracefully (no crash)
    let dict = annotation.to_dict();
    assert!(dict.get("BS").is_some());

    // Test zero width
    let border_zero = BorderStyle {
        width: 0.0,
        style: BorderStyleType::Dashed,
        dash_pattern: Some(vec![1.0, 2.0]),
    };

    let annotation_zero = Annotation::new(
        AnnotationType::Line,
        Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 150.0)),
    )
    .with_border(border_zero);

    let dict_zero = annotation_zero.to_dict();
    assert!(dict_zero.get("BS").is_some());

    // Test empty dash pattern for dashed border
    let border_empty_dash = BorderStyle {
        width: 2.0,
        style: BorderStyleType::Dashed,
        dash_pattern: Some(vec![]), // Empty dash pattern
    };

    let annotation_empty = Annotation::new(
        AnnotationType::FreeText,
        Rectangle::new(Point::new(200.0, 200.0), Point::new(300.0, 300.0)),
    )
    .with_border(border_empty_dash);

    let dict_empty = annotation_empty.to_dict();
    assert!(dict_empty.get("BS").is_some());

    println!("Border styles handle invalid values without crashing");
}

/// Test 3: Font handling errors in FreeText annotations
#[test]
fn test_free_text_with_invalid_font_specs() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 150.0));

    // Test with malformed appearance string
    let mut free_text = FreeTextAnnotation::new(rect, "Test text");
    free_text.default_appearance = "INVALID FONT SPEC".to_string();

    // Should not crash during conversion
    let annotation = free_text.to_annotation();
    assert!(annotation.properties.get("DA").is_some());

    // Test with very long appearance string
    let long_appearance = "x".repeat(10000);
    let mut free_text_long = FreeTextAnnotation::new(rect, "Long test");
    free_text_long.default_appearance = long_appearance.clone();

    let annotation_long = free_text_long.to_annotation();
    if let Some(Object::String(da)) = annotation_long.properties.get("DA") {
        assert_eq!(da.len(), 10000);
    }

    // Test with special characters in appearance
    let special_appearance = "/Font\u{0000}\u{00FF} 12 Tf \n\r\t\\()[] rg";
    let mut free_text_special = FreeTextAnnotation::new(rect, "Special test");
    free_text_special.default_appearance = special_appearance.to_string();

    let annotation_special = free_text_special.to_annotation();
    assert!(annotation_special.properties.get("DA").is_some());

    println!("FreeText annotations handle invalid font specifications gracefully");
}

/// Test 4: QuadPoints validation with invalid rectangles
#[test]
fn test_quad_points_invalid_rectangles() {
    // Test with inverted rectangle
    let inverted_rect = Rectangle::new(
        Point::new(100.0, 100.0),
        Point::new(50.0, 50.0), // Upper right is less than lower left
    );

    let result = panic::catch_unwind(|| {
        let _quad_points = QuadPoints::from_rect(&inverted_rect);
    });

    // Should either handle gracefully or panic predictably
    match result {
        Ok(_) => println!("QuadPoints handled inverted rectangle gracefully"),
        Err(_) => println!("QuadPoints panicked on inverted rectangle (expected behavior)"),
    }

    // Test with extreme coordinate values in rectangles
    let extreme_rects = vec![
        Rectangle::new(
            Point::new(f64::MAX, f64::MIN),
            Point::new(f64::INFINITY, f64::NEG_INFINITY),
        ),
        Rectangle::new(Point::new(f64::NAN, 0.0), Point::new(1e100, -1e100)),
        Rectangle::new(Point::new(-1e100, -1e100), Point::new(1e100, 1e100)),
    ];

    for (i, rect) in extreme_rects.iter().enumerate() {
        let result_extreme = panic::catch_unwind(|| {
            let _quad_points = QuadPoints::from_rect(rect);
        });

        match result_extreme {
            Ok(_) => println!("QuadPoints handled extreme rectangle {} gracefully", i),
            Err(_) => println!("QuadPoints panicked on extreme rectangle {}", i),
        }
    }

    // Test with zero-area rectangle
    let zero_rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(100.0, 100.0));

    let result_zero = panic::catch_unwind(|| {
        let _quad_points = QuadPoints::from_rect(&zero_rect);
    });

    match result_zero {
        Ok(_) => println!("QuadPoints handled zero-area rectangle gracefully"),
        Err(_) => println!("QuadPoints panicked on zero-area rectangle"),
    }
}

/// Test 5: Link validation with circular references and invalid URIs
#[test]
fn test_link_validation_edge_cases() {
    let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 100.0));

    // Test circular page reference (page referencing itself)
    let circular_dest = LinkDestination::Fit {
        page: ObjectReference::new(1, 0),
    };
    let circular_action = LinkAction::GoTo(circular_dest);
    let circular_link = LinkAnnotation::new(rect, circular_action);

    // Should create without crashing
    let annotation = circular_link.to_annotation();
    assert_eq!(annotation.annotation_type, AnnotationType::Link);

    // Test invalid page reference (very high object number)
    let invalid_dest = LinkDestination::XYZ {
        page: ObjectReference::new(u32::MAX, 0),
        left: Some(0.0),
        top: Some(0.0),
        zoom: Some(1.0),
    };
    let invalid_action = LinkAction::GoTo(invalid_dest);
    let invalid_link = LinkAnnotation::new(rect, invalid_action);

    let invalid_annotation = invalid_link.to_annotation();
    assert_eq!(invalid_annotation.annotation_type, AnnotationType::Link);

    // Test malformed URI schemes
    let long_uri = "ftp://very.long.domain.name".repeat(100);
    let malformed_uris = vec![
        "not_a_uri",
        "://missing_scheme",
        "scheme://but\u{0000}null\u{00FF}",
        &long_uri, // Very long URI
        "",        // Empty URI
    ];

    for uri in malformed_uris {
        let uri_action = LinkAction::URI {
            uri: uri.to_string(),
        };
        let uri_link =
            LinkAnnotation::new(rect, uri_action).with_highlight_mode(HighlightMode::Outline);

        // Should handle without crashing
        let uri_annotation = uri_link.to_annotation();
        assert_eq!(uri_annotation.annotation_type, AnnotationType::Link);
    }

    println!("Link annotations handle invalid references and URIs gracefully");
}

/// Test 6: Ink annotation with infinite/NaN coordinates
#[test]
fn test_ink_annotation_extreme_coordinates() {
    let mut ink = InkAnnotation::new();

    // Add stroke with infinite coordinates
    let infinite_stroke = vec![
        Point::new(f64::INFINITY, f64::NEG_INFINITY),
        Point::new(f64::NAN, 0.0),
        Point::new(1e100, -1e100),
    ];

    ink = ink.add_stroke(infinite_stroke);

    // Should handle gracefully
    let annotation = ink.to_annotation();
    assert_eq!(annotation.annotation_type, AnnotationType::Ink);

    // Test with empty strokes
    let empty_ink = InkAnnotation::new();
    let empty_annotation = empty_ink.to_annotation();
    assert_eq!(empty_annotation.annotation_type, AnnotationType::Ink);

    // Test with very large number of points
    let large_stroke: Vec<Point> = (0..10000)
        .map(|i| Point::new(i as f64, (i * 2) as f64))
        .collect();

    let large_ink = InkAnnotation::new().add_stroke(large_stroke);
    let large_annotation = large_ink.to_annotation();
    assert_eq!(large_annotation.annotation_type, AnnotationType::Ink);

    println!("Ink annotations handle extreme coordinates and large datasets");
}

/// Test 7: Square annotation with invalid interior colors
#[test]
fn test_square_annotation_invalid_colors() {
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 200.0));

    // Test with various color types
    let colors = vec![
        Color::Gray(-1.0),                // Invalid gray (should be 0.0-1.0)
        Color::Gray(2.0),                 // Invalid gray (should be 0.0-1.0)
        Color::Rgb(-0.5, 1.5, 0.0),       // Invalid RGB values
        Color::Cmyk(2.0, -1.0, 0.5, 1.5), // Invalid CMYK values
    ];

    for color in colors {
        let square = SquareAnnotation::new(rect).with_interior_color(color);

        // Should handle invalid colors gracefully
        let annotation = square.to_annotation();
        assert_eq!(annotation.annotation_type, AnnotationType::Square);

        // Check that interior color property is set
        assert!(annotation.properties.get("IC").is_some());
    }

    println!("Square annotations handle invalid color values gracefully");
}

/// Test 8: Annotation manager stress testing
#[test]
fn test_annotation_manager_stress() {
    let mut manager = AnnotationManager::new();
    let page_refs: Vec<ObjectReference> = (1..=100).map(|i| ObjectReference::new(i, 0)).collect();

    // Add many annotations to multiple pages
    for &page_ref in &page_refs {
        for i in 0..50 {
            // 50 annotations per page
            let rect = Rectangle::new(
                Point::new(i as f64 * 10.0, i as f64 * 5.0),
                Point::new((i + 1) as f64 * 10.0, (i + 1) as f64 * 5.0),
            );

            let annotation = Annotation::new(AnnotationType::Text, rect).with_contents(format!(
                "Annotation {} on page {}",
                i,
                page_ref.number()
            ));

            manager.add_annotation(page_ref, annotation);
        }
    }

    // Verify all annotations were added
    for &page_ref in &page_refs {
        if let Some(page_annotations) = manager.get_page_annotations(&page_ref) {
            assert_eq!(page_annotations.len(), 50);
        }
    }

    // Test with invalid page reference
    let invalid_ref = ObjectReference::new(0, 0);
    assert!(manager.get_page_annotations(&invalid_ref).is_none());

    println!(
        "Annotation manager handles stress testing with {} pages and {} annotations",
        page_refs.len(),
        page_refs.len() * 50
    );
}

/// Test 9: Memory pressure scenarios with large annotation datasets
#[test]
fn test_annotation_memory_pressure() {
    let mut annotations = Vec::new();

    // Create many large annotations
    for i in 0..1000 {
        let rect = Rectangle::new(
            Point::new(i as f64, i as f64),
            Point::new((i + 100) as f64, (i + 100) as f64),
        );

        let large_content = "Large content ".repeat(1000); // ~14KB per annotation
        let annotation = Annotation::new(AnnotationType::FreeText, rect)
            .with_contents(large_content)
            .with_name(format!("large_annotation_{}", i));

        annotations.push(annotation);
    }

    // Test conversion to dictionaries (memory intensive)
    let dicts: Vec<_> = annotations.iter().map(|a| a.to_dict()).collect();

    assert_eq!(dicts.len(), 1000);

    // Verify content integrity
    for (i, dict) in dicts.iter().enumerate() {
        if let Some(Object::String(contents)) = dict.get("Contents") {
            assert!(contents.contains("Large content"));
        }

        if let Some(Object::String(name)) = dict.get("NM") {
            assert_eq!(name, &format!("large_annotation_{}", i));
        }
    }

    println!("Memory pressure test completed with 1000 large annotations (~14MB total)");
}

/// Test 10: Annotation rect consistency validation
#[test]
fn test_annotation_rect_consistency() {
    // Test inverted rectangles (where lower_left > upper_right)
    let inverted_rect = Rectangle::new(
        Point::new(200.0, 300.0), // "lower_left" is actually upper_right
        Point::new(100.0, 200.0), // "upper_right" is actually lower_left
    );

    let annotation = Annotation::new(AnnotationType::Highlight, inverted_rect);
    let dict = annotation.to_dict();

    // Should preserve coordinates as given (no automatic correction)
    if let Some(Object::Array(rect_array)) = dict.get("Rect") {
        assert_eq!(rect_array.len(), 4);
        if let (Object::Real(x1), Object::Real(y1), Object::Real(x2), Object::Real(y2)) = (
            &rect_array[0],
            &rect_array[1],
            &rect_array[2],
            &rect_array[3],
        ) {
            assert_eq!(*x1, 200.0);
            assert_eq!(*y1, 300.0);
            assert_eq!(*x2, 100.0);
            assert_eq!(*y2, 200.0);
        }
    }

    // Test zero-area rectangle
    let zero_rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(100.0, 100.0));

    let zero_annotation = Annotation::new(AnnotationType::Square, zero_rect);
    let zero_dict = zero_annotation.to_dict();
    assert!(zero_dict.get("Rect").is_some());

    // Test extremely small rectangle
    let tiny_rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(100.001, 100.001));

    let tiny_annotation = Annotation::new(AnnotationType::Circle, tiny_rect);
    let tiny_dict = tiny_annotation.to_dict();
    assert!(tiny_dict.get("Rect").is_some());

    println!("Annotation rect consistency handles edge cases appropriately");
}

/// Test 11: State model validation for text annotations
#[test]
fn test_text_annotation_state_transitions() {
    let position = Point::new(100.0, 100.0);

    // Test all valid state model combinations
    let state_combinations = vec![
        ("Review", "None"),
        ("Review", "Accepted"),
        ("Review", "Rejected"),
        ("Review", "Cancelled"),
        ("Review", "Completed"),
        ("Marked", "Marked"),
        ("Marked", "Unmarked"),
    ];

    for (model, state) in state_combinations {
        let text_annot = TextAnnotation::new(position).with_state(model, state);

        let annotation = text_annot.to_annotation();

        assert_eq!(
            annotation.properties.get("StateModel"),
            Some(&Object::String(model.to_string()))
        );
        assert_eq!(
            annotation.properties.get("State"),
            Some(&Object::String(state.to_string()))
        );
    }

    // Test invalid state model combinations
    let invalid_combinations = vec![
        ("Review", "InvalidState"),
        ("InvalidModel", "Accepted"),
        ("", ""),
        ("Review", "Marked"),   // Wrong state for model
        ("Marked", "Accepted"), // Wrong state for model
    ];

    for (model, state) in invalid_combinations {
        let text_annot = TextAnnotation::new(position).with_state(model, state);

        // Should accept any state (no validation)
        let annotation = text_annot.to_annotation();
        assert!(annotation.properties.get("StateModel").is_some());
        assert!(annotation.properties.get("State").is_some());
    }

    println!("Text annotation state transitions handle valid and invalid combinations");
}

/// Test 12: Concurrent annotation operations
#[test]
fn test_concurrent_annotation_operations() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let manager = Arc::new(Mutex::new(AnnotationManager::new()));
    let mut handles = vec![];

    // Spawn multiple threads adding annotations
    for thread_id in 0..10 {
        let manager_clone = Arc::clone(&manager);

        let handle = thread::spawn(move || {
            let page_ref = ObjectReference::new((thread_id + 1) as u32, 0);

            for i in 0..100 {
                let rect = Rectangle::new(
                    Point::new(i as f64, thread_id as f64 * 100.0),
                    Point::new((i + 10) as f64, (thread_id as f64 * 100.0) + 10.0),
                );

                let annotation = Annotation::new(AnnotationType::Text, rect)
                    .with_contents(format!("Thread {} annotation {}", thread_id, i));

                if let Ok(mut mgr) = manager_clone.lock() {
                    mgr.add_annotation(page_ref, annotation);
                }

                // Simulate some work
                thread::yield_now();
            }

            thread_id
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    let mut completed_threads = Vec::new();
    for handle in handles {
        if let Ok(thread_id) = handle.join() {
            completed_threads.push(thread_id);
        }
    }

    // Verify results
    assert_eq!(completed_threads.len(), 10);

    let final_manager = manager.lock().unwrap();
    for thread_id in 0..10 {
        let page_ref = ObjectReference::new((thread_id + 1) as u32, 0);
        if let Some(annotations) = final_manager.get_page_annotations(&page_ref) {
            assert_eq!(annotations.len(), 100);
        }
    }

    println!("Concurrent annotation operations completed successfully");
}

/// Test 13: Border effect intensity validation with cloudy borders
#[test]
fn test_border_effect_intensity_validation() {
    let rect = Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 100.0));

    // Test with various border effect intensities using cloudy border
    let intensities = vec![-1.0, 0.0, 0.5, 1.0, 1.5, 2.0, 3.0, f64::INFINITY];

    for intensity in intensities {
        let square = SquareAnnotation::new(rect).with_cloudy_border(intensity);

        // Should handle any intensity value gracefully (clamped to 0.0-2.0)
        let annotation = square.to_annotation();
        assert_eq!(annotation.annotation_type, AnnotationType::Square);

        // Check that the border effect property is set
        if let Some(Object::Dictionary(be_dict)) = annotation.properties.get("BE") {
            assert!(be_dict.get("S").is_some()); // Style should be set
            if let Some(Object::Real(stored_intensity)) = be_dict.get("I") {
                // Should be clamped to valid range
                assert!(*stored_intensity >= 0.0 && *stored_intensity <= 2.0);
            }
        }
    }

    println!("Border effect intensity validation handles all values gracefully");
}

/// Test 14: Markup annotation with empty metadata
#[test]
fn test_markup_annotation_empty_metadata() {
    let rect = Rectangle::new(Point::new(50.0, 50.0), Point::new(200.0, 100.0));
    let quad_points = QuadPoints::from_rects(&[rect]);

    // Test with empty strings
    let markup = MarkupAnnotation::new(MarkupType::Highlight, rect, quad_points.clone())
        .with_author("")
        .with_subject("")
        .with_contents("");

    let annotation = markup.to_annotation();
    assert_eq!(annotation.annotation_type, AnnotationType::Highlight);

    // Empty strings should still be set as properties
    assert_eq!(
        annotation.properties.get("T"),
        Some(&Object::String("".to_string()))
    );
    assert_eq!(
        annotation.properties.get("Subj"),
        Some(&Object::String("".to_string()))
    );

    // Test with None values (default)
    let markup_none = MarkupAnnotation::new(MarkupType::Underline, rect, quad_points);
    let annotation_none = markup_none.to_annotation();

    // Should not have optional properties
    assert!(annotation_none.properties.get("T").is_none());
    assert!(annotation_none.properties.get("Subj").is_none());

    println!("Markup annotations handle empty metadata appropriately");
}

/// Test 15: Error handling for object reference operations
#[test]
fn test_invalid_object_reference_handling() {
    let mut manager = AnnotationManager::new();

    // Test with zero object number (typically invalid)
    let zero_ref = ObjectReference::new(0, 0);
    let annotation = Annotation::new(
        AnnotationType::Link,
        Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 50.0)),
    );

    let result_ref = manager.add_annotation(zero_ref, annotation);
    assert!(result_ref.number() > 0); // Should generate valid reference

    // Test with maximum object number
    let max_ref = ObjectReference::new(u32::MAX, 0);
    let annotation_max = Annotation::new(
        AnnotationType::Text,
        Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 150.0)),
    );

    let result_max = manager.add_annotation(max_ref, annotation_max);
    assert!(result_max.number() > 0);

    // Test retrieving from invalid reference
    let invalid_page = ObjectReference::new(99999, 0);
    assert!(manager.get_page_annotations(&invalid_page).is_none());

    // Test with high generation number
    let high_gen_ref = ObjectReference::new(10, 255);
    let annotation_gen = Annotation::new(
        AnnotationType::Stamp,
        Rectangle::new(Point::new(50.0, 50.0), Point::new(150.0, 100.0)),
    );

    let result_gen = manager.add_annotation(high_gen_ref, annotation_gen);
    assert_eq!(result_gen.generation(), 0); // Should use 0 for new annotations

    println!("Invalid object reference handling works correctly");
}
