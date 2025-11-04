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
        TextFragment {
            text: "Name: John Doe".to_string(),
            x: 100.0,
            y: 700.0,
            width: 80.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        TextFragment {
            text: "Email: john@example.com".to_string(),
            x: 100.0,
            y: 680.0,
            width: 120.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        TextFragment {
            text: "Phone: (555) 123-4567".to_string(),
            x: 100.0,
            y: 660.0,
            width: 110.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
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
        TextFragment {
            text: "Subtotal".to_string(),
            x: 100.0,
            y: 700.0,
            width: 50.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        TextFragment {
            text: "$125.00".to_string(),
            x: 300.0, // Gap of 150 units
            y: 700.0,
            width: 50.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        // Second line
        TextFragment {
            text: "Tax".to_string(),
            x: 100.0,
            y: 680.0,
            width: 30.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        TextFragment {
            text: "$12.50".to_string(),
            x: 300.0,
            y: 680.0,
            width: 45.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
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
        TextFragment {
            text: "Status\tActive".to_string(),
            x: 100.0,
            y: 700.0,
            width: 80.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        TextFragment {
            text: "Priority\tHigh".to_string(),
            x: 100.0,
            y: 680.0,
            width: 70.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
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
        TextFragment {
            text: "Invoice #: INV-2025-001".to_string(),
            x: 100.0,
            y: 750.0,
            width: 120.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        TextFragment {
            text: "Date: 2025-10-20".to_string(),
            x: 100.0,
            y: 730.0,
            width: 90.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        // Spatially aligned
        TextFragment {
            text: "Customer".to_string(),
            x: 100.0,
            y: 700.0,
            width: 60.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        TextFragment {
            text: "Acme Corp".to_string(),
            x: 250.0,
            y: 700.0,
            width: 70.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
        // Tab-separated
        TextFragment {
            text: "Terms\t30 days".to_string(),
            x: 100.0,
            y: 680.0,
            width: 80.0,
            height: 12.0,
            font_size: 12.0,
            font_name: None,
            is_bold: false,
            is_italic: false,
            color: None,
        },
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
