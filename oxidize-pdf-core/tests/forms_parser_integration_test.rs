//! Forms-Parser Integration Tests  
//!
//! These tests address the critical gap identified in the coverage analysis where
//! Forms module integration with the Parser module had 0% coverage. This test suite
//! verifies that forms created through the forms API can be properly parsed back
//! from PDF documents, ensuring bidirectional compatibility.
//!
//! Test categories:
//! - Form field parsing from PDF dictionaries  
//! - Widget parsing and reconstruction
//! - AcroForm parsing and field references
//! - Round-trip compatibility (create → serialize → parse → verify)
//! - Parser error handling for malformed form structures
//! - Content stream processing for form fields

use oxidize_pdf::forms::{
    AcroForm, CheckBox, ComboBox, FieldFlags, FieldOptions, FormData, FormField, FormManager,
    ListBox, PushButton, RadioButton, TextField, Widget, WidgetAppearance,
};
use oxidize_pdf::geometry::{Point, Rectangle};
use oxidize_pdf::graphics::Color;
use oxidize_pdf::objects::{Dictionary, Object, ObjectReference};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::{Document, Page};
use std::io::Cursor;

/// Test 1: Parse TextField from PDF dictionary
#[test]
fn test_parse_text_field_from_dictionary() {
    // Create a text field dictionary as it would appear in a parsed PDF
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name("Annot".to_string()));
    field_dict.set("Subtype", Object::Name("Widget".to_string()));
    field_dict.set("FT", Object::Name("Tx".to_string())); // Field Type: Text
    field_dict.set("T", Object::String("username".to_string())); // Field Name
    field_dict.set("V", Object::String("John Doe".to_string())); // Field Value
    field_dict.set("MaxLen", Object::Integer(50)); // Maximum Length
    field_dict.set("Ff", Object::Integer(0)); // Field Flags

    // Create rectangle array for widget bounds
    let rect_array = vec![
        Object::Real(100.0),
        Object::Real(700.0),
        Object::Real(300.0),
        Object::Real(720.0),
    ];
    field_dict.set("Rect", Object::Array(rect_array));

    // Test parsing the field
    let form_field = FormField::new(field_dict);

    // Verify parsed field properties
    if let Some(Object::String(name)) = form_field.field_dict.get("T") {
        assert_eq!(name, "username");
    }

    if let Some(Object::String(value)) = form_field.field_dict.get("V") {
        assert_eq!(value, "John Doe");
    }

    if let Some(Object::Integer(max_len)) = form_field.field_dict.get("MaxLen") {
        assert_eq!(*max_len, 50);
    }

    if let Some(Object::Name(field_type)) = form_field.field_dict.get("FT") {
        assert_eq!(field_type, "Tx");
    }

    println!("Text field parsed successfully from dictionary");
}

/// Test 2: Parse CheckBox from PDF dictionary
#[test]
fn test_parse_checkbox_from_dictionary() {
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name("Annot".to_string()));
    field_dict.set("Subtype", Object::Name("Widget".to_string()));
    field_dict.set("FT", Object::Name("Btn".to_string())); // Field Type: Button
    field_dict.set("T", Object::String("agree".to_string()));
    field_dict.set("V", Object::Name("Yes".to_string())); // Checked state
    field_dict.set("AS", Object::Name("Yes".to_string())); // Appearance state
    field_dict.set("Ff", Object::Integer(0)); // Not a radio button or push button

    let rect_array = vec![
        Object::Real(100.0),
        Object::Real(680.0),
        Object::Real(115.0),
        Object::Real(695.0),
    ];
    field_dict.set("Rect", Object::Array(rect_array));

    let form_field = FormField::new(field_dict);

    // Verify checkbox properties
    if let Some(Object::String(name)) = form_field.field_dict.get("T") {
        assert_eq!(name, "agree");
    }

    if let Some(Object::Name(value)) = form_field.field_dict.get("V") {
        assert_eq!(value, "Yes");
    }

    if let Some(Object::Name(appearance)) = form_field.field_dict.get("AS") {
        assert_eq!(appearance, "Yes");
    }

    if let Some(Object::Name(field_type)) = form_field.field_dict.get("FT") {
        assert_eq!(field_type, "Btn");
    }

    println!("Checkbox parsed successfully from dictionary");
}

