use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{PdfDocument, PdfReader};
use rand::RngExt;
use std::fs::{self, File};
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::path::Path;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 EXHAUSTIVE TEXT EXTRACTION TEST - ALL 766 PDFs from fixtures");
    println!("{}", "=".repeat(80));
    println!("🎯 Goal: Find PDFs with REAL extractable text (not garbled, not empty)");
    println!("⚙️  Strategy: Try up to 5 pages per PDF, strict validation");
    println!();

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

    println!("📊 Found {} PDF files to process", pdf_files.len());

    // Sort for consistent results
    pdf_files.sort();

    let options = ExtractionOptions::default();

    let mut stats = ExhaustiveStats::default();
    let mut detailed_results = Vec::new();
    let overall_start = Instant::now();

    let mut rng = rand::rng();

    // Known problematic PDFs that cause parser panics
    const PROBLEMATIC_PDFS: &[&str] = &[
        "AcreditacionESHOW24_MADRID_V11474238.pdf",
        "AcreditacionESHOW24_MADRID_V11474238_Santiago+Fern%c3%a1ndez-2.pdf",
    ];

    for (i, pdf_path) in pdf_files.iter().enumerate() {
        let pdf_name = pdf_path.file_name().unwrap().to_string_lossy();

        if i % 10 == 0 {
            println!("\n📈 Progress: {}/{} PDFs processed...", i, pdf_files.len());
        }

        // Skip known problematic PDFs that cause deep parser panics
        if PROBLEMATIC_PDFS.iter().any(|&name| pdf_name.contains(name)) {
            println!("⚠️  {} - Skipped (known parser panic issue)", pdf_name);
            let skipped_result = PdfResult {
                filename: pdf_name.to_string(),
                status: PdfStatus::ExtractionError,
                pages_tried: 0,
                chars_extracted: 0,
                text_preview: String::new(),
                processing_time: Duration::default(),
                error_message: Some("Skipped: Known to cause parser panic".to_string()),
            };
            detailed_results.push(skipped_result);
            stats.extraction_errors += 1;
            stats.total_processed += 1;
            continue;
        }

        // Robust error handling with panic catching
        let pdf_result = panic::catch_unwind(AssertUnwindSafe(|| {
            process_single_pdf(pdf_path, &options, &mut rng)
        }))
        .unwrap_or_else(|_| PdfResult {
            filename: pdf_name.to_string(),
            status: PdfStatus::ExtractionError,
            pages_tried: 0,
            chars_extracted: 0,
            text_preview: String::new(),
            processing_time: Duration::default(),
            error_message: Some("Parser panic (UTF-8/corruption issue)".to_string()),
        });

        // Update statistics and show progress
        match &pdf_result.status {
            PdfStatus::Success => {
                stats.successful_real_text += 1;
                if i < 20 || stats.successful_real_text <= 10 {
                    println!(
                        "✅ {} - {} chars extracted",
                        pdf_name, pdf_result.chars_extracted
                    );
                    if !pdf_result.text_preview.is_empty() {
                        println!(
                            "   📝 Preview: \"{}\"",
                            &pdf_result.text_preview[..100.min(pdf_result.text_preview.len())]
                        );
                    }
                }
            }
            PdfStatus::AllPagesEmpty => {
                stats.all_pages_empty += 1;
                if i < 50 {
                    // Show first 50 empty PDFs
                    println!("📭 {} - all pages empty", pdf_name);
                }
            }
            PdfStatus::AllPagesGarbled => {
                stats.all_pages_garbled += 1;
                if i < 50 {
                    // Show first 50 garbled PDFs
                    println!("🔤 {} - all pages garbled", pdf_name);
                }
            }
            PdfStatus::FailedToOpen => {
                stats.failed_to_open += 1;
                println!(
                    "❌ {} - failed to open: {}",
                    pdf_name,
                    pdf_result.error_message.as_deref().unwrap_or("unknown")
                );
            }
            PdfStatus::ExtractionError => {
                stats.extraction_errors += 1;
                println!(
                    "⚠️  {} - extraction error: {}",
                    pdf_name,
                    pdf_result.error_message.as_deref().unwrap_or("unknown")
                );
            }
        }

        // Check if this was a panic case for emergency saving
        let had_panic = matches!(pdf_result.error_message, Some(ref msg) if msg.contains("panic"));

        detailed_results.push(pdf_result);
        stats.total_processed += 1;

        // Save intermediate results every 100 PDFs or immediately after a panic
        let should_save = i % 100 == 99 || i == pdf_files.len() - 1 || had_panic;

        if should_save {
            let intermediate_path = if had_panic {
                format!("examples/results/panic_recovery_{}.csv", i + 1)
            } else {
                format!("examples/results/intermediate_report_{}.csv", i + 1)
            };

            if let Err(e) = save_csv_report(&detailed_results, &intermediate_path) {
                println!("⚠️  Warning: Failed to save intermediate report: {}", e);
            } else if i % 100 == 99 {
                println!("💾 Intermediate results saved ({} PDFs processed)", i + 1);
            } else if had_panic {
                println!("💾 Emergency save after panic (PDF {})", i + 1);
            }
        }
    }

    let total_time = overall_start.elapsed();

    // Print comprehensive summary
    print_final_summary(&stats, total_time);

    // Save detailed CSV report
    save_csv_report(
        &detailed_results,
        "examples/results/exhaustive_extraction_report.csv",
    )?;

    // Print top problematic PDFs
    print_problematic_pdfs(&detailed_results);

    println!("\n{}", "=".repeat(80));
    println!("📊 Detailed report saved to: examples/results/exhaustive_extraction_report.csv");
    println!("🔍 Exhaustive text extraction analysis completed!");

    Ok(())
}

