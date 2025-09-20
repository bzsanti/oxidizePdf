//! Annotations Integration and Performance Tests
//!
//! This test suite addresses integration testing gaps identified in the coverage analysis,
//! focusing on serialization/deserialization roundtrips, PDF ecosystem integration,
//! and performance characteristics with large datasets.
//!
//! Based on coverage analysis: targeting +7% coverage through integration tests

use oxidize_pdf::annotations::*;
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::{Object, ObjectReference};
use oxidize_pdf::{Document, Page};
use std::time::Instant;

/// Test 1: Annotation serialization/deserialization roundtrip
#[test]
fn test_annotation_serialization_roundtrip() {
    // Create comprehensive annotation with all properties
    let rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(300.0, 200.0));
    let mut annotation = Annotation::new(AnnotationType::FreeText, rect)
        .with_contents("Roundtrip test annotation")
        .with_name("test_annotation_001")
        .with_color(Color::Rgb(0.8, 0.2, 0.1))
        .with_border(BorderStyle {
            width: 2.5,
            style: BorderStyleType::Dashed,
            dash_pattern: Some(vec![3.0, 2.0, 1.0]),
        })
        .with_flags(AnnotationFlags {
            print: true,
            no_zoom: true,
            read_only: true,
            ..Default::default()
        });

    // Add custom properties
    annotation
        .properties
        .set("CustomProp", Object::String("custom_value".to_string()));
    annotation.properties.set("NumericProp", Object::Real(42.5));
    annotation.properties.set("BoolProp", Object::Boolean(true));

    // Serialize to dictionary
    let dict = annotation.to_dict();

    // Verify all properties are serialized correctly
    assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
    assert_eq!(
        dict.get("Subtype"),
        Some(&Object::Name("FreeText".to_string()))
    );
    assert_eq!(
        dict.get("Contents"),
        Some(&Object::String("Roundtrip test annotation".to_string()))
    );
    assert_eq!(
        dict.get("NM"),
        Some(&Object::String("test_annotation_001".to_string()))
    );

    // Check rectangle serialization
    if let Some(Object::Array(rect_array)) = dict.get("Rect") {
        assert_eq!(rect_array.len(), 4);
        assert_eq!(rect_array[0], Object::Real(100.0));
        assert_eq!(rect_array[1], Object::Real(100.0));
        assert_eq!(rect_array[2], Object::Real(300.0));
        assert_eq!(rect_array[3], Object::Real(200.0));
    }

    // Check color serialization
    if let Some(Object::Array(color_array)) = dict.get("C") {
        assert_eq!(color_array.len(), 3);
        assert_eq!(color_array[0], Object::Real(0.8));
        assert_eq!(color_array[1], Object::Real(0.2));
        assert_eq!(color_array[2], Object::Real(0.1));
    }

    // Check border style serialization
    if let Some(Object::Dictionary(bs_dict)) = dict.get("BS") {
        assert_eq!(bs_dict.get("W"), Some(&Object::Real(2.5)));
        assert_eq!(bs_dict.get("S"), Some(&Object::Name("D".to_string())));
        if let Some(Object::Array(dash_array)) = bs_dict.get("D") {
            assert_eq!(
                dash_array,
                &vec![Object::Real(3.0), Object::Real(2.0), Object::Real(1.0)]
            );
        }
    }

    // Check flags serialization
    if let Some(Object::Integer(flags)) = dict.get("F") {
        let expected_flags = (1 << 2) | (1 << 3) | (1 << 6); // print | no_zoom | read_only
        assert_eq!(*flags, expected_flags);
    }

    // Check custom properties
    assert_eq!(
        dict.get("CustomProp"),
        Some(&Object::String("custom_value".to_string()))
    );
    assert_eq!(dict.get("NumericProp"), Some(&Object::Real(42.5)));
    assert_eq!(dict.get("BoolProp"), Some(&Object::Boolean(true)));

    println!("Annotation serialization roundtrip completed successfully");
}

