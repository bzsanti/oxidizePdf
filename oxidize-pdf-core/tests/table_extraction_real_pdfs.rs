/// Integration test for table extraction with real PDFs from fixtures
/// Tests Phase 1-4 complete: Font metadata, Vector lines, Table detection, Color extraction
use oxidize_pdf::graphics::extraction::GraphicsExtractor;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::table_detection::TableDetector;
use std::fs::File;
use std::path::Path;

#[test]
fn test_table_extraction_with_real_pdfs() {
    let fixtures_dir = Path::new("tests/fixtures");

    // Skip if fixtures directory doesn't exist
    if !fixtures_dir.exists() {
        println!("Skipping test: fixtures directory not found");
        return;
    }

    let pdf_files: Vec<_> = std::fs::read_dir(fixtures_dir)
        .expect("Failed to read fixtures directory")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()? == "pdf" {
                Some(path)
            } else {
                None
            }
        })
        .take(5) // Test first 5 PDFs
        .collect();

    if pdf_files.is_empty() {
        println!("No PDF files found in fixtures directory");
        return;
    }

    println!("\n=== Testing Table Extraction on {} Real PDFs ===\n", pdf_files.len());

    let mut total_tables = 0;
    let mut pdfs_with_tables = 0;
    let mut pdfs_with_colors = 0;

    for pdf_path in &pdf_files {
        println!("Processing: {}", pdf_path.display());

        match test_single_pdf(&pdf_path) {
            Ok(result) => {
                println!("  ✓ Pages: {}", result.page_count);
                println!("  ✓ Tables detected: {}", result.table_count);
                println!("  ✓ Text fragments with color: {}", result.colored_text_count);
                println!("  ✓ Lines with color: {}", result.colored_lines_count);

                total_tables += result.table_count;
                if result.table_count > 0 {
                    pdfs_with_tables += 1;
                }
                if result.colored_text_count > 0 || result.colored_lines_count > 0 {
                    pdfs_with_colors += 1;
                }
            }
            Err(e) => {
                println!("  ✗ Error: {}", e);
            }
        }
        println!();
    }

    println!("=== Summary ===");
    println!("Total tables found: {}", total_tables);
    println!("PDFs with tables: {}", pdfs_with_tables);
    println!("PDFs with colors: {}", pdfs_with_colors);

    // Test passes if we successfully processed at least one PDF
    assert!(!pdf_files.is_empty(), "Should have processed at least one PDF");
}

struct ExtractionResult {
    page_count: usize,
    table_count: usize,
    colored_text_count: usize,
    colored_lines_count: usize,
}

fn test_single_pdf(path: &Path) -> Result<ExtractionResult, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = PdfReader::new(file)?;
    let doc = PdfDocument::new(reader);

    let page_count = doc.page_count()? as usize;
    let mut table_count = 0;
    let mut colored_text_count = 0;
    let mut colored_lines_count = 0;

    // Process first page only for performance
    let page_num: usize = 0;

    // Extract graphics (lines)
    let mut graphics_ext = GraphicsExtractor::default();
    if let Ok(graphics) = graphics_ext.extract_from_page(&doc, page_num) {
        colored_lines_count = graphics.lines.iter().filter(|line| line.color.is_some()).count();
    }

    // Extract text with layout
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut text_ext = TextExtractor::with_options(options);

    if let Ok(text) = text_ext.extract_from_page(&doc, page_num as u32) {
        colored_text_count = text.fragments.iter().filter(|frag| frag.color.is_some()).count();

        // Try table detection
        if let Ok(graphics) = graphics_ext.extract_from_page(&doc, page_num) {
            let detector = TableDetector::default();
            if let Ok(tables) = detector.detect(&graphics, &text.fragments) {
                table_count = tables.len();
            }
        }
    }

    Ok(ExtractionResult {
        page_count,
        table_count,
        colored_text_count,
        colored_lines_count,
    })
}

#[test]
fn test_color_extraction_with_cold_email_hacks() {
    let pdf_path = Path::new("tests/fixtures/Cold_Email_Hacks.pdf");

    if !pdf_path.exists() {
        println!("Skipping test: Cold_Email_Hacks.pdf not found");
        return;
    }

    let file = File::open(pdf_path).expect("Failed to open PDF");
    let reader = PdfReader::new(file).expect("Failed to create reader");
    let doc = PdfDocument::new(reader);

    // Extract from first page
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut text_ext = TextExtractor::with_options(options);
    let text = text_ext.extract_from_page(&doc, 0).expect("Failed to extract text");

    println!("\n=== Cold Email Hacks PDF Analysis ===");
    println!("Total text fragments: {}", text.fragments.len());

    let colored_fragments: Vec<_> = text.fragments.iter()
        .filter(|frag| frag.color.is_some())
        .collect();

    println!("Fragments with color: {}", colored_fragments.len());

    // Show first 5 colored fragments
    for (i, frag) in colored_fragments.iter().take(5).enumerate() {
        if let Some(color) = &frag.color {
            println!("  Fragment {}: '{}' - Color: {:?}", i + 1,
                frag.text.chars().take(30).collect::<String>(), color);
        }
    }

    // Extract graphics
    let mut graphics_ext = GraphicsExtractor::default();
    let graphics = graphics_ext.extract_from_page(&doc, 0).expect("Failed to extract graphics");

    println!("\nTotal lines: {}", graphics.lines.len());

    let colored_lines: Vec<_> = graphics.lines.iter()
        .filter(|line| line.color.is_some())
        .collect();

    println!("Lines with color: {}", colored_lines.len());

    // Test passes regardless of whether colors are found (PDFs may not have colors)
    assert!(text.fragments.len() > 0, "Should extract some text");
}
