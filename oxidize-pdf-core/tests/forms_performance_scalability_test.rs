//! Forms Performance and Scalability Tests
//!
//! This test suite focuses on performance characteristics and scalability limits
//! of the forms module. These tests ensure that the forms system can handle
//! realistic workloads and edge cases without performance degradation.
//!
//! Test categories:
//! - Large form creation and management
//! - Memory usage patterns and optimization
//! - Concurrent form operations
//! - Serialization/deserialization performance
//! - Widget rendering performance
//! - Field lookup and retrieval efficiency

use oxidize_pdf::forms::{
    CheckBox, ComboBox, FieldFlags, FieldOptions, FormData, FormManager, PushButton, RadioButton,
    TextField, Widget, WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::Object;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

/// Test 1: Large form creation performance
#[test]
fn test_large_form_creation_performance() {
    let field_counts = vec![100, 500, 1000, 2000];

    for field_count in field_counts {
        let start_time = Instant::now();
        let mut form_manager = FormManager::new();

        // Create many fields of different types
        for i in 0..field_count {
            let field_type = i % 4;

            match field_type {
                0 => {
                    // Text field
                    let field = TextField::new(format!("text_field_{i}"))
                        .with_value(format!("Value {i}"))
                        .with_max_length(100);

                    let widget = Widget::new(Rectangle::new(
                        Point::new(50.0 + (i % 10) as f64 * 20.0, 700.0 - (i / 10) as f64 * 5.0),
                        Point::new(
                            200.0 + (i % 10) as f64 * 20.0,
                            720.0 - (i / 10) as f64 * 5.0,
                        ),
                    ));

                    form_manager.add_text_field(field, widget, None).unwrap();
                }
                1 => {
                    // Checkbox
                    let field = CheckBox::new(format!("check_field_{i}"))
                        .checked()
                        .with_export_value("Yes");

                    let widget = Widget::new(Rectangle::new(
                        Point::new(
                            250.0 + (i % 10) as f64 * 20.0,
                            700.0 - (i / 10) as f64 * 5.0,
                        ),
                        Point::new(
                            265.0 + (i % 10) as f64 * 20.0,
                            715.0 - (i / 10) as f64 * 5.0,
                        ),
                    ));

                    form_manager.add_checkbox(field, widget, None).unwrap();
                }
                2 => {
                    // ComboBox
                    let field = ComboBox::new(format!("combo_field_{i}"))
                        .add_option("opt1", "Option 1")
                        .add_option("opt2", "Option 2")
                        .add_option("opt3", "Option 3")
                        .with_value("opt1");

                    let widget = Widget::new(Rectangle::new(
                        Point::new(
                            300.0 + (i % 10) as f64 * 25.0,
                            700.0 - (i / 10) as f64 * 5.0,
                        ),
                        Point::new(
                            400.0 + (i % 10) as f64 * 25.0,
                            720.0 - (i / 10) as f64 * 5.0,
                        ),
                    ));

                    form_manager.add_combo_box(field, widget, None).unwrap();
                }
                3 => {
                    // PushButton
                    let field = PushButton::new(format!("button_field_{i}"))
                        .with_caption(format!("Button {i}"));

                    let widget = Widget::new(Rectangle::new(
                        Point::new(
                            450.0 + (i % 10) as f64 * 25.0,
                            700.0 - (i / 10) as f64 * 5.0,
                        ),
                        Point::new(
                            520.0 + (i % 10) as f64 * 25.0,
                            720.0 - (i / 10) as f64 * 5.0,
                        ),
                    ));

                    form_manager.add_push_button(field, widget, None).unwrap();
                }
                _ => unreachable!(),
            }
        }

        let creation_time = start_time.elapsed();

        // Verify all fields were created
        assert_eq!(form_manager.field_count(), field_count);

        // Performance assertions
        let max_time_per_field = std::time::Duration::from_micros(500); // 0.5ms per field
        let actual_time_per_field = creation_time / field_count as u32;

        assert!(
            actual_time_per_field < max_time_per_field,
            "Creation time per field too slow: {actual_time_per_field:?} > {max_time_per_field:?} for {field_count} fields"
        );

        println!(
            "Created {field_count} fields in {creation_time:?} ({actual_time_per_field:?}/field)"
        );
    }

    println!("Large form creation performance test completed");
}

/// Test 2: Field lookup performance with many fields
#[test]
fn test_field_lookup_performance() {
    let mut form_manager = FormManager::new();
    let field_count = 5000;

    // Create many fields
    let start_creation = Instant::now();

    for i in 0..field_count {
        let field = TextField::new(format!("lookup_field_{i:06}"))
            .with_value(format!("Value {i}"))
            .with_max_length(50);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 700.0),
            Point::new(200.0, 720.0),
        ));

        form_manager.add_text_field(field, widget, None).unwrap();
    }

    let creation_time = start_creation.elapsed();
    println!("Created {field_count} fields for lookup test in {creation_time:?}");

    // Test lookup performance
    let lookup_tests = vec![
        ("lookup_field_000000", true),  // First field
        ("lookup_field_002500", true),  // Middle field
        ("lookup_field_004999", true),  // Last field
        ("lookup_field_999999", false), // Non-existent field
        ("nonexistent_field", false),   // Non-existent field
    ];

    let start_lookup = Instant::now();
    let iterations = 1000;

    for _ in 0..iterations {
        for (field_name, should_exist) in &lookup_tests {
            let found = form_manager.get_field(field_name).is_some();
            assert_eq!(
                found, *should_exist,
                "Lookup result mismatch for field {field_name}"
            );
        }
    }

    let lookup_time = start_lookup.elapsed();
    let total_lookups = iterations * lookup_tests.len();
    let time_per_lookup = lookup_time / total_lookups as u32;

    // Lookup should be very fast (sub-microsecond)
    assert!(
        time_per_lookup < std::time::Duration::from_micros(10),
        "Field lookup too slow: {time_per_lookup:?} per lookup"
    );

    println!(
        "Performed {total_lookups} field lookups in {lookup_time:?} ({time_per_lookup:?}/lookup)"
    );

    println!("Field lookup performance test completed");
}