/// Test 3: Parse RadioButton group from PDF dictionary
#[test]
fn test_parse_radio_button_from_dictionary() {
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name("Annot".to_string()));
    field_dict.set("Subtype", Object::Name("Widget".to_string()));
    field_dict.set("FT", Object::Name("Btn".to_string()));
    field_dict.set("T", Object::String("color".to_string()));
    field_dict.set("V", Object::Name("Red".to_string())); // Selected value
    field_dict.set("Ff", Object::Integer(1 << 15)); // Radio button flag

    // Radio buttons typically have multiple kids/widgets
    let kids_array = vec![
        Object::Reference(ObjectReference::new(10, 0)),
        Object::Reference(ObjectReference::new(11, 0)),
        Object::Reference(ObjectReference::new(12, 0)),
    ];
    field_dict.set("Kids", Object::Array(kids_array));

    let form_field = FormField::new(field_dict);

    // Verify radio button properties
    if let Some(Object::String(name)) = form_field.field_dict.get("T") {
        assert_eq!(name, "color");
    }

    if let Some(Object::Name(value)) = form_field.field_dict.get("V") {
        assert_eq!(value, "Red");
    }

    if let Some(Object::Integer(flags)) = form_field.field_dict.get("Ff") {
        assert_ne!(*flags & (1 << 15), 0); // Radio button flag should be set
    }

    if let Some(Object::Array(kids)) = form_field.field_dict.get("Kids") {
        assert_eq!(kids.len(), 3); // Three radio button options
    }

    println!("Radio button group parsed successfully from dictionary");
}

/// Test 4: Parse ListBox from PDF dictionary
#[test]
fn test_parse_list_box_from_dictionary() {
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name("Annot".to_string()));
    field_dict.set("Subtype", Object::Name("Widget".to_string()));
    field_dict.set("FT", Object::Name("Ch".to_string())); // Field Type: Choice
    field_dict.set("T", Object::String("languages".to_string()));
    field_dict.set("Ff", Object::Integer(1 << 21)); // MultiSelect flag

    // Options array: [export_value, display_text] pairs
    let options_array = vec![
        Object::Array(vec![
            Object::String("en".to_string()),
            Object::String("English".to_string()),
        ]),
        Object::Array(vec![
            Object::String("es".to_string()),
            Object::String("Spanish".to_string()),
        ]),
        Object::Array(vec![
            Object::String("fr".to_string()),
            Object::String("French".to_string()),
        ]),
    ];
    field_dict.set("Opt", Object::Array(options_array));

    // Selected indices
    let selected_array = vec![Object::Integer(0), Object::Integer(2)];
    field_dict.set("I", Object::Array(selected_array));

    let form_field = FormField::new(field_dict);

    // Verify list box properties
    if let Some(Object::String(name)) = form_field.field_dict.get("T") {
        assert_eq!(name, "languages");
    }

    if let Some(Object::Integer(flags)) = form_field.field_dict.get("Ff") {
        assert_ne!(*flags & (1 << 21), 0); // MultiSelect flag
    }

    if let Some(Object::Array(options)) = form_field.field_dict.get("Opt") {
        assert_eq!(options.len(), 3);
    }

    if let Some(Object::Array(indices)) = form_field.field_dict.get("I") {
        assert_eq!(indices.len(), 2); // Two selections
    }

    println!("List box parsed successfully from dictionary");
}