fn process_single_pdf(
    pdf_path: &Path,
    options: &ExtractionOptions,
    rng: &mut impl RngExt,
) -> PdfResult {
    let mut result = PdfResult {
        filename: pdf_path.file_name().unwrap().to_string_lossy().to_string(),
        status: PdfStatus::FailedToOpen,
        pages_tried: 0,
        chars_extracted: 0,
        text_preview: String::new(),
        processing_time: Duration::default(),
        error_message: None,
    };

    let start_time = Instant::now();
    let timeout_duration = Duration::from_secs(10); // 10 second timeout per PDF

    // Create fresh extractor for this PDF to avoid corrupted state
    let mut extractor = TextExtractor::with_options(options.clone());

    // Use lenient parsing options for better error recovery
    use oxidize_pdf::parser::ParseOptions;
    let parse_options = ParseOptions::lenient();

    let reader = match PdfReader::open_with_options(pdf_path, parse_options) {
        Ok(reader) => reader,
        Err(e) => {
            result.error_message = Some(format!("Failed to open: {}", e));
            return result;
        }
    };

    let document = PdfDocument::new(reader);
    let page_count = match document.page_count() {
        Ok(count) => count,
        Err(e) => {
            result.error_message = Some(format!("Failed to get page count: {}", e));
            return result;
        }
    };

    // Try up to 5 random pages to find real text
    let max_attempts = 5.min(page_count as usize);
    let mut pages_to_try = Vec::new();

    // Generate random page numbers (1-indexed)
    while pages_to_try.len() < max_attempts {
        let page_num = rng.random_range(1..=page_count);
        if !pages_to_try.contains(&page_num) {
            pages_to_try.push(page_num);
        }
    }

    let mut all_empty = true;
    let mut all_garbled = true;

    for page_num in pages_to_try {
        // Check timeout
        if start_time.elapsed() > timeout_duration {
            result.error_message = Some("Processing timeout exceeded".to_string());
            result.status = PdfStatus::ExtractionError;
            result.processing_time = start_time.elapsed();
            return result;
        }

        result.pages_tried += 1;

        // Ultra-safe extraction with panic catching at the deepest level
        let extraction_result = panic::catch_unwind(AssertUnwindSafe(|| {
            extractor.extract_from_page(&document, page_num - 1)
        }));

        match extraction_result {
            Ok(Ok(extracted_text)) => {
                let content = extracted_text.text.trim();

                if !content.is_empty() {
                    all_empty = false;

                    let validation = validate_text_quality(content);

                    if validation.is_real_text {
                        // Found real, readable text!
                        result.status = PdfStatus::Success;
                        result.chars_extracted = content.len();
                        // Safe string truncation respecting UTF-8 boundaries
                        let preview_len = 200.min(content.len());
                        let mut end = preview_len;
                        while end > 0 && !content.is_char_boundary(end) {
                            end -= 1;
                        }
                        result.text_preview = content[..end].to_string();
                        result.processing_time = start_time.elapsed();
                        return result;
                    } else if validation.readability_score > 0.5 {
                        // Not completely garbled, but not great either
                        all_garbled = false;
                    }
                }
            }
            Ok(Err(e)) => {
                result.error_message =
                    Some(format!("Extraction error on page {}: {}", page_num, e));
                result.status = PdfStatus::ExtractionError;
                result.processing_time = start_time.elapsed();
                return result;
            }
            Err(_) => {
                // Panic occurred during page extraction - continue to next page
                result.error_message =
                    Some(format!("Parser panic on page {} extraction", page_num));
                // Continue trying other pages, don't return yet
            }
        }
    }

    // No real text found after trying multiple pages
    result.processing_time = start_time.elapsed();

    if all_empty {
        result.status = PdfStatus::AllPagesEmpty;
    } else if all_garbled {
        result.status = PdfStatus::AllPagesGarbled;
    } else {
        result.status = PdfStatus::AllPagesGarbled; // Mixed but no real text found
    }

    result
}

