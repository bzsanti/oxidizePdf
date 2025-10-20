//! Plain Text Extraction Example
//!
//! Demonstrates the use of PlainTextExtractor for fast text extraction
//! without position overhead.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example plaintext_extraction <pdf-file>
//! ```
//!
//! # Features Demonstrated
//!
//! 1. Default configuration (balanced)
//! 2. Dense text configuration (tight spacing)
//! 3. Loose text configuration (wide spacing)
//! 4. Layout preservation mode
//! 5. Line-by-line extraction
//! 6. Line break mode processing (Auto, PreserveAll, Normalize)
//!
//! # Performance
//!
//! Plain text extraction is >30% faster than TextExtractor when position
//! data is not needed.

use oxidize_pdf::parser::document::PdfDocument;
use oxidize_pdf::text::plaintext::{LineBreakMode, PlainTextConfig, PlainTextExtractor};
use std::env;
use std::fs::File;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get PDF file path from command line
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <pdf-file>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  cargo run --example plaintext_extraction test-pdfs/sample.pdf");
        return Ok(());
    }

    let pdf_path = &args[1];
    println!("📄 Opening PDF: {}\n", pdf_path);

    // Open PDF document
    let file = File::open(pdf_path)?;
    let doc = PdfDocument::open(file)?;

    let page_count = doc.page_count()?;
    println!("📖 Document has {} pages\n", page_count);

    // Extract from first page with different configurations
    let page_index = 0;

    // Example 1: Default configuration
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("1️⃣  DEFAULT CONFIGURATION (Balanced)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut extractor = PlainTextExtractor::new();
    let start = Instant::now();
    let result = extractor.extract(&doc, page_index)?;
    let elapsed = start.elapsed();

    println!("⏱️  Extraction time: {:.2?}", elapsed);
    println!("📊 Stats:");
    println!("   - Characters: {}", result.char_count);
    println!("   - Lines: {}", result.line_count);
    println!("\n📝 Text (first 500 chars):");
    println!("{}\n", truncate_text(&result.text, 500));

    // Example 2: Dense configuration (tight spacing)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("2️⃣  DENSE CONFIGURATION (Tight Spacing)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut extractor_dense = PlainTextExtractor::with_config(PlainTextConfig::dense());
    let result_dense = extractor_dense.extract(&doc, page_index)?;

    println!("📊 Stats:");
    println!("   - Characters: {}", result_dense.char_count);
    println!("   - Lines: {}", result_dense.line_count);
    println!("   - Space threshold: 0.1 (more aggressive)");
    println!("\n📝 Text (first 500 chars):");
    println!("{}\n", truncate_text(&result_dense.text, 500));

    // Example 3: Loose configuration (wide spacing)
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("3️⃣  LOOSE CONFIGURATION (Wide Spacing)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut extractor_loose = PlainTextExtractor::with_config(PlainTextConfig::loose());
    let result_loose = extractor_loose.extract(&doc, page_index)?;

    println!("📊 Stats:");
    println!("   - Characters: {}", result_loose.char_count);
    println!("   - Lines: {}", result_loose.line_count);
    println!("   - Space threshold: 0.4 (less aggressive)");
    println!("\n📝 Text (first 500 chars):");
    println!("{}\n", truncate_text(&result_loose.text, 500));

    // Example 4: Layout preservation mode
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("4️⃣  LAYOUT PRESERVATION MODE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut extractor_layout = PlainTextExtractor::with_config(PlainTextConfig::preserve_layout());
    let result_layout = extractor_layout.extract(&doc, page_index)?;

    println!("📊 Stats:");
    println!("   - Characters: {}", result_layout.char_count);
    println!("   - Lines: {}", result_layout.line_count);
    println!("   - Preserve layout: true");
    println!("\n📝 Text (first 500 chars):");
    println!("{}\n", truncate_text(&result_layout.text, 500));

    // Example 5: Line-by-line extraction
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("5️⃣  LINE-BY-LINE EXTRACTION");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut extractor_lines = PlainTextExtractor::new();
    let lines = extractor_lines.extract_lines(&doc, page_index)?;

    println!("📊 Total lines: {}", lines.len());
    println!("\n📝 First 10 lines:");
    for (i, line) in lines.iter().take(10).enumerate() {
        println!("  {:2}. {}", i + 1, line);
    }
    println!();

    // Example 6: Line break mode processing
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("6️⃣  LINE BREAK MODE PROCESSING");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Auto mode
    let mut extractor_auto = PlainTextExtractor::with_config(PlainTextConfig {
        line_break_mode: LineBreakMode::Auto,
        ..Default::default()
    });
    let result_auto = extractor_auto.extract(&doc, page_index)?;
    println!("📝 Auto mode (joins wrapped lines):");
    println!("   Lines: {}", result_auto.line_count);
    println!("   Sample: {}\n", truncate_text(&result_auto.text, 200));

    // PreserveAll mode
    let mut extractor_preserve = PlainTextExtractor::with_config(PlainTextConfig {
        line_break_mode: LineBreakMode::PreserveAll,
        ..Default::default()
    });
    let result_preserve = extractor_preserve.extract(&doc, page_index)?;
    println!("📝 PreserveAll mode (keeps all breaks):");
    println!("   Lines: {}", result_preserve.line_count);
    println!("   Sample: {}\n", truncate_text(&result_preserve.text, 200));

    // Normalize mode
    let mut extractor_normalize = PlainTextExtractor::with_config(PlainTextConfig {
        line_break_mode: LineBreakMode::Normalize,
        ..Default::default()
    });
    let result_normalize = extractor_normalize.extract(&doc, page_index)?;
    println!("📝 Normalize mode (joins hyphenated words):");
    println!("   Lines: {}", result_normalize.line_count);
    println!(
        "   Sample: {}\n",
        truncate_text(&result_normalize.text, 200)
    );

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📈 PERFORMANCE SUMMARY");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    println!("⚡ Plain text extraction completed in {:.2?}", elapsed);
    println!("✅ Feature 2.2.2 - Plain Text Optimization");
    println!("\n💡 Tips:");
    println!("   - Use default config for balanced extraction");
    println!("   - Use dense config for tightly-spaced PDFs");
    println!("   - Use loose config for wide-spaced PDFs");
    println!("   - Use preserve_layout for tabular data");
    println!("   - Use extract_lines() for grep-like operations");
    println!("   - Plain text extraction is >30% faster than TextExtractor\n");

    Ok(())
}

/// Truncate text to a maximum length with ellipsis
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}