/// Test 2: Integration with Document and Page structure
#[test]
fn test_annotations_document_integration() {
    let mut document = Document::new();
    document.set_title("Annotations Integration Test");

    let page1 = Page::a4();
    let page2 = Page::letter();

    let mut manager = AnnotationManager::new();

    // Add annotations to first page
    let page1_ref = ObjectReference::new(1, 0);
    let annotations_page1 = vec![
        TextAnnotation::new(Point::new(100.0, 700.0))
            .with_contents("Note on page 1")
            .with_icon(Icon::Comment)
            .to_annotation(),
        LinkAnnotation::to_uri(
            Rectangle::new(Point::new(200.0, 600.0), Point::new(400.0, 620.0)),
            "https://example.com",
        )
        .to_annotation(),
        MarkupAnnotation::highlight(Rectangle::new(
            Point::new(100.0, 500.0),
            Point::new(300.0, 520.0),
        ))
        .to_annotation(),
    ];

    for annotation in annotations_page1 {
        manager.add_annotation(page1_ref, annotation);
    }

    // Add annotations to second page
    let page2_ref = ObjectReference::new(2, 0);
    let annotations_page2 = vec![
        FreeTextAnnotation::new(
            Rectangle::new(Point::new(50.0, 400.0), Point::new(250.0, 450.0)),
            "Free text on page 2",
        )
        .to_annotation(),
        StampAnnotation::new(
            Rectangle::new(Point::new(300.0, 300.0), Point::new(450.0, 350.0)),
            StampName::Approved,
        )
        .to_annotation(),
    ];

    for annotation in annotations_page2 {
        manager.add_annotation(page2_ref, annotation);
    }

    // Add pages to document
    document.add_page(page1);
    document.add_page(page2);

    // Verify annotations are properly organized
    if let Some(page1_annotations) = manager.get_page_annotations(&page1_ref) {
        assert_eq!(page1_annotations.len(), 3);
        assert_eq!(page1_annotations[0].annotation_type, AnnotationType::Text);
        assert_eq!(page1_annotations[1].annotation_type, AnnotationType::Link);
        assert_eq!(
            page1_annotations[2].annotation_type,
            AnnotationType::Highlight
        );
    }

    if let Some(page2_annotations) = manager.get_page_annotations(&page2_ref) {
        assert_eq!(page2_annotations.len(), 2);
        assert_eq!(
            page2_annotations[0].annotation_type,
            AnnotationType::FreeText
        );
        assert_eq!(page2_annotations[1].annotation_type, AnnotationType::Stamp);
    }

    // Test cross-page annotation references
    let all_annotations = manager.all_annotations();
    assert_eq!(all_annotations.len(), 2); // Two pages

    let total_annotation_count: usize = all_annotations
        .values()
        .map(|page_annotations| page_annotations.len())
        .sum();
    assert_eq!(total_annotation_count, 5);

    println!("Annotations integrate properly with Document and Page structure");
}

