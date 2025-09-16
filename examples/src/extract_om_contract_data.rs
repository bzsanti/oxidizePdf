//! Complete OCR extraction for O&M contracts
//!
//! This example demonstrates full text extraction from scanned O&M contract PDFs,
//! specifically designed to extract all readable text and validate key elements
//! including the target date "30 September 2016" in the FIS2 contract.

use oxidize_pdf::parser::{PdfDocument, PdfReader, ParseOptions};
use oxidize_pdf::operations::page_analysis::{PageContentAnalyzer, AnalysisOptions};
use oxidize_pdf::text::{OcrOptions, RustyTesseractProvider, TextValidator};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 COMPLETE OCR EXTRACTION FOR O&M CONTRACTS");
    println!("===============================================");
    println!("📋 Objective: Extract ALL text from scanned contract PDFs");
    println!("🎯 Validation: Search for '30 September 2016' in FIS2 contract");
    println!("");

    // Create optimized OCR provider for contracts
    let ocr_provider = match RustyTesseractProvider::for_contracts() {
        Ok(provider) => {
            println!("✅ Contract-optimized OCR Provider created");
            println!("   📧 Engine: Tesseract LSTM");
            println!("   🔧 PSM: 1 (Automatic page segmentation with OSD)");
            println!("   📐 DPI: 300");
            provider
        }
        Err(e) => {
            println!("❌ Cannot create OCR provider: {}", e);
            println!("   💡 Make sure tesseract is installed: brew install tesseract");
            return Ok(());
        }
    };

    // Create text validator for searching key elements
    let validator = TextValidator::new();

    // Test PDFs
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/Users/santifdezmunoz".to_string());
    let ocr_dir = format!("{}/Downloads/ocr", home_dir);

    let test_contracts = vec![
        ContractInfo {
            filename: "FIS2 160930 O&M Agreement ESS.pdf".to_string(),
            expected_target: "30 September 2016",
            description: "FIS2 O&M Agreement",
        },
        ContractInfo {
            filename: "MADRIDEJOS_O&M CONTRACT_2013.pdf".to_string(),
            expected_target: "2013", // Year validation for this contract
            description: "Madridejos O&M Contract",
        }
    ];

    let mut successful_extractions = 0;
    let mut total_pages_processed = 0;
    let mut total_characters_extracted = 0;

    for contract in &test_contracts {
        let pdf_path = Path::new(&ocr_dir).join(&contract.filename);

        if !pdf_path.exists() {
            println!("⚠️  File not found: {}", contract.filename);
            continue;
        }

        println!("\n📄 PROCESSING: {}", contract.description);
        println!("   📁 File: {}", contract.filename);
        println!("   📏 Size: {:.2}MB", std::fs::metadata(&pdf_path)?.len() as f64 / 1_048_576.0);
        println!("   🎯 Target: \"{}\"", contract.expected_target);
        println!("================================================");

        match extract_complete_text(&pdf_path, &ocr_provider, &validator, contract) {
            Ok(extraction_result) => {
                successful_extractions += 1;
                total_pages_processed += extraction_result.pages_processed;
                total_characters_extracted += extraction_result.total_characters;

                println!("✅ EXTRACTION COMPLETED!");
                println!("   📊 Pages processed: {}", extraction_result.pages_processed);
                println!("   📝 Total characters: {}", extraction_result.total_characters);
                println!("   ⏱️  Processing time: {:?}", extraction_result.duration);

                if extraction_result.target_found {
                    println!("   🎉 TARGET FOUND: \"{}\"", contract.expected_target);
                    println!("      📍 Found {} occurrences", extraction_result.target_matches);
                } else {
                    println!("   ⚠️  Target not found: \"{}\"", contract.expected_target);
                }

                if extraction_result.validation_result.found {
                    println!("   ✅ Contract validation: {} matches found", extraction_result.validation_result.matches.len());
                    println!("      📈 Confidence: {:.1}%", extraction_result.validation_result.confidence * 100.0);
                } else {
                    println!("   ℹ️  No contract elements detected");
                }

                println!("   📄 Output file: {}", extraction_result.output_file);
            }
            Err(e) => {
                println!("❌ EXTRACTION FAILED: {}", e);
            }
        }
    }

    // Summary
    println!("\n🏆 EXTRACTION SUMMARY");
    println!("====================");
    println!("📊 Successful extractions: {}/{}", successful_extractions, test_contracts.len());
    println!("📄 Total pages processed: {}", total_pages_processed);
    println!("📝 Total characters extracted: {}", total_characters_extracted);

    if successful_extractions > 0 {
        println!("\n✅ SUCCESS! OCR system successfully extracted text from O&M contracts");
        println!("📋 Check output files in examples/results/ for complete extracted text");
    } else {
        println!("\n❌ No successful extractions completed");
    }

    Ok(())
}

#[derive(Debug)]
struct ContractInfo {
    filename: String,
    expected_target: &'static str,
    description: &'static str,
}

#[derive(Debug)]
struct ExtractionResult {
    pages_processed: u32,
    total_characters: usize,
    duration: std::time::Duration,
    target_found: bool,
    target_matches: usize,
    validation_result: oxidize_pdf::text::TextValidationResult,
    output_file: String,
}

