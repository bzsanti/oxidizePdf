//! Tests for form-related actions

use oxidize_pdf::actions::{
    HideAction, ImportDataAction, JavaScriptAction, OCGStateChange, ResetFormAction,
    SetOCGStateAction, SoundAction, SubmitFormAction, SubmitFormFlags,
};
use oxidize_pdf::objects::Object;

#[test]
fn test_submit_form_action_basic() {
    let action = SubmitFormAction::new("https://example.com/submit");
    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("SubmitForm".to_string())));
    assert_eq!(
        dict.get("F"),
        Some(&Object::String("https://example.com/submit".to_string()))
    );
    assert!(dict.get("Flags").is_some());
}

#[test]
fn test_submit_form_action_html() {
    let action = SubmitFormAction::new("https://example.com/submit")
        .as_html()
        .with_charset("UTF-8");

    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("SubmitForm".to_string())));
    assert_eq!(
        dict.get("CharSet"),
        Some(&Object::String("UTF-8".to_string()))
    );

    // Check HTML flag is set (bit 2)
    if let Some(Object::Integer(flags)) = dict.get("Flags") {
        assert!(flags & (1 << 2) != 0);
    } else {
        panic!("Flags not found or wrong type");
    }
}

#[test]
fn test_submit_form_action_xml() {
    let action = SubmitFormAction::new("https://api.example.com/submit")
        .as_xml()
        .with_coordinates();

    let dict = action.to_dict();

    // Check XML flag (bit 5) and coordinates flag (bit 4)
    if let Some(Object::Integer(flags)) = dict.get("Flags") {
        assert!(flags & (1 << 5) != 0); // XML
        assert!(flags & (1 << 4) != 0); // Coordinates
    } else {
        panic!("Flags not found or wrong type");
    }
}

#[test]
fn test_submit_form_action_pdf() {
    let action = SubmitFormAction::new("https://example.com/upload")
        .as_pdf()
        .with_annotations();

    let dict = action.to_dict();

    // Check PDF flag (bit 8) and annotations flag (bit 7)
    if let Some(Object::Integer(flags)) = dict.get("Flags") {
        assert!(flags & (1 << 8) != 0); // PDF
        assert!(flags & (1 << 7) != 0); // Annotations
    } else {
        panic!("Flags not found or wrong type");
    }
}

#[test]
fn test_submit_form_with_fields() {
    let fields = vec!["name".to_string(), "email".to_string(), "phone".to_string()];
    let action = SubmitFormAction::new("https://example.com/submit").with_fields(fields);

    let dict = action.to_dict();

    if let Some(Object::Array(fields_array)) = dict.get("Fields") {
        assert_eq!(fields_array.len(), 3);
        assert_eq!(fields_array[0], Object::String("name".to_string()));
        assert_eq!(fields_array[1], Object::String("email".to_string()));
        assert_eq!(fields_array[2], Object::String("phone".to_string()));
    } else {
        panic!("Fields array not found or wrong type");
    }
}

#[test]
fn test_submit_form_excluding_fields() {
    let fields = vec!["password".to_string(), "ssn".to_string()];
    let action = SubmitFormAction::new("https://example.com/submit").excluding_fields(fields);

    let dict = action.to_dict();

    // Check that include flag is false (bit 0 should be set)
    if let Some(Object::Integer(flags)) = dict.get("Flags") {
        assert!(flags & 1 != 0); // Exclude flag
    }

    if let Some(Object::Array(fields_array)) = dict.get("Fields") {
        assert_eq!(fields_array.len(), 2);
    }
}

#[test]
fn test_submit_form_flags() {
    let mut flags = SubmitFormFlags::default();
    assert_eq!(flags.to_flags(), 0);

    flags.html = true;
    assert!(flags.to_flags() & (1 << 2) != 0);

    flags.xml = true;
    assert!(flags.to_flags() & (1 << 5) != 0);

    flags.pdf = true;
    assert!(flags.to_flags() & (1 << 8) != 0);

    flags.coordinates = true;
    assert!(flags.to_flags() & (1 << 4) != 0);

    flags.canonical_dates = true;
    assert!(flags.to_flags() & (1 << 9) != 0);
}

#[test]
fn test_reset_form_action_all_fields() {
    let action = ResetFormAction::new();
    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("ResetForm".to_string())));
    assert_eq!(dict.get("Flags"), Some(&Object::Integer(0))); // Include flag
    assert!(dict.get("Fields").is_none()); // No fields means all
}

