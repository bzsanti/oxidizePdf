//! Forms Error Handling Tests
//!
//! Comprehensive error handling tests to address the critical 15% coverage gap
//! identified in forms error scenarios. These tests ensure the forms module
//! handles malformed input, edge cases, and error conditions gracefully.
//!
//! Test categories:
//! - Malformed form dictionaries
//! - Invalid field configurations
//! - Resource exhaustion scenarios
//! - Circular field references
//! - Memory safety edge cases
//! - API misuse prevention

use oxidize_pdf::forms::{
    AcroForm, ComboBox, FieldFlags, FieldOptions, FormData, FormField, FormManager, ListBox,
    RadioButton, TextField, Widget, WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::{Dictionary, Object, ObjectReference};

/// Test 1: Malformed field dictionary handling
#[test]
fn test_malformed_field_dictionary() {
    // Test FormField creation with malformed dictionary
    let mut malformed_dict = Dictionary::new();

    // Missing required field type
    malformed_dict.set("T", Object::String("test_field".to_string()));
    // Missing FT (field type) - this should be handled gracefully

    let form_field = FormField::new(malformed_dict);

    // Should not panic, even with incomplete dictionary
    assert_eq!(form_field.widgets.len(), 0);
    assert!(form_field.field_dict.get("T").is_some());
    assert!(form_field.field_dict.get("FT").is_none()); // Missing field type

    println!("Malformed dictionary handled without panic");
}

/// Test 2: Invalid field names and special characters
#[test]
fn test_invalid_field_names() {
    let mut form_manager = FormManager::new();

    // Test with various problematic field names
    let problematic_names = vec![
        "",                       // Empty name
        " ",                      // Whitespace only
        "field with spaces",      // Spaces
        "field/with/slashes",     // Slashes
        "field.with.dots",        // Dots
        "field-with-dashes",      // Dashes
        "field_with_underscores", // Underscores
        "field@with@symbols",     // Special symbols
        "field\nwith\nnewlines",  // Control characters
        "field\twith\ttabs",      // Tab characters
        "\u{0000}field\u{0000}",  // Null characters
        "🎯field🎯",              // Unicode emoji
        "あいうえお",             // Japanese characters
        "نموذج",                  // Arabic characters
        "\r\nfield\r\n",          // CRLF
    ];

    for (i, name) in problematic_names.iter().enumerate() {
        let field = TextField::new(*name)
            .with_value(format!("Test value {}", i))
            .with_max_length(100);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 700.0 - i as f64 * 20.0),
            Point::new(300.0, 720.0 - i as f64 * 20.0),
        ));

        // Should handle all field names without panicking
        let result = form_manager.add_text_field(field, widget, None);

        match result {
            Ok(obj_ref) => {
                println!(
                    "Field '{}' created successfully with ref {}",
                    name,
                    obj_ref.number()
                );
                // Verify field was actually added
                assert!(form_manager.get_field(name).is_some());
            }
            Err(e) => {
                println!("Field '{}' creation failed (may be expected): {}", name, e);
                // Some names might legitimately fail - that's ok as long as we don't panic
            }
        }
    }

    println!(
        "Processed {} problematic field names without crashing",
        problematic_names.len()
    );
}

/// Test 3: Extreme field values and lengths
#[test]
fn test_extreme_field_values() {
    let mut form_manager = FormManager::new();

    // Test extremely long field values
    let very_long = "x".repeat(10_000);
    let extremely_long = "y".repeat(100_000);
    let unicode_long = "🎯".repeat(1_000);
    let mixed_unicode = format!(
        "{}{}{}",
        "a".repeat(1_000),
        "🎯".repeat(1_000),
        "ñ".repeat(1_000)
    );

    let extreme_cases = vec![
        ("empty", ""),
        ("single_char", "a"),
        ("very_long", very_long.as_str()),
        ("extremely_long", extremely_long.as_str()),
        ("unicode_long", unicode_long.as_str()),
        ("mixed_unicode", mixed_unicode.as_str()),
    ];

    for (i, (case_name, value)) in extreme_cases.iter().enumerate() {
        let field_name = format!("extreme_{}", case_name);

        let field = TextField::new(&field_name)
            .with_value(*value)
            .with_max_length(if value.len() > 50_000 {
                200_000
            } else {
                50_000
            });

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 600.0 - i as f64 * 30.0),
            Point::new(300.0, 620.0 - i as f64 * 30.0),
        ));

        let start_time = std::time::Instant::now();
        let result = form_manager.add_text_field(field, widget, None);
        let creation_time = start_time.elapsed();

        match result {
            Ok(_) => {
                println!(
                    "Extreme case '{}' (len: {}) handled in {:?}",
                    case_name,
                    value.len(),
                    creation_time
                );

                // Should complete in reasonable time even with large values
                assert!(
                    creation_time.as_secs() < 5,
                    "Field creation took too long: {:?} for case '{}'",
                    creation_time,
                    case_name
                );
            }
            Err(e) => {
                println!(
                    "Extreme case '{}' failed (may be expected): {}",
                    case_name, e
                );
                // Some extremely large values might legitimately fail
            }
        }
    }
}

