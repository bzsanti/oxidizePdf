//! Tests for public pattern API
//!
//! Tests that users can successfully use the pattern customization API to:
//! - Create custom pattern libraries
//! - Extend default patterns
//! - Merge multiple libraries
//! - Use custom patterns with InvoiceExtractor

use oxidize_pdf::text::invoice::{
    FieldPattern, InvoiceExtractor, InvoiceFieldType, Language, PatternLibrary,
};

/// Test creating empty PatternLibrary and adding custom patterns
#[test]
fn test_create_empty_pattern_library() {
    let mut patterns = PatternLibrary::new();

    // Add a custom invoice number pattern
    let pattern = FieldPattern::new(
        InvoiceFieldType::InvoiceNumber,
        r"Order\s+#([0-9]+)",
        0.9,
        None,
    )
    .expect("Failed to create pattern");

    patterns.add_pattern(pattern);

    // Verify pattern matches
    let text = "Order #12345";
    let matches = patterns.match_text(text);

    assert_eq!(matches.len(), 1, "Should find 1 match");
    assert_eq!(matches[0].1, "12345", "Should extract order number");
    assert_eq!(matches[0].2, 0.9, "Should have correct confidence");
}

/// Test using default_spanish() constructor
#[test]
fn test_default_spanish_patterns() {
    let patterns = PatternLibrary::default_spanish();

    // Test that Spanish patterns work
    let text = "Factura Nº: 2025-001\nFecha: 20/01/2025\nTotal: 1.234,56€";
    let matches = patterns.match_text(text);

    // Should match at least invoice number, date, and total
    assert!(matches.len() >= 3, "Should match multiple Spanish patterns");

    // Verify invoice number was found
    let has_invoice_num = matches
        .iter()
        .any(|(field_type, _, _)| matches!(field_type, InvoiceFieldType::InvoiceNumber));
    assert!(has_invoice_num, "Should extract Spanish invoice number");
}

/// Test using default_english() constructor
#[test]
fn test_default_english_patterns() {
    let patterns = PatternLibrary::default_english();

    // Test that English patterns work
    let text = "Invoice Number: INV-2025-001\nDate: 01/20/2025\nTotal: £1,234.56";
    let matches = patterns.match_text(text);

    // Should match at least invoice number, date, and total
    assert!(matches.len() >= 3, "Should match multiple English patterns");

    // Verify invoice number was found
    let has_invoice_num = matches
        .iter()
        .any(|(field_type, _, _)| matches!(field_type, InvoiceFieldType::InvoiceNumber));
    assert!(has_invoice_num, "Should extract English invoice number");
}

/// Test extending default patterns with custom ones
#[test]
fn test_extend_default_patterns() {
    // Start with Spanish defaults
    let mut patterns = PatternLibrary::default_spanish();

    // Add custom pattern for specific format
    patterns.add_pattern(
        FieldPattern::new(
            InvoiceFieldType::InvoiceNumber,
            r"Ref:\s*([A-Z0-9\-]+)",
            0.85,
            Some(Language::Spanish),
        )
        .expect("Failed to create custom pattern"),
    );

    // Test that both default and custom patterns work
    let text1 = "Factura Nº: 2025-001"; // Default pattern
    let text2 = "Ref: CUSTOM-123"; // Custom pattern

    let matches1 = patterns.match_text(text1);
    let matches2 = patterns.match_text(text2);

    assert_eq!(matches1.len(), 1, "Default pattern should work");
    assert_eq!(matches1[0].1, "2025-001");

    assert_eq!(matches2.len(), 1, "Custom pattern should work");
    assert_eq!(matches2[0].1, "CUSTOM-123");
}

