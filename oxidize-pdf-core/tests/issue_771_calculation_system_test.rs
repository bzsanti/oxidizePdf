use oxidize_pdf::forms::calculation_system::{
    FormCalculationSystem, JavaScriptCalculation, SimpleOperation,
};
use oxidize_pdf::forms::calculations::FieldValue;

fn sum(fields: &[&str]) -> JavaScriptCalculation {
    JavaScriptCalculation::SimpleCalculate {
        operation: SimpleOperation::Sum,
        fields: fields.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn updates_propagate_to_transitive_dependents() {
    let mut system = FormCalculationSystem::new();
    system
        .set_field_value("a", FieldValue::Number(2.0))
        .unwrap();
    system.add_js_calculation("b", sum(&["a"])).unwrap();
    system.add_js_calculation("c", sum(&["b"])).unwrap();
    system.clear_events();
    system
        .set_field_value("a", FieldValue::Number(7.0))
        .unwrap();
    assert_eq!(
        system.get_recent_events(10).len(),
        3,
        "one input event and both dependent calculations"
    );
}

use oxidize_pdf::forms::calculation_system::{CalculationSettings, EventType, PercentMode};
use oxidize_pdf::PdfError;

fn value(system: &FormCalculationSystem, field: &str, expected: f64) {
    assert_eq!(
        system.get_field_value(field),
        Some(&FieldValue::Number(expected)),
        "{field}"
    );
}

#[test]
fn typed_diamond_values_events_and_unaffected_fields() {
    let mut system = FormCalculationSystem::new();
    system
        .set_field_value("a", FieldValue::Number(2.0))
        .unwrap();
    system.add_js_calculation("b", sum(&["a", "a"])).unwrap();
    system
        .add_js_calculation("c", sum(&["a", "a", "a"]))
        .unwrap();
    system.add_js_calculation("d", sum(&["b", "c"])).unwrap();
    system
        .set_field_value("unrelated", FieldValue::Text("keep".into()))
        .unwrap();
    system.clear_events();
    let before = chrono::Utc::now();
    system
        .set_field_value("a", FieldValue::Number(7.0))
        .unwrap();
    let after = chrono::Utc::now();
    value(&system, "b", 14.0);
    value(&system, "c", 21.0);
    value(&system, "d", 35.0);
    assert_eq!(
        system.get_field_value("unrelated"),
        Some(&FieldValue::Text("keep".into()))
    );
    let events = system.get_recent_events(10);
    assert_eq!(
        events.iter().map(|e| e.field()).collect::<Vec<_>>(),
        ["a", "b", "c", "d"]
    );
    for (index, event) in events.iter().enumerate() {
        assert!(*event.timestamp() >= before && *event.timestamp() <= after);
        assert_eq!(
            event.event_type(),
            if index == 0 {
                &EventType::ValueChanged
            } else {
                &EventType::CalculationTriggered
            }
        );
        assert_eq!(
            event.old_value(),
            Some(&FieldValue::Number([2.0, 4.0, 6.0, 10.0][index]))
        );
        assert_eq!(
            event.new_value(),
            Some(&FieldValue::Number([7.0, 14.0, 21.0, 35.0][index]))
        );
    }
    // A rejected replacement must preserve the previous graph and all values/events.
    assert!(matches!(
        system.add_js_calculation("b", sum(&["d"])),
        Err(PdfError::InvalidStructure(_))
    ));
    assert_eq!(system.get_recent_events(10).len(), 4);
    system
        .set_field_value("a", FieldValue::Number(3.0))
        .unwrap();
    value(&system, "d", 15.0);
}

#[test]
fn replacement_propagates_and_missing_inputs_keep_documented_zero_semantics() {
    let mut system = FormCalculationSystem::new();
    system.add_js_calculation("b", sum(&["missing"])).unwrap();
    system.add_js_calculation("c", sum(&["b"])).unwrap();
    value(&system, "c", 0.0);
    assert_eq!(system.get_field_value("missing"), None);
    system
        .set_field_value("missing", FieldValue::Number(4.0))
        .unwrap();
    value(&system, "c", 4.0);
    system
        .add_js_calculation("b", sum(&["missing", "missing"]))
        .unwrap();
    value(&system, "c", 8.0);
}

#[test]
fn disabled_recalculation_and_logging_are_observable() {
    let mut system = FormCalculationSystem::with_settings(CalculationSettings {
        auto_recalculate: false,
        log_events: false,
        ..Default::default()
    });
    system
        .set_field_value("a", FieldValue::Number(2.0))
        .unwrap();
    system.add_js_calculation("b", sum(&["a"])).unwrap();
    system
        .set_field_value("a", FieldValue::Number(7.0))
        .unwrap();
    value(&system, "a", 7.0);
    value(&system, "b", 2.0);
    assert!(system.get_recent_events(100).is_empty());
    assert!(!system.settings().auto_recalculate);
    assert!(!system.settings().log_events);
}

#[test]
fn depth_error_is_atomic_for_values_and_events() {
    let mut system = FormCalculationSystem::with_settings(CalculationSettings {
        max_depth: 1,
        ..Default::default()
    });
    system
        .set_field_value("a", FieldValue::Number(2.0))
        .unwrap();
    system.add_js_calculation("b", sum(&["a"])).unwrap();
    system.add_js_calculation("c", sum(&["b"])).unwrap();
    system.clear_events();
    assert!(matches!(
        system.set_field_value("a", FieldValue::Number(7.0)),
        Err(PdfError::InvalidOperation(_))
    ));
    for field in ["a", "b", "c"] {
        value(&system, field, 2.0);
    }
    assert!(system.get_recent_events(100).is_empty());
}

#[test]
fn arithmetic_overflow_is_an_error_and_restores_state() {
    let mut system = FormCalculationSystem::new();
    system
        .set_field_value("a", FieldValue::Number(2.0))
        .unwrap();
    system.add_js_calculation("b", sum(&["a", "a"])).unwrap();
    system.add_js_calculation("c", sum(&["b"])).unwrap();
    system.clear_events();
    assert!(matches!(
        system.set_field_value("a", FieldValue::Number(f64::MAX)),
        Err(PdfError::InvalidOperation(_))
    ));
    value(&system, "a", 2.0);
    value(&system, "b", 4.0);
    value(&system, "c", 4.0);
    assert!(system.get_recent_events(100).is_empty());
}

#[test]
fn invalid_range_returns_error_instead_of_panicking_or_replacing_definition() {
    let mut system = FormCalculationSystem::new();
    system
        .set_field_value("a", FieldValue::Number(2.0))
        .unwrap();
    system.add_js_calculation("b", sum(&["a"])).unwrap();
    assert!(matches!(
        system.add_js_calculation(
            "b",
            JavaScriptCalculation::RangeCalculate {
                field: "a".into(),
                min: Some(10.0),
                max: Some(1.0)
            }
        ),
        Err(PdfError::InvalidOperation(_))
    ));
    system
        .set_field_value("a", FieldValue::Number(3.0))
        .unwrap();
    value(&system, "b", 3.0);
}

#[test]
fn zero_percentage_base_preserves_existing_zero_result() {
    let mut system = FormCalculationSystem::new();
    system
        .set_field_value("base", FieldValue::Number(0.0))
        .unwrap();
    system
        .set_field_value("part", FieldValue::Number(12.0))
        .unwrap();
    system
        .add_js_calculation(
            "result",
            JavaScriptCalculation::PercentCalculate {
                base_field: "base".into(),
                percent_field: "part".into(),
                mode: PercentMode::PercentageOf,
            },
        )
        .unwrap();
    value(&system, "result", 0.0);
}

#[test]
fn out_of_range_dates_return_errors_and_do_not_install_calculations() {
    let mut system = FormCalculationSystem::new();
    system
        .set_field_value("date", FieldValue::Text("2026-10-18".into()))
        .unwrap();
    system
        .set_field_value("days", FieldValue::Number(f64::MAX))
        .unwrap();
    assert!(matches!(
        system.add_js_calculation(
            "later",
            JavaScriptCalculation::DateCalculate {
                start_date_field: "date".into(),
                days_field: Some("days".into()),
                format: "%Y-%m-%d".into()
            }
        ),
        Err(PdfError::InvalidOperation(_))
    ));
    assert_eq!(system.get_field_value("later"), None);
    assert_eq!(system.get_summary().js_calculations, 0);
}

#[test]
fn numeric_input_and_format_errors_leave_no_success_events() {
    use oxidize_pdf::forms::calculation_system::{FieldFormat, SeparatorStyle};
    let mut system = FormCalculationSystem::new();
    for input in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            system.set_field_value("bad", FieldValue::Number(input)),
            Err(PdfError::InvalidOperation(_))
        ));
    }
    assert_eq!(system.get_field_value("bad"), None);
    assert!(system.get_recent_events(10).is_empty());
    system
        .set_field_value("a", FieldValue::Number(1.234))
        .unwrap();
    system
        .add_js_calculation(
            "rounded",
            JavaScriptCalculation::NumberCalculate {
                field: "a".into(),
                decimals: 2,
                sep_style: SeparatorStyle::None,
                currency: None,
            },
        )
        .unwrap();
    value(&system, "rounded", 1.23);
    assert!(matches!(
        system.add_js_calculation(
            "rounded",
            JavaScriptCalculation::NumberCalculate {
                field: "a".into(),
                decimals: usize::MAX,
                sep_style: SeparatorStyle::None,
                currency: None
            }
        ),
        Err(PdfError::InvalidOperation(_))
    ));
    value(&system, "rounded", 1.23);
    system.set_field_format("percent", FieldFormat::Percent { decimals: 2 });
    system.add_js_calculation("percent", sum(&["a"])).unwrap();
    assert_eq!(
        system.get_field_value("percent"),
        Some(&FieldValue::Text("123.4%".into()))
    );
    system.clear_events();
    assert!(matches!(
        system.set_field_value("a", FieldValue::Number(f64::MAX)),
        Err(PdfError::InvalidOperation(_))
    ));
    value(&system, "a", 1.234);
    assert_eq!(
        system.get_field_value("percent"),
        Some(&FieldValue::Text("123.4%".into()))
    );
    assert!(system.get_recent_events(10).is_empty());
}

