//! Analyze what table detection is finding in a specific PDF

use oxidize_pdf::graphics::extraction::GraphicsExtractor;
use oxidize_pdf::parser::{PdfDocument, PdfReader};
use oxidize_pdf::text::extraction::{ExtractionOptions, TextExtractor};
use oxidize_pdf::text::table_detection::TableDetector;
use std::env;
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let pdf_path = if args.len() > 1 {
        &args[1]
    } else {
        "/Users/santifdezmunoz/Documents/repos/BelowZero/oxidize-pdf-render/tests/fixtures/Factura_22058.pdf"
    };

    println!("=== Analyzing: {} ===\n", pdf_path);

    let file = File::open(pdf_path)?;
    let reader = PdfReader::new(file)?;
    let doc = PdfDocument::new(reader);

    // Extract graphics (lines)
    let mut graphics_ext = GraphicsExtractor::default();
    let graphics = graphics_ext.extract_from_page(&doc, 0)?;

    println!("Lines found: {}", graphics.lines.len());
    if graphics.lines.len() > 0 {
        println!("Sample lines (first 5):");
        for (i, line) in graphics.lines.iter().take(5).enumerate() {
            println!("  {:?}", line.orientation);
        }
    }

    // Extract text
    let options = ExtractionOptions {
        preserve_layout: true,
        ..Default::default()
    };
    let mut text_ext = TextExtractor::with_options(options);
    let text = text_ext.extract_from_page(&doc, 0)?;

    println!("\nText fragments: {}", text.fragments.len());
    if text.fragments.len() > 0 {
        println!("Sample text (first 5):");
        for (i, frag) in text.fragments.iter().take(5).enumerate() {
            let preview: String = frag.text.chars().take(30).collect();
            println!("  '{}' at ({:.0},{:.0})", preview, frag.x, frag.y);
        }
    }

    // Try table detection
    let detector = TableDetector::default();
    let tables = detector.detect(&graphics, &text.fragments)?;

    println!("\n=== TABLE DETECTION RESULTS ===");
    println!("Tables found: {}\n", tables.len());

    for (i, table) in tables.iter().enumerate() {
        println!("Table {}:", i + 1);
        println!("  Size: {} rows x {} columns", table.rows, table.columns);
        println!("  Confidence: {:.2}%", table.confidence * 100.0);
        println!("  Total cells: {}", table.cells.len());
        println!(
            "  Non-empty cells: {}",
            table.cells.iter().filter(|c| !c.text.is_empty()).count()
        );
        println!(
            "  Bbox: ({:.0}, {:.0}) size {:.0}x{:.0}",
            table.bbox.x, table.bbox.y, table.bbox.width, table.bbox.height
        );

        // Show cell content sample
        println!("\n  Cell content (first 3 rows):");
        let populated: Vec<_> = table
            .cells
            .iter()
            .filter(|c| !c.text.is_empty())
            .take(15)
            .collect();

        for cell in populated {
            let preview: String = cell.text.chars().take(40).collect();
            println!("    Cell({},{}) = '{}'", cell.row, cell.column, preview);
        }
    }

    Ok(())
}