#[derive(Debug)]
struct TextValidation {
    is_real_text: bool,
    readability_score: f64,
    _has_words: bool,
    _has_sentences: bool,
    _excessive_spaces: bool,
}

fn validate_text_quality(text: &str) -> TextValidation {
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

    if total_chars < 50 {
        return TextValidation {
            is_real_text: false,
            readability_score: 0.0,
            _has_words: false,
            _has_sentences: false,
            _excessive_spaces: false,
        };
    }

    // Count readable characters
    let readable_chars = chars
        .iter()
        .filter(|c| {
            c.is_alphabetic() || c.is_whitespace() || c.is_ascii_punctuation() || c.is_numeric()
        })
        .count();

    let readability_score = readable_chars as f64 / total_chars as f64;

    // Check for words (sequences of alphabetic characters)
    let word_count = text
        .split_whitespace()
        .filter(|word| word.chars().any(|c| c.is_alphabetic()))
        .count();

    let has_words = word_count >= 5;

    // Check for sentence-like structures
    let sentence_endings = text.matches(&['.', '!', '?']).count();
    let has_sentences = sentence_endings > 0 && text.len() > 100;

    // Check for excessive spaces (sign of garbled text)
    let space_sequences = text.matches("  ").count(); // Two or more spaces
    let excessive_spaces = space_sequences > text.len() / 20; // More than 5% of text

    // Strict validation criteria
    let is_real_text = readability_score >= 0.85 &&  // At least 85% readable chars
                      has_words &&                    // Contains actual words
                      !excessive_spaces &&            // Not excessive spacing
                      word_count >= 10; // Reasonable word count

    TextValidation {
        is_real_text,
        readability_score,
        _has_words: has_words,
        _has_sentences: has_sentences,
        _excessive_spaces: excessive_spaces,
    }
}

#[derive(Default)]
struct ExhaustiveStats {
    total_processed: usize,
    successful_real_text: usize,
    all_pages_empty: usize,
    all_pages_garbled: usize,
    failed_to_open: usize,
    extraction_errors: usize,
}

#[derive(Debug)]
struct PdfResult {
    filename: String,
    status: PdfStatus,
    pages_tried: usize,
    chars_extracted: usize,
    text_preview: String,
    processing_time: Duration,
    error_message: Option<String>,
}

#[derive(Debug)]
enum PdfStatus {
    Success,
    AllPagesEmpty,
    AllPagesGarbled,
    FailedToOpen,
    ExtractionError,
}

impl PdfStatus {
    fn as_str(&self) -> &str {
        match self {
            PdfStatus::Success => "SUCCESS",
            PdfStatus::AllPagesEmpty => "ALL_EMPTY",
            PdfStatus::AllPagesGarbled => "ALL_GARBLED",
            PdfStatus::FailedToOpen => "FAILED_OPEN",
            PdfStatus::ExtractionError => "EXTRACT_ERROR",
        }
    }
}

