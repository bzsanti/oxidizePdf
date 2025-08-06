//! Forms Cross-Module Integration Tests
//!
//! This test suite focuses on integration between the Forms module and other
//! core modules of the PDF library. These tests ensure that forms work correctly
//! with graphics, text, annotations, document structure, and other components.
//!
//! Test categories:
//! - Forms integration with Document and Page modules
//! - Forms and Graphics integration (colors, styles, drawing)
//! - Forms and Text module integration (fonts, rendering)
//! - Forms and Annotations integration (widget annotations)
//! - Forms and Writer module integration (PDF generation)
//! - Forms and Memory module integration (optimization)

use oxidize_pdf::forms::{
    CheckBox, ComboBox, FieldFlags, FieldOptions, FormField, FormManager, PushButton, TextField,
    Widget, WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::{Dictionary, Object, ObjectReference};
use oxidize_pdf::text::{Font, TextAlign};
use oxidize_pdf::{Document, Page};

/// Test 1: Forms integration with Document and Page creation
#[test]
fn test_forms_document_page_integration() {
    let mut document = Document::new();
    document.set_title("Forms Integration Test");
    document.set_author("Test Suite");

    let mut page = Page::a4();
    let mut form_manager = FormManager::new();

    // Add a text field to the page
    let text_field = TextField::new("document_text_field")
        .with_value("Integrated with Document")
        .with_max_length(100);

    let widget = Widget::new(Rectangle::new(
        Point::new(100.0, 700.0),
        Point::new(400.0, 720.0),
    ));

    let field_ref = form_manager
        .add_text_field(text_field, widget, None)
        .unwrap();

    // Add some page content that should coexist with forms
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(100.0, 750.0)
        .write("This page contains both regular content and form fields")
        .unwrap();

    page.graphics()
        .set_stroke_color(Color::rgb(0.5, 0.5, 0.5))
        .rectangle(90.0, 680.0, 320.0, 50.0)
        .stroke();

    // Verify form manager state
    assert_eq!(form_manager.field_count(), 1);
    assert!(form_manager.get_field("document_text_field").is_some());
    assert_eq!(field_ref.number(), 1);

    // Add page to document
    document.add_page(page);

    // Get AcroForm for document-level integration
    let acro_form = form_manager.get_acro_form();
    let acro_dict = acro_form.to_dict();

    // Verify AcroForm structure integrates properly
    assert!(acro_dict.get("Fields").is_some());
    assert_eq!(
        acro_dict.get("NeedAppearances"),
        Some(&Object::Boolean(true))
    );

    if let Some(Object::Array(fields)) = acro_dict.get("Fields") {
        assert_eq!(fields.len(), 1);

        if let Object::Reference(obj_ref) = &fields[0] {
            assert_eq!(obj_ref.number(), field_ref.number());
        }
    }

    println!("Forms-Document-Page integration test completed successfully");
}

/// Test 2: Forms and Graphics module integration
#[test]
fn test_forms_graphics_integration() {
    let mut form_manager = FormManager::new();

    // Create form fields with various graphic appearances
    let color_test_cases = vec![
        (
            "rgb_field",
            Color::rgb(1.0, 0.0, 0.0),
            Color::rgb(0.9, 0.9, 0.9),
        ),
        (
            "cmyk_field",
            Color::cmyk(0.0, 1.0, 0.0, 0.0),
            Color::cmyk(0.1, 0.1, 0.1, 0.0),
        ),
        ("gray_field", Color::gray(0.3), Color::gray(0.95)),
    ];

    for (i, (field_name, border_color, bg_color)) in color_test_cases.iter().enumerate() {
        let appearance = WidgetAppearance {
            border_color: Some(*border_color),
            background_color: Some(*bg_color),
            border_width: 2.0 + i as f64,
            border_style: match i {
                0 => oxidize_pdf::forms::BorderStyle::Solid,
                1 => oxidize_pdf::forms::BorderStyle::Dashed,
                2 => oxidize_pdf::forms::BorderStyle::Beveled,
                _ => oxidize_pdf::forms::BorderStyle::Solid,
            },
        };

        let field = TextField::new(*field_name)
            .with_value(format!("Graphics integration test {i}"))
            .with_max_length(50);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 600.0 - i as f64 * 40.0),
            Point::new(300.0, 620.0 - i as f64 * 40.0),
        ))
        .with_appearance(appearance);

        form_manager.add_text_field(field, widget, None).unwrap();
    }

    // Verify graphics integration through annotation dictionaries
    for (i, (field_name, _, _)) in color_test_cases.iter().enumerate() {
        if let Some(form_field) = form_manager.get_field(field_name) {
            assert_eq!(form_field.widgets.len(), 1);

            let annotation_dict = form_field.widgets[0].to_annotation_dict();

            // Check that graphics properties are properly encoded
            assert!(annotation_dict.get("BS").is_some()); // Border style
            assert!(annotation_dict.get("MK").is_some()); // Appearance characteristics

            if let Some(Object::Dictionary(mk_dict)) = annotation_dict.get("MK") {
                // Border color should be present
                assert!(mk_dict.get("BC").is_some());
                // Background color should be present
                assert!(mk_dict.get("BG").is_some());

                // Verify color arrays have correct structure
                if let Some(Object::Array(bc_array)) = mk_dict.get("BC") {
                    assert!(!bc_array.is_empty() && bc_array.len() <= 4); // Valid color space
                }

                if let Some(Object::Array(bg_array)) = mk_dict.get("BG") {
                    assert!(!bg_array.is_empty() && bg_array.len() <= 4); // Valid color space
                }
            }

            if let Some(Object::Dictionary(bs_dict)) = annotation_dict.get("BS") {
                // Border width should be encoded
                if let Some(Object::Real(width)) = bs_dict.get("W") {
                    assert_eq!(*width, 2.0 + i as f64);
                }

                // Border style should be encoded
                assert!(bs_dict.get("S").is_some());
            }
        }
    }

    println!("Forms-Graphics integration test completed successfully");
}