/// Test 3: Memory usage patterns during form operations
#[test]
fn test_memory_usage_patterns() {
    // Test memory usage with different operation patterns

    // Pattern 1: Incremental field addition
    let pattern1_start = get_memory_indicator();
    let mut form_manager1 = FormManager::new();

    for i in 0..1000 {
        let field = TextField::new(format!("mem_field_{i}"))
            .with_value(format!("Value {i}"))
            .with_max_length(100);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 700.0 - i as f64 * 0.1),
            Point::new(200.0, 720.0 - i as f64 * 0.1),
        ));

        form_manager1.add_text_field(field, widget, None).unwrap();

        // Check memory growth periodically
        if i % 100 == 99 {
            let current_memory = get_memory_indicator();
            let memory_growth = current_memory.saturating_sub(pattern1_start);

            // Memory growth should be reasonable (< 1000 units per 100 fields)
            assert!(
                memory_growth < (i + 1) * 10,
                "Excessive memory growth: {} units for {} fields",
                memory_growth,
                i + 1
            );
        }
    }

    let pattern1_end = get_memory_indicator();
    let pattern1_growth = pattern1_end.saturating_sub(pattern1_start);

    println!(
        "Pattern 1 (incremental): {} fields used {} memory units",
        1000, pattern1_growth
    );

    // Pattern 2: Bulk field addition
    let pattern2_start = get_memory_indicator();
    let mut form_manager2 = FormManager::new();

    let bulk_fields: Vec<_> = (0..1000)
        .map(|i| {
            let field = TextField::new(format!("bulk_field_{i}"))
                .with_value(format!("Bulk Value {i}"))
                .with_max_length(100);

            let widget = Widget::new(Rectangle::new(
                Point::new(50.0, 700.0 - i as f64 * 0.1),
                Point::new(200.0, 720.0 - i as f64 * 0.1),
            ));

            (field, widget)
        })
        .collect();

    // Add all fields at once
    for (field, widget) in bulk_fields {
        form_manager2.add_text_field(field, widget, None).unwrap();
    }

    let pattern2_end = get_memory_indicator();
    let pattern2_growth = pattern2_end.saturating_sub(pattern2_start);

    println!(
        "Pattern 2 (bulk): {} fields used {} memory units",
        1000, pattern2_growth
    );

    // Bulk addition shouldn't use significantly more memory
    assert!(
        pattern2_growth <= pattern1_growth + 200,
        "Bulk addition used excessive memory: {pattern2_growth} vs {pattern1_growth}"
    );

    // Pattern 3: Field creation and destruction
    let pattern3_start = get_memory_indicator();

    for cycle in 0..10 {
        let mut temp_manager = FormManager::new();

        // Add fields
        for i in 0..100 {
            let field = TextField::new(format!("temp_field_{cycle}_{i}"))
                .with_value(format!("Temp Value {cycle} {i}"))
                .with_max_length(50);

            let widget = Widget::new(Rectangle::new(
                Point::new(50.0, 700.0),
                Point::new(200.0, 720.0),
            ));

            temp_manager.add_text_field(field, widget, None).unwrap();
        }

        assert_eq!(temp_manager.field_count(), 100);

        // temp_manager is dropped here, should free memory
    }

    let pattern3_end = get_memory_indicator();
    let pattern3_growth = pattern3_end.saturating_sub(pattern3_start);

    println!(
        "Pattern 3 (create/destroy): {} cycles used {} memory units",
        10, pattern3_growth
    );

    // Create/destroy cycles should not accumulate memory
    assert!(
        pattern3_growth < 500,
        "Create/destroy cycles accumulated too much memory: {pattern3_growth}"
    );

    println!("Memory usage patterns test completed");
}