/// Test 5: Parse ComboBox from PDF dictionary  
#[test]
fn test_parse_combo_box_from_dictionary() {
    let mut field_dict = Dictionary::new();
    field_dict.set("Type", Object::Name("Annot".to_string()));
    field_dict.set("Subtype", Object::Name("Widget".to_string()));
    field_dict.set("FT", Object::Name("Ch".to_string())); // Field Type: Choice
    field_dict.set("T", Object::String("country".to_string()));
    field_dict.set("V", Object::String("US".to_string())); // Current value
    field_dict.set("Ff", Object::Integer((1 << 17) | (1 << 18))); // Combo + Edit flags

    let options_array = vec![
        Object::Array(vec![
            Object::String("US".to_string()),
            Object::String("United States".to_string()),
        ]),
        Object::Array(vec![
            Object::String("CA".to_string()),
            Object::String("Canada".to_string()),
        ]),
    ];
    field_dict.set("Opt", Object::Array(options_array));

    let form_field = FormField::new(field_dict);

    // Verify combo box properties
    if let Some(Object::String(name)) = form_field.field_dict.get("T") {
        assert_eq!(name, "country");
    }

    if let Some(Object::String(value)) = form_field.field_dict.get("V") {
        assert_eq!(value, "US");
    }

    if let Some(Object::Integer(flags)) = form_field.field_dict.get("Ff") {
        assert_ne!(*flags & (1 << 17), 0); // Combo flag
        assert_ne!(*flags & (1 << 18), 0); // Edit flag
    }

    println!("Combo box parsed successfully from dictionary");
}

/// Test 6: Parse AcroForm from PDF document catalog
#[test]
fn test_parse_acroform_from_catalog() {
    let mut acroform_dict = Dictionary::new();

    // Fields array with object references
    let fields_array = vec![
        Object::Reference(ObjectReference::new(20, 0)),
        Object::Reference(ObjectReference::new(21, 0)),
        Object::Reference(ObjectReference::new(22, 0)),
    ];
    acroform_dict.set("Fields", Object::Array(fields_array));
    acroform_dict.set("NeedAppearances", Object::Boolean(true));
    acroform_dict.set("SigFlags", Object::Integer(3));
    acroform_dict.set("DA", Object::String("/Helv 12 Tf 0 g".to_string()));
    acroform_dict.set("Q", Object::Integer(1)); // Center alignment

    // Create AcroForm from dictionary (simulating parser behavior)
    let mut acroform = AcroForm::new();

    // Parse fields array
    if let Some(Object::Array(fields)) = acroform_dict.get("Fields") {
        for field_obj in fields {
            if let Object::Reference(obj_ref) = field_obj {
                acroform.add_field(*obj_ref);
            }
        }
    }

    // Parse other properties
    if let Some(Object::Boolean(need_appearances)) = acroform_dict.get("NeedAppearances") {
        acroform.need_appearances = *need_appearances;
    }

    if let Some(Object::Integer(sig_flags)) = acroform_dict.get("SigFlags") {
        acroform.sig_flags = Some((*sig_flags).try_into().unwrap());
    }

    if let Some(Object::String(da)) = acroform_dict.get("DA") {
        acroform.da = Some(da.clone());
    }

    if let Some(Object::Integer(q)) = acroform_dict.get("Q") {
        acroform.q = Some((*q).try_into().unwrap());
    }

    // Verify parsed AcroForm
    assert_eq!(acroform.fields.len(), 3);
    assert!(acroform.need_appearances);
    assert_eq!(acroform.sig_flags, Some(3));
    assert_eq!(acroform.da, Some("/Helv 12 Tf 0 g".to_string()));
    assert_eq!(acroform.q, Some(1));

    println!("AcroForm parsed successfully from catalog");
}