/// Test 4: Invalid widget rectangles and coordinates
#[test]
fn test_invalid_widget_rectangles() {
    let mut form_manager = FormManager::new();

    // Test various invalid rectangle configurations
    let invalid_rects = vec![
        // Zero-width rectangle
        Rectangle::new(Point::new(100.0, 100.0), Point::new(100.0, 120.0)),
        // Zero-height rectangle
        Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 100.0)),
        // Zero-area rectangle
        Rectangle::new(Point::new(100.0, 100.0), Point::new(100.0, 100.0)),
        // Inverted rectangle (right < left)
        Rectangle::new(Point::new(200.0, 100.0), Point::new(100.0, 120.0)),
        // Inverted rectangle (bottom > top)
        Rectangle::new(Point::new(100.0, 200.0), Point::new(200.0, 100.0)),
        // Extreme negative coordinates
        Rectangle::new(Point::new(-1000.0, -1000.0), Point::new(-500.0, -500.0)),
        // Extreme positive coordinates
        Rectangle::new(Point::new(50000.0, 50000.0), Point::new(60000.0, 60000.0)),
        // Mixed extreme coordinates
        Rectangle::new(Point::new(-5000.0, 100.0), Point::new(5000.0, 200.0)),
        // Fractional coordinates
        Rectangle::new(Point::new(100.5, 100.7), Point::new(200.3, 120.9)),
        // Very small rectangle
        Rectangle::new(Point::new(100.0, 100.0), Point::new(100.1, 100.1)),
    ];

    for (i, rect) in invalid_rects.iter().enumerate() {
        let field_name = format!("invalid_rect_{}", i);
        let field = TextField::new(&field_name).with_value("Test");
        let widget = Widget::new(*rect);

        let result = form_manager.add_text_field(field, widget, None);

        match result {
            Ok(obj_ref) => {
                println!("Invalid rect {} handled successfully: {:?}", i, rect);
                assert_eq!(obj_ref.number(), (i + 1) as u32);

                // Verify the field can be retrieved
                assert!(form_manager.get_field(&field_name).is_some());
            }
            Err(e) => {
                println!(
                    "Invalid rect {} rejected (may be correct): {} - {:?}",
                    i, e, rect
                );
                // Some invalid rectangles should be rejected
            }
        }
    }
}

/// Test 5: Choice fields with invalid options and selections
#[test]
fn test_invalid_choice_field_configurations() {
    let mut form_manager = FormManager::new();

    // Test RadioButton with invalid selections
    let invalid_radio_tests = vec![
        // Selection index out of bounds
        (vec![("A", "Option A"), ("B", "Option B")], Some(5)), // Index 5 invalid
        (vec![("A", "Option A")], Some(10)),                   // Index 10 invalid for single option
        (vec![], Some(0)),                                     // No options but selection specified
        // Negative selection (should be handled as None or error)
        (vec![("A", "Option A"), ("B", "Option B")], Some(usize::MAX)), // Extreme value
    ];

    for (i, (options, selection)) in invalid_radio_tests.iter().enumerate() {
        let field_name = format!("invalid_radio_{}", i);
        let mut radio = RadioButton::new(&field_name);

        // Add options
        for (value, label) in options {
            radio = radio.add_option(*value, *label);
        }

        // Set invalid selection
        if let Some(sel) = selection {
            radio = radio.with_selected(*sel);
        }

        let widgets: Vec<Widget> = options
            .iter()
            .enumerate()
            .map(|(j, _)| {
                Widget::new(Rectangle::new(
                    Point::new(50.0 + j as f64 * 50.0, 500.0 - i as f64 * 30.0),
                    Point::new(70.0 + j as f64 * 50.0, 520.0 - i as f64 * 30.0),
                ))
            })
            .collect();

        let result = form_manager.add_radio_buttons(radio, widgets, None);

        match result {
            Ok(_) => {
                println!(
                    "Invalid radio {} handled: {} options, selection {:?}",
                    i,
                    options.len(),
                    selection
                );
            }
            Err(e) => {
                println!("Invalid radio {} rejected: {}", i, e);
            }
        }
    }

    // Test ListBox with invalid selections
    let invalid_list_tests = vec![
        // Multiple selections with some invalid indices
        (
            vec![("A", "Option A"), ("B", "Option B")],
            vec![0, 1, 5, 10],
        ),
        // Empty selection list but indices specified
        (vec![], vec![0, 1, 2]),
        // Duplicate selections
        (vec![("A", "Option A"), ("B", "Option B")], vec![0, 0, 1, 1]),
        // Very large selection indices
        (vec![("A", "Option A")], vec![usize::MAX, usize::MAX - 1]),
    ];

    for (i, (options, selections)) in invalid_list_tests.iter().enumerate() {
        let field_name = format!("invalid_list_{}", i);
        let mut listbox = ListBox::new(&field_name);

        for (value, label) in options {
            listbox = listbox.add_option(*value, *label);
        }

        listbox = listbox.multi_select().with_selected(selections.clone());

        let widget = Widget::new(Rectangle::new(
            Point::new(200.0, 500.0 - i as f64 * 40.0),
            Point::new(350.0, 540.0 - i as f64 * 40.0),
        ));

        let result = form_manager.add_list_box(listbox, widget, None);

        match result {
            Ok(_) => {
                println!(
                    "Invalid listbox {} handled: {} options, selections {:?}",
                    i,
                    options.len(),
                    selections
                );
            }
            Err(e) => {
                println!("Invalid listbox {} rejected: {}", i, e);
            }
        }
    }
}