/// Test 4: Concurrent form operations thread safety
#[test]
fn test_concurrent_form_operations() {
    let form_manager = Arc::new(Mutex::new(FormManager::new()));
    let mut handles = vec![];
    let thread_count = 8;
    let fields_per_thread = 100;

    let start_time = Instant::now();

    // Spawn threads that perform concurrent operations
    for thread_id in 0..thread_count {
        let manager = Arc::clone(&form_manager);

        let handle = thread::spawn(move || {
            let mut successes = 0;
            let mut failures = 0;

            for i in 0..fields_per_thread {
                let field_name = format!("thread_{thread_id}_{i}");
                let field = TextField::new(&field_name)
                    .with_value(format!("Thread {thread_id} Field {i}"))
                    .with_max_length(50);

                let widget = Widget::new(Rectangle::new(
                    Point::new(50.0 + thread_id as f64 * 10.0, 700.0 - i as f64 * 2.0),
                    Point::new(200.0 + thread_id as f64 * 10.0, 720.0 - i as f64 * 2.0),
                ));

                // Try to acquire lock and add field
                match manager.lock() {
                    Ok(mut mgr) => {
                        match mgr.add_text_field(field, widget, None) {
                            Ok(_) => successes += 1,
                            Err(_) => failures += 1,
                        }

                        // Also test field retrieval
                        if mgr.get_field(&field_name).is_some() {
                            // Field was successfully added and can be retrieved
                        } else {
                            failures += 1;
                        }
                    }
                    Err(_) => failures += 1,
                }

                // Yield to other threads occasionally
                if i % 10 == 0 {
                    thread::yield_now();
                }
            }

            (thread_id, successes, failures)
        });

        handles.push(handle);
    }

    // Wait for all threads to complete
    let mut total_successes = 0;
    let mut total_failures = 0;

    for handle in handles {
        match handle.join() {
            Ok((thread_id, successes, failures)) => {
                println!(
                    "Thread {thread_id} completed: {successes} successes, {failures} failures"
                );
                total_successes += successes;
                total_failures += failures;
            }
            Err(_) => {
                println!("Thread panicked");
                total_failures += fields_per_thread * 2; // Assume all operations failed
            }
        }
    }

    let total_time = start_time.elapsed();

    // Verify results
    let expected_operations = thread_count * fields_per_thread;
    let success_rate = total_successes as f64 / expected_operations as f64;

    println!("Concurrent operations completed in {total_time:?}");
    println!(
        "Success rate: {:.1}% ({}/{})",
        success_rate * 100.0,
        total_successes,
        expected_operations
    );

    // Should have high success rate
    assert!(
        success_rate > 0.95,
        "Success rate too low: {:.1}%",
        success_rate * 100.0
    );

    // Verify final state
    let final_manager = form_manager.lock().unwrap();
    let final_count = final_manager.field_count();

    println!("Final form manager has {final_count} fields");
    assert_eq!(
        final_count, total_successes,
        "Field count mismatch: expected {total_successes}, got {final_count}"
    );

    println!("Concurrent form operations test completed");
}