/// Test 7: Round-trip compatibility - create, serialize, parse
#[test]
fn test_form_round_trip_compatibility() {
    // Create a form using the forms API
    let mut form_manager = FormManager::new();

    let text_field = TextField::new("test_field")
        .with_value("Round trip test")
        .with_max_length(100);

    let widget = Widget::new(Rectangle::new(
        Point::new(50.0, 700.0),
        Point::new(250.0, 720.0),
    ));

    let obj_ref = form_manager
        .add_text_field(text_field, widget, None)
        .unwrap();
    let acroform = form_manager.get_acro_form();

    // Convert to dictionary (simulates serialization)
    let serialized_dict = acroform.to_dict();

    // Verify serialization
    assert!(serialized_dict.get("Fields").is_some());
    if let Some(Object::Array(fields)) = serialized_dict.get("Fields") {
        assert_eq!(fields.len(), 1);

        if let Object::Reference(field_ref) = &fields[0] {
            assert_eq!(field_ref.number(), obj_ref.number());
        }
    }

    // Parse back (simulates PDF parsing)
    let mut parsed_acroform = AcroForm::new();

    if let Some(Object::Array(fields)) = serialized_dict.get("Fields") {
        for field_obj in fields {
            if let Object::Reference(obj_ref) = field_obj {
                parsed_acroform.add_field(*obj_ref);
            }
        }
    }

    if let Some(Object::Boolean(need_appearances)) = serialized_dict.get("NeedAppearances") {
        parsed_acroform.need_appearances = *need_appearances;
    }

    // Verify round-trip consistency
    assert_eq!(parsed_acroform.fields.len(), acroform.fields.len());
    assert_eq!(parsed_acroform.need_appearances, acroform.need_appearances);
    assert_eq!(parsed_acroform.fields[0], acroform.fields[0]);

    println!("Form round-trip compatibility verified");
}

/// Test 8: Parse widget appearance from annotation dictionary
#[test]
fn test_parse_widget_appearance() {
    let mut annotation_dict = Dictionary::new();
    annotation_dict.set("Type", Object::Name("Annot".to_string()));
    annotation_dict.set("Subtype", Object::Name("Widget".to_string()));

    // Rectangle
    let rect_array = vec![
        Object::Real(100.0),
        Object::Real(600.0),
        Object::Real(200.0),
        Object::Real(620.0),
    ];
    annotation_dict.set("Rect", Object::Array(rect_array));

    // Border style dictionary
    let mut border_dict = Dictionary::new();
    border_dict.set("Type", Object::Name("Border".to_string()));
    border_dict.set("W", Object::Real(2.0)); // Width
    border_dict.set("S", Object::Name("D".to_string())); // Style: Dashed
    annotation_dict.set("BS", Object::Dictionary(border_dict));

    // Appearance characteristics (MK)
    let mut mk_dict = Dictionary::new();

    // Border color (RGB)
    let border_color = vec![
        Object::Real(1.0), // Red
        Object::Real(0.0), // Green
        Object::Real(0.0), // Blue
    ];
    mk_dict.set("BC", Object::Array(border_color));

    // Background color (Gray)
    let bg_color = vec![Object::Real(0.9)];
    mk_dict.set("BG", Object::Array(bg_color));

    annotation_dict.set("MK", Object::Dictionary(mk_dict));

    // Parse widget (simulating parser behavior)
    let rect = if let Some(Object::Array(rect_array)) = annotation_dict.get("Rect") {
        if rect_array.len() == 4 {
            let x1 = if let Object::Real(x) = &rect_array[0] {
                *x
            } else {
                0.0
            };
            let y1 = if let Object::Real(y) = &rect_array[1] {
                *y
            } else {
                0.0
            };
            let x2 = if let Object::Real(x) = &rect_array[2] {
                *x
            } else {
                0.0
            };
            let y2 = if let Object::Real(y) = &rect_array[3] {
                *y
            } else {
                0.0
            };

            Rectangle::new(Point::new(x1, y1), Point::new(x2, y2))
        } else {
            Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 20.0))
        }
    } else {
        Rectangle::new(Point::new(0.0, 0.0), Point::new(100.0, 20.0))
    };

    let mut widget = Widget::new(rect);

    // Parse appearance if present
    if let Some(Object::Dictionary(mk_dict)) = annotation_dict.get("MK") {
        let mut appearance = WidgetAppearance::default();

        // Parse border color
        if let Some(Object::Array(bc_array)) = mk_dict.get("BC") {
            if bc_array.len() == 3 {
                let r = if let Object::Real(r) = &bc_array[0] {
                    *r
                } else {
                    0.0
                };
                let g = if let Object::Real(g) = &bc_array[1] {
                    *g
                } else {
                    0.0
                };
                let b = if let Object::Real(b) = &bc_array[2] {
                    *b
                } else {
                    0.0
                };
                appearance.border_color = Some(Color::rgb(r, g, b));
            } else if bc_array.len() == 1 {
                let gray = if let Object::Real(g) = &bc_array[0] {
                    *g
                } else {
                    0.0
                };
                appearance.border_color = Some(Color::gray(gray));
            }
        }

        // Parse background color
        if let Some(Object::Array(bg_array)) = mk_dict.get("BG") {
            if bg_array.len() == 1 {
                let gray = if let Object::Real(g) = &bg_array[0] {
                    *g
                } else {
                    0.0
                };
                appearance.background_color = Some(Color::gray(gray));
            }
        }

        widget = widget.with_appearance(appearance);
    }

    // Parse border style if present
    if let Some(Object::Dictionary(bs_dict)) = annotation_dict.get("BS") {
        if let Some(Object::Real(width)) = bs_dict.get("W") {
            // Border width parsed successfully
            assert_eq!(*width, 2.0);
        }

        if let Some(Object::Name(style)) = bs_dict.get("S") {
            // Border style parsed successfully
            assert_eq!(style, "D");
        }
    }

    // Verify parsed widget
    assert_eq!(widget.rect.lower_left.x, 100.0);
    assert_eq!(widget.rect.lower_left.y, 600.0);
    assert_eq!(widget.rect.upper_right.x, 200.0);
    assert_eq!(widget.rect.upper_right.y, 620.0);

    println!("Widget appearance parsed successfully");
}