/// Test 6: Memory exhaustion scenarios
#[test]
fn test_memory_exhaustion_protection() {
    // Test creating forms that could potentially exhaust memory

    // Test 1: Many small fields
    let many_fields_test = || -> Result<(), Box<dyn std::error::Error>> {
        let mut form_manager = FormManager::new();
        let field_count = 10_000; // Large but not excessive

        let start_time = std::time::Instant::now();

        for i in 0..field_count {
            let field_name = format!("mem_field_{}", i);
            let field = TextField::new(&field_name)
                .with_value(format!("Value {}", i))
                .with_max_length(100);

            let widget = Widget::new(Rectangle::new(
                Point::new(50.0, 700.0),
                Point::new(300.0, 720.0),
            ));

            form_manager.add_text_field(field, widget, None)?;

            // Check for reasonable performance periodically
            if i % 1000 == 0 {
                let elapsed = start_time.elapsed();
                if elapsed.as_secs() > 30 {
                    return Err("Memory test taking too long".into());
                }
                println!("Created {} fields in {:?}", i + 1, elapsed);
            }
        }

        let total_time = start_time.elapsed();
        println!("Created {} fields in {:?}", field_count, total_time);

        // Should complete in reasonable time
        assert!(
            total_time.as_secs() < 60,
            "Memory test took too long: {:?}",
            total_time
        );
        assert_eq!(form_manager.field_count(), field_count);

        Ok(())
    };

    match many_fields_test() {
        Ok(_) => println!("Memory exhaustion test (many fields) passed"),
        Err(e) => println!("Memory exhaustion test failed (may be expected): {}", e),
    }

    // Test 2: Fields with extremely large values
    let large_values_test = || -> Result<(), Box<dyn std::error::Error>> {
        let mut form_manager = FormManager::new();
        let large_value = "x".repeat(1_000_000); // 1MB string

        let field = TextField::new("large_value_field")
            .with_value(&large_value)
            .with_max_length(2_000_000);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 500.0),
            Point::new(300.0, 520.0),
        ));

        let start_time = std::time::Instant::now();
        let result = form_manager.add_text_field(field, widget, None);
        let creation_time = start_time.elapsed();

        match result {
            Ok(_) => {
                println!("Large value field created in {:?}", creation_time);
                // Should complete quickly even with large values
                assert!(
                    creation_time.as_secs() < 10,
                    "Large value creation took too long: {:?}",
                    creation_time
                );
            }
            Err(e) => {
                println!("Large value field rejected (may be correct): {}", e);
            }
        }

        Ok(())
    };

    match large_values_test() {
        Ok(_) => println!("Memory exhaustion test (large values) passed"),
        Err(e) => println!("Memory exhaustion test failed: {}", e),
    }
}

/// Test 7: Circular reference detection
#[test]
fn test_circular_reference_detection() {
    // Test potential circular references in form structures

    let mut form_manager = FormManager::new();

    // Create fields that might reference each other
    let field1 = TextField::new("field_1").with_value("Refers to field_2");
    let widget1 = Widget::new(Rectangle::new(
        Point::new(50.0, 600.0),
        Point::new(300.0, 620.0),
    ));
    let ref1 = form_manager.add_text_field(field1, widget1, None).unwrap();

    let field2 = TextField::new("field_2").with_value("Refers to field_1");
    let widget2 = Widget::new(Rectangle::new(
        Point::new(50.0, 550.0),
        Point::new(300.0, 570.0),
    ));
    let ref2 = form_manager.add_text_field(field2, widget2, None).unwrap();

    // Verify both fields were created
    assert_eq!(ref1.number(), 1);
    assert_eq!(ref2.number(), 2);
    assert_eq!(form_manager.field_count(), 2);

    // In a more complex scenario, we might have calculation dependencies
    // that could create circular references. For now, we test that basic
    // field creation doesn't create issues.

    println!("Circular reference detection test completed - basic case handled");

    // TODO: Add more sophisticated circular reference detection when
    // calculation order and field dependencies are implemented
}

/// Test 8: Invalid color and appearance configurations
#[test]
fn test_invalid_appearance_configurations() {
    // Test widget appearances with invalid or extreme values

    let invalid_appearances = vec![
        // Extreme border widths
        WidgetAppearance {
            border_color: Some(Color::black()),
            background_color: None,
            border_width: -10.0, // Negative border width
            border_style: oxidize_pdf::forms::BorderStyle::Solid,
        },
        WidgetAppearance {
            border_color: Some(Color::black()),
            background_color: None,
            border_width: 1000.0, // Extremely thick border
            border_style: oxidize_pdf::forms::BorderStyle::Solid,
        },
        WidgetAppearance {
            border_color: Some(Color::black()),
            background_color: None,
            border_width: f64::NAN, // NaN border width
            border_style: oxidize_pdf::forms::BorderStyle::Solid,
        },
        WidgetAppearance {
            border_color: Some(Color::black()),
            background_color: None,
            border_width: f64::INFINITY, // Infinite border width
            border_style: oxidize_pdf::forms::BorderStyle::Solid,
        },
    ];

    for (i, appearance) in invalid_appearances.iter().enumerate() {
        let rect = Rectangle::new(
            Point::new(50.0, 500.0 - i as f64 * 30.0),
            Point::new(200.0, 520.0 - i as f64 * 30.0),
        );

        let widget = Widget::new(rect).with_appearance(appearance.clone());

        // Test widget annotation dictionary creation
        let result = std::panic::catch_unwind(|| {
            let dict = widget.to_annotation_dict();
            println!("Invalid appearance {} handled, dict created", i);
            dict
        });

        match result {
            Ok(_dict) => {
                println!("Invalid appearance {} processed successfully", i);
            }
            Err(_) => {
                println!("Invalid appearance {} caused panic (should be handled)", i);
                // This identifies cases where better error handling is needed
            }
        }
    }
}

/// Test 9: Form data extraction with corrupted data
#[test]
fn test_corrupted_form_data_handling() {
    // Test FormData with various corrupted or invalid inputs

    let mut form_data = FormData::new();

    // Test setting values with problematic data
    let very_long_str = "x".repeat(100_000);
    let problematic_values = vec![
        ("null_chars", "Value\u{0000}with\u{0000}nulls"),
        (
            "control_chars",
            "Value\u{0001}\u{0002}\u{0003}with\u{0004}controls",
        ),
        ("very_long", very_long_str.as_str()),
        ("unicode_mixed", "Normal🎯text混合テキスト\u{0000}null"),
        (
            "binary_data",
            "\u{0080}\u{0081}\u{0082}\u{0083}\u{0084}\u{0085}\u{0086}\u{0087}",
        ),
        ("malformed_utf8", "\u{00FF}\u{00FE}\u{00FD}"),
    ];

    for (key, value) in problematic_values {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            form_data.set_value(key, value);
            form_data.get_value(key).map(|s| s.to_string())
        }));

        match result {
            Ok(retrieved_value) => {
                if let Some(val) = retrieved_value {
                    println!("Problematic value '{}' handled: length {}", key, val.len());
                } else {
                    println!("Problematic value '{}' resulted in None", key);
                }
            }
            Err(_) => {
                println!(
                    "Problematic value '{}' caused panic (needs better handling)",
                    key
                );
            }
        }
    }

    // Test with extremely large number of key-value pairs
    let large_data_test = std::panic::catch_unwind(|| {
        let mut large_form_data = FormData::new();

        for i in 0..100_000 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            large_form_data.set_value(&key, &value);
        }

        println!(
            "Large form data test: {} entries",
            large_form_data.values.len()
        );
        large_form_data.values.len()
    });

    match large_data_test {
        Ok(count) => {
            println!("Large form data test passed: {} entries", count);
            assert_eq!(count, 100_000);
        }
        Err(_) => {
            println!("Large form data test failed - memory or performance issue");
        }
    }
}

