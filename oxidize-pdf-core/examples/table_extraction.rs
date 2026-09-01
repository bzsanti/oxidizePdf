//! Example: Table extraction from PDF documents
//!
//! This example demonstrates how to automatically detect and extract tables
//! from PDF documents using spatial clustering algorithms.

use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::structured::{StructuredDataConfig, StructuredDataDetector};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PDF Table Extraction Demo ===\n");

    // Demo with synthetic data
    demo_table_detection()?;

    // Uncomment to extract tables from a real PDF:
    // extract_tables_from_pdf("path/to/document.pdf")?;

    println!("\n=== Example completed successfully ===");
    Ok(())
}

fn demo_table_detection() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating table detection with synthetic data...\n");

    // Create synthetic text fragments representing a 3x3 table
    use oxidize_pdf::text::extraction::TextFragment;

    let fragments = vec![
        // Header row (Y = 700)
        TextFragment::new("Name".to_string(), 100.0, 700.0, 50.0, 12.0, 12.0),
        TextFragment::new("Age".to_string(), 200.0, 700.0, 30.0, 12.0, 12.0),
        TextFragment::new("City".to_string(), 300.0, 700.0, 40.0, 12.0, 12.0),
        // Data row 1 (Y = 680)
        TextFragment::new("Alice".to_string(), 100.0, 680.0, 40.0, 12.0, 12.0),
        TextFragment::new("30".to_string(), 200.0, 680.0, 20.0, 12.0, 12.0),
        TextFragment::new("NYC".to_string(), 300.0, 680.0, 30.0, 12.0, 12.0),
        // Data row 2 (Y = 660)
        TextFragment::new("Bob".to_string(), 100.0, 660.0, 30.0, 12.0, 12.0),
        TextFragment::new("25".to_string(), 200.0, 660.0, 20.0, 12.0, 12.0),
        TextFragment::new("LA".to_string(), 300.0, 660.0, 20.0, 12.0, 12.0),
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
        println!(
            "  Dimensions: {} rows × {} columns",
            table.row_count(),
            table.column_count()
        );
        println!("  Confidence: {:.2}%", table.confidence * 100.0);
        println!(
            "  Bounding box: ({:.1}, {:.1}) - ({:.1}, {:.1})",
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
            let csv_line: Vec<String> = row
                .cells
                .iter()
                .map(|cell| format!("\"{}\"", cell.text))
                .collect();
            println!("{}", csv_line.join(","));
        }
    }

    Ok(())
}

// Helper function to extract tables from a real PDF file
// Uncomment the call in main() to use this function
#[allow(dead_code)]
fn extract_tables_from_pdf(pdf_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Open PDF document using correct API
    let document = PdfReader::open_document(pdf_path)?;

    // Extract text from first page (page_index = 0)
    let mut extractor = TextExtractor::with_options(ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    });

    let extracted = extractor.extract_from_page(&document, 0)?;

    // Detect tables
    let config = StructuredDataConfig::default();
    let detector = StructuredDataDetector::new(config);
    let result = detector.detect(&extracted.fragments)?;

    println!("Found {} tables in PDF", result.tables.len());

    Ok(())
}
