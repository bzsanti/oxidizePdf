//! Phase 3: Table Detection Test
//!
//! Tests the complete table detection pipeline:
//! 1. Vector line extraction (from Phase 2)
//! 2. Text fragment extraction (from Phase 1)
//! 3. Table detection with cell assignment
//!
//! Run with: cargo run --example phase3_table_detection_test

use oxidize_pdf::document::Document;
use oxidize_pdf::graphics::extraction::{ExtractionConfig, GraphicsExtractor};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::table_detection::{TableDetectionError, TableDetector};
use oxidize_pdf::text::Font;
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::Color;
use oxidize_pdf::Page;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Phase 3: Table Detection Test\n");

    // Step 1: Create PDF with a 3x2 table
    println!("1. Creating PDF with 3x2 table...");
    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4 size

    // Define table structure: 3 columns x 2 rows
    // Starting at (100, 700), cell size: 100x50
    let start_x = 100.0;
    let start_y = 700.0;
    let cell_width = 100.0;
    let cell_height = 50.0;

    // Draw table borders (rectangles for each cell)
    for row in 0..2 {
        for col in 0..3 {
            let x = start_x + (col as f64 * cell_width);
            let y = start_y - (row as f64 * cell_height);

            page.graphics()
                .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
                .set_line_width(1.0)
                .rect(x, y, cell_width, cell_height)
                .stroke();
        }
    }

    // Add text content to cells
    let table_data = vec![
        vec!["Header 1", "Header 2", "Header 3"],
        vec!["Data 1", "Data 2", "Data 3"],
    ];

    for (row, row_data) in table_data.iter().enumerate() {
        for (col, &text) in row_data.iter().enumerate() {
            let x = start_x + (col as f64 * cell_width) + 10.0; // 10pt padding
            let y = start_y - (row as f64 * cell_height) + 30.0; // Center vertically

            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(x, y)
                .write(text)?;
        }
    }

    doc.add_page(page);

    let pdf_path = "examples/results/phase3_table_detection_test.pdf";
    std::fs::create_dir_all("examples/results")?;
    let file = File::create(pdf_path)?;
    let mut writer = PdfWriter::with_config(BufWriter::new(file), WriterConfig::default());
    writer.write_document(&mut doc)?;
    println!("   ✓ Created: {}\n", pdf_path);

    // Step 2: Extract vector lines
    println!("2. Extracting vector lines...");
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new(file)?;
    let pdf_doc = PdfDocument::new(reader);

    let graphics_config = ExtractionConfig::default();
    let mut graphics_extractor = GraphicsExtractor::new(graphics_config);
    let graphics = graphics_extractor.extract_from_page(&pdf_doc, 0)?;

    println!(
        "   ✓ Extracted {} lines ({} H, {} V)\n",
        graphics.lines.len(),
        graphics.horizontal_count,
        graphics.vertical_count
    );

    // Debug: Print H/V line positions
    println!("   Horizontal line Y positions:");
    for line in graphics.horizontal_lines().take(15) {
        println!("      Y={:.2}", line.y1);
    }
    println!("\n   Vertical line X positions:");
    for line in graphics.vertical_lines().take(15) {
        println!("      X={:.2}", line.x1);
    }
    println!();

    // Step 3: Extract text fragments
    println!("3. Extracting text fragments...");
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
        "   ✓ Extracted {} text fragments\n",
        text_result.fragments.len()
    );

    // Step 4: Detect tables
    println!("4. Detecting tables...");
    let detector = TableDetector::default();
    let tables = detector.detect(&graphics, &text_result.fragments)?;

    println!("   ✓ Detected {} table(s)\n", tables.len());

    // Step 5: Verify results
    println!("5. Verification:");

    let test1_pass = tables.len() == 1;
    println!(
        "   {} Table count: {} (expected 1)",
        if test1_pass { "✓" } else { "✗" },
        tables.len()
    );

    if let Some(table) = tables.first() {
        let test2_pass = table.row_count() == 2;
        println!(
            "   {} Row count: {} (expected 2)",
            if test2_pass { "✓" } else { "✗" },
            table.row_count()
        );

        let test3_pass = table.column_count() == 3;
        println!(
            "   {} Column count: {} (expected 3)",
            if test3_pass { "✓" } else { "✗" },
            table.column_count()
        );

        let test4_pass = table.cells.len() == 6;
        println!(
            "   {} Cell count: {} (expected 6)",
            if test4_pass { "✓" } else { "✗" },
            table.cells.len()
        );

        let test5_pass = table.confidence > 0.7;
        println!(
            "   {} Confidence: {:.2} (expected > 0.7)",
            if test5_pass { "✓" } else { "✗" },
            table.confidence
        );

        // Test 6: Verify specific cell contents
        println!("\n6. Cell Contents:");
        let mut content_tests_passed = 0;
        let mut content_tests_failed = 0;

        let expected_contents = vec![
            (0, 0, "Header 1"),
            (0, 1, "Header 2"),
            (0, 2, "Header 3"),
            (1, 0, "Data 1"),
            (1, 1, "Data 2"),
            (1, 2, "Data 3"),
        ];

        for (row, col, expected) in &expected_contents {
            if let Some(cell) = table.get_cell(*row, *col) {
                let matches = cell.text.contains(expected);
                println!(
                    "   {} Cell({},{}) = '{}' (expected '{}')",
                    if matches { "✓" } else { "✗" },
                    row,
                    col,
                    cell.text,
                    expected
                );

                if matches {
                    content_tests_passed += 1;
                } else {
                    content_tests_failed += 1;
                }
            } else {
                println!("   ✗ Cell({},{}) not found", row, col);
                content_tests_failed += 1;
            }
        }

        println!("\n7. Test Summary:");
        let all_basic_tests = test1_pass && test2_pass && test3_pass && test4_pass && test5_pass;
        let all_content_tests = content_tests_failed == 0 && content_tests_passed == 6;

        if all_basic_tests && all_content_tests {
            println!("✅ Phase 3 Complete: Table detection working correctly!");
            println!("   - Structure: ✓");
            println!("   - Cell assignment: ✓");
            println!("   - Text extraction: ✓");
            Ok(())
        } else {
            eprintln!("❌ Phase 3 Failed:");
            if !all_basic_tests {
                eprintln!("   - Structure detection has issues");
            }
            if !all_content_tests {
                eprintln!(
                    "   - Cell assignment has issues ({}/{} cells correct)",
                    content_tests_passed,
                    expected_contents.len()
                );
            }
            std::process::exit(1);
        }
    } else {
        eprintln!("❌ Phase 3 Failed: No table detected");
        std::process::exit(1);
    }
}