#[test]
fn ordinary_date_calculation_keeps_typed_text_values() {
    let mut system = FormCalculationSystem::new();
    system
        .set_field_value("date", FieldValue::Text("2024-02-28".into()))
        .unwrap();
    system
        .set_field_value("days", FieldValue::Number(1.0))
        .unwrap();
    system
        .add_js_calculation(
            "later",
            JavaScriptCalculation::DateCalculate {
                start_date_field: "date".into(),
                days_field: Some("days".into()),
                format: "%Y-%m-%d".into(),
            },
        )
        .unwrap();
    assert_eq!(
        system.get_field_value("later"),
        Some(&FieldValue::Text("2024-02-29".into()))
    );
    system
        .set_field_value("days", FieldValue::Number(2.0))
        .unwrap();
    assert_eq!(
        system.get_field_value("later"),
        Some(&FieldValue::Text("2024-03-01".into()))
    );
}

#[test]
fn topological_order_overrides_name_and_registration_order() {
    let mut system = FormCalculationSystem::new();
    system
        .set_field_value("input", FieldValue::Number(2.0))
        .unwrap();
    // Register the sink before its prerequisite; alphabetical order is also wrong.
    system
        .add_js_calculation("a_sink", sum(&["input", "z_middle"]))
        .unwrap();
    system
        .add_js_calculation("z_middle", sum(&["input", "input"]))
        .unwrap();
    value(&system, "a_sink", 6.0);
    system.clear_events();
    system
        .set_field_value("input", FieldValue::Number(7.0))
        .unwrap();
    value(&system, "z_middle", 14.0);
    value(&system, "a_sink", 21.0);
    assert_eq!(
        system
            .get_recent_events(10)
            .iter()
            .map(|e| e.field())
            .collect::<Vec<_>>(),
        ["input", "z_middle", "a_sink"]
    );
}