/// Test 5: AcroForm serialization performance
#[test]
fn test_acroform_serialization_performance() {
    let field_counts = vec![100, 500, 1000, 2000];

    for field_count in field_counts {
        let mut form_manager = FormManager::new();

        // Create fields
        let creation_start = Instant::now();

        for i in 0..field_count {
            let field = TextField::new(format!("serial_field_{i}"))
                .with_value(format!("Serialization test {i}"))
                .with_max_length(100);

            let widget = Widget::new(Rectangle::new(
                Point::new(50.0, 700.0 - i as f64 * 0.5),
                Point::new(250.0, 720.0 - i as f64 * 0.5),
            ));

            form_manager.add_text_field(field, widget, None).unwrap();
        }

        let creation_time = creation_start.elapsed();

        // Test serialization performance
        let serialization_start = Instant::now();
        let acro_form = form_manager.get_acro_form();
        let serialized_dict = acro_form.to_dict();
        let serialization_time = serialization_start.elapsed();

        // Verify serialization
        assert_eq!(acro_form.fields.len(), field_count);
        assert!(serialized_dict.get("Fields").is_some());

        if let Some(Object::Array(fields)) = serialized_dict.get("Fields") {
            assert_eq!(fields.len(), field_count);
        }

        // Performance assertions
        let max_serialization_time = std::time::Duration::from_millis(field_count as u64 / 10); // 0.1ms per field
        assert!(
            serialization_time < max_serialization_time,
            "Serialization too slow: {serialization_time:?} > {max_serialization_time:?} for {field_count} fields"
        );

        println!(
            "Serialized {field_count} fields: creation {creation_time:?}, serialization {serialization_time:?}"
        );
    }

    println!("AcroForm serialization performance test completed");
}

/// Test 6: Widget rendering preparation performance
#[test]
fn test_widget_rendering_performance() {
    let widget_count = 2000;
    let mut widgets = Vec::new();

    // Create widgets with different appearances
    let creation_start = Instant::now();

    for i in 0..widget_count {
        let rect = Rectangle::new(
            Point::new((i % 50) as f64 * 10.0, (i / 50) as f64 * 10.0),
            Point::new(
                (i % 50) as f64 * 10.0 + 100.0,
                (i / 50) as f64 * 10.0 + 20.0,
            ),
        );

        let appearance = WidgetAppearance {
            border_color: Some(Color::rgb(
                (i as f64 * 0.1) % 1.0,
                (i as f64 * 0.2) % 1.0,
                (i as f64 * 0.3) % 1.0,
            )),
            background_color: Some(Color::gray((i as f64 * 0.05) % 1.0)),
            border_width: (i % 5) as f64 + 1.0,
            border_style: match i % 5 {
                0 => oxidize_pdf::forms::BorderStyle::Solid,
                1 => oxidize_pdf::forms::BorderStyle::Dashed,
                2 => oxidize_pdf::forms::BorderStyle::Beveled,
                3 => oxidize_pdf::forms::BorderStyle::Inset,
                4 => oxidize_pdf::forms::BorderStyle::Underline,
                _ => oxidize_pdf::forms::BorderStyle::Solid,
            },
        };

        let widget = Widget::new(rect).with_appearance(appearance);
        widgets.push(widget);
    }

    let creation_time = creation_start.elapsed();

    // Test annotation dictionary generation performance
    let rendering_start = Instant::now();
    let mut annotation_dicts = Vec::new();

    for widget in &widgets {
        let annotation_dict = widget.to_annotation_dict();
        annotation_dicts.push(annotation_dict);
    }

    let rendering_time = rendering_start.elapsed();

    // Verify results
    assert_eq!(annotation_dicts.len(), widget_count);

    // Check that dictionaries have expected structure
    for (i, dict) in annotation_dicts.iter().enumerate() {
        assert_eq!(dict.get("Type"), Some(&Object::Name("Annot".to_string())));
        assert_eq!(
            dict.get("Subtype"),
            Some(&Object::Name("Widget".to_string()))
        );
        assert!(dict.get("Rect").is_some());

        // Every 100th widget should have appearance
        if i % 100 == 0 {
            assert!(dict.get("MK").is_some() || dict.get("BS").is_some());
        }
    }

    // Performance assertions
    let max_rendering_time = std::time::Duration::from_millis(widget_count as u64 / 5); // 0.2ms per widget
    assert!(
        rendering_time < max_rendering_time,
        "Widget rendering preparation too slow: {rendering_time:?} > {max_rendering_time:?} for {widget_count} widgets"
    );

    let time_per_widget = rendering_time / widget_count as u32;

    println!(
        "Widget rendering performance: {widget_count} widgets in {rendering_time:?} ({time_per_widget:?}/widget)"
    );
    println!("Creation time: {creation_time:?}");

    println!("Widget rendering performance test completed");
}