/// Test merging two pattern libraries
#[test]
fn test_merge_pattern_libraries() {
    // Create first library with Spanish patterns
    let mut spanish = PatternLibrary::default_spanish();

    // Create second library with custom patterns
    let mut custom = PatternLibrary::new();
    custom.add_pattern(
        FieldPattern::new(
            InvoiceFieldType::InvoiceNumber,
            r"Order\s+#([0-9]+)",
            0.8,
            None,
        )
        .unwrap(),
    );

    // Merge custom into spanish
    spanish.merge(custom);

    // Test that both work
    let text1 = "Factura Nº: 2025-001"; // Spanish pattern
    let text2 = "Order #9999"; // Custom pattern

    let matches1 = spanish.match_text(text1);
    let matches2 = spanish.match_text(text2);

    assert_eq!(matches1.len(), 1, "Spanish pattern should still work");
    assert_eq!(matches2.len(), 1, "Custom pattern should work after merge");
}

/// Test using custom patterns with InvoiceExtractor
#[test]
fn test_extractor_with_custom_patterns() {
    // Create custom pattern library
    let mut patterns = PatternLibrary::new();
    patterns.add_pattern(
        FieldPattern::new(
            InvoiceFieldType::InvoiceNumber,
            r"Order\s+#([0-9]+)",
            0.9,
            None,
        )
        .unwrap(),
    );
    patterns.add_pattern(
        FieldPattern::new(
            InvoiceFieldType::TotalAmount,
            r"Amount:\s*\$([0-9,]+\.[0-9]{2})",
            0.9,
            None,
        )
        .unwrap(),
    );

    // Build extractor with custom patterns
    let extractor = InvoiceExtractor::builder()
        .with_custom_patterns(patterns)
        .confidence_threshold(0.8)
        .build();

    // Test extraction
    let invoice_text = "Order #12345\nAmount: $1,234.56";
    let result = extractor
        .extract_from_text(invoice_text)
        .expect("Extraction should succeed");

    assert_eq!(
        result.fields.len(),
        2,
        "Should extract 2 fields with custom patterns"
    );
}

/// Test that with_custom_patterns() overrides with_language()
#[test]
fn test_custom_patterns_override_language() {
    // Create minimal custom library (NOT Spanish)
    let mut patterns = PatternLibrary::new();
    patterns.add_pattern(
        FieldPattern::new(
            InvoiceFieldType::InvoiceNumber,
            r"Order\s+#([0-9]+)",
            0.9,
            None,
        )
        .unwrap(),
    );

    // Build with both language and custom patterns
    let extractor = InvoiceExtractor::builder()
        .with_language("es") // This should be ignored
        .with_custom_patterns(patterns)
        .build();

    // Spanish pattern should NOT work
    let spanish_text = "Factura Nº: 2025-001";
    let result1 = extractor.extract_from_text(spanish_text);
    let fields1 = result1.unwrap().fields;
    assert_eq!(
        fields1.len(),
        0,
        "Spanish pattern should NOT work (overridden by custom)"
    );

    // Custom pattern should work
    let custom_text = "Order #9999";
    let result2 = extractor.extract_from_text(custom_text);
    let fields2 = result2.unwrap().fields;
    assert_eq!(fields2.len(), 1, "Custom pattern should work");
}

/// Test combining default patterns with custom additions
#[test]
fn test_combine_default_and_custom() {
    // Start with German defaults
    let mut patterns = PatternLibrary::default_german();

    // Add custom pattern (alphanumeric format)
    patterns.add_pattern(
        FieldPattern::new(
            InvoiceFieldType::InvoiceNumber,
            r"Bestellnummer:\s*([A-Z0-9\-]+)",
            0.85,
            Some(Language::German),
        )
        .unwrap(),
    );

    let extractor = InvoiceExtractor::builder()
        .with_custom_patterns(patterns)
        .confidence_threshold(0.7)
        .build();

    // Test default German pattern
    let text1 = "Rechnung Nr. 2025-001";
    let result1 = extractor.extract_from_text(text1).unwrap();
    assert!(
        !result1.fields.is_empty(),
        "Default German pattern should work"
    );

    // Test custom pattern
    let text2 = "Bestellnummer: CUSTOM-999";
    let result2 = extractor.extract_from_text(text2).unwrap();
    assert_eq!(result2.fields.len(), 1, "Custom pattern should work");
}

/// Test that PatternLibrary is Send + Sync (thread-safe)
#[test]
fn test_pattern_library_is_thread_safe() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<PatternLibrary>();
    assert_sync::<PatternLibrary>();
}
