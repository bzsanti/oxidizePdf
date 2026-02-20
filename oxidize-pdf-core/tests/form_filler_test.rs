//! Integration tests for form filling functionality

use std::collections::HashMap;
use std::io::Cursor;

use oxidize_pdf::operations::{
    FormFieldInfo, FormFieldType, FormFiller, FormFillerError, FormFillerOptions, PdfEditor,
};
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::{Document, Page};

/// Helper function to create a minimal valid PDF for testing
fn create_test_pdf(page_count: usize) -> Vec<u8> {
    let mut doc = Document::new();
    for _ in 0..page_count {
        // Create A4 page (595 x 842 points)
        let page = Page::new(595.0, 842.0);
        doc.add_page(page);
    }

    let config = WriterConfig::default();
    let mut output = Vec::new();
    {
        let cursor = Cursor::new(&mut output);
        let mut writer = PdfWriter::with_config(cursor, config);
        writer
            .write_document(&mut doc)
            .expect("Failed to create test PDF");
    }
    output
}

// TI4.1 - List fields returns empty for PDF without AcroForm
#[test]
fn test_list_fields_empty_pdf() {
    let pdf_bytes = create_test_pdf(1);
    let editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let fields = FormFiller::list_fields(&editor).unwrap();
    assert!(fields.is_empty(), "PDF without forms should have no fields");
}

// TI4.2 - FormFiller with synthetic fields in editor
#[test]
fn test_form_filler_with_synthetic_fields() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    // Simulate detected form fields (in real use, these would be parsed from PDF)
    editor.add_test_form_field(FormFieldInfo::new("name", FormFieldType::Text));
    editor.add_test_form_field(FormFieldInfo::new("agree", FormFieldType::Checkbox));
    editor.add_test_form_field(
        FormFieldInfo::new("country", FormFieldType::Dropdown).with_options(vec![
            "US".to_string(),
            "UK".to_string(),
            "CA".to_string(),
        ]),
    );

    let fields = FormFiller::list_fields(&editor).unwrap();
    assert_eq!(fields.len(), 3);

    // Check field types
    assert_eq!(fields[0].name, "name");
    assert_eq!(fields[0].field_type, FormFieldType::Text);
    assert_eq!(fields[1].name, "agree");
    assert_eq!(fields[1].field_type, FormFieldType::Checkbox);
    assert_eq!(fields[2].name, "country");
    assert_eq!(fields[2].field_type, FormFieldType::Dropdown);
}

// TI4.3 - Set text field value
#[test]
fn test_set_text_field_value() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    editor.add_test_form_field(FormFieldInfo::new("username", FormFieldType::Text));

    FormFiller::set_text_field(&mut editor, "username", "john_doe").unwrap();

    // Verify value was set
    let value = FormFiller::get_field_value(&editor, "username").unwrap();
    assert_eq!(value, Some("john_doe".to_string()));

    // Verify pending updates
    assert!(editor.pending_form_update_count() > 0);
}

// TI4.4 - Set checkbox value
#[test]
fn test_set_checkbox_value() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    editor.add_test_form_field(FormFieldInfo::new("terms", FormFieldType::Checkbox));

    // Set to checked
    FormFiller::set_checkbox(&mut editor, "terms", true).unwrap();

    let value = FormFiller::get_field_value(&editor, "terms").unwrap();
    assert_eq!(value, Some("Yes".to_string()));

    // Set to unchecked
    FormFiller::set_checkbox(&mut editor, "terms", false).unwrap();

    let value = FormFiller::get_field_value(&editor, "terms").unwrap();
    assert_eq!(value, Some("Off".to_string()));
}

// TI4.5 - Wrong field type error
#[test]
fn test_wrong_field_type_error() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    editor.add_test_form_field(FormFieldInfo::new("text_field", FormFieldType::Text));

    // Try to set checkbox on a text field
    let result = FormFiller::set_checkbox(&mut editor, "text_field", true);
    assert!(result.is_err());

    match result.unwrap_err() {
        FormFillerError::WrongFieldType { expected, got } => {
            assert_eq!(expected, FormFieldType::Checkbox);
            assert_eq!(got, FormFieldType::Text);
        }
        e => panic!("Expected WrongFieldType, got {:?}", e),
    }
}

