//! Example: Table extraction from PDF documents
//!
//! This example demonstrates how to automatically detect and extract tables
//! from PDF documents using spatial clustering algorithms.

use oxidize_pdf_core::text::extraction::{TextExtractor, ExtractionOptions};
use oxidize_pdf_core::text::structured::{StructuredDataDetector, StructuredDataConfig};
use oxidize_pdf_core::parser::PdfReader;
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PDF Table Extraction Demo ===\n");

    // For this example, we'll create a synthetic dataset
    // In a real scenario, you would load a PDF with tables
    demo_table_detection()?;

    println!("\n=== Example completed successfully ===");
    Ok(())
}

fn demo_table_detection() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating table detection with synthetic data...\n");

    // Create synthetic text fragments representing a 3x3 table
    use oxidize_pdf_core::text::extraction::TextFragment;

    let fragments = vec![
        // Header row (Y = 700)
        TextFragment {
            text: "Name".to_string(),
            x: 100.0,
            y: 700.0,
            width: 50.0,
            height: 12.0,
            font_size: 12.0,
        },
        TextFragment {
            text: "Age".to_string(),
            x: 200.0,
            y: 700.0,
            width: 30.0,
            height: 12.0,
            font_size: 12.0,
        },
        TextFragment {
            text: "City".to_string(),
            x: 300.0,
            y: 700.0,
            width: 40.0,
            height: 12.0,
            font_size: 12.0,
        },
        // Data row 1 (Y = 680)
        TextFragment {
            text: "Alice".to_string(),
            x: 100.0,
            y: 680.0,
            width: 40.0,
            height: 12.0,
            font_size: 12.0,
        },
        TextFragment {
            text: "30".to_string(),
            x: 200.0,
            y: 680.0,
            width: 20.0,
            height: 12.0,
            font_size: 12.0,
        },
        TextFragment {
            text: "NYC".to_string(),
            x: 300.0,
            y: 680.0,
            width: 30.0,
            height: 12.0,
            font_size: 12.0,
        },
        // Data row 2 (Y = 660)
        TextFragment {
            text: "Bob".to_string(),
            x: 100.0,
            y: 660.0,
            width: 30.0,
            height: 12.0,
            font_size: 12.0,
        },
        TextFragment {
            text: "25".to_string(),
            x: 200.0,
            y: 660.0,
            width: 20.0,
            height: 12.0,
            font_size: 12.0,
        },
        TextFragment {
            text: "LA".to_string(),
            x: 300.0,
            y: 660.0,
            width: 20.0,
            height: 12.0,
            font_size: 12.0,
        },
    ];

    // Configure table detection
    let config = StructuredDataConfig::default()
        .with_min_table_rows(2)
        .with_min_table_columns(2)
        .with_column_tolerance(5.0)
        .with_row_tolerance(3.0);

    let detector = StructuredDataDetector::new(config);

    // Detect tables
    let result = detector.detect(&fragments)?;

    // Display results
    println!("Detected {} table(s)\n", result.tables.len());

    for (idx, table) in result.tables.iter().enumerate() {
        println!("Table #{}:", idx + 1);
        println!("  Dimensions: {} rows × {} columns", table.row_count(), table.column_count());
        println!("  Confidence: {:.2}%", table.confidence * 100.0);
        println!("  Bounding box: ({:.1}, {:.1}) - ({:.1}, {:.1})",
            table.bounding_box.x,
            table.bounding_box.y,
            table.bounding_box.right(),
            table.bounding_box.top()
        );

        println!("\n  Table contents:");
        for (row_idx, row) in table.rows.iter().enumerate() {
            print!("  Row {}: ", row_idx + 1);
            for cell in &row.cells {
                print!("| {:15} ", cell.text);
            }
            println!("|");
        }
        println!();
    }

    // Export to CSV format
    if let Some(table) = result.tables.first() {
        println!("CSV Export:");
        for row in &table.rows {
            let csv_line: Vec<String> = row.cells.iter()
                .map(|cell| format!("\"{}\"", cell.text))
                .collect();
            println!("{}", csv_line.join(","));
        }
    }

    Ok(())
}

// Helper function to extract tables from a real PDF file
#[allow(dead_code)]
fn extract_tables_from_pdf(pdf_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Open PDF file
    let file = File::open(pdf_path)?;
    let reader = BufReader::new(file);
    let pdf_reader = PdfReader::new(reader)?;

    // Extract text from first page
    let extractor = TextExtractor::new();
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };

    let extracted = extractor.extract_text(&pdf_reader, &options)?;

    // Detect tables
    let config = StructuredDataConfig::default();
    let detector = StructuredDataDetector::new(config);
    let result = detector.detect(&extracted.fragments)?;

    println!("Found {} tables in PDF", result.tables.len());

    Ok(())
}