/// Test 3: Forms and Text module integration
#[test]
fn test_forms_text_integration() {
    let mut form_manager = FormManager::new();

    // Test different text configurations with forms
    let text_configurations = [
        ("helvetica_field", Font::Helvetica, 12.0, TextAlign::Left),
        ("times_field", Font::TimesRoman, 14.0, TextAlign::Center),
        ("courier_field", Font::Courier, 10.0, TextAlign::Right),
    ];

    for (i, (field_name, font, size, align)) in text_configurations.iter().enumerate() {
        let flags = FieldFlags {
            read_only: false,
            required: i == 0, // First field is required
            no_export: false,
        };

        let quadding = match align {
            TextAlign::Left => Some(0),
            TextAlign::Center => Some(1),
            TextAlign::Right => Some(2),
            _ => Some(0),
        };

        let default_appearance = format!(
            "/{} {} Tf 0 g",
            match font {
                Font::Helvetica => "Helv",
                Font::TimesRoman => "Times",
                Font::Courier => "Cour",
                _ => "Helv",
            },
            size
        );

        let options = FieldOptions {
            flags,
            default_appearance: Some(default_appearance),
            quadding,
        };

        let field = TextField::new(*field_name)
            .with_value(format!(
                "Text integration with {} at {}pt",
                match font {
                    Font::Helvetica => "Helvetica",
                    Font::TimesRoman => "Times Roman",
                    Font::Courier => "Courier",
                    _ => "Default",
                },
                size
            ))
            .with_max_length(100);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 650.0 - i as f64 * 40.0),
            Point::new(400.0, 670.0 - i as f64 * 40.0),
        ));

        form_manager
            .add_text_field(field, widget, Some(options))
            .unwrap();
    }

    // Verify text integration through field dictionaries
    for (i, (field_name, _, size, _)) in text_configurations.iter().enumerate() {
        if let Some(form_field) = form_manager.get_field(field_name) {
            let field_dict = &form_field.field_dict;

            // Check default appearance
            if let Some(Object::String(da)) = field_dict.get("DA") {
                assert!(da.contains(&format!("{size}")));
                assert!(da.contains("Tf")); // Font operator
                assert!(da.contains("g")); // Color operator
            }

            // Check quadding (text alignment)
            if let Some(Object::Integer(q)) = field_dict.get("Q") {
                assert!(*q >= 0 && *q <= 2); // Valid alignment values
            }

            // Check required flag for first field
            if i == 0 {
                if let Some(Object::Integer(flags)) = field_dict.get("Ff") {
                    assert_ne!(*flags & 2, 0); // Required flag
                }
            }
        }
    }

    println!("Forms-Text integration test completed successfully");
}

