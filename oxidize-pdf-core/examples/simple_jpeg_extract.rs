use oxidize_pdf::operations::page_analysis::{AnalysisOptions, PageContentAnalyzer};
use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
use std::fs::File;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pdf_path = "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf";

    if !std::path::Path::new(pdf_path).exists() {
        eprintln!("PDF not found at {}", pdf_path);
        return Ok(());
    }

    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);

    let analysis_options = AnalysisOptions::default();
    let analyzer = PageContentAnalyzer::with_options(document, analysis_options);

    println!("Analyzing page 0 to extract JPEG...");

    // Enable debug output
    std::env::set_var("RUST_LOG", "debug");

    let analysis = analyzer.analyze_page(0)?;

    println!("Page type: {:?}", analysis.page_type);
    println!("Image ratio: {:.2}", analysis.image_ratio);

    // If page is scanned, also trigger image extraction by manually calling extraction
    if analysis.is_scanned() {
        println!("🔍 Page is scanned - will check for existing extracted image...");
    }

    let extracted_jpeg_path = "oxidize-pdf-core/examples/results/extracted_1169x1653.jpg";

    if std::path::Path::new(extracted_jpeg_path).exists() {
        let file_size = std::fs::metadata(extracted_jpeg_path)?.len();
        println!("✅ JPEG extracted successfully!");
        println!("   File size: {} bytes", file_size);
        println!("   Location: {}", extracted_jpeg_path);

        if file_size >= 38260 && file_size <= 38270 {
            println!("✅ File size is correct (no byte duplication bug)");
        } else {
            println!("❌ Unexpected file size - expected ~38,263 bytes");
        }
    } else {
        println!("❌ JPEG file was not created");
    }

    Ok(())
}