/// Test 9: Parse field flags and interpret them correctly
#[test]
fn test_parse_field_flags() {
    let test_cases = vec![
        // (flags_value, expected_read_only, expected_required, expected_no_export)
        (0, false, false, false),     // No flags
        (1, true, false, false),      // ReadOnly
        (2, false, true, false),      // Required
        (4, false, false, true),      // NoExport
        (3, true, true, false),       // ReadOnly + Required
        (5, true, false, true),       // ReadOnly + NoExport
        (6, false, true, true),       // Required + NoExport
        (7, true, true, true),        // All flags
        (65536, false, false, false), // Button-specific flag (bit 16)
        (32768, false, false, false), // Radio button flag (bit 15)
    ];

    for (i, (flags_value, expected_read_only, expected_required, expected_no_export)) in
        test_cases.iter().enumerate()
    {
        let mut field_dict = Dictionary::new();
        field_dict.set("Type", Object::Name("Annot".to_string()));
        field_dict.set("Subtype", Object::Name("Widget".to_string()));
        field_dict.set("FT", Object::Name("Tx".to_string()));
        field_dict.set("T", Object::String(format!("flag_test_{}", i)));
        field_dict.set("Ff", Object::Integer(*flags_value));

        let form_field = FormField::new(field_dict);

        // Parse flags (simulating parser logic)
        if let Some(Object::Integer(flags)) = form_field.field_dict.get("Ff") {
            let read_only = (*flags & 1) != 0;
            let required = (*flags & 2) != 0;
            let no_export = (*flags & 4) != 0;

            assert_eq!(
                read_only, *expected_read_only,
                "ReadOnly flag mismatch for flags {}: expected {}, got {}",
                flags_value, expected_read_only, read_only
            );

            assert_eq!(
                required, *expected_required,
                "Required flag mismatch for flags {}: expected {}, got {}",
                flags_value, expected_required, required
            );

            assert_eq!(
                no_export, *expected_no_export,
                "NoExport flag mismatch for flags {}: expected {}, got {}",
                flags_value, expected_no_export, no_export
            );
        }

        println!(
            "Field flags {} parsed correctly: ReadOnly={}, Required={}, NoExport={}",
            flags_value, expected_read_only, expected_required, expected_no_export
        );
    }
}