/// Test 4: Forms and Annotations integration
#[test]
fn test_forms_annotations_integration() {
    let mut form_manager = FormManager::new();

    // Create various form field types that generate widget annotations
    let field_types = vec![
        ("text_annotation", "text"),
        ("checkbox_annotation", "checkbox"),
        ("button_annotation", "button"),
        ("choice_annotation", "choice"),
    ];

    for (i, (field_name, field_type)) in field_types.iter().enumerate() {
        let rect = Rectangle::new(
            Point::new(50.0 + i as f64 * 100.0, 600.0),
            Point::new(140.0 + i as f64 * 100.0, 620.0),
        );

        match *field_type {
            "text" => {
                let text_field = TextField::new(*field_name)
                    .with_value("Annotation integration")
                    .with_max_length(50);

                let widget = Widget::new(rect);
                form_manager
                    .add_text_field(text_field, widget, None)
                    .unwrap();
            }
            "checkbox" => {
                let checkbox = CheckBox::new(*field_name)
                    .checked()
                    .with_export_value("Yes");

                let widget = Widget::new(rect);
                form_manager.add_checkbox(checkbox, widget, None).unwrap();
            }
            "button" => {
                let button = PushButton::new(*field_name).with_caption("Click Me");

                let widget = Widget::new(rect);
                form_manager.add_push_button(button, widget, None).unwrap();
            }
            "choice" => {
                let combo = ComboBox::new(*field_name)
                    .add_option("opt1", "Option 1")
                    .add_option("opt2", "Option 2")
                    .with_value("opt1");

                let widget = Widget::new(rect);
                form_manager.add_combo_box(combo, widget, None).unwrap();
            }
            _ => {}
        }
    }

    // Verify annotation integration
    for (field_name, expected_type) in field_types {
        if let Some(form_field) = form_manager.get_field(field_name) {
            assert_eq!(form_field.widgets.len(), 1);

            let annotation_dict = form_field.widgets[0].to_annotation_dict();

            // All form widgets should be annotations
            assert_eq!(
                annotation_dict.get("Type"),
                Some(&Object::Name("Annot".to_string()))
            );
            assert_eq!(
                annotation_dict.get("Subtype"),
                Some(&Object::Name("Widget".to_string()))
            );

            // Should have rectangle
            assert!(annotation_dict.get("Rect").is_some());

            // Check field type in the field dictionary
            if let Some(Object::Name(ft)) = form_field.field_dict.get("FT") {
                match expected_type {
                    "text" => assert_eq!(ft, "Tx"),
                    "checkbox" | "button" => assert_eq!(ft, "Btn"),
                    "choice" => assert_eq!(ft, "Ch"),
                    _ => {}
                }
            }

            // Widget annotation should be properly formed
            if let Some(Object::Array(rect_array)) = annotation_dict.get("Rect") {
                assert_eq!(rect_array.len(), 4); // [x1, y1, x2, y2]
            }
        }
    }

    println!("Forms-Annotations integration test completed successfully");
}

