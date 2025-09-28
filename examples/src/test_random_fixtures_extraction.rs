use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{PdfDocument, PdfReader};
#[cfg(feature = "rand")]
use rand::seq::SliceRandom;
#[cfg(feature = "rand")]
use rand::Rng;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎲 Testing text extraction on 10 random PDFs from tests/fixtures/");
    println!("{}", "=".repeat(70));

    // Get all PDF files from tests/fixtures
    let fixtures_dir = "tests/fixtures";
    if !Path::new(fixtures_dir).exists() {
        println!("❌ Directory {} does not exist", fixtures_dir);
        return Ok(());
    }

    let mut pdf_files = Vec::new();
    for entry in fs::read_dir(fixtures_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
            pdf_files.push(path);
        }
    }

    if pdf_files.is_empty() {
        println!("❌ No PDF files found in {}", fixtures_dir);
        return Ok(());
    }

    println!(
        "📊 Found {} PDF files in fixtures directory",
        pdf_files.len()
    );

    // Randomly select 10 PDFs
    #[cfg(feature = "rand")]
    {
        let mut rng = rand::rng();
        pdf_files.shuffle(&mut rng);
    }
    let selected_pdfs = pdf_files.into_iter().take(10).collect::<Vec<_>>();

    let options = ExtractionOptions::default();
    let mut extractor = TextExtractor::with_options(options);

    let mut stats = ExtractionStats::new();
    let overall_start = Instant::now();

    for (i, pdf_path) in selected_pdfs.iter().enumerate() {
        let pdf_name = pdf_path.file_name().unwrap().to_string_lossy();
        println!("\n📄 {} | {}", i + 1, pdf_name);
        println!("{}", "-".repeat(60));

        let start_time = Instant::now();

        // Use lenient parsing options for better error recovery
        use oxidize_pdf::parser::ParseOptions;
        let parse_options = ParseOptions::lenient();

        match PdfReader::open_with_options(pdf_path, parse_options) {
            Ok(reader) => {
                let document = PdfDocument::new(reader);

                match document.page_count() {
                    Ok(page_count) => {
                        println!("   📊 Total pages: {}", page_count);
                        stats.total_pdfs_processed += 1;

                        // Select page according to rules:
                        // - If >3 pages: random page excluding first and last
                        // - If ≤3 pages: any random page
                        let selected_page = {
                            #[cfg(feature = "rand")]
                            {
                                if page_count > 3 {
                                    // Exclude first (page 1) and last (page page_count)
                                    // So we can select from page 2 to page_count-1 (inclusive)
                                    let min_page = 2;
                                    let max_page = page_count - 1;
                                    let mut rng = rand::rng();
                                    rng.gen_range(min_page..=max_page)
                                } else {
                                    // Any page from 1 to page_count
                                    let mut rng = rand::rng();
                                    rng.gen_range(1..=page_count)
                                }
                            }
                            #[cfg(not(feature = "rand"))]
                            {
                                // Without rand, just use page 0 (first page)
                                if page_count > 0 {
                                    0
                                } else {
                                    0
                                }
                            }
                        };

                        println!(
                            "   🎯 Selected page: {} (strategy: {})",
                            selected_page,
                            if page_count > 3 {
                                "middle pages only"
                            } else {
                                "any page"
                            }
                        );

                        match extractor.extract_from_page(&document, selected_page - 1) {
                            Ok(extracted_text) => {
                                let content = extracted_text.text.trim();
                                let processing_time = start_time.elapsed();

                                stats.successful_extractions += 1;
                                stats.total_processing_time += processing_time;
                                stats.total_chars_extracted += content.len();

                                if content.is_empty() {
                                    println!(
                                        "   ⚠️  Page {} is empty or has no extractable text",
                                        selected_page
                                    );
                                    stats.empty_pages += 1;
                                } else {
                                    println!(
                                        "   ✅ Extracted {} characters in {:.2}ms",
                                        content.len(),
                                        processing_time.as_secs_f64() * 1000.0
                                    );

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
                                        stats.readable_extractions += 1;
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

                                    // Fragment info if available
                                    if !extracted_text.fragments.is_empty() {
                                        println!(
                                            "   📍 Found {} text fragments with position data",
                                            extracted_text.fragments.len()
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                println!("   ❌ Extraction failed: {}", e);
                                stats.failed_extractions += 1;
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Could not get page count: {}", e);
                        stats.failed_pdf_opens += 1;
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Could not open PDF: {}", e);
                stats.failed_pdf_opens += 1;
            }
        }
    }

    let total_time = overall_start.elapsed();

    // Print comprehensive summary
    println!("\n{}", "=".repeat(70));
    println!("📈 COMPREHENSIVE RESULTS SUMMARY");
    println!("{}", "=".repeat(70));

    println!("🔢 Processing Statistics:");
    println!("   • Total PDFs selected: {}", selected_pdfs.len());
    println!("   • Successfully opened: {}", stats.total_pdfs_processed);
    println!("   • Failed to open: {}", stats.failed_pdf_opens);

    println!("\n📄 Text Extraction Results:");
    println!(
        "   • Successful extractions: {}",
        stats.successful_extractions
    );
    println!("   • Failed extractions: {}", stats.failed_extractions);
    println!("   • Empty pages found: {}", stats.empty_pages);
    println!(
        "   • Readable extractions: {} ({:.1}%)",
        stats.readable_extractions,
        if stats.successful_extractions > 0 {
            stats.readable_extractions as f64 / stats.successful_extractions as f64 * 100.0
        } else {
            0.0
        }
    );

    println!("\n📊 Content Statistics:");
    println!(
        "   • Total characters extracted: {}",
        stats.total_chars_extracted
    );
    if stats.successful_extractions > 0 {
        println!(
            "   • Average chars per extraction: {}",
            stats.total_chars_extracted / stats.successful_extractions
        );
    }

    println!("\n⏱️  Performance Metrics:");
    println!(
        "   • Total processing time: {:.2}ms",
        total_time.as_secs_f64() * 1000.0
    );
    if stats.successful_extractions > 0 {
        let avg_time =
            stats.total_processing_time.as_secs_f64() / stats.successful_extractions as f64;
        println!("   • Average extraction time: {:.2}ms", avg_time * 1000.0);
        println!(
            "   • Throughput: {:.1} extractions/sec",
            stats.successful_extractions as f64 / total_time.as_secs_f64()
        );
    }

    // Success rate
    let success_rate = if selected_pdfs.len() > 0 {
        stats.successful_extractions as f64 / selected_pdfs.len() as f64 * 100.0
    } else {
        0.0
    };

    println!("\n🎯 Overall Success Rate: {:.1}%", success_rate);

    if success_rate >= 80.0 {
        println!("✅ Excellent extraction performance!");
    } else if success_rate >= 60.0 {
        println!("⚠️  Moderate extraction performance - some issues detected");
    } else {
        println!("❌ Poor extraction performance - significant issues detected");
    }

    println!("\n{}", "=".repeat(70));
    println!("🔍 Random fixture extraction test completed!");

    Ok(())
}

#[derive(Default)]
struct ExtractionStats {
    total_pdfs_processed: usize,
    failed_pdf_opens: usize,
    successful_extractions: usize,
    failed_extractions: usize,
    empty_pages: usize,
    readable_extractions: usize,
    total_chars_extracted: usize,
    total_processing_time: Duration,
}

impl ExtractionStats {
    fn new() -> Self {
        Self::default()
    }
}
