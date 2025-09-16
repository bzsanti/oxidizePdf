//! Multi-page OCR search for "30 September 2016" in contract PDFs

#[cfg(feature = "ocr-tesseract")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::operations::page_analysis::{AnalysisOptions, PageContentAnalyzer};
    use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
    use oxidize_pdf::text::{OcrOptions, OcrProvider, RustyTesseractProvider};
    use std::fs::File;
    use std::path::Path;
    use std::time::Instant;

    println!("🔍 MULTI-PAGE SEARCH FOR '30 SEPTEMBER 2016'");
    println!("==============================================");

    // Create OCR provider with different configurations
    let contract_provider = match RustyTesseractProvider::for_contracts() {
        Ok(provider) => {
            println!("✅ Contract-optimized OCR Provider ready");
            provider
        }
        Err(e) => {
            println!("❌ Cannot create OCR provider: {}", e);
            return Ok(());
        }
    };

    let standard_provider = match RustyTesseractProvider::new() {
        Ok(provider) => {
            println!("✅ Standard OCR Provider ready");
            provider
        }
        Err(e) => {
            println!("❌ Cannot create standard OCR provider: {}", e);
            return Ok(());
        }
    };

    let test_pdfs = [
        "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf",
        "/Users/santifdezmunoz/Downloads/ocr/MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    for (pdf_index, pdf_path) in test_pdfs.iter().enumerate() {
        let path = Path::new(pdf_path);
        if !path.exists() {
            continue;
        }

        println!(
            "\n📄 PROCESSING PDF {}: {}",
            pdf_index + 1,
            path.file_name().unwrap().to_string_lossy()
        );
        println!("===============================================");

        let start_total = Instant::now();

        match process_pdf_for_date(path, &contract_provider, &standard_provider) {
            Ok(results) => {
                println!("✅ PDF PROCESSING COMPLETED");
                println!("   📊 Pages processed: {}", results.pages_processed);
                println!("   📝 Total text extracted: {} chars", results.total_text_length);

                if results.found_target {
                    println!("   🎉 SUCCESS! Found '30 September 2016'!");
                    for occurrence in &results.target_occurrences {
                        println!("      📍 Page {}: \"{}\"", occurrence.page, occurrence.context);
                    }
                } else {
                    println!("   ⚠️  Target '30 September 2016' not found");

                    if !results.date_patterns.is_empty() {
                        println!("   📅 Other dates found:");
                        for (i, date) in results.date_patterns.iter().take(5).enumerate() {
                            println!("      {}. \"{}\"", i + 1, date);
                        }
                    }

                    println!("   🔍 Date components found:");
                    println!("      '30': {}", results.found_30);
                    println!("      'September': {}", results.found_september);
                    println!("      '2016': {}", results.found_2016);
                }

                if !results.extracted_samples.is_empty() {
                    println!("   📖 Text samples extracted:");
                    for sample in &results.extracted_samples {
                        println!("      Page {}: \"{}\"", sample.page, sample.text);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ PDF processing failed: {}", e);
            }
        }

        println!("   ⏱️  Total processing time: {:?}", start_total.elapsed());
    }

    println!("\n🏁 Multi-page search completed!");
    Ok(())
}

#[cfg(feature = "ocr-tesseract")]
#[derive(Debug)]
struct SearchResults {
    pages_processed: usize,
    total_text_length: usize,
    found_target: bool,
    target_occurrences: Vec<TargetOccurrence>,
    date_patterns: Vec<String>,
    found_30: bool,
    found_september: bool,
    found_2016: bool,
    extracted_samples: Vec<TextSample>,
}

#[cfg(feature = "ocr-tesseract")]
#[derive(Debug)]
struct TargetOccurrence {
    page: usize,
    context: String,
}

#[cfg(feature = "ocr-tesseract")]
#[derive(Debug)]
struct TextSample {
    page: usize,
    text: String,
}

#[cfg(feature = "ocr-tesseract")]
fn process_pdf_for_date(
    pdf_path: &Path,
    contract_provider: &RustyTesseractProvider,
    standard_provider: &RustyTesseractProvider,
) -> Result<SearchResults, Box<dyn std::error::Error>> {
    use regex::Regex;

    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);
    let page_count = document.page_count()?;

    println!("   📊 Document has {} pages", page_count);

    let mut results = SearchResults {
        pages_processed: 0,
        total_text_length: 0,
        found_target: false,
        target_occurrences: Vec::new(),
        date_patterns: Vec::new(),
        found_30: false,
        found_september: false,
        found_2016: false,
        extracted_samples: Vec::new(),
    };

    // Process first 10 pages or all if fewer
    let pages_to_check = std::cmp::min(page_count as usize, 10);
    println!("   🔍 Checking first {} pages for text/OCR...", pages_to_check);

    for page_idx in 0..pages_to_check {
        print!("      📄 Page {} ... ", page_idx + 1);

        let mut page_text = String::new();
        let mut extraction_method = String::new();

        // Try native text extraction first
        let file2 = File::open(pdf_path)?;
        let reader2 = PdfReader::new_with_options(file2, ParseOptions::tolerant())?;
        let document2 = PdfDocument::new(reader2);

        match document2.extract_text_from_page(page_idx as u32) {
            Ok(text_result) => {
                if !text_result.text.trim().is_empty() {
                    page_text = text_result.text;
                    extraction_method = format!("native ({} chars)", page_text.len());
                }
            }
            Err(_) => {} // Ignore errors, try OCR
        }

        // If no native text, try OCR
        if page_text.trim().is_empty() {
            // Create analyzer for this specific page
            let file3 = File::open(pdf_path)?;
            let reader3 = PdfReader::new_with_options(file3, ParseOptions::tolerant())?;
            let document3 = PdfDocument::new(reader3);

            let options = AnalysisOptions {
                min_text_fragment_size: 1,
                min_image_size: 5,
                scanned_threshold: 0.5,
                text_threshold: 0.3,
                ocr_options: Some(OcrOptions {
                    min_confidence: 0.1,
                    preserve_layout: false, // Try without layout preservation
                    language: "eng".to_string(),
                    ..Default::default()
                }),
            };

            let analyzer = PageContentAnalyzer::with_options(document3, options);

            // Try both OCR providers
            for (provider_name, provider) in [("contract", contract_provider), ("standard", standard_provider)] {
                if !page_text.trim().is_empty() {
                    break; // Already got text
                }

                match analyzer.extract_text_from_scanned_page(page_idx, provider) {
                    Ok(ocr_result) => {
                        if !ocr_result.text.trim().is_empty() {
                            page_text = ocr_result.text;
                            extraction_method = format!("OCR-{} ({} chars, {:.1}% conf)",
                                provider_name, page_text.len(), ocr_result.confidence * 100.0);
                            break;
                        }
                    }
                    Err(_) => {} // Try next provider
                }
            }
        }

        if !page_text.trim().is_empty() {
            println!("✓ {}", extraction_method);

            results.pages_processed += 1;
            results.total_text_length += page_text.len();

            // Search for target
            if page_text.contains("30 September 2016") {
                results.found_target = true;
                if let Some(pos) = page_text.find("30 September 2016") {
                    let start = pos.saturating_sub(30);
                    let end = (pos + 80).min(page_text.len());
                    let context = page_text[start..end].replace('\n', " ");
                    results.target_occurrences.push(TargetOccurrence {
                        page: page_idx + 1,
                        context,
                    });
                }
            }

            // Check components
            if page_text.contains("30") { results.found_30 = true; }
            if page_text.to_lowercase().contains("september") { results.found_september = true; }
            if page_text.contains("2016") { results.found_2016 = true; }

            // Extract date patterns
            let date_regex = Regex::new(r"\b\d{1,2}\s+\w+\s+\d{4}\b").unwrap();
            for mat in date_regex.find_iter(&page_text) {
                let date_str = mat.as_str().to_string();
                if !results.date_patterns.contains(&date_str) {
                    results.date_patterns.push(date_str);
                }
            }

            // Store sample if interesting
            if page_text.len() > 50 && results.extracted_samples.len() < 3 {
                let sample = if page_text.len() > 150 {
                    format!("{}...", &page_text[..150].replace('\n', " "))
                } else {
                    page_text.replace('\n', " ")
                };
                results.extracted_samples.push(TextSample {
                    page: page_idx + 1,
                    text: sample,
                });
            }
        } else {
            println!("empty/failed");
        }
    }

    Ok(results)
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("❌ OCR feature not enabled");
}