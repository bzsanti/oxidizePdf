//! Phase 2: Vector Line Extraction Test
//!
//! Tests that GraphicsExtractor correctly extracts horizontal and vertical lines
//! from PDF graphics operations (rectangles, line segments, paths).
//!
//! Run with: cargo run --example phase2_vector_extraction_test

use oxidize_pdf::document::Document;
use oxidize_pdf::graphics::extraction::{ExtractionConfig, GraphicsExtractor, LineOrientation};
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::writer::{PdfWriter, WriterConfig};
use oxidize_pdf::Color;
use oxidize_pdf::Page;
use std::fs::File;
use std::io::BufWriter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📐 Phase 2: Vector Line Extraction Test\n");

    // Step 1: Create PDF with table-like structure
    println!("1. Creating PDF with table structure...");
    let mut doc = Document::new();
    let mut page = Page::new(595.0, 842.0); // A4 size

    // Draw a simple 3x2 table using rectangles
    // Table: 3 columns x 2 rows, starting at (100, 700), cell size 100x50

    // Rectangle 1 (top-left cell)
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
        .set_line_width(1.0)
        .rect(100.0, 700.0, 100.0, 50.0)
        .stroke();

    // Rectangle 2 (top-middle cell)
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
        .set_line_width(1.0)
        .rect(200.0, 700.0, 100.0, 50.0)
        .stroke();

    // Rectangle 3 (top-right cell)
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
        .set_line_width(1.0)
        .rect(300.0, 700.0, 100.0, 50.0)
        .stroke();

    // Rectangle 4 (bottom-left cell)
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
        .set_line_width(1.0)
        .rect(100.0, 650.0, 100.0, 50.0)
        .stroke();

    // Rectangle 5 (bottom-middle cell)
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
        .set_line_width(1.0)
        .rect(200.0, 650.0, 100.0, 50.0)
        .stroke();

    // Rectangle 6 (bottom-right cell)
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
        .set_line_width(1.0)
        .rect(300.0, 650.0, 100.0, 50.0)
        .stroke();

    // Add a diagonal line (should NOT be extracted with default config)
    page.graphics()
        .set_stroke_color(Color::rgb(0.5, 0.5, 0.5))
        .set_line_width(0.5)
        .move_to(50.0, 600.0)
        .line_to(150.0, 550.0)
        .stroke();

    doc.add_page(page);

    let pdf_path = "examples/results/phase2_vector_extraction_test.pdf";
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

    let config = ExtractionConfig::default(); // stroked_only=true, extract_diagonals=false
    let mut extractor = GraphicsExtractor::new(config);
    let graphics = extractor.extract_from_page(&pdf_doc, 0)?;

    println!("   ✓ Extracted {} lines\n", graphics.lines.len());

    // Step 3: Analyze extracted lines
    println!("3. Analyzing extracted lines:");
    println!(
        "   Total lines: {} ({} horizontal, {} vertical)",
        graphics.lines.len(),
        graphics.horizontal_count,
        graphics.vertical_count
    );

    // Expected: 6 rectangles = 24 lines (4 lines per rectangle)
    // Diagonal line should be filtered out
    let expected_total = 24;
    let expected_h = 12; // 6 rectangles × 2 horizontal edges
    let expected_v = 12; // 6 rectangles × 2 vertical edges

    println!("\n4. Verification:");

    // Test 1: Total line count
    let test1_pass = graphics.lines.len() == expected_total;
    println!(
        "   {} Total lines: {} (expected {})",
        if test1_pass { "✓" } else { "✗" },
        graphics.lines.len(),
        expected_total
    );

    // Test 2: Horizontal count
    let test2_pass = graphics.horizontal_count == expected_h;
    println!(
        "   {} Horizontal lines: {} (expected {})",
        if test2_pass { "✓" } else { "✗" },
        graphics.horizontal_count,
        expected_h
    );

    // Test 3: Vertical count
    let test3_pass = graphics.vertical_count == expected_v;
    println!(
        "   {} Vertical lines: {} (expected {})",
        if test3_pass { "✓" } else { "✗" },
        graphics.vertical_count,
        expected_v
    );

    // Test 4: No diagonal lines (filtered by config)
    let diagonal_count = graphics
        .lines
        .iter()
        .filter(|l| l.orientation == LineOrientation::Diagonal)
        .count();
    let test4_pass = diagonal_count == 0;
    println!(
        "   {} Diagonal lines filtered: {} (expected 0)",
        if test4_pass { "✓" } else { "✗" },
        diagonal_count
    );

    // Test 5: Table structure detected
    let test5_pass = graphics.has_table_structure();
    println!(
        "   {} Table structure detected: {}",
        if test5_pass { "✓" } else { "✗" },
        test5_pass
    );

    // Test 6: Sample line coordinates
    println!("\n5. Sample extracted lines (first 3):");
    for (i, line) in graphics.lines.iter().take(3).enumerate() {
        println!(
            "   Line {}: ({:.1}, {:.1}) -> ({:.1}, {:.1}) [{:?}, stroke_width={:.1}]",
            i + 1,
            line.x1,
            line.y1,
            line.x2,
            line.y2,
            line.orientation,
            line.stroke_width
        );
    }

    println!("\n6. Test Summary:");
    let all_pass = test1_pass && test2_pass && test3_pass && test4_pass && test5_pass;

    if all_pass {
        println!("✅ Phase 2 Complete: Vector line extraction working correctly!");
        Ok(())
    } else {
        eprintln!("❌ Phase 2 Failed: Vector line extraction has issues");
        std::process::exit(1);
    }
}