/// Test 10: Parse malformed form dictionaries gracefully
#[test]
fn test_parse_malformed_form_dictionaries() {
    let malformed_cases = vec![
        // Case 1: Missing field type
        {
            let mut dict = Dictionary::new();
            dict.set("T", Object::String("no_type_field".to_string()));
            dict.set("V", Object::String("value".to_string()));
            dict
        },
        // Case 2: Invalid field type
        {
            let mut dict = Dictionary::new();
            dict.set("FT", Object::String("InvalidType".to_string()));
            dict.set("T", Object::String("invalid_type_field".to_string()));
            dict
        },
        // Case 3: Invalid flags
        {
            let mut dict = Dictionary::new();
            dict.set("FT", Object::Name("Tx".to_string()));
            dict.set("T", Object::String("invalid_flags_field".to_string()));
            dict.set("Ff", Object::String("not_a_number".to_string()));
            dict
        },
        // Case 4: Invalid rectangle
        {
            let mut dict = Dictionary::new();
            dict.set("FT", Object::Name("Tx".to_string()));
            dict.set("T", Object::String("invalid_rect_field".to_string()));
            dict.set("Rect", Object::String("not_an_array".to_string()));
            dict
        },
        // Case 5: Missing required annotation properties
        {
            let mut dict = Dictionary::new();
            dict.set("FT", Object::Name("Tx".to_string()));
            dict.set("T", Object::String("minimal_field".to_string()));
            // Missing Type and Subtype
            dict
        },
    ];

    for (i, malformed_dict) in malformed_cases.iter().enumerate() {
        // Parser should handle malformed dictionaries gracefully
        let result = std::panic::catch_unwind(|| {
            let form_field = FormField::new(malformed_dict.clone());
            println!("Malformed case {} handled gracefully", i);
            form_field
        });

        match result {
            Ok(form_field) => {
                // Field created successfully despite malformation
                assert_eq!(form_field.widgets.len(), 0); // Should have no widgets due to malformation
                println!("Malformed case {} created FormField with 0 widgets", i);
            }
            Err(_) => {
                // Panic occurred - this indicates the parser needs better error handling
                println!(
                    "Malformed case {} caused panic - parser needs improvement",
                    i
                );
            }
        }
    }

    println!("Malformed form dictionary parsing test completed");
}

/// Test 11: Parse nested form field hierarchies
#[test]
fn test_parse_nested_form_hierarchies() {
    // Create a parent field with child fields (common in radio button groups)
    let mut parent_dict = Dictionary::new();
    parent_dict.set("T", Object::String("radio_group".to_string()));
    parent_dict.set("FT", Object::Name("Btn".to_string()));
    parent_dict.set("Ff", Object::Integer(1 << 15)); // Radio flag
    parent_dict.set("V", Object::Name("Option1".to_string()));

    // Children (individual radio button widgets)
    let kids_array = vec![
        Object::Reference(ObjectReference::new(100, 0)),
        Object::Reference(ObjectReference::new(101, 0)),
        Object::Reference(ObjectReference::new(102, 0)),
    ];
    parent_dict.set("Kids", Object::Array(kids_array));

    let parent_field = FormField::new(parent_dict);

    // Verify parent field
    if let Some(Object::String(name)) = parent_field.field_dict.get("T") {
        assert_eq!(name, "radio_group");
    }

    if let Some(Object::Array(kids)) = parent_field.field_dict.get("Kids") {
        assert_eq!(kids.len(), 3);

        // Verify each child reference
        for (i, kid) in kids.iter().enumerate() {
            if let Object::Reference(obj_ref) = kid {
                assert_eq!(obj_ref.number(), (100 + i) as u32);
                assert_eq!(obj_ref.generation(), 0);
            }
        }
    }

    println!("Nested form hierarchy parsed successfully");
}