fn extract_complete_text(
    pdf_path: &Path,
    ocr_provider: &RustyTesseractProvider,
    validator: &TextValidator,
    contract: &ContractInfo,
) -> Result<ExtractionResult, Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    // Open PDF document
    let file = File::open(pdf_path)?;
    let reader = PdfReader::new_with_options(file, ParseOptions::tolerant())?;
    let document = PdfDocument::new(reader);

    // Get page count
    let page_count = document.page_count()?;
    println!("   📊 Document has {} pages", page_count);

    // Create analyzer with optimized options for contracts
    let options = AnalysisOptions {
        min_text_fragment_size: 2,
        min_image_size: 50,
        scanned_threshold: 0.7,
        text_threshold: 0.8,
        ocr_options: Some(OcrOptions {
            min_confidence: 0.3, // Lower threshold to capture more text
            preserve_layout: true,
            language: "eng".to_string(),
            ..Default::default()
        }),
    };

    let analyzer = PageContentAnalyzer::with_options(document, options);

    // Extract text from all pages
    let mut all_extracted_text = String::new();
    let mut pages_processed = 0u32;
    let mut total_characters = 0usize;

    for page_num in 0..page_count as usize {
        print!("   🔍 Processing page {}... ", page_num + 1);
        std::io::stdout().flush()?;

        match analyzer.analyze_page(page_num) {
            Ok(analysis) => {
                if analysis.is_scanned() {
                    // Extract text using OCR
                    match analyzer.extract_text_from_scanned_page(page_num, ocr_provider) {
                        Ok(ocr_result) => {
                            if !ocr_result.text.trim().is_empty() {
                                all_extracted_text.push_str(&format!("\n\n--- PAGE {} (OCR) ---\n", page_num + 1));
                                all_extracted_text.push_str(&ocr_result.text);
                                total_characters += ocr_result.text.len();
                                println!("OCR ✅ ({} chars, {:.1}% conf)", ocr_result.text.len(), ocr_result.confidence * 100.0);
                            } else {
                                println!("OCR 🔇 (no text)");
                            }
                            pages_processed += 1;
                        }
                        Err(e) => {
                            println!("OCR ❌ ({})", e);
                        }
                    }
                } else {
                    // Extract native text
                    let file2 = File::open(pdf_path)?;
                    let reader2 = PdfReader::new_with_options(file2, ParseOptions::tolerant())?;
                    let document2 = PdfDocument::new(reader2);

                    match document2.extract_text_from_page(page_num as u32) {
                        Ok(text_result) => {
                            if !text_result.text.trim().is_empty() {
                                all_extracted_text.push_str(&format!("\n\n--- PAGE {} (NATIVE) ---\n", page_num + 1));
                                all_extracted_text.push_str(&text_result.text);
                                total_characters += text_result.text.len();
                                println!("Native ✅ ({} chars)", text_result.text.len());
                            } else {
                                println!("Native 🔇 (no text)");
                            }
                            pages_processed += 1;
                        }
                        Err(e) => {
                            println!("Native ❌ ({})", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("Analysis ❌ ({})", e);
            }
        }
    }

    let duration = start_time.elapsed();

    // Search for target string
    let target_result = validator.search_for_target(&all_extracted_text, contract.expected_target);

    // Perform comprehensive validation
    let validation_result = validator.validate_contract_text(&all_extracted_text);

    // Save extracted text to file
    let output_filename = format!("extracted_text_{}.txt",
        contract.filename.replace(".pdf", "").replace(" ", "_").replace("&", "and"));
    let output_path = Path::new("examples/results").join(&output_filename);

    // Ensure output directory exists
    std::fs::create_dir_all("examples/results")?;

    let mut output_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&output_path)?;

    writeln!(output_file, "COMPLETE TEXT EXTRACTION FROM: {}", contract.filename)?;
    writeln!(output_file, "Generated on: {}", std::time::SystemTime::now().elapsed().unwrap_or_default().as_secs())?;
    writeln!(output_file, "Pages processed: {}", pages_processed)?;
    writeln!(output_file, "Total characters: {}", total_characters)?;
    writeln!(output_file, "Processing time: {:?}", duration)?;
    writeln!(output_file, "Target '{}' found: {}", contract.expected_target, target_result.found)?;
    writeln!(output_file, "")?;
    writeln!(output_file, "=== EXTRACTED TEXT ===")?;
    writeln!(output_file, "{}", all_extracted_text)?;

    if validation_result.found {
        writeln!(output_file, "\n\n=== KEY ELEMENTS DETECTED ===")?;
        for mat in &validation_result.matches {
            writeln!(output_file, "{:?}: \"{}\" (confidence: {:.1}%)",
                mat.match_type, mat.text, mat.confidence * 100.0)?;
        }
    }

    output_file.flush()?;

    Ok(ExtractionResult {
        pages_processed,
        total_characters,
        duration,
        target_found: target_result.found,
        target_matches: target_result.matches.len(),
        validation_result,
        output_file: output_path.to_string_lossy().to_string(),
    })
}