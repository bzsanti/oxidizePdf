use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{PdfDocument, PdfReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test a subset of PDFs that are more likely to work
    let pdfs = vec![
        ("tests/fixtures/1002579 - FIRMADO.pdf", "FIRMADO", 1),
        (
            "tests/fixtures/111 4T-2021 COMPAÑIA COMERCIALIZADORA DE LAS COSAS.pdf",
            "COMPAÑIA",
            1,
        ),
        (
            "tests/fixtures/applied_cryptography_protocols_algorithms_and_source_code_in_c.pdf",
            "Applied Cryptography",
            50,
        ),
        (
            "tests/fixtures/ssasperfguide2008r2 (1).pdf",
            "SQL Server Performance",
            5,
        ),
    ];

    println!("🔍 Testing text extraction from selected PDFs");
    println!("{}", "=".repeat(60));

    for (pdf_path, name, page_num) in pdfs {
        if !Path::new(pdf_path).exists() {
            println!("\n❌ {} - File not found", name);
            continue;
        }

        println!("\n📄 {} - Extracting page {}", name, page_num);

        let reader = PdfReader::open(pdf_path)?;
        let document = PdfDocument::new(reader);

        let options = ExtractionOptions::default();
        let mut extractor = TextExtractor::with_options(options);

        match extractor.extract_from_page(&document, page_num) {
            Ok(text) => {
                let content = text.text.trim();
                if content.is_empty() {
                    println!("   ⚠️  Page {} is empty", page_num);
                    println!("   📊 Trying another page...");

                    // Try page 100 for Applied Cryptography, page 10 for SQL Server
                    let alt_page = if name.contains("Cryptography") {
                        100
                    } else {
                        10
                    };

                    match extractor.extract_from_page(&document, alt_page) {
                        Ok(text) => {
                            let content = text.text.trim();
                            if content.is_empty() {
                                println!("   ⚠️  Page {} is also empty", alt_page);
                            } else {
                                println!(
                                    "   ✅ Page {} - Extracted {} chars",
                                    alt_page,
                                    content.len()
                                );
                                let preview_len = 500.min(content.len());
                                let preview = &content[..preview_len];
                                println!("\n   📝 Full text preview from page {}:", alt_page);
                                println!("   {}", "─".repeat(50));
                                println!("{}", preview);
                                println!("   {}", "─".repeat(50));
                            }
                        }
                        Err(e) => {
                            println!("   ❌ Failed to extract page {}: {}", alt_page, e);
                        }
                    }
                } else {
                    println!("   ✅ Extracted {} chars", content.len());
                    let preview_len = 500.min(content.len());
                    let preview = &content[..preview_len];
                    println!("\n   📝 Full text preview:");
                    println!("   {}", "─".repeat(50));
                    println!("{}", preview);
                    println!("   {}", "─".repeat(50));
                }
            }
            Err(e) => {
                println!("   ❌ Extraction failed: {}", e);
            }
        }
    }

    // Also test Cold_Email_Hacks.pdf to verify our fix still works
    println!("\n📄 Cold_Email_Hacks - Verifying Issue #47 fix");

    let reader = PdfReader::open("test-pdfs/Cold_Email_Hacks.pdf")?;
    let document = PdfDocument::new(reader);
    let options = ExtractionOptions::default();
    let mut extractor = TextExtractor::with_options(options);

    match extractor.extract_from_page(&document, 14) {
        Ok(text) => {
            let content = text.text.trim();
            // Handle spaces in text (e.g., "R ead" instead of "Read")
            let normalized = content.replace(" ", "");
            if (content.contains("Read") || normalized.contains("Read"))
                && (content.contains("your") || normalized.contains("your"))
                && (content.contains("email") || normalized.contains("email"))
            {
                println!("   ✅ Page 14 extraction working correctly!");
                let preview_len = 300.min(content.len());
                let preview = &content[..preview_len];
                println!("\n   📝 Text preview:");
                println!("   {}", "─".repeat(50));
                println!("{}", preview);
                println!("   {}", "─".repeat(50));
            } else {
                println!("   ❌ Page 14 text not correctly decoded");
                println!(
                    "   🔍 First 200 chars: {:?}",
                    &content[..200.min(content.len())]
                );
            }
        }
        Err(e) => {
            println!("   ❌ Failed to extract page 14: {}", e);
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("✅ Test completed");
    Ok(())
}