#[test]
fn test_reset_form_action_specific_fields() {
    let fields = vec!["field1".to_string(), "field2".to_string()];
    let action = ResetFormAction::new().with_fields(fields);

    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("ResetForm".to_string())));
    assert_eq!(dict.get("Flags"), Some(&Object::Integer(0))); // Include flag

    if let Some(Object::Array(fields_array)) = dict.get("Fields") {
        assert_eq!(fields_array.len(), 2);
        assert_eq!(fields_array[0], Object::String("field1".to_string()));
    }
}

#[test]
fn test_reset_form_action_excluding_fields() {
    let fields = vec!["readonly1".to_string(), "readonly2".to_string()];
    let action = ResetFormAction::new().excluding_fields(fields);

    let dict = action.to_dict();

    assert_eq!(dict.get("Flags"), Some(&Object::Integer(1))); // Exclude flag

    if let Some(Object::Array(fields_array)) = dict.get("Fields") {
        assert_eq!(fields_array.len(), 2);
    }
}

#[test]
fn test_import_data_action() {
    let action = ImportDataAction::new("data.fdf");
    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("ImportData".to_string())));
    assert_eq!(dict.get("F"), Some(&Object::String("data.fdf".to_string())));
}

#[test]
fn test_hide_action_single_target() {
    let action = HideAction::new(vec!["field1".to_string()]);
    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("Hide".to_string())));
    assert_eq!(dict.get("T"), Some(&Object::String("field1".to_string())));
    assert_eq!(dict.get("H"), Some(&Object::Boolean(true))); // Hide by default
}

#[test]
fn test_hide_action_multiple_targets() {
    let targets = vec![
        "field1".to_string(),
        "field2".to_string(),
        "field3".to_string(),
    ];
    let action = HideAction::new(targets);
    let dict = action.to_dict();

    if let Some(Object::Array(targets_array)) = dict.get("T") {
        assert_eq!(targets_array.len(), 3);
        assert_eq!(targets_array[0], Object::String("field1".to_string()));
    } else {
        panic!("Targets array not found");
    }
}

#[test]
fn test_hide_action_show() {
    let action = HideAction::new(vec!["field1".to_string()]).show();
    let dict = action.to_dict();

    assert_eq!(dict.get("H"), Some(&Object::Boolean(false))); // Show
}

#[test]
fn test_set_ocg_state_action() {
    let action = SetOCGStateAction::new()
        .turn_on(vec!["Layer1".to_string(), "Layer2".to_string()])
        .turn_off(vec!["Layer3".to_string()])
        .toggle(vec!["Layer4".to_string()]);

    let dict = action.to_dict();

    assert_eq!(
        dict.get("S"),
        Some(&Object::Name("SetOCGState".to_string()))
    );
    assert_eq!(dict.get("PreserveRB"), Some(&Object::Boolean(true)));

    if let Some(Object::Array(state_array)) = dict.get("State") {
        // Should have: ON, Layer1, Layer2, OFF, Layer3, Toggle, Layer4
        assert!(state_array.len() >= 7);
        assert_eq!(state_array[0], Object::Name("ON".to_string()));
        assert_eq!(state_array[3], Object::Name("OFF".to_string()));
    }
}

#[test]
fn test_javascript_action() {
    let script = "app.alert('Hello from PDF!');";
    let action = JavaScriptAction::new(script);
    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("JavaScript".to_string())));
    assert_eq!(dict.get("JS"), Some(&Object::String(script.to_string())));
}

#[test]
fn test_javascript_action_complex() {
    let script = r#"
        var field = this.getField("total");
        var qty = this.getField("quantity").value;
        var price = this.getField("price").value;
        field.value = qty * price;
    "#;

    let action = JavaScriptAction::new(script);
    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("JavaScript".to_string())));
    assert!(dict.get("JS").is_some());
}

#[test]
fn test_sound_action() {
    let action = SoundAction::new("bell.wav")
        .with_volume(0.5)
        .synchronous()
        .repeat();

    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("Sound".to_string())));
    assert_eq!(
        dict.get("Sound"),
        Some(&Object::String("bell.wav".to_string()))
    );
    assert_eq!(dict.get("Volume"), Some(&Object::Real(0.5)));
    assert_eq!(dict.get("Synchronous"), Some(&Object::Boolean(true)));
    assert_eq!(dict.get("Repeat"), Some(&Object::Boolean(true)));
    assert_eq!(dict.get("Mix"), Some(&Object::Boolean(false)));
}

#[test]
fn test_sound_action_volume_clamping() {
    let action1 = SoundAction::new("sound.wav").with_volume(-0.5);
    let dict1 = action1.to_dict();
    assert_eq!(dict1.get("Volume"), Some(&Object::Real(0.0)));

    let action2 = SoundAction::new("sound.wav").with_volume(1.5);
    let dict2 = action2.to_dict();
    assert_eq!(dict2.get("Volume"), Some(&Object::Real(1.0)));
}

