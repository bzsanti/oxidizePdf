// Example: Test text extraction on random PDFs from middle pages
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{PdfDocument, PdfReader};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎲 Testing text extraction on 10 random PDFs");
    println!("{}", "=".repeat(60));

    let test_pdfs = vec![
        "./test-pdfs/unicode_glyph_mapping_test.pdf",
        "./test-pdfs/Cold_Email_Hacks.pdf",
        "./oxidize-pdf-core/output.pdf",
        "./test-pdfs/unicode_showcase.pdf",
        "./oxidize-pdf-core/examples/results/empty_dashboard_test.pdf",
        "./test-pdfs/unicode_professional_demo.pdf",
        "./oxidize-pdf-core/examples/results/large_dashboard_test.pdf",
        "./test-pdfs/jpeg_extraction_test.pdf",
        "./test-pdfs/png_transparency.pdf",
        "./test-pdfs/page_tree_inheritance.pdf",
    ];

    let options = ExtractionOptions::default();
    let mut extractor = TextExtractor::with_options(options);

    let mut successful_extractions = 0;
    let mut total_chars_extracted = 0;
    let mut readable_pdfs = 0;

    for (i, pdf_path) in test_pdfs.iter().enumerate() {
        println!("\n📄 {} | {}", i + 1, pdf_path);
        println!("{}", "-".repeat(50));

        if !Path::new(pdf_path).exists() {
            println!("   ❌ File does not exist - skipping");
            continue;
        }

        match PdfReader::open(pdf_path) {
            Ok(reader) => {
                let document = PdfDocument::new(reader);

                match document.page_count() {
                    Ok(page_count) => {
                        println!("   📊 Total pages: {}", page_count);

                        // Get middle page (1-indexed)
                        let middle_page = if page_count == 1 {
                            1
                        } else {
                            (page_count / 2) + 1
                        };

                        println!("   🎯 Testing middle page: {}", middle_page);

                        match extractor.extract_from_page(&document, middle_page) {
                            Ok(text) => {
                                let content = text.text.trim();
                                successful_extractions += 1;
                                total_chars_extracted += content.len();

                                if content.is_empty() {
                                    println!("   ⚠️  Empty or no extractable text");
                                } else {
                                    println!("   ✅ Extracted {} characters", content.len());

                                    // Show preview (first 200 chars)
                                    let preview_len = 200.min(content.len());
                                    let mut end = preview_len;
                                    while end > 0 && !content.is_char_boundary(end) {
                                        end -= 1;
                                    }
                                    let preview = &content[..end];

                                    println!(
                                        "   📝 Preview: \"{}{}\"",
                                        preview,
                                        if content.len() > preview_len {
                                            "..."
                                        } else {
                                            ""
                                        }
                                    );

                                    // Check readability
                                    let readable_chars = content
                                        .chars()
                                        .filter(|c| {
                                            c.is_alphabetic()
                                                || c.is_whitespace()
                                                || c.is_ascii_punctuation()
                                        })
                                        .count();
                                    let total_chars = content.chars().count();
                                    let readability = if total_chars > 0 {
                                        readable_chars as f64 / total_chars as f64
                                    } else {
                                        0.0
                                    };

                                    if readability > 0.8 {
                                        readable_pdfs += 1;
                                        println!(
                                            "   ✅ Text is readable ({}% standard chars)",
                                            (readability * 100.0) as u32
                                        );
                                    } else {
                                        println!(
                                            "   ⚠️  Text may be garbled ({}% standard chars)",
                                            (readability * 100.0) as u32
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                println!("   ❌ Extraction failed: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Could not get page count: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Could not open PDF: {}", e);
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("📈 SUMMARY RESULTS");
    println!("{}", "=".repeat(60));
    println!("📊 Total PDFs tested: {}", test_pdfs.len());
    println!("✅ Successful extractions: {}", successful_extractions);
    println!("📝 Total characters extracted: {}", total_chars_extracted);
    println!(
        "🔤 Readable PDFs: {} ({:.1}%)",
        readable_pdfs,
        if successful_extractions > 0 {
            readable_pdfs as f64 / successful_extractions as f64 * 100.0
        } else {
            0.0
        }
    );

    if successful_extractions > 0 {
        println!(
            "📏 Average chars per extraction: {}",
            total_chars_extracted / successful_extractions
        );
    }

    Ok(())
}