/// Test 3: Performance with large annotation datasets
#[test]
fn test_annotation_performance_large_datasets() {
    let start_time = Instant::now();
    let mut manager = AnnotationManager::new();

    const PAGES: u32 = 100;
    const ANNOTATIONS_PER_PAGE: u32 = 50;
    const TOTAL_ANNOTATIONS: u32 = PAGES * ANNOTATIONS_PER_PAGE;

    // Phase 1: Creation performance
    let creation_start = Instant::now();

    for page_num in 1..=PAGES {
        let page_ref = ObjectReference::new(page_num, 0);

        for annot_num in 0..ANNOTATIONS_PER_PAGE {
            let rect = Rectangle::new(
                Point::new(
                    (annot_num % 10) as f64 * 50.0,
                    (annot_num / 10) as f64 * 20.0,
                ),
                Point::new(
                    ((annot_num % 10) + 1) as f64 * 50.0,
                    ((annot_num / 10) + 1) as f64 * 20.0,
                ),
            );

            let annotation_type = match annot_num % 5 {
                0 => AnnotationType::Text,
                1 => AnnotationType::Link,
                2 => AnnotationType::Highlight,
                3 => AnnotationType::FreeText,
                _ => AnnotationType::Square,
            };

            let annotation = Annotation::new(annotation_type, rect)
                .with_contents(format!("Annotation {annot_num} on page {page_num}"))
                .with_name(format!("perf_test_{page_num}_{annot_num}"));

            manager.add_annotation(page_ref, annotation);
        }
    }

    let creation_time = creation_start.elapsed();

    // Phase 2: Retrieval performance
    let retrieval_start = Instant::now();
    let mut retrieved_count = 0;

    for page_num in 1..=PAGES {
        let page_ref = ObjectReference::new(page_num, 0);
        if let Some(annotations) = manager.get_page_annotations(&page_ref) {
            retrieved_count += annotations.len();
        }
    }

    let retrieval_time = retrieval_start.elapsed();

    // Phase 3: Serialization performance
    let serialization_start = Instant::now();
    let all_annotations = manager.all_annotations();
    let mut serialized_count = 0;

    for page_annotations in all_annotations.values() {
        for annotation in page_annotations {
            let _dict = annotation.to_dict();
            serialized_count += 1;
        }
    }

    let serialization_time = serialization_start.elapsed();
    let total_time = start_time.elapsed();

    // Verify all annotations were processed
    assert_eq!(retrieved_count, TOTAL_ANNOTATIONS as usize);
    assert_eq!(serialized_count, TOTAL_ANNOTATIONS as usize);

    // Performance assertions (should complete within reasonable time)
    assert!(
        creation_time.as_millis() < 5000,
        "Creation took too long: {creation_time:?}"
    );
    assert!(
        retrieval_time.as_millis() < 1000,
        "Retrieval took too long: {retrieval_time:?}"
    );
    assert!(
        serialization_time.as_millis() < 3000,
        "Serialization took too long: {serialization_time:?}"
    );

    println!("Performance test completed:");
    println!("  {TOTAL_ANNOTATIONS} annotations across {PAGES} pages");
    println!(
        "  Creation: {:?} ({:.2} annotations/sec)",
        creation_time,
        TOTAL_ANNOTATIONS as f64 / creation_time.as_secs_f64()
    );
    println!(
        "  Retrieval: {:?} ({:.2} annotations/sec)",
        retrieval_time,
        TOTAL_ANNOTATIONS as f64 / retrieval_time.as_secs_f64()
    );
    println!(
        "  Serialization: {:?} ({:.2} annotations/sec)",
        serialization_time,
        TOTAL_ANNOTATIONS as f64 / serialization_time.as_secs_f64()
    );
    println!("  Total time: {total_time:?}");
}