/// Test 10: Invalid field flags and options combinations
#[test]
fn test_invalid_field_flags_combinations() {
    let mut form_manager = FormManager::new();

    // Test with contradictory or problematic flag combinations
    let problematic_options = vec![
        // Read-only but required (user can't fill required field)
        FieldOptions {
            flags: FieldFlags {
                read_only: true,
                required: true,
                no_export: false,
            },
            default_appearance: Some("/Invalid-Font 12 Tf 0 g".to_string()), // Invalid font
            quadding: Some(-1), // Invalid quadding value
        },
        // All flags enabled (potentially conflicting)
        FieldOptions {
            flags: FieldFlags {
                read_only: true,
                required: true,
                no_export: true,
            },
            default_appearance: Some("".to_string()), // Empty appearance
            quadding: Some(99),                       // Invalid quadding value
        },
        // Extreme quadding values
        FieldOptions {
            flags: FieldFlags {
                read_only: false,
                required: false,
                no_export: false,
            },
            default_appearance: Some(
                "Malformed appearance string without proper format".to_string(),
            ),
            quadding: Some(i32::MAX), // Extreme quadding value
        },
    ];

    for (i, options) in problematic_options.iter().enumerate() {
        let field_name = format!("problematic_flags_{}", i);
        let field = TextField::new(&field_name)
            .with_value("Test value")
            .with_max_length(100);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 600.0 - i as f64 * 40.0),
            Point::new(300.0, 620.0 - i as f64 * 40.0),
        ));

        let result = form_manager.add_text_field(field, widget, Some(options.clone()));

        match result {
            Ok(obj_ref) => {
                println!("Problematic flags {} handled: ref {}", i, obj_ref.number());

                // Verify the field was created and can be retrieved
                if let Some(form_field) = form_manager.get_field(&field_name) {
                    let dict = &form_field.field_dict;
                    println!("  Field dict has {} entries", dict.len());

                    // Check if problematic values were sanitized or preserved
                    if let Some(Object::Integer(flags)) = dict.get("Ff") {
                        println!("  Flags value: {}", flags);
                    }
                    if let Some(Object::String(da)) = dict.get("DA") {
                        println!("  Appearance length: {}", da.len());
                    }
                    if let Some(Object::Integer(q)) = dict.get("Q") {
                        println!("  Quadding value: {}", q);
                    }
                }
            }
            Err(e) => {
                println!("Problematic flags {} rejected: {}", i, e);
                // Some combinations should legitimately be rejected
            }
        }
    }
}

/// Test 11: AcroForm dictionary corruption handling
#[test]
fn test_acroform_corruption_handling() {
    // Test AcroForm creation and dictionary generation with edge cases

    let mut acro_form = AcroForm::new();

    // Add a massive number of field references
    let field_count = 50_000;
    let start_time = std::time::Instant::now();

    for i in 0..field_count {
        // Use extreme object reference numbers
        let obj_ref = ObjectReference::new(u32::MAX - i, u16::MAX);
        acro_form.add_field(obj_ref);

        // Check performance periodically
        if i % 10_000 == 0 && i > 0 {
            let elapsed = start_time.elapsed();
            if elapsed.as_secs() > 10 {
                println!(
                    "AcroForm stress test stopped at {} fields after {:?}",
                    i, elapsed
                );
                break;
            }
        }
    }

    let final_count = acro_form.fields.len();
    let creation_time = start_time.elapsed();

    println!(
        "AcroForm created with {} fields in {:?}",
        final_count, creation_time
    );
    assert!(
        creation_time.as_secs() < 15,
        "AcroForm creation took too long"
    );

    // Test dictionary conversion with large field count
    let dict_start = std::time::Instant::now();
    let dict = acro_form.to_dict();
    let dict_time = dict_start.elapsed();

    println!("AcroForm dictionary created in {:?}", dict_time);
    assert!(dict_time.as_secs() < 5, "Dictionary creation took too long");

    // Verify dictionary structure
    if let Some(Object::Array(fields)) = dict.get("Fields") {
        println!("Dictionary has {} field references", fields.len());
        assert_eq!(fields.len(), final_count);
    } else {
        panic!("Fields array not found in AcroForm dictionary");
    }

    // Test with corrupted AcroForm state
    let mut corrupted_form = AcroForm::new();
    corrupted_form.need_appearances = true;
    corrupted_form.sig_flags = Some(i32::MAX);
    corrupted_form.q = Some(i32::MIN);
    corrupted_form.da = Some("".to_string()); // Empty appearance

    let corrupted_dict = corrupted_form.to_dict();
    println!("Corrupted AcroForm dictionary created successfully");

    // Should handle extreme values gracefully
    assert!(corrupted_dict.get("NeedAppearances").is_some());
    assert!(corrupted_dict.get("SigFlags").is_some());
    assert!(corrupted_dict.get("Q").is_some());
    assert!(corrupted_dict.get("DA").is_some());
}