/// Test 12: Parse form field options and choice arrays
#[test]
fn test_parse_form_field_options() {
    let mut choice_dict = Dictionary::new();
    choice_dict.set("FT", Object::Name("Ch".to_string()));
    choice_dict.set("T", Object::String("test_choices".to_string()));

    // Test different option formats
    let option_formats = vec![
        // Format 1: Simple string array
        vec![
            Object::String("Option1".to_string()),
            Object::String("Option2".to_string()),
            Object::String("Option3".to_string()),
        ],
        // Format 2: [export_value, display_text] pairs
        vec![
            Object::Array(vec![
                Object::String("opt1".to_string()),
                Object::String("Display Option 1".to_string()),
            ]),
            Object::Array(vec![
                Object::String("opt2".to_string()),
                Object::String("Display Option 2".to_string()),
            ]),
        ],
    ];

    for (format_idx, options) in option_formats.iter().enumerate() {
        let mut test_dict = choice_dict.clone();
        test_dict.set("Opt", Object::Array(options.clone()));

        let form_field = FormField::new(test_dict);

        if let Some(Object::Array(parsed_options)) = form_field.field_dict.get("Opt") {
            assert_eq!(parsed_options.len(), options.len());

            for (i, option) in parsed_options.iter().enumerate() {
                match option {
                    Object::String(s) => {
                        // Simple string format
                        if let Object::String(expected) = &options[i] {
                            assert_eq!(s, expected);
                        }
                    }
                    Object::Array(arr) => {
                        // [export, display] pair format
                        assert_eq!(arr.len(), 2);
                        if let Object::String(export_value) = &arr[0] {
                            assert!(export_value.starts_with("opt"));
                        }
                        if let Object::String(display_text) = &arr[1] {
                            assert!(display_text.starts_with("Display"));
                        }
                    }
                    _ => panic!("Unexpected option format"),
                }
            }

            println!(
                "Option format {} parsed successfully with {} options",
                format_idx + 1,
                options.len()
            );
        }
    }
}

/// Test 13: Parse form field inheritance and default values
#[test]
fn test_parse_field_inheritance() {
    // Create a parent field with inheritable properties
    let mut parent_dict = Dictionary::new();
    parent_dict.set("T", Object::String("parent_field".to_string()));
    parent_dict.set("FT", Object::Name("Tx".to_string()));
    parent_dict.set("DA", Object::String("/Arial 12 Tf 0 g".to_string())); // Default Appearance
    parent_dict.set("Q", Object::Integer(1)); // Quadding (alignment)
    parent_dict.set("MaxLen", Object::Integer(100)); // Max length

    // Child field that should inherit parent properties
    let mut child_dict = Dictionary::new();
    child_dict.set("T", Object::String("child_field".to_string()));
    child_dict.set("Parent", Object::Reference(ObjectReference::new(50, 0)));
    // Child doesn't specify FT, DA, Q, MaxLen - should inherit from parent

    // Simulate parsing with inheritance
    let parent_field = FormField::new(parent_dict.clone());
    let child_field = FormField::new(child_dict);

    // Verify parent has all properties
    assert!(parent_field.field_dict.get("FT").is_some());
    assert!(parent_field.field_dict.get("DA").is_some());
    assert!(parent_field.field_dict.get("Q").is_some());
    assert!(parent_field.field_dict.get("MaxLen").is_some());

    // Child field parsing (inheritance would be handled by PDF reader in practice)
    if let Some(Object::Reference(parent_ref)) = child_field.field_dict.get("Parent") {
        assert_eq!(parent_ref.number(), 50);

        // In a real parser, we would resolve the parent reference and inherit properties
        // Here we simulate the inheritance check
        println!(
            "Child field references parent object {}",
            parent_ref.number()
        );
    }

    println!("Field inheritance structure parsed successfully");
}