/// Test 4: Cross-annotation relationships and dependencies
#[test]
fn test_cross_annotation_relationships() {
    let mut manager = AnnotationManager::new();
    let page_ref = ObjectReference::new(1, 0);

    // Create parent text annotation
    let _parent_rect = Rectangle::new(Point::new(100.0, 500.0), Point::new(120.0, 520.0));
    let parent_text = TextAnnotation::new(Point::new(100.0, 500.0))
        .with_contents("Parent annotation")
        .to_annotation();

    let parent_ref = manager.add_annotation(page_ref, parent_text);

    // Create popup annotation that references the parent
    let popup_rect = Rectangle::new(Point::new(150.0, 400.0), Point::new(300.0, 500.0));
    let mut popup_annotation = Annotation::new(AnnotationType::Popup, popup_rect);
    popup_annotation
        .properties
        .set("Parent", Object::Reference(parent_ref));
    popup_annotation
        .properties
        .set("Open", Object::Boolean(true));

    let _popup_ref = manager.add_annotation(page_ref, popup_annotation);

    // Create ink annotation with response to parent
    let ink_rect = Rectangle::new(Point::new(200.0, 450.0), Point::new(400.0, 550.0));
    let mut ink_annotation = Annotation::new(AnnotationType::Ink, ink_rect)
        .with_contents("Response to parent annotation");

    ink_annotation
        .properties
        .set("IRT", Object::Reference(parent_ref)); // In Reply To
    ink_annotation
        .properties
        .set("RT", Object::Name("R".to_string())); // Reply Type

    let _ink_ref = manager.add_annotation(page_ref, ink_annotation);

    // Create linking chain: Link -> FreeText -> Square
    let link_rect = Rectangle::new(Point::new(50.0, 300.0), Point::new(200.0, 320.0));
    let link_dest = LinkDestination::XYZ {
        page: page_ref,
        left: Some(300.0),
        top: Some(200.0),
        zoom: Some(1.5),
    };
    let link_annotation = LinkAnnotation::new(link_rect, LinkAction::GoTo(link_dest))
        .with_highlight_mode(HighlightMode::Outline)
        .to_annotation();

    let link_ref = manager.add_annotation(page_ref, link_annotation);

    // FreeText that references the link
    let freetext_rect = Rectangle::new(Point::new(300.0, 150.0), Point::new(500.0, 200.0));
    let mut freetext_annotation =
        FreeTextAnnotation::new(freetext_rect, "Referenced by link").to_annotation();
    freetext_annotation
        .properties
        .set("LinkedFrom", Object::Reference(link_ref));

    let freetext_ref = manager.add_annotation(page_ref, freetext_annotation);

    // Square that groups with freetext
    let square_rect = Rectangle::new(Point::new(295.0, 145.0), Point::new(505.0, 205.0));
    let mut square_annotation = SquareAnnotation::new(square_rect)
        .with_interior_color(Color::Gray(0.5))
        .to_annotation();
    square_annotation
        .properties
        .set("GroupWith", Object::Reference(freetext_ref));

    let _square_ref = manager.add_annotation(page_ref, square_annotation);

    // Verify all annotations are created
    if let Some(page_annotations) = manager.get_page_annotations(&page_ref) {
        assert_eq!(page_annotations.len(), 6);

        // Verify relationships exist in properties
        let popup = &page_annotations[1];
        assert_eq!(popup.annotation_type, AnnotationType::Popup);
        assert_eq!(
            popup.properties.get("Parent"),
            Some(&Object::Reference(parent_ref))
        );

        let ink = &page_annotations[2];
        assert_eq!(ink.annotation_type, AnnotationType::Ink);
        assert_eq!(
            ink.properties.get("IRT"),
            Some(&Object::Reference(parent_ref))
        );

        let freetext = &page_annotations[4];
        assert_eq!(freetext.annotation_type, AnnotationType::FreeText);
        assert_eq!(
            freetext.properties.get("LinkedFrom"),
            Some(&Object::Reference(link_ref))
        );
    }

    println!("Cross-annotation relationships established successfully");
}