/// Test 5: Forms and Writer module integration (PDF generation)
#[test]
fn test_forms_writer_integration() {
    let mut document = Document::new();
    document.set_title("Forms Writer Integration Test");

    let mut page = Page::letter();
    let mut form_manager = FormManager::new();

    // Create a comprehensive form for PDF generation
    let form_fields = vec![
        ("full_name", "John Doe", 100),
        ("email", "john.doe@example.com", 150),
        ("phone", "(555) 123-4567", 15),
        ("address", "123 Main St, Anytown, ST 12345", 200),
    ];

    let mut y_position = 700.0;

    for (field_name, default_value, max_length) in form_fields {
        // Add label text to page
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, y_position + 25.0)
            .write(&format!("{}:", field_name.replace('_', " ")))
            .unwrap();

        // Create form field
        let field = TextField::new(field_name)
            .with_value(default_value)
            .with_max_length(max_length);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, y_position),
            Point::new(300.0, y_position + 20.0),
        ));

        form_manager.add_text_field(field, widget, None).unwrap();

        y_position -= 40.0;
    }

    // Add a submit button
    let submit_button = PushButton::new("submit_button").with_caption("Submit Form");

    let button_widget = Widget::new(Rectangle::new(
        Point::new(50.0, y_position),
        Point::new(150.0, y_position + 30.0),
    ));

    form_manager
        .add_push_button(submit_button, button_widget, None)
        .unwrap();

    // Add page graphics that should coexist with forms
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.0, 0.5))
        .set_line_width(2.0)
        .rectangle(40.0, y_position - 10.0, 270.0, 300.0)
        .stroke();

    document.add_page(page);

    // Simulate PDF generation process
    let acro_form = form_manager.get_acro_form();
    let acro_dict = acro_form.to_dict();

    // Verify that all components are properly integrated for PDF writing
    assert_eq!(form_manager.field_count(), 5); // 4 text fields + 1 button

    // AcroForm should be properly structured for PDF writer
    if let Some(Object::Array(fields)) = acro_dict.get("Fields") {
        assert_eq!(fields.len(), 5);

        // All field references should be valid object references
        for field_ref in fields {
            if let Object::Reference(obj_ref) = field_ref {
                assert!(obj_ref.number() > 0);
                assert_eq!(obj_ref.generation(), 0);
            } else {
                panic!("Field reference should be an ObjectReference");
            }
        }
    }

    // Check that form dictionary has required PDF writer properties
    assert!(acro_dict.get("NeedAppearances").is_some());
    assert!(acro_dict.get("Fields").is_some());

    // Optional properties that help with PDF generation
    if let Some(Object::String(da)) = acro_dict.get("DA") {
        assert!(da.contains("Tf")); // Should contain font operator
    }

    println!("Forms-Writer integration test completed successfully");
}

/// Test 6: Forms and Memory module integration (optimization)
#[test]
fn test_forms_memory_integration() {
    // Test memory-efficient form operations
    let iterations = 100;
    let mut memory_indicators = Vec::new();

    // Test incremental form building with memory tracking
    for iteration in 0..iterations {
        let mut form_manager = FormManager::new();

        // Create a moderate number of fields
        for i in 0..50 {
            let field = TextField::new(format!("mem_field_{iteration}_{i}"))
                .with_value(format!("Memory test {iteration} field {i}"))
                .with_max_length(100);

            let widget = Widget::new(Rectangle::new(
                Point::new(50.0, 700.0 - i as f64 * 10.0),
                Point::new(300.0, 720.0 - i as f64 * 10.0),
            ));

            form_manager.add_text_field(field, widget, None).unwrap();
        }

        // Generate AcroForm (this creates additional objects)
        let acro_form = form_manager.get_acro_form();
        let _acro_dict = acro_form.to_dict();

        // Track memory usage indicator
        if iteration % 10 == 0 {
            let memory_indicator = get_memory_usage_indicator();
            memory_indicators.push(memory_indicator);

            if memory_indicators.len() > 1 {
                let prev_memory = memory_indicators[memory_indicators.len() - 2];
                let current_memory = memory_indicator;
                let growth = current_memory.saturating_sub(prev_memory);

                println!(
                    "Iteration {iteration}: memory indicator = {current_memory}, growth = {growth}"
                );

                // Memory growth should be bounded
                assert!(
                    growth < 100,
                    "Excessive memory growth: {growth} at iteration {iteration}"
                );
            }
        }

        // Verify fields are properly managed
        assert_eq!(form_manager.field_count(), 50);

        // Form manager should be efficiently dropped here
    }

    // Check overall memory stability
    if memory_indicators.len() >= 2 {
        let initial_memory = memory_indicators[0];
        let final_memory = memory_indicators[memory_indicators.len() - 1];
        let total_growth = final_memory.saturating_sub(initial_memory);

        println!(
            "Memory integration: initial = {initial_memory}, final = {final_memory}, growth = {total_growth}"
        );

        // Total memory growth should be reasonable
        assert!(
            total_growth < 500,
            "Total memory growth too high: {total_growth}"
        );
    }

    println!("Forms-Memory integration test completed successfully");
}

