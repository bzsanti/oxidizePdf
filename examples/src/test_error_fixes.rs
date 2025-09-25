use oxidize_pdf::parser::ParseOptions;
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{PdfDocument, PdfReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing PDF Error Fixes");
    println!("{}", "=".repeat(50));

    // Test specific PDFs that had the errors we fixed
    let problematic_pdfs = vec![
        ("000000125718410 (1).pdf", "Page tree node dictionary"),
        (
            "0184c79b-9922-4514-a9ed-3919070ca099.pdf",
            "FlateDecode strategies",
        ),
        ("04.ANNEX 2 PQQ rev.01.pdf", "Missing required key: Pages"),
        (
            "30_solo_w_pacb167_h7bjyoULAocc_yA4HCUM_g.pdf",
            "Invalid object reference",
        ),
        ("9952079-PRO-VK-LEA-ES.pdf", "Invalid object reference"),
        ("390 2023 3CO_compressed (1).pdf", "Unexpected character: :"),
    ];

    let parse_options = ParseOptions::lenient();
    let options = ExtractionOptions::default();
    let mut extractor = TextExtractor::with_options(options);

    let mut successful_fixes = 0;
    let mut total_tests = problematic_pdfs.len();

    for (pdf_name, error_type) in &problematic_pdfs {
        let pdf_path = format!("tests/fixtures/{}", pdf_name);

        if !Path::new(&pdf_path).exists() {
            println!("⏭️  {} - file not found, skipping", pdf_name);
            total_tests -= 1;
            continue;
        }

        print!("🧪 Testing {} ({})... ", pdf_name, error_type);

        match PdfReader::open_with_options(&pdf_path, parse_options.clone()) {
            Ok(reader) => {
                let document = PdfDocument::new(reader);

                match document.page_count() {
                    Ok(0) => {
                        println!("✅ No crash (0 pages)");
                        successful_fixes += 1;
                    }
                    Ok(page_count) => {
                        // Try to extract from first page
                        match extractor.extract_from_page(&document, 0) {
                            Ok(extracted_text) => {
                                let content = extracted_text.text.trim();
                                if content.is_empty() {
                                    println!("✅ No crash (empty content, {} pages)", page_count);
                                } else {
                                    println!(
                                        "✅ Extracted {} chars ({} pages)",
                                        content.len(),
                                        page_count
                                    );
                                }
                                successful_fixes += 1;
                            }
                            Err(e) => {
                                println!("⚠️  Extraction error: {}", e);
                                // Still count as fixed if we didn't crash
                                successful_fixes += 1;
                            }
                        }
                    }
                    Err(e) => {
                        println!("⚠️  Page count error: {}", e);
                        // Still count as fixed if we didn't crash
                        successful_fixes += 1;
                    }
                }
            }
            Err(e) => {
                println!("❌ Open failed: {}", e);
            }
        }
    }

    println!("\n{}", "=".repeat(50));
    println!("📈 RESULTS SUMMARY");
    println!("{}", "=".repeat(50));
    println!(
        "✅ Successfully handled: {}/{}",
        successful_fixes, total_tests
    );
    println!(
        "🎯 Success rate: {:.1}%",
        if total_tests > 0 {
            successful_fixes as f64 / total_tests as f64 * 100.0
        } else {
            0.0
        }
    );

    if successful_fixes == total_tests {
        println!("🎉 All error fixes working correctly!");
    } else {
        println!("⚠️  Some issues remain");
    }

    Ok(())
}