#[test]
fn test_submit_form_all_flags() {
    let flags = SubmitFormFlags {
        include: false,
        html: true,
        coordinates: true,
        xml: true,
        include_annotations: true,
        pdf: true,
        canonical_dates: true,
        exclude_non_user: true,
        include_no_value_fields: true,
        export_format: true,
    };

    let flags_value = flags.to_flags();

    // Test each flag bit
    assert!(flags_value & (1 << 0) != 0); // !include
    assert!(flags_value & (1 << 2) != 0); // html
    assert!(flags_value & (1 << 4) != 0); // coordinates
    assert!(flags_value & (1 << 5) != 0); // xml
    assert!(flags_value & (1 << 7) != 0); // include_annotations
    assert!(flags_value & (1 << 8) != 0); // pdf
    assert!(flags_value & (1 << 9) != 0); // canonical_dates
    assert!(flags_value & (1 << 10) != 0); // exclude_non_user
    assert!(flags_value & (1 << 11) != 0); // include_no_value_fields
    assert!(flags_value & (1 << 12) != 0); // export_format
}

#[test]
fn test_ocg_state_changes() {
    // Test all three types of changes
    let on_change = OCGStateChange::On(vec!["Layer1".to_string()]);
    let off_change = OCGStateChange::Off(vec!["Layer2".to_string()]);
    let toggle_change = OCGStateChange::Toggle(vec!["Layer3".to_string()]);

    let action = SetOCGStateAction {
        state_changes: vec![on_change, off_change, toggle_change],
        preserve_rb: false,
    };

    let dict = action.to_dict();
    assert_eq!(dict.get("PreserveRB"), Some(&Object::Boolean(false)));
}

#[test]
fn test_hide_action_builder() {
    let targets = vec!["field1".to_string(), "field2".to_string()];

    // Test hide
    let hide_action = HideAction::new(targets.clone()).hide();
    let hide_dict = hide_action.to_dict();
    assert_eq!(hide_dict.get("H"), Some(&Object::Boolean(true)));

    // Test show
    let show_action = HideAction::new(targets).show();
    let show_dict = show_action.to_dict();
    assert_eq!(show_dict.get("H"), Some(&Object::Boolean(false)));
}

#[test]
fn test_submit_form_empty_fields() {
    let action = SubmitFormAction::new("https://example.com/submit");
    let dict = action.to_dict();

    // No fields array means submit all fields
    assert!(dict.get("Fields").is_none());
}

#[test]
fn test_reset_form_default() {
    let action = ResetFormAction::default();
    let dict = action.to_dict();

    assert_eq!(dict.get("S"), Some(&Object::Name("ResetForm".to_string())));
    assert!(dict.get("Fields").is_none()); // Reset all
}

#[test]
fn test_set_ocg_state_default() {
    let action = SetOCGStateAction::default();
    let dict = action.to_dict();

    assert_eq!(
        dict.get("S"),
        Some(&Object::Name("SetOCGState".to_string()))
    );
    assert_eq!(dict.get("PreserveRB"), Some(&Object::Boolean(true))); // Default true

    if let Some(Object::Array(state)) = dict.get("State") {
        assert_eq!(state.len(), 0); // No changes by default
    }
}

#[test]
fn test_submit_form_url_types() {
    // Test with different URL formats
    let urls = vec![
        "https://example.com/submit",
        "http://localhost:8080/form",
        "ftp://ftp.example.com/upload",
        "/relative/path",
        "mailto:test@example.com",
    ];

    for url in urls {
        let action = SubmitFormAction::new(url);
        let dict = action.to_dict();
        assert_eq!(dict.get("F"), Some(&Object::String(url.to_string())));
    }
}

#[test]
fn test_import_data_file_types() {
    let file_types = vec![
        "data.fdf",
        "form_data.xfdf",
        "/absolute/path/data.fdf",
        "relative/path/data.xfdf",
    ];

    for file in file_types {
        let action = ImportDataAction::new(file);
        let dict = action.to_dict();
        assert_eq!(dict.get("F"), Some(&Object::String(file.to_string())));
    }
}

#[test]
fn test_sound_action_defaults() {
    let action = SoundAction::new("default.wav");
    let dict = action.to_dict();

    assert_eq!(dict.get("Volume"), Some(&Object::Real(1.0))); // Default volume
    assert_eq!(dict.get("Synchronous"), Some(&Object::Boolean(false))); // Default async
    assert_eq!(dict.get("Repeat"), Some(&Object::Boolean(false))); // Default no repeat
    assert_eq!(dict.get("Mix"), Some(&Object::Boolean(false))); // Default no mix
}