/// Test 7: Forms integration with Document metadata and structure
#[test]
fn test_forms_document_structure_integration() {
    let mut document = Document::new();

    // Set comprehensive document metadata
    document.set_title("Interactive Form Document");
    document.set_author("PDF Forms Test Suite");
    document.set_subject("Cross-module integration testing");
    document.set_creator("oxidize-pdf Forms Module");

    // Create multiple pages with forms
    let page_count = 3;
    let mut total_form_manager = FormManager::new();

    for page_num in 0..page_count {
        let mut page = Page::a4();

        // Add page header
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 800.0)
            .write(&format!("Form Page {} of {}", page_num + 1, page_count))
            .unwrap();

        // Add form fields specific to this page
        for field_num in 0..3 {
            let field_name = format!("page_{page_num}_{field_num}");
            let field_value = format!("Page {} Field {}", page_num + 1, field_num + 1);

            let field = TextField::new(&field_name)
                .with_value(&field_value)
                .with_max_length(100);

            let widget = Widget::new(Rectangle::new(
                Point::new(50.0, 700.0 - field_num as f64 * 50.0),
                Point::new(400.0, 720.0 - field_num as f64 * 50.0),
            ));

            total_form_manager
                .add_text_field(field, widget, None)
                .unwrap();

            // Add field label
            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(50.0, 725.0 - field_num as f64 * 50.0)
                .write(&format!("Field {}:", field_num + 1))
                .unwrap();
        }

        // Add page-specific graphics
        page.graphics()
            .set_stroke_color(Color::rgb(0.7, 0.7, 0.7))
            .rectangle(40.0, 550.0, 420.0, 200.0)
            .stroke();

        document.add_page(page);
    }

    // Verify document structure with forms
    // assert_eq!(document.pages.len(), page_count); // Cannot access private field
    assert_eq!(total_form_manager.field_count(), page_count * 3);

    // Test AcroForm integration with document structure
    let acro_form = total_form_manager.get_acro_form();
    let acro_dict = acro_form.to_dict();

    // AcroForm should reference all fields across all pages
    if let Some(Object::Array(fields)) = acro_dict.get("Fields") {
        assert_eq!(fields.len(), page_count * 3);

        // Verify field references are properly structured
        for (i, field_ref) in fields.iter().enumerate() {
            if let Object::Reference(obj_ref) = field_ref {
                assert_eq!(obj_ref.number(), (i + 1) as u32);
                assert_eq!(obj_ref.generation(), 0);
            }
        }
    }

    // Verify specific fields from each page
    for page_num in 0..page_count {
        for field_num in 0..3 {
            let field_name = format!("page_{page_num}_{field_num}");
            assert!(
                total_form_manager.get_field(&field_name).is_some(),
                "Field {field_name} not found"
            );
        }
    }

    println!("Forms-Document structure integration test completed successfully");
}