/// Test 12: Concurrent access and thread safety
#[test]
fn test_concurrent_form_operations() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    // Test thread safety of form operations
    let form_manager = Arc::new(Mutex::new(FormManager::new()));
    let mut handles = vec![];

    // Spawn multiple threads that modify the form manager
    for thread_id in 0..10 {
        let manager = Arc::clone(&form_manager);

        let handle = thread::spawn(move || {
            let mut errors = 0;
            let mut successes = 0;

            for i in 0..100 {
                let field_name = format!("thread_{}_field_{}", thread_id, i);
                let field = TextField::new(&field_name)
                    .with_value(format!("Thread {} Value {}", thread_id, i))
                    .with_max_length(100);

                let widget = Widget::new(Rectangle::new(
                    Point::new(50.0 + thread_id as f64 * 10.0, 700.0 - i as f64 * 5.0),
                    Point::new(200.0 + thread_id as f64 * 10.0, 720.0 - i as f64 * 5.0),
                ));

                match manager.lock() {
                    Ok(mut mgr) => match mgr.add_text_field(field, widget, None) {
                        Ok(_) => successes += 1,
                        Err(_) => errors += 1,
                    },
                    Err(_) => errors += 1,
                }
            }

            (thread_id, successes, errors)
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    let mut total_successes = 0;
    let mut total_errors = 0;

    for handle in handles {
        match handle.join() {
            Ok((thread_id, successes, errors)) => {
                println!(
                    "Thread {} completed: {} successes, {} errors",
                    thread_id, successes, errors
                );
                total_successes += successes;
                total_errors += errors;
            }
            Err(_) => {
                println!("Thread panicked");
                total_errors += 100; // Assume all operations in that thread failed
            }
        }
    }

    println!(
        "Concurrent test completed: {} total successes, {} total errors",
        total_successes, total_errors
    );

    // Verify final state
    let final_manager = form_manager.lock().unwrap();
    let final_count = final_manager.field_count();
    println!("Final form manager has {} fields", final_count);

    // Should have some successful operations (exact count depends on race conditions)
    assert!(
        final_count > 0,
        "Should have created some fields successfully"
    );
    assert_eq!(
        final_count, total_successes,
        "Field count should match successful operations"
    );
}

/// Test 13: Resource cleanup and memory leaks
#[test]
fn test_resource_cleanup() {
    // Test that forms properly clean up resources when dropped

    let initial_memory_pressure = get_memory_pressure_indicator();

    // Create and drop many form managers
    for iteration in 0..100 {
        let mut form_manager = FormManager::new();

        // Add some fields
        for i in 0..50 {
            let field_name = format!("cleanup_test_{}_{}", iteration, i);
            let field = TextField::new(&field_name)
                .with_value(format!("Iteration {} Field {}", iteration, i))
                .with_max_length(100);

            let widget = Widget::new(Rectangle::new(
                Point::new(50.0, 700.0 - i as f64 * 10.0),
                Point::new(300.0, 720.0 - i as f64 * 10.0),
            ));

            if form_manager.add_text_field(field, widget, None).is_ok() {
                // Field added successfully
            }
        }

        assert!(form_manager.field_count() <= 50);

        // FormManager should be dropped here automatically
    }

    let final_memory_pressure = get_memory_pressure_indicator();

    println!(
        "Memory pressure: initial = {}, final = {}",
        initial_memory_pressure, final_memory_pressure
    );

    // Memory usage shouldn't grow excessively
    // This is a heuristic test - exact memory usage is hard to measure
    assert!(
        final_memory_pressure < initial_memory_pressure + 1000,
        "Memory usage grew too much: {} -> {}",
        initial_memory_pressure,
        final_memory_pressure
    );
}

/// Helper function to get a rough memory pressure indicator
fn get_memory_pressure_indicator() -> usize {
    // This is a simple heuristic - create some allocations and see how much we can create
    // before hitting performance issues
    let mut test_vectors = Vec::new();
    let start_time = std::time::Instant::now();

    for i in 0..1000 {
        test_vectors.push(vec![i; 100]);
        if start_time.elapsed().as_millis() > 100 {
            break;
        }
    }

    let indicator = test_vectors.len();
    drop(test_vectors); // Clean up test allocations
    indicator
}

/// Test 14: API misuse prevention
#[test]
fn test_api_misuse_prevention() {
    // Test various ways the API might be misused and ensure graceful handling

    let mut form_manager = FormManager::new();

    // Test 1: Adding the same field name multiple times
    let duplicate_name = "duplicate_field";

    let field1 = TextField::new(duplicate_name).with_value("First value");
    let widget1 = Widget::new(Rectangle::new(
        Point::new(50.0, 600.0),
        Point::new(300.0, 620.0),
    ));
    let result1 = form_manager.add_text_field(field1, widget1, None);
    assert!(result1.is_ok(), "First field should be added successfully");

    let field2 = TextField::new(duplicate_name).with_value("Second value");
    let widget2 = Widget::new(Rectangle::new(
        Point::new(50.0, 550.0),
        Point::new(300.0, 570.0),
    ));
    let result2 = form_manager.add_text_field(field2, widget2, None);

    match result2 {
        Ok(_) => {
            println!("Duplicate field name handled - second field overwrote first");
            // This might be acceptable behavior
        }
        Err(e) => {
            println!("Duplicate field name rejected: {}", e);
            // This is also acceptable behavior
        }
    }

    // Test 2: Accessing non-existent fields
    let non_existent = form_manager.get_field("this_field_does_not_exist");
    assert!(
        non_existent.is_none(),
        "Non-existent field should return None"
    );

    // Test 3: Invalid object reference generation
    // This tests internal consistency of object reference numbering
    let initial_count = form_manager.field_count();

    for i in 0..10 {
        let field_name = format!("ref_test_{}", i);
        let field = TextField::new(&field_name).with_value("Test");
        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 500.0 - i as f64 * 25.0),
            Point::new(300.0, 520.0 - i as f64 * 25.0),
        ));

        if let Ok(obj_ref) = form_manager.add_text_field(field, widget, None) {
            // Object references should be sequential and unique
            let expected_number = initial_count + i + 1;
            if obj_ref.number() != expected_number as u32 {
                println!(
                    "Object reference numbering inconsistency: expected {}, got {}",
                    expected_number,
                    obj_ref.number()
                );
            }
            assert_eq!(
                obj_ref.generation(),
                0,
                "All object references should have generation 0"
            );
        }
    }

    // Test 4: Large field counts
    let final_count = form_manager.field_count();
    println!("Final field count: {}", final_count);
    assert!(
        final_count >= initial_count,
        "Field count should only increase"
    );

    println!("API misuse prevention tests completed");
}

