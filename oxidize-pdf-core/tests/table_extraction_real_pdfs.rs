/// Integration test for table extraction with real PDFs
/// Tests Phase 1-4 complete: Font metadata, Vector lines, Table detection, Color extraction
use oxidize_pdf::graphics::extraction::GraphicsExtractor;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::table_detection::TableDetector;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Collect PDFs from multiple test directories
fn collect_test_pdfs() -> Vec<PathBuf> {
    let mut pdfs = Vec::new();

    // Priority directories with PDFs likely to contain tables
    let search_dirs = vec![
        "/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf-render/tests/fixtures",
        "tests/fixtures",
        "../test-pdfs",
        "examples/results",
    ];

    for &dir in &search_dirs {
        let path = Path::new(dir);
        if path.exists() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if entry_path.extension().map_or(false, |ext| ext == "pdf") {
                        // Prioritize PDFs with "table", "invoice", or "advanced" in name
                        let name = entry_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");
                        if name.contains("table") ||
                           name.contains("invoice") ||
                           name.contains("Invoice") ||
                           name.contains("Factura") ||
                           name.contains("advanced") ||
                           name.contains("Cold_Email") {
                            pdfs.push(entry_path);
                            if pdfs.len() >= 20 {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // If no priority PDFs found, collect any PDFs
    if pdfs.is_empty() {
        for &dir in &search_dirs {
            let path = Path::new(dir);
            if path.exists() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        let entry_path = entry.path();
                        if entry_path.extension().map_or(false, |ext| ext == "pdf") {
                            pdfs.push(entry_path);
                            if pdfs.len() >= 10 {
                                break;
                            }
                        }
                    }
                }
            }
            if pdfs.len() >= 10 {
                break;
            }
        }
    }

    pdfs
}

#[test]
fn test_table_extraction_with_real_pdfs() {
    let pdf_files = collect_test_pdfs();

    if pdf_files.is_empty() {
        println!("No PDF files found in fixtures directory");
        return;
    }

    println!("\n╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Table Extraction Test - Phase 1-4 Complete                 ║");
    println!("║  Testing {} PDFs from multiple directories                   ║", pdf_files.len());
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    let mut total_tables = 0;
    let mut total_pages = 0;
    let mut pdfs_with_tables = 0;
    let mut pdfs_with_colors = 0;
    let mut total_colored_text = 0;
    let mut total_colored_lines = 0;
    let mut successful_extractions = 0;

    for (idx, pdf_path) in pdf_files.iter().enumerate() {
        let filename = pdf_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("[{}/{}] 📄 {}", idx + 1, pdf_files.len(), filename);

        match test_single_pdf(pdf_path) {
            Ok(result) => {
                successful_extractions += 1;
                total_pages += result.page_count;
                total_tables += result.table_count;
                total_colored_text += result.colored_text_count;
                total_colored_lines += result.colored_lines_count;

                if result.table_count > 0 {
                    pdfs_with_tables += 1;
                    println!("  ✅ Tables: {} | Pages: {} | Colored text: {} | Colored lines: {}",
                        result.table_count, result.page_count,
                        result.colored_text_count, result.colored_lines_count);
                } else {
                    println!("  ⚠️  No tables detected | Pages: {} | Colored text: {} | Colored lines: {}",
                        result.page_count, result.colored_text_count, result.colored_lines_count);
                }

                if result.colored_text_count > 0 || result.colored_lines_count > 0 {
                    pdfs_with_colors += 1;
                }
            }
            Err(e) => {
                println!("  ❌ Error: {}", e);
            }
        }
        println!();
    }

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                      SUMMARY STATISTICS                       ║");
    println!("╠═══════════════════════════════════════════════════════════════╣");
    println!("║  Successful extractions: {}/{} ({:.1}%)",
        successful_extractions, pdf_files.len(),
        (successful_extractions as f64 / pdf_files.len() as f64) * 100.0);
    println!("║  Total pages processed: {}", total_pages);
    println!("║  Total tables found: {}", total_tables);
    println!("║  PDFs with tables: {}/{} ({:.1}%)",
        pdfs_with_tables, successful_extractions,
        if successful_extractions > 0 {
            (pdfs_with_tables as f64 / successful_extractions as f64) * 100.0
        } else { 0.0 });
    println!("║  PDFs with color info: {}/{} ({:.1}%)",
        pdfs_with_colors, successful_extractions,
        if successful_extractions > 0 {
            (pdfs_with_colors as f64 / successful_extractions as f64) * 100.0
        } else { 0.0 });
    println!("║  Total colored text fragments: {}", total_colored_text);
    println!("║  Total colored lines: {}", total_colored_lines);
    println!("╚═══════════════════════════════════════════════════════════════╝\n");

    // Test passes if we successfully processed at least one PDF
    assert!(!pdf_files.is_empty(), "Should have found at least one PDF");
    assert!(successful_extractions > 0, "Should have successfully processed at least one PDF");
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
                // Only count tables that have actual text content and reasonable confidence
                table_count = tables.iter()
                    .filter(|t| {
                        let non_empty_cells = t.cells.iter()
                            .filter(|c| !c.text.trim().is_empty())
                            .count();
                        // Table must have text AND confidence >= 30%
                        non_empty_cells > 0 && t.confidence >= 0.30
                    })
                    .count();
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
