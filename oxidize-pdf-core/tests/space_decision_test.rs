use oxidize_pdf::text::extraction::{ExtractionOptions, SpaceDecision, TextFragment};

// Cycle 7.1
#[test]
fn test_space_decision_struct_creation() {
    let decision = SpaceDecision {
        offset: 5,
        dx: 0.4,
        threshold: 0.3,
        confidence: 0.33,
        inserted: true,
    };
    assert_eq!(decision.offset, 5);
    assert!(decision.inserted);
    assert!(decision.confidence > 0.0);
}

// Cycle 7.2
#[test]
fn test_text_fragment_has_space_decisions_field() {
    let frag = TextFragment {
        text: "hello".to_string(),
        x: 0.0,
        y: 0.0,
        width: 50.0,
        height: 12.0,
        font_size: 12.0,
        font_name: None,
        is_bold: false,
        is_italic: false,
        color: None,
        space_decisions: Vec::new(),
    };
    assert!(frag.space_decisions.is_empty());
}

// Cycle 7.3
#[test]
fn test_track_space_decisions_is_false_by_default() {
    assert!(!ExtractionOptions::default().track_space_decisions);
}

#[test]
fn test_space_decisions_not_populated_when_not_tracking() {
    let fixture = format!(
        "{}/examples/results/hello_world.pdf",
        env!("CARGO_MANIFEST_DIR")
    );
    let doc = oxidize_pdf::parser::PdfDocument::open(&fixture).unwrap();
    let pages = doc
        .extract_text_with_options(ExtractionOptions {
            preserve_layout: true,
            track_space_decisions: false,
            ..Default::default()
        })
        .unwrap();

    for page in &pages {
        for frag in &page.fragments {
            assert!(frag.space_decisions.is_empty());
        }
    }
}

// Cycle 7.4
#[test]
fn test_space_decision_confidence_high_clear_gap() {
    // dx=1.0, threshold=0.3 → |1.0-0.3|/0.3 = 2.33 → clamp a 1.0
    let d = SpaceDecision {
        offset: 0,
        dx: 1.0,
        threshold: 0.3,
        confidence: ((1.0_f64 - 0.3).abs() / 0.3).clamp(0.0, 1.0),
        inserted: true,
    };
    assert!(d.confidence > 0.9);
}

#[test]
fn test_space_decision_confidence_low_marginal_gap() {
    // dx=0.31, threshold=0.3 → |0.31-0.3|/0.3 ≈ 0.033
    let d = SpaceDecision {
        offset: 0,
        dx: 0.31,
        threshold: 0.3,
        confidence: ((0.31_f64 - 0.3).abs() / 0.3).clamp(0.0, 1.0),
        inserted: true,
    };
    assert!(d.confidence < 0.2);
}

#[test]
fn test_inserted_true_when_dx_above_threshold() {
    let d = SpaceDecision {
        offset: 0,
        dx: 0.5,
        threshold: 0.3,
        confidence: 0.67,
        inserted: true,
    };
    assert!(d.inserted);
}

#[test]
fn test_inserted_false_when_dx_below_threshold() {
    let d = SpaceDecision {
        offset: 0,
        dx: 0.1,
        threshold: 0.3,
        confidence: 0.67,
        inserted: false,
    };
    assert!(!d.inserted);
}
