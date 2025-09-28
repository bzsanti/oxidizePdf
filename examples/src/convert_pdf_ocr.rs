//! PDF OCR Converter - Command Line Tool
//!
//! Convert scanned PDFs to searchable PDFs using OCR technology.
//! This tool automatically detects scanned pages and adds an invisible text layer
//! while preserving the original visual appearance.
//!
//! # Usage
//!
//! ```bash
//! # Convert a single PDF
//! cargo run --example convert_pdf_ocr -- input.pdf output.pdf
//!
//! # Convert with custom language
//! cargo run --example convert_pdf_ocr -- input.pdf output.pdf --lang spa
//!
//! # Convert with high DPI for better accuracy
//! cargo run --example convert_pdf_ocr -- input.pdf output.pdf --dpi 600
//!
//! # Batch convert all PDFs in a directory
//! cargo run --example convert_pdf_ocr -- --batch input_dir/ output_dir/
//!
//! # Show help
//! cargo run --example convert_pdf_ocr -- --help
//! ```

use oxidize_pdf::operations::pdf_ocr_converter::{ConversionOptions, PdfOcrConverter};
use oxidize_pdf::text::{OcrOptions, RustyTesseractProvider};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug)]
struct CliOptions {
    input_path: PathBuf,
    output_path: PathBuf,
    language: String,
    dpi: u32,
    min_confidence: f64,
    batch_mode: bool,
    verbose: bool,
    skip_text_pages: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            input_path: PathBuf::new(),
            output_path: PathBuf::new(),
            language: "eng".to_string(),
            dpi: 300,
            min_confidence: 0.7,
            batch_mode: false,
            verbose: false,
            skip_text_pages: true,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = parse_args()?;

    if options.verbose {
        println!("🔍 PDF OCR Converter v1.2.3");
        println!(
            "Converting with language: {}, DPI: {}",
            options.language, options.dpi
        );
    }

    // Create OCR provider
    let ocr_provider = create_ocr_provider(&options)?;
    let converter = PdfOcrConverter::new()?;

    let conversion_options = ConversionOptions {
        ocr_options: OcrOptions {
            language: options.language.clone(),
            min_confidence: options.min_confidence,
            ..Default::default()
        },
        min_confidence: options.min_confidence,
        skip_text_pages: options.skip_text_pages,
        dpi: options.dpi,
        progress_callback: if options.verbose {
            Some(Box::new(|current, total| {
                println!("📄 Processing page {} of {}", current + 1, total);
            }))
        } else {
            None
        },
        ..Default::default()
    };

    let start_time = Instant::now();

    if options.batch_mode {
        process_batch(&converter, &options, &conversion_options, &ocr_provider)?;
    } else {
        process_single_file(&converter, &options, &conversion_options, &ocr_provider)?;
    }

    let total_time = start_time.elapsed();
    if options.verbose {
        println!(
            "✅ Conversion completed in {:.2}s",
            total_time.as_secs_f64()
        );
    }

    Ok(())
}

fn parse_args() -> Result<CliOptions, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        std::process::exit(0);
    }

    let mut options = CliOptions::default();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--batch" => {
                options.batch_mode = true;
                i += 1;
            }
            "--lang" | "-l" => {
                if i + 1 >= args.len() {
                    return Err("Missing language argument".into());
                }
                options.language = args[i + 1].clone();
                i += 2;
            }
            "--dpi" | "-d" => {
                if i + 1 >= args.len() {
                    return Err("Missing DPI argument".into());
                }
                options.dpi = args[i + 1].parse().map_err(|_| "Invalid DPI value")?;
                i += 2;
            }
            "--confidence" | "-c" => {
                if i + 1 >= args.len() {
                    return Err("Missing confidence argument".into());
                }
                options.min_confidence = args[i + 1]
                    .parse()
                    .map_err(|_| "Invalid confidence value")?;
                if options.min_confidence < 0.0 || options.min_confidence > 1.0 {
                    return Err("Confidence must be between 0.0 and 1.0".into());
                }
                i += 2;
            }
            "--verbose" | "-v" => {
                options.verbose = true;
                i += 1;
            }
            "--no-skip-text" => {
                options.skip_text_pages = false;
                i += 1;
            }
            arg if arg.starts_with('-') => {
                return Err(format!("Unknown option: {}", arg).into());
            }
            _ => {
                // Input and output paths
                if options.input_path.as_os_str().is_empty() {
                    options.input_path = PathBuf::from(&args[i]);
                } else if options.output_path.as_os_str().is_empty() {
                    options.output_path = PathBuf::from(&args[i]);
                } else {
                    return Err("Too many arguments".into());
                }
                i += 1;
            }
        }
    }

    if options.input_path.as_os_str().is_empty() {
        return Err("Missing input path".into());
    }

    if options.output_path.as_os_str().is_empty() {
        if options.batch_mode {
            return Err("Missing output directory for batch mode".into());
        } else {
            return Err("Missing output path".into());
        }
    }

    Ok(options)
}

fn create_ocr_provider(
    options: &CliOptions,
) -> Result<RustyTesseractProvider, Box<dyn std::error::Error>> {
    if options.verbose {
        println!("🔧 Initializing Tesseract OCR engine...");
    }

    let provider = RustyTesseractProvider::new()
        .map_err(|e| format!("Failed to initialize OCR provider: {}", e))?;

    if options.verbose {
        println!("✅ OCR engine ready");
    }

    Ok(provider)
}