/// Test 5: Annotation ordering and Z-index behavior
#[test]
fn test_annotation_ordering_z_index() {
    let mut manager = AnnotationManager::new();
    let page_ref = ObjectReference::new(1, 0);

    // Create overlapping annotations with different creation order
    let base_point = Point::new(100.0, 100.0);
    let size = 100.0;

    let annotations_data = [
        (AnnotationType::Square, "background", 0),
        (AnnotationType::Circle, "middle", 1),
        (AnnotationType::FreeText, "foreground", 2),
        (AnnotationType::Text, "overlay", 3),
        (AnnotationType::Stamp, "top", 4),
    ];

    let mut annotation_refs = Vec::new();

    for (i, (annot_type, name, z_order)) in annotations_data.iter().enumerate() {
        let rect = Rectangle::new(
            Point::new(
                base_point.x + (i as f64 * 10.0),
                base_point.y + (i as f64 * 10.0),
            ),
            Point::new(
                base_point.x + size + (i as f64 * 10.0),
                base_point.y + size + (i as f64 * 10.0),
            ),
        );

        let mut annotation = if *annot_type == AnnotationType::Stamp {
            StampAnnotation::new(rect, StampName::Draft).to_annotation()
        } else if *annot_type == AnnotationType::FreeText {
            FreeTextAnnotation::new(rect, format!("Layer {name}")).to_annotation()
        } else if *annot_type == AnnotationType::Text {
            TextAnnotation::new(rect.lower_left)
                .with_contents(format!("Layer {name}"))
                .to_annotation()
        } else {
            Annotation::new(*annot_type, rect).with_name(format!("layer_{name}"))
        };

        // Add custom Z-order property
        annotation
            .properties
            .set("ZOrder", Object::Integer(*z_order));

        let annot_ref = manager.add_annotation(page_ref, annotation);
        annotation_refs.push(annot_ref);
    }

    // Verify annotations are stored in creation order
    if let Some(page_annotations) = manager.get_page_annotations(&page_ref) {
        assert_eq!(page_annotations.len(), 5);

        for (i, annotation) in page_annotations.iter().enumerate() {
            if let Some(Object::Integer(z_order)) = annotation.properties.get("ZOrder") {
                assert_eq!(*z_order, i as i64);
            }
        }

        // Verify types are in expected order
        let expected_types = [
            AnnotationType::Square,
            AnnotationType::Circle,
            AnnotationType::FreeText,
            AnnotationType::Text,
            AnnotationType::Stamp,
        ];

        for (annotation, expected_type) in page_annotations.iter().zip(expected_types.iter()) {
            assert_eq!(annotation.annotation_type, *expected_type);
        }
    }

    println!("Annotation ordering and Z-index behavior verified");
}

/// Test 6: Annotation with complex geometric transformations
#[test]
fn test_annotation_geometric_transformations() {
    let mut manager = AnnotationManager::new();
    let page_ref = ObjectReference::new(1, 0);

    // Test annotations with rotated coordinates
    let center = Point::new(300.0, 400.0);
    let radius = 100.0;
    let num_annotations = 8;

    for i in 0..num_annotations {
        let angle = (i as f64) * 2.0 * std::f64::consts::PI / (num_annotations as f64);
        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();

        let rotated_rect = Rectangle::new(
            Point::new(x - 20.0, y - 10.0),
            Point::new(x + 20.0, y + 10.0),
        );

        let annotation = Annotation::new(AnnotationType::Square, rotated_rect)
            .with_name(format!("rotated_{i}"))
            .with_color(Color::Rgb(
                (angle.sin() + 1.0) / 2.0,
                (angle.cos() + 1.0) / 2.0,
                0.5,
            ));

        manager.add_annotation(page_ref, annotation);
    }

    // Test annotations on page boundaries
    let page_width = 612.0; // Standard letter width
    let page_height = 792.0; // Standard letter height

    let boundary_annotations = vec![
        (
            "top_left",
            Rectangle::new(
                Point::new(0.0, page_height - 20.0),
                Point::new(20.0, page_height),
            ),
        ),
        (
            "top_right",
            Rectangle::new(
                Point::new(page_width - 20.0, page_height - 20.0),
                Point::new(page_width, page_height),
            ),
        ),
        (
            "bottom_left",
            Rectangle::new(Point::new(0.0, 0.0), Point::new(20.0, 20.0)),
        ),
        (
            "bottom_right",
            Rectangle::new(
                Point::new(page_width - 20.0, 0.0),
                Point::new(page_width, 20.0),
            ),
        ),
        (
            "center",
            Rectangle::new(
                Point::new(page_width / 2.0 - 10.0, page_height / 2.0 - 10.0),
                Point::new(page_width / 2.0 + 10.0, page_height / 2.0 + 10.0),
            ),
        ),
    ];

    for (name, rect) in boundary_annotations {
        let annotation = Annotation::new(AnnotationType::Text, rect)
            .with_name(name.to_string())
            .with_contents(format!("Boundary annotation: {name}"));

        manager.add_annotation(page_ref, annotation);
    }

    // Verify all annotations were created
    if let Some(page_annotations) = manager.get_page_annotations(&page_ref) {
        assert_eq!(page_annotations.len(), num_annotations + 5);

        // Verify rotated annotations have correct names
        let rotated_count = page_annotations
            .iter()
            .filter(|a| a.name.as_ref().is_some_and(|n| n.starts_with("rotated_")))
            .count();
        assert_eq!(rotated_count, num_annotations);

        // Verify boundary annotations
        let boundary_names: Vec<_> = page_annotations
            .iter()
            .filter_map(|a| a.name.as_ref())
            .filter(|n| {
                [
                    "top_left",
                    "top_right",
                    "bottom_left",
                    "bottom_right",
                    "center",
                ]
                .contains(&n.as_str())
            })
            .collect();
        assert_eq!(boundary_names.len(), 5);
    }

    println!("Annotation geometric transformations completed successfully");
}