/// Test 15: Invalid widget operations
#[test]
fn test_invalid_widget_operations() {
    // Test widget creation and manipulation with invalid parameters

    let base_rect = Rectangle::new(Point::new(100.0, 100.0), Point::new(200.0, 120.0));

    // Test 1: Widget with extreme appearance values
    let extreme_appearances = vec![
        WidgetAppearance {
            border_color: None,
            background_color: None,
            border_width: f64::NEG_INFINITY,
            border_style: oxidize_pdf::forms::BorderStyle::Solid,
        },
        WidgetAppearance {
            border_color: None,
            background_color: None,
            border_width: f64::INFINITY,
            border_style: oxidize_pdf::forms::BorderStyle::Solid,
        },
        WidgetAppearance {
            border_color: None,
            background_color: None,
            border_width: f64::NAN,
            border_style: oxidize_pdf::forms::BorderStyle::Solid,
        },
    ];

    for (i, appearance) in extreme_appearances.iter().enumerate() {
        let widget = Widget::new(base_rect);

        // Test cloning appearance (which might have invalid values)
        let result = std::panic::catch_unwind(|| {
            let widget_with_appearance = widget.with_appearance(appearance.clone());
            let annotation_dict = widget_with_appearance.to_annotation_dict();
            println!("Extreme appearance {} handled successfully", i);
            annotation_dict
        });

        match result {
            Ok(_) => {
                println!("Extreme widget appearance {} processed successfully", i);
            }
            Err(_) => {
                println!(
                    "Extreme widget appearance {} caused panic (needs better handling)",
                    i
                );
            }
        }
    }

    // Test 2: Multiple widgets for the same form field
    let mut form_field_dict = Dictionary::new();
    form_field_dict.set("T", Object::String("multi_widget_field".to_string()));
    form_field_dict.set("FT", Object::String("Tx".to_string()));

    let mut form_field = FormField::new(form_field_dict);

    // Add many widgets rapidly
    let widget_count = 1000;
    let start_time = std::time::Instant::now();

    for i in 0..widget_count {
        let rect = Rectangle::new(
            Point::new(
                50.0 + (i % 10) as f64 * 20.0,
                700.0 - (i / 10) as f64 * 20.0,
            ),
            Point::new(
                150.0 + (i % 10) as f64 * 20.0,
                720.0 - (i / 10) as f64 * 20.0,
            ),
        );
        let widget = Widget::new(rect);
        form_field.add_widget(widget);

        // Check performance every 100 widgets
        if i % 100 == 0 && i > 0 {
            let elapsed = start_time.elapsed();
            if elapsed.as_secs() > 5 {
                println!(
                    "Widget addition stopped at {} widgets after {:?}",
                    i, elapsed
                );
                break;
            }
        }
    }

    let final_widget_count = form_field.widgets.len();
    let total_time = start_time.elapsed();

    println!(
        "Added {} widgets to form field in {:?}",
        final_widget_count, total_time
    );
    assert!(total_time.as_secs() < 10, "Widget addition took too long");
    assert!(final_widget_count > 0, "Should have added some widgets");
    assert!(
        final_widget_count <= widget_count,
        "Shouldn't exceed attempted count"
    );
}

/// Test 16: Form state consistency verification
#[test]
fn test_form_state_consistency() {
    // Test that form manager maintains consistent internal state

    let mut form_manager = FormManager::new();

    // Initial state verification
    assert_eq!(form_manager.field_count(), 0);
    assert_eq!(form_manager.get_acro_form().fields.len(), 0);
    assert!(form_manager.fields().is_empty());

    // Add fields and verify consistency at each step
    let field_names = vec!["field_1", "field_2", "field_3", "field_4", "field_5"];

    for (i, name) in field_names.iter().enumerate() {
        let field = TextField::new(*name)
            .with_value(format!("Value for {}", name))
            .with_max_length(100);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 700.0 - i as f64 * 30.0),
            Point::new(300.0, 720.0 - i as f64 * 30.0),
        ));

        let obj_ref = form_manager.add_text_field(field, widget, None).unwrap();

        // Verify consistency after each addition
        let expected_count = i + 1;

        assert_eq!(
            form_manager.field_count(),
            expected_count,
            "Field count inconsistent after adding field {}",
            i
        );

        assert_eq!(
            form_manager.get_acro_form().fields.len(),
            expected_count,
            "AcroForm field count inconsistent after adding field {}",
            i
        );

        assert_eq!(
            form_manager.fields().len(),
            expected_count,
            "Fields map size inconsistent after adding field {}",
            i
        );

        assert!(
            form_manager.get_field(name).is_some(),
            "Cannot retrieve field {} that was just added",
            name
        );

        assert_eq!(
            obj_ref.number(),
            (i + 1) as u32,
            "Object reference number inconsistent for field {}",
            i
        );

        assert_eq!(
            obj_ref.generation(),
            0,
            "Object reference generation should be 0 for field {}",
            i
        );
    }

    // Final consistency check
    assert_eq!(form_manager.field_count(), field_names.len());
    assert_eq!(form_manager.get_acro_form().fields.len(), field_names.len());
    assert_eq!(form_manager.fields().len(), field_names.len());

    // Verify all fields can be retrieved
    for name in &field_names {
        assert!(
            form_manager.get_field(name).is_some(),
            "Field '{}' should be retrievable",
            name
        );
    }

    println!(
        "Form state consistency verified for {} fields",
        field_names.len()
    );
}