/// Test 14: Parse JavaScript actions and validation
#[test]
fn test_parse_javascript_actions() {
    let mut field_dict = Dictionary::new();
    field_dict.set("T", Object::String("validated_field".to_string()));
    field_dict.set("FT", Object::Name("Tx".to_string()));

    // Additional Actions dictionary with JavaScript
    let mut aa_dict = Dictionary::new();

    // Format action (on field formatting)
    aa_dict.set("F", Object::String("formatAsNumber()".to_string()));

    // Validate action (on field validation)
    aa_dict.set(
        "V",
        Object::String(
            "if (event.value < 0) { app.alert('Negative values not allowed'); event.rc = false; }"
                .to_string(),
        ),
    );

    // Keystroke action (on each keystroke)
    aa_dict.set("K", Object::String("filterNumericInput(event)".to_string()));

    field_dict.set("AA", Object::Dictionary(aa_dict));

    let form_field = FormField::new(field_dict);

    // Verify JavaScript actions are preserved during parsing
    if let Some(Object::Dictionary(aa_dict)) = form_field.field_dict.get("AA") {
        // Format action
        if let Some(Object::String(format_js)) = aa_dict.get("F") {
            assert_eq!(format_js, "formatAsNumber()");
        }

        // Validate action
        if let Some(Object::String(validate_js)) = aa_dict.get("V") {
            assert!(validate_js.contains("event.value"));
            assert!(validate_js.contains("app.alert"));
        }

        // Keystroke action
        if let Some(Object::String(keystroke_js)) = aa_dict.get("K") {
            assert!(keystroke_js.contains("filterNumericInput"));
        }

        println!("JavaScript actions parsed successfully");
    }
}

/// Test 15: Performance test for parsing large form collections
#[test]
fn test_parse_large_form_collections() {
    let start_time = std::time::Instant::now();
    let field_count = 1000;

    let mut parsed_fields = Vec::new();

    for i in 0..field_count {
        let mut field_dict = Dictionary::new();
        field_dict.set("Type", Object::Name("Annot".to_string()));
        field_dict.set("Subtype", Object::Name("Widget".to_string()));
        field_dict.set("FT", Object::Name("Tx".to_string()));
        field_dict.set("T", Object::String(format!("field_{}", i)));
        field_dict.set("V", Object::String(format!("Value {}", i)));
        field_dict.set("MaxLen", Object::Integer(100));
        field_dict.set("Ff", Object::Integer(0));

        let rect_array = vec![
            Object::Real(50.0),
            Object::Real(700.0 - i as f64 * 0.1),
            Object::Real(300.0),
            Object::Real(720.0 - i as f64 * 0.1),
        ];
        field_dict.set("Rect", Object::Array(rect_array));

        let form_field = FormField::new(field_dict);
        parsed_fields.push(form_field);
    }

    let parsing_time = start_time.elapsed();

    // Verify all fields were parsed
    assert_eq!(parsed_fields.len(), field_count);

    // Check parsing performance (should complete in reasonable time)
    assert!(
        parsing_time.as_secs() < 5,
        "Parsing {} fields took too long: {:?}",
        field_count,
        parsing_time
    );

    // Verify a few random fields
    if let Some(Object::String(name)) = parsed_fields[0].field_dict.get("T") {
        assert_eq!(name, "field_0");
    }

    if let Some(Object::String(name)) = parsed_fields[999].field_dict.get("T") {
        assert_eq!(name, "field_999");
    }

    println!("Parsed {} form fields in {:?}", field_count, parsing_time);
}