fn print_final_summary(stats: &ExhaustiveStats, total_time: Duration) {
    println!("\n{}", "=".repeat(80));
    println!("📊 FINAL EXHAUSTIVE RESULTS");
    println!("{}", "=".repeat(80));

    let total = stats.total_processed;

    println!("🔢 Processing Summary:");
    println!("   • Total PDFs processed: {}", total);
    println!(
        "   • Total processing time: {:.2} seconds",
        total_time.as_secs_f64()
    );
    println!(
        "   • Average time per PDF: {:.2}ms",
        if total > 0 {
            total_time.as_millis() as f64 / total as f64
        } else {
            0.0
        }
    );

    println!("\n📄 Extraction Results:");
    println!(
        "   ✅ REAL TEXT EXTRACTED: {} ({:.1}%)",
        stats.successful_real_text,
        percentage(stats.successful_real_text, total)
    );

    println!(
        "   📭 All pages empty: {} ({:.1}%)",
        stats.all_pages_empty,
        percentage(stats.all_pages_empty, total)
    );

    println!(
        "   🔤 All pages garbled: {} ({:.1}%)",
        stats.all_pages_garbled,
        percentage(stats.all_pages_garbled, total)
    );

    println!(
        "   ❌ Failed to open: {} ({:.1}%)",
        stats.failed_to_open,
        percentage(stats.failed_to_open, total)
    );

    println!(
        "   ⚠️  Extraction errors: {} ({:.1}%)",
        stats.extraction_errors,
        percentage(stats.extraction_errors, total)
    );

    println!("\n🎯 SYSTEM ASSESSMENT:");
    let success_rate = percentage(stats.successful_real_text, total);

    if success_rate >= 70.0 {
        println!("   ✅ EXCELLENT: Text extraction system works very well!");
    } else if success_rate >= 50.0 {
        println!("   ✅ GOOD: Text extraction system works reasonably well");
    } else if success_rate >= 30.0 {
        println!("   ⚠️  MODERATE: Text extraction has significant issues");
    } else {
        println!("   ❌ POOR: Text extraction system has major problems");
    }
}

fn percentage(part: usize, total: usize) -> f64 {
    if total > 0 {
        part as f64 / total as f64 * 100.0
    } else {
        0.0
    }
}

fn save_csv_report(
    results: &[PdfResult],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure directory exists
    if let Some(parent) = Path::new(output_path).parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = File::create(output_path)?;

    // CSV header
    writeln!(
        file,
        "filename,status,pages_tried,chars_extracted,processing_time_ms,error_message,text_preview"
    )?;

    for result in results {
        let error_msg = result.error_message.as_deref().unwrap_or("");
        let preview = result
            .text_preview
            .replace("\"", "\"\"") // Escape quotes for CSV
            .replace("\n", "\\n") // Replace newlines
            .replace("\r", "\\r"); // Replace carriage returns

        writeln!(
            file,
            "\"{}\",{},{},{},{:.2},\"{}\",\"{}\"",
            result.filename,
            result.status.as_str(),
            result.pages_tried,
            result.chars_extracted,
            result.processing_time.as_secs_f64() * 1000.0,
            error_msg,
            &preview[..200.min(preview.len())]
        )?;
    }

    Ok(())
}

fn print_problematic_pdfs(results: &[PdfResult]) {
    println!("\n📋 PROBLEMATIC PDFs (top 10 of each category):");

    // All empty pages
    let empty_pdfs: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.status, PdfStatus::AllPagesEmpty))
        .take(10)
        .collect();

    if !empty_pdfs.is_empty() {
        println!("\n📭 ALL PAGES EMPTY:");
        for pdf in empty_pdfs {
            println!("   • {}", pdf.filename);
        }
    }

    // All garbled text
    let garbled_pdfs: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.status, PdfStatus::AllPagesGarbled))
        .take(10)
        .collect();

    if !garbled_pdfs.is_empty() {
        println!("\n🔤 ALL PAGES GARBLED:");
        for pdf in garbled_pdfs {
            println!("   • {}", pdf.filename);
        }
    }

    // Failed to open
    let failed_pdfs: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.status, PdfStatus::FailedToOpen))
        .take(10)
        .collect();

    if !failed_pdfs.is_empty() {
        println!("\n❌ FAILED TO OPEN:");
        for pdf in failed_pdfs {
            println!(
                "   • {} - {}",
                pdf.filename,
                pdf.error_message.as_deref().unwrap_or("Unknown error")
            );
        }
    }

    // Top successful extractions
    let mut successful_pdfs: Vec<_> = results
        .iter()
        .filter(|r| matches!(r.status, PdfStatus::Success))
        .collect();

    successful_pdfs.sort_by_key(|r| std::cmp::Reverse(r.chars_extracted));

    if !successful_pdfs.is_empty() {
        println!("\n✅ TOP SUCCESSFUL EXTRACTIONS:");
        for pdf in successful_pdfs.iter().take(10) {
            println!("   • {} - {} chars", pdf.filename, pdf.chars_extracted);
        }
    }
}