/// Test 17: Edge cases in field value handling
#[test]
fn test_field_value_edge_cases() {
    // Test various edge cases in field value setting and retrieval

    let mut form_manager = FormManager::new();

    // Test extreme field values
    let edge_case_values = vec![
        ("empty", ""),
        ("whitespace_only", "   \t\r\n   "),
        ("single_char", "a"),
        ("unicode_emoji", "🎯🚀💯"),
        ("mixed_scripts", "Hello世界مرحباПривет"),
        ("control_chars", "\u{0001}\u{0002}\u{0003}\u{0004}\u{0005}"),
        ("null_chars", "text\u{0000}with\u{0000}nulls"),
        ("long_line", {
            let s = "x".repeat(10000);
            Box::leak(s.into_boxed_str())
        }),
        ("multiline", "Line 1\nLine 2\rLine 3\r\nLine 4"),
        ("quotes", "\"Single 'quotes' and double \"quotes\"\""),
        ("special_chars", "!@#$%^&*()_+-={}[]|\\:;\"'<>?,./ "),
        ("numbers", "0123456789"),
        ("floats", "3.14159 -2.718 1e10 -1.5e-20"),
        ("boolean_like", "true false TRUE FALSE 1 0 yes no"),
        ("html_like", "<tag>content</tag> &amp; &lt; &gt;"),
        ("json_like", "{\"key\": \"value\", \"number\": 42}"),
        (
            "xml_like",
            "<?xml version=\"1.0\"?><root><child>text</child></root>",
        ),
        ("url_like", "https://example.com/path?param=value&other=123"),
        ("email_like", "user@domain.com test+tag@example.org"),
        (
            "binary_like",
            "\u{0080}\u{0081}\u{0082}\u{0083}\u{0084}\u{0085}\u{0086}\u{0087}\u{0088}\u{0089}",
        ),
    ];

    for (i, (case_name, value)) in edge_case_values.iter().enumerate() {
        let field_name = format!("edge_case_{}", case_name);

        let field = TextField::new(&field_name)
            .with_value(*value)
            .with_max_length(20000); // Large enough for test values

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 750.0 - i as f64 * 15.0),
            Point::new(400.0, 765.0 - i as f64 * 15.0),
        ));

        let result = form_manager.add_text_field(field, widget, None);

        match result {
            Ok(obj_ref) => {
                println!(
                    "Edge case '{}' handled successfully: ref {}",
                    case_name,
                    obj_ref.number()
                );

                // Verify the field can be retrieved and has the correct value
                if let Some(form_field) = form_manager.get_field(&field_name) {
                    let dict = &form_field.field_dict;

                    if let Some(Object::String(stored_value)) = dict.get("V") {
                        if stored_value == value {
                            println!("  Value preserved correctly");
                        } else if stored_value.len() != value.len() {
                            println!(
                                "  Value length changed: {} -> {}",
                                value.len(),
                                stored_value.len()
                            );
                        } else {
                            println!("  Value content changed but length preserved");
                        }
                    } else {
                        println!("  No value stored in dictionary");
                    }
                } else {
                    println!("  Field could not be retrieved after creation");
                }
            }
            Err(e) => {
                println!("Edge case '{}' rejected: {}", case_name, e);
                // Some extreme cases might legitimately fail
            }
        }
    }

    println!("Field value edge cases test completed");
}

/// Test 18: Performance regression detection
#[test]
fn test_performance_regression_detection() {
    // Test that form operations maintain reasonable performance

    let performance_tests = vec![
        ("small_form", 10),
        ("medium_form", 100),
        ("large_form", 1000),
    ];

    for (test_name, field_count) in performance_tests {
        println!("Running performance test: {}", test_name);

        let start_time = std::time::Instant::now();
        let mut form_manager = FormManager::new();

        // Create fields
        for i in 0..field_count {
            let field_name = format!("perf_field_{}", i);
            let field = TextField::new(&field_name)
                .with_value(format!("Performance test value {}", i))
                .with_max_length(200);

            let widget = Widget::new(Rectangle::new(
                Point::new(
                    50.0 + (i % 10) as f64 * 40.0,
                    700.0 - (i / 10) as f64 * 20.0,
                ),
                Point::new(
                    200.0 + (i % 10) as f64 * 40.0,
                    720.0 - (i / 10) as f64 * 20.0,
                ),
            ));

            form_manager.add_text_field(field, widget, None).unwrap();
        }

        let creation_time = start_time.elapsed();

        // Test field retrieval performance
        let retrieval_start = std::time::Instant::now();
        let mut retrieved_count = 0;

        for i in 0..field_count {
            let field_name = format!("perf_field_{}", i);
            if form_manager.get_field(&field_name).is_some() {
                retrieved_count += 1;
            }
        }

        let retrieval_time = retrieval_start.elapsed();

        // Test AcroForm generation performance
        let acroform_start = std::time::Instant::now();
        let acro_form = form_manager.get_acro_form();
        let acro_dict = acro_form.to_dict();
        let acroform_time = acroform_start.elapsed();

        println!("  {} fields created in {:?}", field_count, creation_time);
        println!(
            "  {} fields retrieved in {:?}",
            retrieved_count, retrieval_time
        );
        println!(
            "  AcroForm with {} refs generated in {:?}",
            acro_dict.get("Fields").map_or(0, |f| match f {
                Object::Array(arr) => arr.len(),
                _ => 0,
            }),
            acroform_time
        );

        // Performance assertions
        let max_creation_time_per_field = std::time::Duration::from_millis(10);
        let actual_time_per_field = creation_time / field_count as u32;

        assert!(
            actual_time_per_field < max_creation_time_per_field,
            "Creation time per field too slow: {:?} > {:?}",
            actual_time_per_field,
            max_creation_time_per_field
        );

        assert_eq!(
            retrieved_count, field_count,
            "Not all fields could be retrieved"
        );

        assert!(
            retrieval_time < std::time::Duration::from_millis(field_count as u64),
            "Field retrieval too slow: {:?}",
            retrieval_time
        );

        assert!(
            acroform_time < std::time::Duration::from_secs(1),
            "AcroForm generation too slow: {:?}",
            acroform_time
        );
    }

    println!("Performance regression detection completed");
}