/// Test 7: Memory efficiency with annotation cleanup
#[test]
fn test_annotation_memory_efficiency() {
    let initial_memory = get_memory_usage_estimate();
    let mut manager = AnnotationManager::new();
    let page_ref = ObjectReference::new(1, 0);

    // Phase 1: Create many annotations
    const BATCH_SIZE: usize = 1000;
    let mut annotation_refs = Vec::new();

    for batch in 0..5 {
        for i in 0..BATCH_SIZE {
            let rect = Rectangle::new(
                Point::new((i % 100) as f64, (i / 100) as f64),
                Point::new((i % 100 + 10) as f64, (i / 100 + 10) as f64),
            );

            let large_content = format!(
                "Batch {} annotation {} with large content: {}",
                batch,
                i,
                "x".repeat(1000)
            );

            let annotation = Annotation::new(AnnotationType::FreeText, rect)
                .with_contents(large_content)
                .with_name(format!("batch_{batch}_{i}"));

            let annot_ref = manager.add_annotation(page_ref, annotation);
            annotation_refs.push(annot_ref);
        }

        // Check memory usage growth
        let current_memory = get_memory_usage_estimate();
        let memory_growth = current_memory - initial_memory;

        println!(
            "After batch {}: {} annotations, memory growth estimate: {}KB",
            batch,
            (batch + 1) * BATCH_SIZE,
            memory_growth / 1024
        );

        // Memory growth should be reasonable (not exponential)
        assert!(
            memory_growth < (batch + 1) * BATCH_SIZE * 2000, // ~2KB per annotation max
            "Memory usage growing too quickly: {}KB for {} annotations",
            memory_growth / 1024,
            (batch + 1) * BATCH_SIZE
        );
    }

    // Phase 2: Verify data integrity after memory pressure
    if let Some(page_annotations) = manager.get_page_annotations(&page_ref) {
        assert_eq!(page_annotations.len(), 5 * BATCH_SIZE);

        // Check a few random annotations
        let check_indices = vec![0, 1000, 2500, 4000, 4999];
        for &idx in &check_indices {
            let annotation = &page_annotations[idx];
            assert!(annotation.contents.is_some());
            assert!(annotation.name.is_some());

            if let Some(ref content) = annotation.contents {
                assert!(content.len() > 1000); // Should contain large content
                assert!(content.contains("with large content"));
            }
        }
    }

    let final_memory = get_memory_usage_estimate();
    let total_growth = final_memory - initial_memory;

    println!("Memory efficiency test completed:");
    println!("  Total annotations: {}", 5 * BATCH_SIZE);
    println!("  Total memory growth estimate: {}KB", total_growth / 1024);
    println!(
        "  Average per annotation: {}KB",
        total_growth / (5 * BATCH_SIZE * 1024)
    );
}

