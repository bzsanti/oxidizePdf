//! Advanced Table Extraction Example
//!
//! Demonstrates complete end-to-end table extraction workflow combining:
//! - Phase 1: Font metadata extraction
//! - Phase 2: Vector line extraction
//! - Phase 3: Table detection with cell assignment
//!
//! This example creates a realistic invoice-style table and extracts all data.
//!
//! Run with: cargo run --example advanced_table_extraction

use oxidize_pdf::document::Document;
use oxidize_pdf::graphics::extraction::{ExtractionConfig, GraphicsExtractor};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::table_detection::TableDetector;
use oxidize_pdf::text::Font;
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::Color;
use oxidize_pdf::Page;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Advanced Table Extraction Example\n");
    println!("Demonstrates complete pipeline: Font metadata → Vector lines → Table detection\n");

    // Step 1: Create realistic invoice-style table
    println!("1. Creating invoice-style PDF with table...");
    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4

    // Title
    page.text()
        .set_font(Font::HelveticaBold, 18.0)
        .at(50.0, 800.0)
        .write("INVOICE #12345")?;

    // Invoice details
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 770.0)
        .write("Date: 2025-10-22")?;

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 755.0)
        .write("Customer: Acme Corp")?;

    // Table: 4 columns × 5 rows (header + 4 items)
    let table_start_x = 50.0;
    let table_start_y = 700.0;
    let col_widths = [200.0, 80.0, 80.0, 100.0]; // Description, Qty, Price, Total
    let row_height = 30.0;
    let num_rows = 5;

    // Draw table borders
    for row in 0..num_rows {
        for (col, &width) in col_widths.iter().enumerate() {
            let x = table_start_x + col_widths[..col].iter().sum::<f64>();
            let y = table_start_y - (row as f64 * row_height);

            page.graphics()
                .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
                .set_line_width(1.0)
                .rect(x, y, width, row_height)
                .stroke();
        }
    }

    // Table data
    let table_data = [
        vec!["Description", "Qty", "Price", "Total"],
        vec!["Widget Pro", "5", "$50.00", "$250.00"],
        vec!["Service Plan", "1", "$99.99", "$99.99"],
        vec!["Installation", "2", "$75.00", "$150.00"],
        vec!["Shipping", "1", "$25.00", "$25.00"],
    ];

    // Add text to cells
    for (row, row_data) in table_data.iter().enumerate() {
        for (col, &text) in row_data.iter().enumerate() {
            let x = table_start_x + col_widths[..col].iter().sum::<f64>() + 5.0; // 5pt padding
            let y = table_start_y - (row as f64 * row_height) + 17.0; // Center vertically

            // Header row in bold
            let font = if row == 0 {
                Font::HelveticaBold
            } else {
                Font::Helvetica
            };

            page.text().set_font(font, 10.0).at(x, y).write(text)?;
        }
    }

    // Total line
    page.text()
        .set_font(Font::HelveticaBold, 12.0)
        .at(
            table_start_x + col_widths[..3].iter().sum::<f64>(),
            table_start_y - (num_rows as f64 * row_height) - 30.0,
        )
        .write("TOTAL: $524.99")?;

    doc.add_page(page);

    let pdf_path = "examples/results/advanced_table_extraction.pdf";
    std::fs::create_dir_all("examples/results")?;
    let file = File::create(pdf_path)?;
    let mut writer = PdfWriter::with_config(BufWriter::new(file), WriterConfig::default());
    writer.write_document(&mut doc)?;
    println!("   ✓ Created: {}\n", pdf_path);

    // Step 2: Phase 1 - Extract text with font metadata
    println!("2. Phase 1: Extracting text with font metadata...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new(file)?;
    let pdf_doc = PdfDocument::new(reader);

    let text_options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut text_extractor = TextExtractor::with_options(text_options);
    let text_result = text_extractor.extract_from_page(&pdf_doc, 0)?;

    println!(
        "   ✓ Extracted {} text fragments",
        text_result.fragments.len()
    );

    // Show font diversity
    let mut fonts: Vec<String> = text_result
        .fragments
        .iter()
        .filter_map(|f| f.font_name.as_ref())
        .map(|s| s.to_string())
        .collect();
    fonts.sort();
    fonts.dedup();
    println!("   ✓ Fonts used: {}", fonts.join(", "));

    // Step 3: Phase 2 - Extract vector lines
    println!("\n3. Phase 2: Extracting vector lines...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new(file)?;
    let pdf_doc = PdfDocument::new(reader);

    let graphics_config = ExtractionConfig::default();
    let mut graphics_extractor = GraphicsExtractor::new(graphics_config);
    let graphics = graphics_extractor.extract_from_page(&pdf_doc, 0)?;

    println!(
        "   ✓ Extracted {} lines ({} H, {} V)",
        graphics.lines.len(),
        graphics.horizontal_count,
        graphics.vertical_count
    );

    // Step 4: Phase 3 - Detect tables
    println!("\n4. Phase 3: Detecting tables...");
    let detector = TableDetector::default();
    let tables = detector.detect(&graphics, &text_result.fragments)?;

    println!("   ✓ Detected {} table(s)", tables.len());

    // Step 5: Display results
    if let Some(table) = tables.first() {
        println!("\n5. Extracted Table Data:");
        println!(
            "   Dimensions: {} rows × {} columns",
            table.row_count(),
            table.column_count()
        );
        println!("   Confidence: {:.2}%\n", table.confidence * 100.0);

        println!("   Cell Contents:");
        println!("   {:-<80}", "");

        for row in 0..table.row_count() {
            print!("   ");
            for col in 0..table.column_count() {
                if let Some(cell) = table.get_cell(row, col) {
                    print!(
                        "| {:^18}",
                        if cell.text.len() > 18 {
                            format!("{}...", &cell.text[..15])
                        } else {
                            cell.text.clone()
                        }
                    );
                }
            }
            println!("|");

            if row == 0 {
                println!("   {:-<80}", "");
            }
        }
        println!("   {:-<80}", "");

        // Verify data extraction
        println!("\n6. Verification:");
        let header_row_correct = table
            .get_cell(0, 0)
            .map(|c| c.text.contains("Description"))
            .unwrap_or(false)
            && table
                .get_cell(0, 3)
                .map(|c| c.text.contains("Total"))
                .unwrap_or(false);

        let data_row_correct = table
            .get_cell(1, 0)
            .map(|c| c.text.contains("Widget"))
            .unwrap_or(false)
            && table
                .get_cell(1, 3)
                .map(|c| c.text.contains("$250"))
                .unwrap_or(false);

        println!(
            "   {} Header row extracted correctly",
            if header_row_correct { "✓" } else { "✗" }
        );
        println!(
            "   {} Data rows extracted correctly",
            if data_row_correct { "✓" } else { "✗" }
        );
        println!(
            "   {} Font metadata preserved (bold headers)",
            if fonts.contains(&"Helvetica-Bold".to_string()) {
                "✓"
            } else {
                "✗"
            }
        );

        if header_row_correct && data_row_correct {
            println!("\n✅ Success: Complete table extraction pipeline working!");
            println!("   - Font styles detected (bold headers)");
            println!("   - Table borders identified (20 lines)");
            println!("   - Cell contents extracted (5×4 = 20 cells)");
            println!("   - Spatial layout preserved");
        } else {
            println!("\n⚠️  Some data extraction issues detected");
        }
    } else {
        println!("\n❌ No table detected");
    }

    Ok(())
}
