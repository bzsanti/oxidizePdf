use oxidize_pdf::pipeline::profile::ExtractionProfile;

// Cycle 4.1
#[test]
fn test_profile_standard_is_default() {
    let profile = ExtractionProfile::default();
    assert!(matches!(profile, ExtractionProfile::Standard));
}

#[test]
fn test_all_profiles_are_constructible() {
    let _ = ExtractionProfile::Standard;
    let _ = ExtractionProfile::Academic;
    let _ = ExtractionProfile::Form;
    let _ = ExtractionProfile::Government;
    let _ = ExtractionProfile::Dense;
    let _ = ExtractionProfile::Presentation;
}

// Cycle 4.2
#[test]
fn test_standard_profile_matches_current_defaults() {
    let profile_cfg = ExtractionProfile::Standard.config();
    assert!((profile_cfg.extraction.space_threshold - 0.3).abs() < f64::EPSILON);
    assert!((profile_cfg.partition.title_min_font_ratio - 1.3).abs() < f64::EPSILON);
    assert!((profile_cfg.partition.header_zone - 0.05).abs() < f64::EPSILON);
}

// Cycle 4.3
#[test]
fn test_academic_profile_enables_column_detection() {
    let cfg = ExtractionProfile::Academic.config();
    assert!(cfg.extraction.detect_columns);
    assert!(cfg.extraction.space_threshold < 0.3);
    assert!(cfg.partition.title_min_font_ratio > 1.3);
}

// Cycle 4.4
#[test]
fn test_form_profile_has_high_title_ratio() {
    let cfg = ExtractionProfile::Form.config();
    assert!(cfg.partition.title_min_font_ratio >= 1.5);
    assert!(!cfg.extraction.detect_columns);
}

// Cycle 4.5
#[test]
fn test_dense_profile_has_lower_space_threshold() {
    let cfg = ExtractionProfile::Dense.config();
    assert!(cfg.extraction.space_threshold < 0.3);
}

// Cycle 4.6
#[test]
fn test_presentation_profile_has_lower_title_ratio() {
    let cfg = ExtractionProfile::Presentation.config();
    assert!(cfg.partition.title_min_font_ratio < 1.3);
    assert!(cfg.extraction.space_threshold > 0.3);
}

// Cycle 4.7
#[test]
fn test_partition_with_profile_standard_backwards_compatible() {
    let fixture = format!(
        "{}/tests/fixtures/Cold_Email_Hacks.pdf",
        env!("CARGO_MANIFEST_DIR")
    );
    let doc = oxidize_pdf::parser::PdfDocument::open(&fixture).unwrap();
    let default_elements = doc.partition().unwrap();
    let profile_elements = doc
        .partition_with_profile(oxidize_pdf::pipeline::ExtractionProfile::Standard)
        .unwrap();
    assert_eq!(default_elements.len(), profile_elements.len());
}

// Cycle 4.8
#[test]
fn test_all_profiles_produce_valid_config() {
    let profiles = [
        ExtractionProfile::Standard,
        ExtractionProfile::Academic,
        ExtractionProfile::Form,
        ExtractionProfile::Government,
        ExtractionProfile::Dense,
        ExtractionProfile::Presentation,
        ExtractionProfile::Rag,
    ];
    for profile in &profiles {
        let cfg = profile.config();
        assert!(cfg.extraction.space_threshold > 0.0);
        assert!(cfg.extraction.space_threshold < 2.0);
        assert!(cfg.partition.title_min_font_ratio > 1.0);
        assert!(cfg.partition.header_zone > 0.0);
        assert!(cfg.partition.footer_zone > 0.0);
    }
}

// ── ExtractionProfile::Rag ─────────────────────────────────────────────────

#[test]
fn test_rag_profile_column_detection_off_xycut_handles_layout() {
    let cfg = ExtractionProfile::Rag.config();
    // Column detection disabled (known overflow bug with some PDFs).
    // XYCut reading order handles multi-column layout ordering instead.
    assert!(!cfg.extraction.detect_columns);
}

#[test]
fn test_rag_profile_uses_xycut_reading_order() {
    use oxidize_pdf::pipeline::ReadingOrderStrategy;
    let cfg = ExtractionProfile::Rag.config();
    assert!(matches!(
        cfg.partition.reading_order,
        ReadingOrderStrategy::XYCut { .. }
    ));
}

#[test]
fn test_rag_profile_space_threshold_is_default() {
    let cfg = ExtractionProfile::Rag.config();
    assert!(
        (cfg.extraction.space_threshold - 0.3).abs() < f64::EPSILON,
        "got {}",
        cfg.extraction.space_threshold
    );
}

#[test]
fn test_rag_profile_has_elevated_table_confidence() {
    let cfg = ExtractionProfile::Rag.config();
    assert!(
        cfg.partition.min_table_confidence > 0.5,
        "got {}",
        cfg.partition.min_table_confidence
    );
}
