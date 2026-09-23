//! Example: Key-value pair extraction from PDF documents
//!
//! This example demonstrates how to automatically detect and extract
//! key-value pairs from forms and structured documents using multiple
//! pattern matching strategies.

use oxidize_pdf::text::extraction::TextFragment;
use oxidize_pdf::text::structured::{KeyValuePattern, StructuredDataDetector};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PDF Key-Value Extraction Demo ===\n");

    demo_colon_pattern()?;
    demo_spatial_pattern()?;
    demo_tabular_pattern()?;
    demo_mixed_patterns()?;

    println!("\n=== Example completed successfully ===");
    Ok(())
}

fn demo_colon_pattern() -> Result<(), Box<dyn std::error::Error>> {
    println!("1. Colon-separated pattern (\"Label: Value\")");
    println!("   Common in forms and documents\n");

    let fragments = vec![
        TextFragment::new("Name: John Doe".to_string(), 100.0, 700.0, 80.0, 12.0, 12.0),
        TextFragment::new(
            "Email: john@example.com".to_string(),
            100.0,
            680.0,
            120.0,
            12.0,
            12.0,
        ),
        TextFragment::new(
            "Phone: (555) 123-4567".to_string(),
            100.0,
            660.0,
            110.0,
            12.0,
            12.0,
        ),
    ];

    let detector = StructuredDataDetector::default();
    let result = detector.detect(&fragments)?;

    println!("   Found {} key-value pairs:", result.key_value_pairs.len());
    for pair in &result.key_value_pairs {
        println!(
            "   {} = {} (confidence: {:.0}%)",
            pair.key,
            pair.value,
            pair.confidence * 100.0
        );
    }
    println!();

    Ok(())
}

fn demo_spatial_pattern() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. Spatially-aligned pattern (\"Label      Value\")");
    println!("   Common in invoices and receipts\n");

    let fragments = vec![
        // First line with significant gap
        TextFragment::new("Subtotal".to_string(), 100.0, 700.0, 50.0, 12.0, 12.0),
        // Gap of 150 units from the preceding fragment.
        TextFragment::new("$125.00", 300.0, 700.0, 50.0, 12.0, 12.0),
        // Second line
        TextFragment::new("Tax".to_string(), 100.0, 680.0, 30.0, 12.0, 12.0),
        TextFragment::new("$12.50".to_string(), 300.0, 680.0, 45.0, 12.0, 12.0),
    ];

    let detector = StructuredDataDetector::default();
    let result = detector.detect(&fragments)?;

    println!("   Found {} key-value pairs:", result.key_value_pairs.len());
    for pair in result
        .key_value_pairs
        .iter()
        .filter(|p| p.pattern == KeyValuePattern::SpatialAlignment)
    {
        println!(
            "   {} = {} (confidence: {:.0}%)",
            pair.key,
            pair.value,
            pair.confidence * 100.0
        );
    }
    println!();

    Ok(())
}

fn demo_tabular_pattern() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Tab-separated pattern (\"Label\\tValue\")");
    println!("   Common in exported data\n");

    let fragments = vec![
        TextFragment::new("Status\tActive".to_string(), 100.0, 700.0, 80.0, 12.0, 12.0),
        TextFragment::new("Priority\tHigh".to_string(), 100.0, 680.0, 70.0, 12.0, 12.0),
    ];

    let detector = StructuredDataDetector::default();
    let result = detector.detect(&fragments)?;

    println!("   Found {} key-value pairs:", result.key_value_pairs.len());
    for pair in result
        .key_value_pairs
        .iter()
        .filter(|p| p.pattern == KeyValuePattern::Tabular)
    {
        println!(
            "   {} = {} (confidence: {:.0}%)",
            pair.key,
            pair.value,
            pair.confidence * 100.0
        );
    }
    println!();

    Ok(())
}

fn demo_mixed_patterns() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Mixed patterns (all types)");
    println!("   Real-world documents often combine multiple patterns\n");

    let fragments = vec![
        // Colon-separated
        TextFragment::new(
            "Invoice #: INV-2025-001".to_string(),
            100.0,
            750.0,
            120.0,
            12.0,
            12.0,
        ),
        TextFragment::new(
            "Date: 2025-10-20".to_string(),
            100.0,
            730.0,
            90.0,
            12.0,
            12.0,
        ),
        // Spatially aligned
        TextFragment::new("Customer".to_string(), 100.0, 700.0, 60.0, 12.0, 12.0),
        TextFragment::new("Acme Corp".to_string(), 250.0, 700.0, 70.0, 12.0, 12.0),
        // Tab-separated
        TextFragment::new("Terms\t30 days".to_string(), 100.0, 680.0, 80.0, 12.0, 12.0),
    ];

    let detector = StructuredDataDetector::default();
    let result = detector.detect(&fragments)?;

    println!("   Found {} key-value pairs:", result.key_value_pairs.len());
    for pair in &result.key_value_pairs {
        let pattern_name = match pair.pattern {
            KeyValuePattern::ColonSeparated => "colon",
            KeyValuePattern::SpatialAlignment => "spatial",
            KeyValuePattern::Tabular => "tabular",
        };
        println!(
            "   {} = {} ({}, {:.0}%)",
            pair.key,
            pair.value,
            pattern_name,
            pair.confidence * 100.0
        );
    }

    // Export as JSON
    println!("\n   JSON Export:");
    let json_data = json!({
        "key_value_pairs": result.key_value_pairs.iter().map(|pair| {
            json!({
                "key": pair.key,
                "value": pair.value,
                "pattern": format!("{:?}", pair.pattern),
                "confidence": pair.confidence
            })
        }).collect::<Vec<_>>()
    });
    println!("{}", serde_json::to_string_pretty(&json_data)?);

    Ok(())
}
