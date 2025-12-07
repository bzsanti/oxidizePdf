use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{PdfDocument, PdfReader};
use std::path::Path;

fn main() {
    let pdfs = vec![
        (
            "tests/fixtures/CONTRATO PRESTAMO_signed.pdf",
            "CONTRATO PRESTAMO",
        ),
        (
            "tests/fixtures/d5a4cb2f-175d-422c-8fc8-2296ae51ee14.pdf",
            "d5a4cb2f",
        ),
        (
            "tests/fixtures/G085278584_36d6c890f36d404ba7979777fd4854d2.pdf",
            "G085278584",
        ),
        (
            "tests/fixtures/20220603 QE Biometrical Consent def.pdf",
            "Biometrical Consent",
        ),
        (
            "tests/fixtures/PE24-1056 Oferta O365, Azure y SBC - Quintas Energy 19_08_2024.pdf",
            "Quintas Energy",
        ),
        (
            "tests/fixtures/IONOS factura 2024-09-25 - FA_202780573832.pdf",
            "IONOS factura",
        ),
        (
            "tests/fixtures/applied_cryptography_protocols_algorithms_and_source_code_in_c.pdf",
            "Applied Cryptography",
        ),
        (
            "tests/fixtures/TRANSFERENCIAAFAVORDEMARIANOJOSEAMOLOSADACONCEPTO_Factura2023006.pdf",
            "TRANSFERENCIA",
        ),
        (
            "tests/fixtures/Resguardo_1efbb1ea-6275-6800-b800-e7d338df3c30.pdf",
            "Resguardo",
        ),
        (
            "tests/fixtures/ssasperfguide2008r2 (1).pdf",
            "SQL Server Performance",
        ),
    ];

    println!("🔍 Testing text extraction from 10 random PDFs");
    println!("{}", "=".repeat(60));

    for (pdf_path, name) in pdfs {
        if !Path::new(pdf_path).exists() {
            println!("\n❌ {} - File not found", name);
            continue;
        }

        print!("\n📄 {} - ", name);

        match PdfReader::open(pdf_path) {
            Ok(reader) => {
                let document = PdfDocument::new(reader);

                // Get page count
                let page_count = match document.page_count() {
                    Ok(count) => count,
                    Err(_) => {
                        println!("Failed to get page count");
                        continue;
                    }
                };

                if page_count <= 2 {
                    println!("Only {} page(s), skipping", page_count);
                    continue;
                }

                // Select a random middle page
                let page_num = if page_count > 3 { page_count / 2 } else { 2 };

                println!("Extracting page {} of {}", page_num, page_count);

                let options = ExtractionOptions::default();
                let mut extractor = TextExtractor::with_options(options);

                match extractor.extract_from_page(&document, page_num) {
                    Ok(text) => {
                        let content = text.text.trim();
                        if content.is_empty() {
                            println!("   ⚠️  Page {} is empty", page_num);
                        } else {
                            let preview_len = 200.min(content.len());
                            let preview = &content[..preview_len];
                            println!("   ✅ Extracted {} chars", content.len());
                            println!("   📝 Preview: {:?}...", preview);
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Extraction failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to open: {}", e);
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("✅ Test completed");
}
