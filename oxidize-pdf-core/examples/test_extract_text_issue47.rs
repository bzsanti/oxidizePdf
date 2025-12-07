use oxidize_pdf::{PdfDocument, PdfReader};
use std::error::Error;
use std::path::Path;

fn main() -> Result<(), Box<dyn Error>> {
    println!("📝 Testing Text Extraction from Issue #47 - Corrupted PDF");
    println!("=======================================================");

    let pdf_path = "test-pdfs/Cold_Email_Hacks.pdf";

    if !Path::new(pdf_path).exists() {
        println!("❌ Test PDF not found: {}", pdf_path);
        println!("💡 Download it from: https://github.com/user-attachments/files/22399799/Cold.Email.Hacks.pdf");
        return Ok(());
    }

    println!("📖 Opening PDF: {}", pdf_path);

    // Test 1: Verify page count with PdfReader
    match PdfReader::open(pdf_path) {
        Ok(mut reader) => {
            println!("✅ PDF opened successfully with PdfReader");
            println!("📄 PDF version: {}", reader.version());

            match reader.page_count() {
                Ok(count) => {
                    println!("✅ Page count: {}", count);
                    if count == 44 {
                        println!("🎯 Correct page count confirmed!");
                    } else {
                        println!("⚠️  Unexpected page count, expected 44");
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get page count: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to open PDF with PdfReader: {}", e);
            return Ok(());
        }
    }

    println!("\n📝 Attempting text extraction with PdfDocument...");

    // Test 2: Try text extraction with PdfDocument
    match PdfReader::open(pdf_path) {
        Ok(reader) => {
            let document = PdfDocument::new(reader);
            println!("✅ PDF opened successfully with PdfDocument");

            match document.page_count() {
                Ok(page_count) => {
                    println!("📊 Document info:");
                    println!("   Pages: {}", page_count);

                    // Try to extract text from first few pages
                    for page_idx in 0..std::cmp::min(3, page_count) {
                        println!("\n📄 Extracting text from page {}...", page_idx + 1);

                        match document.extract_text_from_page(page_idx) {
                            Ok(extracted_text) => {
                                let text = &extracted_text.text;
                                println!("✅ Got page {} successfully", page_idx + 1);

                                // Check extracted text
                                let trimmed = text.trim();
                                if trimmed.is_empty() {
                                    println!("⚠️  Page {} appears to be empty", page_idx + 1);
                                } else {
                                    println!("✅ Text extracted from page {}", page_idx + 1);
                                    println!("📊 Text stats:");
                                    println!("   Characters: {}", trimmed.len());
                                    println!("   Words: {}", trimmed.split_whitespace().count());
                                    println!("   Lines: {}", trimmed.lines().count());

                                    // Show first 200 characters as preview
                                    let preview_len = std::cmp::min(200, trimmed.len());
                                    let preview = &trimmed[..preview_len];
                                    println!("📝 Text preview:");
                                    println!(
                                        "   \"{}{}\"",
                                        preview,
                                        if trimmed.len() > 200 { "..." } else { "" }
                                    );
                                }
                            }
                            Err(e) => {
                                println!(
                                    "❌ Failed to extract text from page {}: {}",
                                    page_idx + 1,
                                    e
                                );
                            }
                        }
                    }

                    // Try document-level text extraction
                    println!("\n📝 Trying document-level text extraction...");
                    match document.extract_text() {
                        Ok(text) => {
                            let total_chars: usize = text.iter().map(|page| page.text.len()).sum();
                            println!("✅ Document text extraction successful");
                            println!("📊 Total characters across all pages: {}", total_chars);

                            // Show stats for each page
                            for (idx, page_text) in text.iter().take(3).enumerate() {
                                println!("   Page {}: {} chars", idx + 1, page_text.text.len());
                            }
                        }
                        Err(e) => {
                            println!("❌ Document text extraction failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to get page count: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to open PDF with PdfDocument: {}", e);
            println!("🔍 Error chain:");
            let mut current = e.source();
            while let Some(err) = current {
                println!("  → {}", err);
                current = err.source();
            }
        }
    }

    println!("\n🏁 Text extraction test completed!");
    Ok(())
}
