//! OCR test specifically searching for "30 September 2016" in contract PDFs

#[cfg(feature = "ocr-tesseract")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use oxidize_pdf::operations::page_analysis::{AnalysisOptions, PageContentAnalyzer};
    use oxidize_pdf::parser::{ParseOptions, PdfDocument, PdfReader};
    use oxidize_pdf::text::{OcrOptions, OcrProvider, RustyTesseractProvider};
    use oxidize_pdf::text::validation::{TextValidator, MatchType};
    use std::fs::File;
    use std::path::Path;
    use std::time::Instant;

    println!("🔍 SEARCHING FOR '30 SEPTEMBER 2016' IN CONTRACT PDFs");
    println!("====================================================");
    println!("Target: Extract and find '30 September 2016' in PDF text");

    // Create OCR provider optimized for contracts
    let ocr_provider = match RustyTesseractProvider::for_contracts() {
        Ok(provider) => {
            println!("✅ OCR Provider ready (contract-optimized)");
            provider
        }
        Err(e) => {
            println!("❌ Cannot create OCR provider: {}", e);
            return Ok(());
        }
    };

    // Create text validator
    let validator = TextValidator::new();
    println!("✅ Text validator ready");

    let test_pdfs = [
        "/Users/santifdezmunoz/Downloads/ocr/FIS2 160930 O&M Agreement ESS.pdf",
        "/Users/santifdezmunoz/Downloads/ocr/MADRIDEJOS_O&M CONTRACT_2013.pdf",
    ];

    let mut found_target = false;
    let mut total_text_found = String::new();

    for (pdf_index, pdf_path) in test_pdfs.iter().enumerate() {
        let path = Path::new(pdf_path);
        if !path.exists() {
            println!("⚠️  File not found: {}", pdf_path);
            continue;
        }

        println!(
            "\n📄 PROCESSING PDF {}: {}",
            pdf_index + 1,
            path.file_name().unwrap().to_string_lossy()
        );
        println!("===============================================");

        let start = Instant::now();

        match File::open(path) {
            Ok(file) => {
                println!("   ✅ File opened successfully");

                match PdfReader::new_with_options(file, ParseOptions::tolerant()) {
                    Ok(reader) => {
                        let document = PdfDocument::new(reader);

                        match document.page_count() {
                            Ok(page_count) => {
                                println!("   📊 Document has {} pages", page_count);

                                // Process first 5 pages (or all if fewer)
                                let pages_to_check = std::cmp::min(page_count as usize, 5);
                                println!("   🔍 Checking first {} pages...", pages_to_check);

                                // Create analyzer with optimized settings
                                let options = AnalysisOptions {
                                    min_text_fragment_size: 1, // Lower threshold
                                    min_image_size: 5,
                                    scanned_threshold: 0.5, // More sensitive
                                    text_threshold: 0.3,
                                    ocr_options: Some(OcrOptions {
                                        min_confidence: 0.1, // Lower confidence threshold
                                        preserve_layout: true,
                                        language: "eng".to_string(),
                                        ..Default::default()
                                    }),
                                };

                                let analyzer = PageContentAnalyzer::with_options(document, options);

                                for page_idx in 0..pages_to_check {
                                    print!("      📄 Page {} ... ", page_idx + 1);

                                    match analyzer.analyze_page(page_idx) {
                                        Ok(analysis) => {
                                            println!("{:?} (img:{:.0}%, txt:{:.0}%)",
                                                    analysis.page_type,
                                                    analysis.image_ratio * 100.0,
                                                    analysis.text_ratio * 100.0);

                                            let mut page_text = String::new();

                                            // Try native text first
                                            let file2 = File::open(path)?;
                                            let reader2 = PdfReader::new_with_options(file2, ParseOptions::tolerant())?;
                                            let document2 = PdfDocument::new(reader2);

                                            match document2.extract_text_from_page(page_idx as u32) {
                                                Ok(text_result) => {
                                                    if !text_result.text.trim().is_empty() {
                                                        page_text = text_result.text;
                                                        println!("         ✓ Native text: {} chars", page_text.len());
                                                    }
                                                }
                                                Err(_) => {} // Ignore native text errors
                                            }

                                            // If no native text and page is scanned, try OCR
                                            if page_text.trim().is_empty() && analysis.is_scanned() {
                                                println!("         🔤 Attempting OCR...");
                                                match analyzer.extract_text_from_scanned_page(page_idx, &ocr_provider) {
                                                    Ok(ocr_result) => {
                                                        if !ocr_result.text.trim().is_empty() {
                                                            page_text = ocr_result.text;
                                                            println!("         ✓ OCR text: {} chars (conf: {:.1}%)",
                                                                   page_text.len(), ocr_result.confidence * 100.0);
                                                        } else {
                                                            println!("         ❌ OCR returned no text");
                                                        }
                                                    }
                                                    Err(e) => {
                                                        println!("         ❌ OCR failed: {}", e);
                                                    }
                                                }
                                            }

                                            // Search for target date in page text
                                            if !page_text.trim().is_empty() {
                                                total_text_found.push_str(&page_text);
                                                total_text_found.push('\n');

                                                // Search for exact target
                                                if page_text.contains("30 September 2016") {
                                                    println!("         🎉 FOUND TARGET: '30 September 2016'!");
                                                    found_target = true;

                                                    // Show context
                                                    if let Some(pos) = page_text.find("30 September 2016") {
                                                        let start = pos.saturating_sub(50);
                                                        let end = (pos + 80).min(page_text.len());
                                                        let context = &page_text[start..end].replace('\n', " ");
                                                        println!("         📍 Context: \"...{}...\"", context);
                                                    }
                                                }

                                                // Search using validator
                                                let search_result = validator.search_for_target(&page_text, "30 September 2016");
                                                if search_result.found {
                                                    println!("         ✅ Validator found {} matches", search_result.matches.len());
                                                    found_target = true;
                                                }

                                                // Look for date patterns
                                                let validation_result = validator.validate_contract_text(&page_text);
                                                let date_matches: Vec<_> = validation_result.matches.iter()
                                                    .filter(|m| m.match_type == MatchType::Date)
                                                    .collect();

                                                if !date_matches.is_empty() {
                                                    println!("         📅 Found {} date patterns:", date_matches.len());
                                                    for (i, mat) in date_matches.iter().take(3).enumerate() {
                                                        println!("            {}. \"{}\"", i + 1, mat.text);
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            println!("Error: {}", e);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("   ❌ Cannot get page count: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Cannot read PDF: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Cannot open file: {}", e);
            }
        }

        println!("   ⏱️  Processing time: {:?}", start.elapsed());
    }

    // Final summary
    println!("\n🏁 SEARCH SUMMARY");
    println!("==================");

    if found_target {
        println!("🎉 SUCCESS! Found '30 September 2016' in the documents!");
    } else {
        println!("⚠️  Target '30 September 2016' not found");

        if !total_text_found.trim().is_empty() {
            println!("📊 Total text extracted: {} characters", total_text_found.len());

            // Search for components
            println!("🔍 Checking for date components in all extracted text:");
            let components = ["30", "September", "Sept", "2016", "09"];
            for component in &components {
                if total_text_found.contains(component) {
                    println!("   ✓ Found: '{}'", component);
                } else {
                    println!("   ✗ Missing: '{}'", component);
                }
            }

            // Show any dates found
            let overall_validation = validator.validate_contract_text(&total_text_found);
            let all_dates: Vec<_> = overall_validation.matches.iter()
                .filter(|m| m.match_type == MatchType::Date)
                .collect();

            if !all_dates.is_empty() {
                println!("📅 All dates found in documents:");
                for (i, mat) in all_dates.iter().take(10).enumerate() {
                    println!("   {}. \"{}\"", i + 1, mat.text);
                }
                if all_dates.len() > 10 {
                    println!("   ... and {} more dates", all_dates.len() - 10);
                }
            }

            // Show sample of extracted text
            if total_text_found.len() > 1000 {
                println!("\n📖 Sample of extracted text (first 500 chars):");
                println!("\"{}...\"", &total_text_found[..500].replace('\n', " "));
            }
        } else {
            println!("❌ No text could be extracted from any PDF pages");
            println!("💡 The PDFs may contain only images or be heavily corrupted");
        }
    }

    Ok(())
}

#[cfg(not(feature = "ocr-tesseract"))]
fn main() {
    println!("❌ OCR feature not enabled");
    println!("💡 Use: cargo run --example ocr_date_search --features ocr-tesseract");
}