fn process_single_file(
    converter: &PdfOcrConverter,
    options: &CliOptions,
    conversion_options: &ConversionOptions,
    ocr_provider: &RustyTesseractProvider,
) -> Result<(), Box<dyn std::error::Error>> {
    if !options.input_path.exists() {
        return Err(format!(
            "Input file does not exist: {}",
            options.input_path.display()
        )
        .into());
    }

    if !options
        .input_path
        .extension()
        .map_or(false, |ext| ext == "pdf")
    {
        return Err("Input file must be a PDF".into());
    }

    if options.verbose {
        println!(
            "📂 Converting: {} → {}",
            options.input_path.display(),
            options.output_path.display()
        );
    }

    let result = converter.convert_to_searchable_pdf(
        &options.input_path,
        &options.output_path,
        ocr_provider,
        conversion_options,
    )?;

    if options.verbose {
        print_conversion_result(&result, &options.output_path);
    } else {
        println!("✅ Converted: {}", options.output_path.display());
    }

    Ok(())
}

fn process_batch(
    converter: &PdfOcrConverter,
    options: &CliOptions,
    conversion_options: &ConversionOptions,
    ocr_provider: &RustyTesseractProvider,
) -> Result<(), Box<dyn std::error::Error>> {
    if !options.input_path.is_dir() {
        return Err("Input path must be a directory for batch mode".into());
    }

    // Create output directory if it doesn't exist
    if !options.output_path.exists() {
        fs::create_dir_all(&options.output_path)?;
    }

    // Find all PDF files in input directory
    let mut pdf_files = Vec::new();
    for entry in fs::read_dir(&options.input_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "pdf") {
            pdf_files.push(path);
        }
    }

    if pdf_files.is_empty() {
        return Err("No PDF files found in input directory".into());
    }

    if options.verbose {
        println!("📂 Found {} PDF files to process", pdf_files.len());
    }

    let results = converter.batch_convert(
        &pdf_files,
        &options.output_path,
        ocr_provider,
        conversion_options,
    )?;

    // Print summary
    let total_pages: usize = results.iter().map(|r| r.pages_processed).sum();
    let total_ocr_pages: usize = results.iter().map(|r| r.pages_ocr_processed).sum();
    let successful_conversions = results.len();

    println!("📊 Batch conversion summary:");
    println!("   Files processed: {}", successful_conversions);
    println!("   Total pages: {}", total_pages);
    println!("   Pages with OCR: {}", total_ocr_pages);

    if options.verbose {
        for (i, result) in results.iter().enumerate() {
            println!(
                "   File {}: {} pages, {} OCR'd, {:.1}% avg confidence",
                i + 1,
                result.pages_processed,
                result.pages_ocr_processed,
                result.average_confidence * 100.0
            );
        }
    }

    Ok(())
}

fn print_conversion_result(
    result: &oxidize_pdf::operations::pdf_ocr_converter::ConversionResult,
    output_path: &Path,
) {
    println!("📊 Conversion Results:");
    println!("   Output: {}", output_path.display());
    println!("   Pages processed: {}", result.pages_processed);
    println!("   Pages with OCR: {}", result.pages_ocr_processed);
    println!("   Pages skipped: {}", result.pages_skipped);
    println!(
        "   Processing time: {:.2}s",
        result.processing_time.as_secs_f64()
    );
    println!(
        "   Average confidence: {:.1}%",
        result.average_confidence * 100.0
    );
    println!(
        "   Characters extracted: {}",
        result.total_characters_extracted
    );
}

fn print_help() {
    println!("PDF OCR Converter v1.2.3");
    println!("Convert scanned PDFs to searchable PDFs using OCR");
    println!();
    println!("USAGE:");
    println!("    convert_pdf_ocr [OPTIONS] <INPUT> <OUTPUT>");
    println!("    convert_pdf_ocr [OPTIONS] --batch <INPUT_DIR> <OUTPUT_DIR>");
    println!();
    println!("ARGS:");
    println!("    <INPUT>      Input PDF file or directory (for batch mode)");
    println!("    <OUTPUT>     Output PDF file or directory (for batch mode)");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help                     Print help information");
    println!("    -v, --verbose                  Enable verbose output");
    println!("    -l, --lang <LANGUAGE>          OCR language (default: eng)");
    println!("                                   Examples: eng, spa, fra, deu, chi_sim, jpn");
    println!("                                   Multiple: eng+spa+fra");
    println!("    -d, --dpi <DPI>                Image DPI for OCR (default: 300)");
    println!("    -c, --confidence <THRESHOLD>   Minimum OCR confidence (0.0-1.0, default: 0.7)");
    println!("    --batch                        Batch process all PDFs in input directory");
    println!("    --no-skip-text                 Don't skip pages that already contain text");
    println!();
    println!("EXAMPLES:");
    println!("    # Convert a single PDF");
    println!("    convert_pdf_ocr input.pdf output.pdf");
    println!();
    println!("    # Convert with Spanish OCR");
    println!("    convert_pdf_ocr input.pdf output.pdf --lang spa");
    println!();
    println!("    # Convert with high DPI for better accuracy");
    println!("    convert_pdf_ocr input.pdf output.pdf --dpi 600");
    println!();
    println!("    # Batch convert directory");
    println!("    convert_pdf_ocr --batch input_dir/ output_dir/");
    println!();
    println!("    # Convert with multiple languages and verbose output");
    println!("    convert_pdf_ocr input.pdf output.pdf --lang eng+spa --verbose");
    println!();
    println!("NOTES:");
    println!("    - Requires Tesseract OCR to be installed on your system");
    println!("    - Install language packs for non-English OCR");
    println!("    - Higher DPI values improve accuracy but increase processing time");
    println!("    - The tool preserves original visual appearance while adding searchable text");
}
