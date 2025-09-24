use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{PdfDocument, PdfReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Demonstrating text extraction from Cold_Email_Hacks.pdf");
    println!("{}", "=".repeat(70));

    let reader = PdfReader::open("test-pdfs/Cold_Email_Hacks.pdf")?;
    let document = PdfDocument::new(reader);

    // Get page count
    let page_count = document.page_count()?;
    println!("📊 Total pages in document: {}", page_count);

    let options = ExtractionOptions::default();
    let mut extractor = TextExtractor::with_options(options);

    // Test extraction from multiple pages to verify our fix works across the document
    let test_pages = vec![14, 21, 11, 5, 30];

    for page_num in test_pages {
        if page_num > page_count {
            continue;
        }

        println!("\n📄 Extracting text from page {}...", page_num);
        println!("{}", "─".repeat(50));

        match extractor.extract_from_page(&document, page_num) {
            Ok(text) => {
                let content = text.text.trim();
                if content.is_empty() {
                    println!(
                        "   ⚠️  Page {} is empty or has no extractable text",
                        page_num
                    );
                } else {
                    println!("   ✅ Successfully extracted {} characters", content.len());

                    // Show a preview of the text (safe Unicode slicing)
                    let preview = if content.len() <= 300 {
                        content
                    } else {
                        let mut end = 300;
                        while end > 0 && !content.is_char_boundary(end) {
                            end -= 1;
                        }
                        &content[..end]
                    };

                    println!("\n   📝 Text preview:");
                    println!("   {}", "·".repeat(40));
                    println!("{}", preview);
                    if content.len() > preview.len() {
                        println!(
                            "   ...(truncated, {} more chars)",
                            content.len() - preview.len()
                        );
                    }
                    println!("   {}", "·".repeat(40));

                    // Check if this looks like readable text (not garbled)
                    let readable_chars = content
                        .chars()
                        .filter(|c| {
                            c.is_alphabetic() || c.is_whitespace() || c.is_ascii_punctuation()
                        })
                        .count();
                    let total_chars = content.chars().count();
                    let readability_ratio = if total_chars > 0 {
                        readable_chars as f64 / total_chars as f64
                    } else {
                        0.0
                    };

                    if readability_ratio > 0.8 {
                        println!(
                            "   ✅ Text appears readable ({}% standard characters)",
                            (readability_ratio * 100.0) as u32
                        );
                    } else {
                        println!(
                            "   ⚠️  Text may be garbled (only {}% standard characters)",
                            (readability_ratio * 100.0) as u32
                        );
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Failed to extract text: {}", e);
            }
        }
    }

    // Demonstrate the Issue #47 fix specifically
    println!("\n");
    println!("🎯 Issue #47 Verification");
    println!("{}", "=".repeat(30));
    println!("Before fix: Text was garbled like '5 H D G \\ R X U H P D L O'");
    println!("After fix: Text should read 'Read your email to somebody else'");
    println!();

    match extractor.extract_from_page(&document, 14) {
        Ok(text) => {
            let content = text.text.trim();
            let first_line = content.lines().next().unwrap_or("").trim();

            println!("📄 Page 14, first line extracted:");
            println!("   \"{}\"", first_line);

            // Check for the key words that should be present
            let normalized = content.replace(" ", "").replace("​", ""); // Remove spaces and zero-width spaces
            if normalized.contains("Read")
                && normalized.contains("your")
                && normalized.contains("email")
            {
                println!("   ✅ SUCCESS: Issue #47 is resolved! Text is readable.");
            } else {
                println!("   ❌ FAILED: Text doesn't contain expected content.");
                let safe_preview = if content.len() <= 100 {
                    content
                } else {
                    let mut end = 100;
                    while end > 0 && !content.is_char_boundary(end) {
                        end -= 1;
                    }
                    &content[..end]
                };
                println!("      First 100 chars: {:?}", safe_preview);
            }
        }
        Err(e) => {
            println!("   ❌ FAILED to extract page 14: {}", e);
        }
    }

    println!("\n{}", "=".repeat(70));
    println!("✅ Demonstration completed");
    Ok(())
}