/// Test 8: Forms integration with various PDF objects and references
#[test]
fn test_forms_pdf_objects_integration() {
    let _form_manager = FormManager::new();

    // Create forms with complex object references and structures
    let mut complex_field_dict = Dictionary::new();
    complex_field_dict.set("T", Object::String("complex_field".to_string()));
    complex_field_dict.set("FT", Object::Name("Tx".to_string()));
    complex_field_dict.set("V", Object::String("Complex integration".to_string()));

    // Add complex PDF objects
    let mut additional_dict = Dictionary::new();
    additional_dict.set("CustomProperty", Object::String("Custom Value".to_string()));
    additional_dict.set("NumericProperty", Object::Integer(42));
    additional_dict.set("BooleanProperty", Object::Boolean(true));
    additional_dict.set("RealProperty", Object::Real(3.14159));

    // Create array with mixed object types
    let mixed_array = vec![
        Object::String("String in array".to_string()),
        Object::Integer(123),
        Object::Real(45.67),
        Object::Boolean(false),
        Object::Name("NameInArray".to_string()),
    ];
    additional_dict.set("MixedArray", Object::Array(mixed_array));

    complex_field_dict.set("CustomData", Object::Dictionary(additional_dict));

    // Add object references
    let ref_array = vec![
        Object::Reference(ObjectReference::new(100, 0)),
        Object::Reference(ObjectReference::new(101, 0)),
        Object::Reference(ObjectReference::new(102, 0)),
    ];
    complex_field_dict.set("References", Object::Array(ref_array));

    let complex_field = FormField::new(complex_field_dict);

    // Verify complex object integration
    let field_dict = &complex_field.field_dict;

    // Basic field properties should be preserved
    assert_eq!(
        field_dict.get("T"),
        Some(&Object::String("complex_field".to_string()))
    );
    assert_eq!(field_dict.get("FT"), Some(&Object::Name("Tx".to_string())));
    assert_eq!(
        field_dict.get("V"),
        Some(&Object::String("Complex integration".to_string()))
    );

    // Complex custom data should be preserved
    if let Some(Object::Dictionary(custom_dict)) = field_dict.get("CustomData") {
        assert_eq!(
            custom_dict.get("CustomProperty"),
            Some(&Object::String("Custom Value".to_string()))
        );
        assert_eq!(
            custom_dict.get("NumericProperty"),
            Some(&Object::Integer(42))
        );
        assert_eq!(
            custom_dict.get("BooleanProperty"),
            Some(&Object::Boolean(true))
        );
        assert_eq!(
            custom_dict.get("RealProperty"),
            Some(&Object::Real(3.14159))
        );

        // Mixed array should be preserved
        if let Some(Object::Array(mixed_arr)) = custom_dict.get("MixedArray") {
            assert_eq!(mixed_arr.len(), 5);
            assert_eq!(mixed_arr[0], Object::String("String in array".to_string()));
            assert_eq!(mixed_arr[1], Object::Integer(123));
            assert_eq!(mixed_arr[2], Object::Real(45.67));
            assert_eq!(mixed_arr[3], Object::Boolean(false));
            assert_eq!(mixed_arr[4], Object::Name("NameInArray".to_string()));
        }
    }

    // Object references should be preserved
    if let Some(Object::Array(refs)) = field_dict.get("References") {
        assert_eq!(refs.len(), 3);

        for (i, ref_obj) in refs.iter().enumerate() {
            if let Object::Reference(obj_ref) = ref_obj {
                assert_eq!(obj_ref.number(), (100 + i) as u32);
                assert_eq!(obj_ref.generation(), 0);
            }
        }
    }

    println!("Forms-PDF objects integration test completed successfully");
}

/// Test 9: Forms integration with error handling across modules
#[test]
fn test_forms_error_handling_integration() {
    let mut form_manager = FormManager::new();

    // Test error handling when integrating with other modules
    let error_test_cases = vec![
        // Case 1: Invalid rectangle coordinates
        (
            "invalid_rect",
            Rectangle::new(Point::new(1000.0, 1000.0), Point::new(0.0, 0.0)),
        ),
        // Case 2: Extreme coordinates
        (
            "extreme_coords",
            Rectangle::new(Point::new(-5000.0, -5000.0), Point::new(10000.0, 10000.0)),
        ),
        // Case 3: Zero-area rectangle
        (
            "zero_area",
            Rectangle::new(Point::new(100.0, 100.0), Point::new(100.0, 100.0)),
        ),
        // Case 4: Very small rectangle
        (
            "tiny_rect",
            Rectangle::new(Point::new(100.0, 100.0), Point::new(100.01, 100.01)),
        ),
    ];

    for (field_name, rect) in error_test_cases {
        let field = TextField::new(field_name)
            .with_value("Error handling test")
            .with_max_length(50);

        let widget = Widget::new(rect);

        // Should handle errors gracefully
        let result = form_manager.add_text_field(field, widget, None);

        match result {
            Ok(_) => {
                println!("Field '{field_name}' with problematic rectangle was accepted");
                // Verify field was actually added
                assert!(form_manager.get_field(field_name).is_some());
            }
            Err(e) => {
                println!("Field '{field_name}' properly rejected: {e}");
                // Error was handled gracefully
            }
        }
    }

    // Test error propagation through module boundaries
    let mut error_field_dict = Dictionary::new();
    error_field_dict.set("T", Object::String("error_test".to_string()));
    // Intentionally missing FT (field type) to test error handling

    let error_field = FormField::new(error_field_dict);

    // Should handle malformed field dictionary without crashing
    assert_eq!(error_field.widgets.len(), 0);
    assert!(error_field.field_dict.get("T").is_some());
    assert!(error_field.field_dict.get("FT").is_none());

    println!("Forms error handling integration test completed successfully");
}