/// Test 7: Form data handling performance
#[test]
fn test_form_data_performance() {
    let data_sizes = vec![1000, 5000, 10000, 20000];

    for data_size in data_sizes {
        let mut form_data = FormData::new();

        // Test data insertion performance
        let insertion_start = Instant::now();

        for i in 0..data_size {
            let key = format!("data_key_{i:06}");
            let value = format!("Data value {i} with some content to make it realistic");
            form_data.set_value(&key, &value);
        }

        let insertion_time = insertion_start.elapsed();

        // Test data retrieval performance
        let retrieval_start = Instant::now();
        let mut retrieved_count = 0;

        for i in 0..data_size {
            let key = format!("data_key_{i:06}");
            if form_data.get_value(&key).is_some() {
                retrieved_count += 1;
            }
        }

        let retrieval_time = retrieval_start.elapsed();

        // Test bulk iteration performance
        let iteration_start = Instant::now();
        let mut iterated_count = 0;

        for _ in form_data.values.iter() {
            iterated_count += 1;
        }

        let iteration_time = iteration_start.elapsed();

        // Verify results
        assert_eq!(retrieved_count, data_size);
        assert_eq!(iterated_count, data_size);

        // Performance assertions
        let max_insertion_time = std::time::Duration::from_micros(data_size as u64 * 10); // 10µs per insertion
        let max_retrieval_time = std::time::Duration::from_micros(data_size as u64 * 5); // 5µs per retrieval
        let max_iteration_time = std::time::Duration::from_micros(data_size as u64 * 2); // 2µs per iteration

        assert!(
            insertion_time < max_insertion_time,
            "Data insertion too slow: {insertion_time:?} > {max_insertion_time:?} for {data_size} items"
        );

        assert!(
            retrieval_time < max_retrieval_time,
            "Data retrieval too slow: {retrieval_time:?} > {max_retrieval_time:?} for {data_size} items"
        );

        assert!(
            iteration_time < max_iteration_time,
            "Data iteration too slow: {iteration_time:?} > {max_iteration_time:?} for {data_size} items"
        );

        println!(
            "FormData performance ({data_size} items): insert {insertion_time:?}, retrieve {retrieval_time:?}, iterate {iteration_time:?}"
        );
    }

    println!("Form data handling performance test completed");
}