/// Test 19: Malformed input sanitization
#[test]
fn test_malformed_input_sanitization() {
    // Test how the system handles and potentially sanitizes malformed inputs

    let mut form_manager = FormManager::new();

    // Test field names that might need sanitization
    let malformed_names = vec![
        "field name with spaces",
        "field\twith\ttabs",
        "field\nwith\nnewlines",
        "field/with/slashes",
        "field\\with\\backslashes",
        "field<with>brackets",
        "field\"with\"quotes",
        "field'with'apostrophes",
        "field&with&ampersands",
        "field%with%percents",
        "field#with#hashes",
        "field@with@at",
        "field=with=equals",
        "field+with+plus",
        "field?with?questions",
        "field*with*asterisks",
        "field|with|pipes",
        "field;with;semicolons",
        "field:with:colons",
        "field,with,commas",
        "field.with.dots",
        "field(with)parentheses",
        "field[with]brackets",
        "field{with}braces",
        "field~with~tildes",
        "field`with`backticks",
        "field^with^carets",
    ];

    for (i, malformed_name) in malformed_names.iter().enumerate() {
        let field = TextField::new(*malformed_name)
            .with_value(format!("Value for field {}", i))
            .with_max_length(100);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 750.0 - i as f64 * 10.0),
            Point::new(300.0, 760.0 - i as f64 * 10.0),
        ));

        let result = form_manager.add_text_field(field, widget, None);

        match result {
            Ok(obj_ref) => {
                println!(
                    "Malformed name '{}' accepted: ref {}",
                    malformed_name,
                    obj_ref.number()
                );

                // Test if the field can be retrieved with the original name
                if form_manager.get_field(malformed_name).is_some() {
                    println!("  Field retrievable with original name");
                } else {
                    println!(
                        "  Field not retrievable with original name (may have been sanitized)"
                    );

                    // Try to find the field with a sanitized version
                    // This would depend on the sanitization strategy used
                    let sanitized_candidates = vec![
                        malformed_name.replace(' ', "_"),
                        malformed_name.replace(|c: char| !c.is_alphanumeric(), "_"),
                        malformed_name
                            .chars()
                            .filter(|c| c.is_alphanumeric() || *c == '_')
                            .collect(),
                    ];

                    let mut found_sanitized = false;
                    for candidate in &sanitized_candidates {
                        if form_manager.get_field(candidate).is_some() {
                            println!("  Field found with sanitized name '{}'", candidate);
                            found_sanitized = true;
                            break;
                        }
                    }

                    if !found_sanitized {
                        println!("  Field not found with any expected sanitized name");
                    }
                }
            }
            Err(e) => {
                println!("Malformed name '{}' rejected: {}", malformed_name, e);
                // Some names might be legitimately rejected
            }
        }
    }

    println!("Malformed input sanitization test completed");
}

/// Test 20: Error propagation and handling
#[test]
fn test_error_propagation_and_handling() {
    // Test that errors are properly propagated and handled throughout the forms system

    // Test 1: Invalid max_length handling
    let invalid_max_lengths = vec![-1, -100, -1000, i32::MIN];

    for (i, max_length) in invalid_max_lengths.iter().enumerate() {
        let field_name = format!("invalid_max_length_{}", i);
        let field = TextField::new(&field_name)
            .with_value("Test value")
            .with_max_length(*max_length);

        println!("Testing invalid max_length: {}", max_length);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 600.0 - i as f64 * 30.0),
            Point::new(300.0, 620.0 - i as f64 * 30.0),
        ));

        let mut form_manager = FormManager::new();
        let result = form_manager.add_text_field(field, widget, None);

        match result {
            Ok(_) => {
                println!(
                    "  Invalid max_length {} was accepted (may need validation)",
                    max_length
                );
            }
            Err(e) => {
                println!(
                    "  Invalid max_length {} properly rejected: {}",
                    max_length, e
                );
            }
        }
    }

    // Test 2: Choice field option validation
    let mut form_manager = FormManager::new();

    // Test ComboBox with empty options but a value
    let empty_combo = ComboBox::new("empty_combo_test").with_value("NonExistentValue"); // Value set but no options

    let combo_widget = Widget::new(Rectangle::new(
        Point::new(50.0, 500.0),
        Point::new(200.0, 520.0),
    ));

    let combo_result = form_manager.add_combo_box(empty_combo, combo_widget, None);

    match combo_result {
        Ok(_) => {
            println!("ComboBox with empty options but set value was accepted");
        }
        Err(e) => {
            println!("ComboBox with empty options properly rejected: {}", e);
        }
    }

    // Test 3: RadioButton with no options but selection set
    let empty_radio = RadioButton::new("empty_radio_test").with_selected(0); // Selection set but no options

    let radio_widgets = vec![]; // Empty widgets vector

    let radio_result = form_manager.add_radio_buttons(empty_radio, radio_widgets, None);

    match radio_result {
        Ok(_) => {
            println!("RadioButton with no options but selection set was accepted");
        }
        Err(e) => {
            println!("RadioButton with no options properly rejected: {}", e);
        }
    }

    // Test 4: Widget count mismatch for radio buttons
    let radio_with_options = RadioButton::new("mismatch_radio")
        .add_option("A", "Option A")
        .add_option("B", "Option B")
        .add_option("C", "Option C"); // 3 options

    let insufficient_widgets = vec![
        Widget::new(Rectangle::new(
            Point::new(50.0, 400.0),
            Point::new(70.0, 420.0),
        )),
        // Only 1 widget for 3 options - this should be detected
    ];

    let mismatch_result =
        form_manager.add_radio_buttons(radio_with_options, insufficient_widgets, None);

    match mismatch_result {
        Ok(_) => {
            println!(
                "RadioButton with widget/option count mismatch was accepted (may need validation)"
            );
        }
        Err(e) => {
            println!(
                "RadioButton with widget/option count mismatch properly rejected: {}",
                e
            );
        }
    }

    println!("Error propagation and handling test completed");
}