// TI4.6 - Field not found error
#[test]
fn test_field_not_found_error() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    let result = FormFiller::set_text_field(&mut editor, "nonexistent", "value");
    assert!(result.is_err());

    match result.unwrap_err() {
        FormFillerError::FieldNotFound(name) => {
            assert_eq!(name, "nonexistent");
        }
        e => panic!("Expected FieldNotFound, got {:?}", e),
    }
}

// TI4.7 - Fill all fields at once
#[test]
fn test_fill_all_fields() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    editor.add_test_form_field(FormFieldInfo::new("first_name", FormFieldType::Text));
    editor.add_test_form_field(FormFieldInfo::new("last_name", FormFieldType::Text));
    editor.add_test_form_field(FormFieldInfo::new("email", FormFieldType::Text));

    let mut values = HashMap::new();
    values.insert("first_name".to_string(), "John".to_string());
    values.insert("last_name".to_string(), "Doe".to_string());
    values.insert("email".to_string(), "john@example.com".to_string());

    let options = FormFillerOptions::default();
    let count = FormFiller::fill_all(&mut editor, values, &options).unwrap();

    assert_eq!(count, 3);
    assert_eq!(editor.pending_form_update_count(), 3);
}

// TI4.8 - Fill all with strict mode
#[test]
fn test_fill_all_strict_mode() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    editor.add_test_form_field(FormFieldInfo::new("name", FormFieldType::Text));

    let mut values = HashMap::new();
    values.insert("name".to_string(), "John".to_string());
    values.insert("missing_field".to_string(), "value".to_string());

    // Lenient mode should succeed
    let options = FormFillerOptions::default();
    let count = FormFiller::fill_all(&mut editor, values.clone(), &options).unwrap();
    assert_eq!(count, 1); // Only "name" was filled

    // Reset editor
    let pdf_bytes = create_test_pdf(1);
    let mut editor2 = PdfEditor::from_bytes(pdf_bytes).unwrap();
    editor2.add_test_form_field(FormFieldInfo::new("name", FormFieldType::Text));

    // Strict mode should fail
    let strict_options = FormFillerOptions::default().with_strict();
    let result = FormFiller::fill_all(&mut editor2, values, &strict_options);
    assert!(result.is_err());
}

// TI4.9 - Flatten form
#[test]
fn test_flatten_form() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    assert!(!editor.is_form_flattened());

    FormFiller::flatten(&mut editor).unwrap();

    assert!(editor.is_form_flattened());
}

// TI4.10 - Dropdown with invalid option
#[test]
fn test_dropdown_invalid_option() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    editor.add_test_form_field(
        FormFieldInfo::new("country", FormFieldType::Dropdown)
            .with_options(vec!["US".to_string(), "UK".to_string()]),
    );

    // Try to set invalid option
    let result = FormFiller::set_dropdown(&mut editor, "country", "XX");
    assert!(result.is_err());

    match result.unwrap_err() {
        FormFillerError::InvalidOptionValue { field, value } => {
            assert_eq!(field, "country");
            assert_eq!(value, "XX");
        }
        e => panic!("Expected InvalidOptionValue, got {:?}", e),
    }
}

// TI4.11 - Dropdown with valid option
#[test]
fn test_dropdown_valid_option() {
    let pdf_bytes = create_test_pdf(1);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    editor.add_test_form_field(
        FormFieldInfo::new("country", FormFieldType::Dropdown)
            .with_options(vec!["US".to_string(), "UK".to_string()]),
    );

    FormFiller::set_dropdown(&mut editor, "country", "UK").unwrap();

    let value = FormFiller::get_field_value(&editor, "country").unwrap();
    assert_eq!(value, Some("UK".to_string()));
}

// TI4.12 - Output PDF is parseable after form operations
#[test]
fn test_form_operations_output_parseable() {
    let pdf_bytes = create_test_pdf(2);
    let mut editor = PdfEditor::from_bytes(pdf_bytes).unwrap();

    editor.add_test_form_field(FormFieldInfo::new("test", FormFieldType::Text));
    FormFiller::set_text_field(&mut editor, "test", "value").unwrap();

    // Save and verify parseable
    let output_bytes = editor.save_to_bytes().unwrap();

    // Verify PDF header
    assert!(output_bytes.starts_with(b"%PDF-"));

    // Re-parse should succeed
    let editor2 = PdfEditor::from_bytes(output_bytes);
    assert!(editor2.is_ok());
    assert_eq!(editor2.unwrap().page_count(), 2);
}