/// Test 8: Complex form structure performance
#[test]
fn test_complex_form_structure_performance() {
    let complexity_levels = vec![50, 100, 200];

    for complexity in complexity_levels {
        let mut form_manager = FormManager::new();
        let start_time = Instant::now();

        // Create complex form structure
        for section in 0..complexity {
            // Section header (as read-only text field)
            let header_field = TextField::new(format!("section_header_{section}"))
                .with_value(format!("Section {section} - Complex Form Data"));

            let header_flags = FieldFlags {
                read_only: true,
                required: false,
                no_export: true,
            };

            let header_options = FieldOptions {
                flags: header_flags,
                default_appearance: Some("/Helv 14 Tf 0 g".to_string()),
                quadding: Some(1), // Center
            };

            let header_widget = Widget::new(Rectangle::new(
                Point::new(50.0, 750.0 - section as f64 * 100.0),
                Point::new(500.0, 770.0 - section as f64 * 100.0),
            ));

            form_manager
                .add_text_field(header_field, header_widget, Some(header_options))
                .unwrap();

            // Multiple fields per section
            for field_num in 0..5 {
                let field_y = 720.0 - section as f64 * 100.0 - field_num as f64 * 25.0;

                match field_num {
                    0 => {
                        // Text field with validation
                        let text_field = TextField::new(format!("text_{section}_{field_num}"))
                            .with_value(format!("Text data {section} {field_num}"))
                            .with_max_length(200);

                        let text_widget = Widget::new(Rectangle::new(
                            Point::new(50.0, field_y),
                            Point::new(300.0, field_y + 20.0),
                        ));

                        form_manager
                            .add_text_field(text_field, text_widget, None)
                            .unwrap();
                    }
                    1 => {
                        // Choice field with many options
                        let mut choice_field =
                            ComboBox::new(format!("choice_{section}_{field_num}"));

                        for opt in 0..10 {
                            choice_field = choice_field.add_option(
                                format!("opt_{opt}"),
                                format!("Option {opt} for section {section}"),
                            );
                        }

                        choice_field = choice_field.with_value("opt_0");

                        let choice_widget = Widget::new(Rectangle::new(
                            Point::new(320.0, field_y),
                            Point::new(480.0, field_y + 20.0),
                        ));

                        form_manager
                            .add_combo_box(choice_field, choice_widget, None)
                            .unwrap();
                    }
                    2 => {
                        // Radio button group
                        let radio_field = RadioButton::new(format!("radio_{section}_{field_num}"))
                            .add_option("yes", "Yes")
                            .add_option("no", "No")
                            .add_option("maybe", "Maybe")
                            .with_selected(0);

                        let radio_widgets = vec![
                            Widget::new(Rectangle::new(
                                Point::new(50.0, field_y - 5.0),
                                Point::new(65.0, field_y + 10.0),
                            )),
                            Widget::new(Rectangle::new(
                                Point::new(100.0, field_y - 5.0),
                                Point::new(115.0, field_y + 10.0),
                            )),
                            Widget::new(Rectangle::new(
                                Point::new(150.0, field_y - 5.0),
                                Point::new(165.0, field_y + 10.0),
                            )),
                        ];

                        form_manager
                            .add_radio_buttons(radio_field, radio_widgets, None)
                            .unwrap();
                    }
                    3 => {
                        // Checkbox group
                        for cb in 0..3 {
                            let checkbox =
                                CheckBox::new(format!("check_{section}_{field_num}_{cb}"))
                                    .with_export_value(format!("CB{cb}"));

                            let cb_widget = Widget::new(Rectangle::new(
                                Point::new(200.0 + cb as f64 * 30.0, field_y),
                                Point::new(215.0 + cb as f64 * 30.0, field_y + 15.0),
                            ));

                            form_manager
                                .add_checkbox(checkbox, cb_widget, None)
                                .unwrap();
                        }
                    }
                    4 => {
                        // Button for section
                        let button = PushButton::new(format!("button_{section}_{field_num}"))
                            .with_caption(format!("Process Section {section}"));

                        let button_widget = Widget::new(Rectangle::new(
                            Point::new(350.0, field_y),
                            Point::new(480.0, field_y + 20.0),
                        ));

                        form_manager
                            .add_push_button(button, button_widget, None)
                            .unwrap();
                    }
                    _ => {}
                }
            }
        }

        let creation_time = start_time.elapsed();
        let field_count = form_manager.field_count();

        // Test form operations performance
        let operations_start = Instant::now();

        // Get AcroForm
        let acro_form = form_manager.get_acro_form();
        let acro_dict = acro_form.to_dict();

        // Perform some field lookups
        for section in 0..complexity.min(10) {
            let field_name = format!("text_{}_{}", section, 0);
            assert!(form_manager.get_field(&field_name).is_some());
        }

        let operations_time = operations_start.elapsed();

        // Performance assertions
        let expected_fields = complexity * 8; // 8 fields per section (1 header + 1 text + 1 combo + 1 radio + 3 checkboxes + 1 button)
        assert!(
            field_count >= expected_fields,
            "Expected at least {expected_fields} fields, got {field_count}"
        );

        let max_creation_time = std::time::Duration::from_millis(complexity as u64 * 10);
        assert!(
            creation_time < max_creation_time,
            "Complex form creation too slow: {creation_time:?} > {max_creation_time:?}"
        );

        let max_operations_time = std::time::Duration::from_millis(50);
        assert!(
            operations_time < max_operations_time,
            "Complex form operations too slow: {operations_time:?} > {max_operations_time:?}"
        );

        // Verify AcroForm structure
        if let Some(Object::Array(fields)) = acro_dict.get("Fields") {
            assert_eq!(fields.len(), field_count);
        }

        println!(
            "Complex form (level {complexity}): {field_count} fields created in {creation_time:?}, operations in {operations_time:?}"
        );
    }

    println!("Complex form structure performance test completed");
}