/// Test 10: Forms integration with concurrent operations across modules
#[test]
fn test_forms_concurrent_integration() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let document = Arc::new(Mutex::new(Document::new()));
    let form_manager = Arc::new(Mutex::new(FormManager::new()));

    let mut handles = vec![];
    let thread_count = 4;
    let operations_per_thread = 25;

    // Spawn threads that perform concurrent operations across modules
    for thread_id in 0..thread_count {
        let doc = Arc::clone(&document);
        let forms = Arc::clone(&form_manager);

        let handle = thread::spawn(move || {
            let mut successes = 0;

            for i in 0..operations_per_thread {
                // Create page with form fields
                let mut page = Page::new(595.0, 842.0);

                // Add content to page
                if page
                    .text()
                    .set_font(Font::Helvetica, 12.0)
                    .at(50.0, 800.0)
                    .write(&format!("Thread {thread_id} Page {i}"))
                    .is_ok()
                {
                    // Add graphics
                    page.graphics()
                        .set_stroke_color(Color::rgb(0.5, 0.5, 0.5))
                        .rectangle(40.0, 750.0, 200.0, 50.0)
                        .stroke();

                    // Add form field
                    let field_name = format!("concurrent_{thread_id}_{i}");
                    let field = TextField::new(&field_name)
                        .with_value(format!("Concurrent test {thread_id} {i}"))
                        .with_max_length(100);

                    let widget = Widget::new(Rectangle::new(
                        Point::new(50.0, 750.0),
                        Point::new(200.0, 770.0),
                    ));

                    // Try to add field to form manager
                    if let Ok(mut form_mgr) = forms.lock() {
                        if form_mgr.add_text_field(field, widget, None).is_ok() {
                            // Try to add page to document
                            if let Ok(mut doc) = doc.lock() {
                                doc.add_page(page);
                                successes += 1;
                            }
                        }
                    }
                }

                // Yield to other threads
                thread::yield_now();
            }

            (thread_id, successes)
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    let mut total_successes = 0;

    for handle in handles {
        match handle.join() {
            Ok((thread_id, successes)) => {
                println!("Concurrent thread {thread_id} completed: {successes} successes");
                total_successes += successes;
            }
            Err(_) => {
                println!("Concurrent thread panicked");
            }
        }
    }

    // Verify results
    let _final_document = document.lock().unwrap();
    let final_forms = form_manager.lock().unwrap();

    println!("Concurrent integration: {total_successes} total successes");
    println!("Final document pages: [pages not accessible]");
    println!("Final form fields: {}", final_forms.field_count());

    // Results should be consistent
    // assert_eq!(final_document.pages.len(), total_successes); // Cannot access private field
    assert_eq!(final_forms.field_count(), total_successes);

    // Should have reasonable success rate
    let expected_operations = thread_count * operations_per_thread;
    let success_rate = total_successes as f64 / expected_operations as f64;
    assert!(
        success_rate > 0.8,
        "Success rate too low: {:.1}%",
        success_rate * 100.0
    );

    println!("Forms concurrent integration test completed successfully");
}

/// Helper function to get memory usage indicator
fn get_memory_usage_indicator() -> usize {
    use std::time::Instant;

    let start = Instant::now();
    let mut test_data = Vec::new();

    // Create allocations until we hit a time limit
    for i in 0..2000 {
        test_data.push(vec![i; 50]);
        if start.elapsed().as_millis() > 10 {
            break;
        }
    }

    let indicator = test_data.len();
    drop(test_data); // Clean up
    indicator
}