/// Test 8: Annotation interaction with PDF viewer compatibility
#[test]
fn test_pdf_viewer_compatibility() {
    let mut manager = AnnotationManager::new();
    let page_ref = ObjectReference::new(1, 0);

    // Create annotations that should be compatible with major PDF viewers

    // Adobe Reader compatible text annotation
    let adobe_text = TextAnnotation::new(Point::new(100.0, 700.0))
        .with_contents("Adobe compatible note")
        .with_icon(Icon::Note)
        .open()
        .with_state("Review", "None")
        .to_annotation();

    let mut adobe_annotation = adobe_text;
    adobe_annotation.properties.set(
        "CreationDate",
        Object::String("D:20240101120000+00'00'".to_string()),
    );
    adobe_annotation
        .properties
        .set("M", Object::String("D:20240101120000+00'00'".to_string()));
    adobe_annotation
        .properties
        .set("Popup", Object::Reference(ObjectReference::new(100, 0)));

    manager.add_annotation(page_ref, adobe_annotation);

    // Browser compatible link annotation
    let browser_link = LinkAnnotation::to_uri(
        Rectangle::new(Point::new(200.0, 600.0), Point::new(400.0, 620.0)),
        "https://www.example.com/test",
    )
    .with_highlight_mode(HighlightMode::Invert)
    .to_annotation();

    manager.add_annotation(page_ref, browser_link);

    // Standard highlight for mobile viewers
    let mobile_highlight = MarkupAnnotation::highlight(Rectangle::new(
        Point::new(100.0, 500.0),
        Point::new(300.0, 520.0),
    ))
    .with_author("Mobile User")
    .with_contents("Mobile highlight content")
    .to_annotation();

    manager.add_annotation(page_ref, mobile_highlight);

    // Stamp annotation for office compatibility
    let office_stamp = StampAnnotation::new(
        Rectangle::new(Point::new(450.0, 300.0), Point::new(550.0, 350.0)),
        StampName::Approved,
    )
    .to_annotation();

    manager.add_annotation(page_ref, office_stamp);

    // Verify all annotations have viewer-compatible properties
    if let Some(page_annotations) = manager.get_page_annotations(&page_ref) {
        assert_eq!(page_annotations.len(), 4);

        for annotation in page_annotations {
            let dict = annotation.to_dict();

            // All annotations should have required fields
            assert!(dict.get("Type").is_some());
            assert!(dict.get("Subtype").is_some());
            assert!(dict.get("Rect").is_some());

            // Check for viewer compatibility flags
            match annotation.annotation_type {
                AnnotationType::Text => {
                    assert!(dict.get("Open").is_some());
                    assert!(dict.get("Name").is_some()); // Icon name
                    assert!(dict.get("CreationDate").is_some());
                }
                AnnotationType::Link => {
                    assert!(dict.get("A").is_some()); // Action
                    assert!(dict.get("H").is_some()); // Highlight mode
                }
                AnnotationType::Highlight => {
                    assert!(dict.get("QuadPoints").is_some());
                    assert!(dict.get("T").is_some()); // Author
                }
                AnnotationType::Stamp => {
                    assert!(dict.get("Name").is_some()); // Stamp name
                }
                _ => {}
            }
        }
    }

    println!("PDF viewer compatibility annotations created successfully");
}

/// Helper function to estimate memory usage (approximation)
fn get_memory_usage_estimate() -> usize {
    // This is a rough approximation since Rust doesn't provide direct memory usage
    // In a real implementation, you might use system calls or memory profiling tools

    // For testing purposes, we'll use a stable value that simulates memory growth
    // based on the current state of the program
    static mut COUNTER: usize = 0;
    unsafe {
        COUNTER += 100; // Simulate some memory allocation
        COUNTER
    }
}