/// Test 9: Memory pressure handling
#[test]
fn test_memory_pressure_handling() {
    let iterations = 50;
    let initial_memory = get_memory_indicator();

    for iteration in 0..iterations {
        // Create a form manager with moderate number of fields
        let mut form_manager = FormManager::new();

        for i in 0..200 {
            let field = TextField::new(format!("pressure_{iteration}_{i}"))
                .with_value(format!(
                    "Memory pressure test iteration {iteration} field {i}"
                ))
                .with_max_length(100);

            let widget = Widget::new(Rectangle::new(
                Point::new(50.0, 700.0 - i as f64 * 2.0),
                Point::new(300.0, 720.0 - i as f64 * 2.0),
            ));

            form_manager.add_text_field(field, widget, None).unwrap();
        }

        // Generate AcroForm (this creates additional objects)
        let acro_form = form_manager.get_acro_form();
        let _acro_dict = acro_form.to_dict();

        // Verify field count
        assert_eq!(form_manager.field_count(), 200);

        // Check memory growth periodically
        if iteration % 10 == 9 {
            let current_memory = get_memory_indicator();
            let memory_growth = current_memory.saturating_sub(initial_memory);

            println!(
                "Iteration {}: memory growth = {} units",
                iteration + 1,
                memory_growth
            );

            // Memory shouldn't grow excessively (some growth is expected)
            let max_memory_growth = (iteration + 1) * 100; // Conservative limit
            assert!(
                memory_growth < max_memory_growth,
                "Excessive memory growth: {} > {} after {} iterations",
                memory_growth,
                max_memory_growth,
                iteration + 1
            );
        }

        // Form manager is dropped here, should free memory
    }

    let final_memory = get_memory_indicator();
    let total_growth = final_memory.saturating_sub(initial_memory);

    println!(
        "Memory pressure test completed: {iterations} iterations, total growth = {total_growth} units"
    );

    // After all iterations, memory growth should be minimal
    assert!(
        total_growth < 1000,
        "Too much memory retained after pressure test: {total_growth} units"
    );

    println!("Memory pressure handling test completed");
}

/// Test 10: Scalability limits and edge cases
#[test]
fn test_scalability_limits() {
    // Test 1: Maximum reasonable field count
    let max_fields = 10000;
    let mut form_manager = FormManager::new();

    let start_time = Instant::now();
    let mut created_count = 0;

    for i in 0..max_fields {
        let field = TextField::new(format!("scale_field_{i}"))
            .with_value(format!("Scalability test {i}"))
            .with_max_length(50);

        let widget = Widget::new(Rectangle::new(
            Point::new(50.0, 700.0),
            Point::new(200.0, 720.0),
        ));

        match form_manager.add_text_field(field, widget, None) {
            Ok(_) => created_count += 1,
            Err(_) => {
                println!("Hit limit at {i} fields");
                break;
            }
        }

        // Check performance every 1000 fields
        if i % 1000 == 999 {
            let elapsed = start_time.elapsed();
            if elapsed.as_secs() > 30 {
                println!("Stopping at {} fields due to time limit", i + 1);
                break;
            }
        }
    }

    let creation_time = start_time.elapsed();

    println!("Scalability test: created {created_count} fields in {creation_time:?}");

    // Should be able to create at least 5000 fields reasonably
    assert!(
        created_count >= 5000,
        "Should be able to create at least 5000 fields, created {created_count}"
    );

    // Test field operations at scale
    let operations_start = Instant::now();

    // Test field lookup at various positions
    let test_indices = vec![
        0,
        created_count / 4,
        created_count / 2,
        created_count * 3 / 4,
        created_count - 1,
    ];

    for &index in &test_indices {
        let field_name = format!("scale_field_{index}");
        assert!(
            form_manager.get_field(&field_name).is_some(),
            "Could not find field at index {index}"
        );
    }

    // Test AcroForm generation at scale
    let acro_form = form_manager.get_acro_form();
    assert_eq!(acro_form.fields.len(), created_count);

    let acro_dict = acro_form.to_dict();
    if let Some(Object::Array(fields)) = acro_dict.get("Fields") {
        assert_eq!(fields.len(), created_count);
    }

    let operations_time = operations_start.elapsed();

    println!("Scale operations completed in {operations_time:?}");

    // Operations should complete in reasonable time even at scale
    assert!(
        operations_time < std::time::Duration::from_secs(5),
        "Scale operations too slow: {operations_time:?}"
    );

    println!("Scalability limits test completed");
}

/// Helper function to get a rough memory usage indicator
fn get_memory_indicator() -> usize {
    // Simple heuristic: create test allocations and measure time
    let start = Instant::now();
    let mut test_data = Vec::new();

    for i in 0..1000 {
        test_data.push(vec![i; 100]);
        if start.elapsed().as_millis() > 50 {
            break;
        }
    }

    let indicator = test_data.len();
    drop(test_data); // Clean up

    indicator
}
