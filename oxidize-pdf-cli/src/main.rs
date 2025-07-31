//! # oxidize-pdf-cli
//!
//! Command-line interface for oxidize-pdf - a comprehensive, pure Rust PDF library.
//!
//! ## Overview
//!
//! oxidize-pdf-cli provides a powerful command-line interface to create, manipulate, and analyze PDF files
//! using the oxidize-pdf library. It supports PDF generation, merging, splitting, rotation, text extraction,
//! and document analysis.
//!
//! ## Installation
//!
//! ```bash
//! cargo install oxidize-pdf-cli
//! ```
//!
//! Or build from source:
//! ```bash
//! git clone https://github.com/bzsanti/oxidizePdf.git
//! cd oxidizePdf
//! cargo build --release -p oxidize-pdf-cli
//! ```
//!
//! ## Quick Start
//!
//! Create a simple PDF:
//! ```bash
//! oxidizepdf create -o hello.pdf -t "Hello, World!"
//! ```
//!
//! Generate a demo PDF with graphics:
//! ```bash
//! oxidizepdf demo -o demo.pdf
//! ```
//!
//! Extract text from a PDF:
//! ```bash
//! oxidizepdf extract-text document.pdf
//! ```
//!
//! Get PDF information:
//! ```bash
//! oxidizepdf info document.pdf --detailed
//! ```
//!
//! ## Command Reference
//!
//! ### Create
//! Create a simple PDF with text content.
//!
//! ```bash
//! oxidizepdf create -o output.pdf -t "Your text here"
//! ```
//!
//! ### Demo
//! Generate a demonstration PDF showcasing graphics and text capabilities.
//!
//! ```bash
//! oxidizepdf demo [-o demo.pdf]
//! ```
//!
//! ### Info
//! Display information about a PDF file including metadata and structure.
//!
//! ```bash
//! oxidizepdf info input.pdf [--detailed]
//! ```
//!
//! ### Extract Text
//! Extract text content from PDF files with optional layout preservation.
//!
//! ```bash
//! oxidizepdf extract-text input.pdf [-o output.txt] [-p page_number] [--preserve-layout]
//! ```
//!
//! ### Rotate
//! Rotate pages in a PDF document.
//!
//! ```bash
//! oxidizepdf rotate input.pdf -o output.pdf [-a angle] [-p pages]
//! # Examples:
//! oxidizepdf rotate doc.pdf -o rotated.pdf -a 90 -p "1,3,5"
//! oxidizepdf rotate doc.pdf -o rotated.pdf -a 180 -p "2-6"
//! ```
//!
//! ### Merge (Coming Soon)
//! Merge multiple PDF files into a single document.
//!
//! ```bash
//! oxidizepdf merge file1.pdf file2.pdf -o merged.pdf [-p "1-5,all,2,4-6"]
//! ```
//!
//! ### Split (Coming Soon)
//! Split a PDF into multiple files.
//!
//! ```bash
//! oxidizepdf split input.pdf [-p pattern] [-m mode] [--spec pages]
//! ```
//!
//! ## Features
//!
//! - **Pure Rust**: No external PDF dependencies
//! - **Fast**: Native performance with zero overhead
//! - **Comprehensive**: Supports creation, manipulation, and analysis
//! - **Cross-platform**: Works on Windows, macOS, and Linux
//! - **Memory efficient**: Streaming operations for large files
//!
//! ## Examples
//!
//! ### Creating a PDF with formatted text
//! ```bash
//! oxidizepdf create -o report.pdf -t "Annual Report 2025"
//! ```
//!
//! ### Extracting text with layout preservation
//! ```bash
//! oxidizepdf extract-text invoice.pdf -o invoice.txt --preserve-layout
//! ```
//!
//! ### Rotating specific pages
//! ```bash
//! oxidizepdf rotate document.pdf -o fixed.pdf -a 90 -p "1,3,5-10"
//! ```
//!
//! ### Getting detailed PDF information
//! ```bash
//! oxidizepdf info complex.pdf --detailed
//! ```
//!
//! ## Error Handling
//!
//! The CLI provides detailed error messages for common issues:
//! - Invalid file paths
//! - Unsupported PDF features
//! - Invalid page ranges
//! - Parsing errors
//!
//! Exit codes:
//! - 0: Success
//! - 1: Error occurred
//!
//! ## Performance
//!
//! oxidize-pdf-cli is optimized for performance:
//! - Fast PDF parsing (< 50ms for typical documents)
//! - Efficient memory usage with streaming operations
//! - Multi-threaded operations where applicable
//!
//! ## Limitations
//!
//! Current limitations (as of v0.1.2):
//! - No support for encrypted PDFs
//! - Limited to JPEG images (PNG support planned)
//! - Text extraction limited to simple encoding
//! - Merge and split operations coming in Q2 2025
//!
//! ## License
//!
//! GPL v3.0 - See LICENSE file for details

use anyhow::Result;
use clap::{Parser, Subcommand};
use oxidize_pdf::operations::{rotate_pdf_pages, PageRange, RotateOptions, RotationAngle};
use oxidize_pdf::text::{ExtractionOptions, TextExtractor};
use oxidize_pdf::{Color, Document, Font, Page, PdfReader};
use std::path::PathBuf;

/// Command-line interface for oxidize-pdf operations.
///
/// This CLI provides access to all major PDF operations including creation,
/// manipulation, analysis, and text extraction. It's designed to be fast,
/// memory-efficient, and easy to use.
///
/// # Examples
///
/// Create a simple PDF:
/// ```bash
/// oxidizepdf create -o hello.pdf -t "Hello, World!"
/// ```
///
/// Extract text from a PDF:
/// ```bash
/// oxidizepdf extract-text document.pdf -o extracted.txt
/// ```
///
/// Get PDF information:
/// ```bash
/// oxidizepdf info document.pdf --detailed
/// ```
#[derive(Parser)]
#[command(
    name = "oxidizepdf",
    about = "A native Rust PDF processing tool",
    version,
    author
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available commands for PDF operations.
///
/// Each command provides specific functionality for working with PDF files,
/// from simple creation to complex manipulation and analysis.
#[derive(Subcommand)]
enum Commands {
    /// Create a simple PDF with text
    ///
    /// Generates a new PDF document with the specified text content.
    /// The text is rendered using Helvetica font at 24pt size.
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Create a PDF with custom text
    /// oxidizepdf create -o hello.pdf -t "Hello, World!"
    ///
    /// # Create a PDF with multi-line text (use quotes)
    /// oxidizepdf create -o report.pdf -t "Annual Report\n2025"
    /// ```
    Create {
        /// Output file path for the generated PDF
        #[arg(short, long)]
        output: PathBuf,

        /// Text content to include in the PDF
        #[arg(short, long)]
        text: String,
    },

    /// Generate a demo PDF with graphics
    ///
    /// Creates a demonstration PDF showcasing various features including:
    /// - Vector graphics (rectangles, circles)
    /// - Text with different fonts and sizes
    /// - Colors and styling
    /// - Document metadata
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Generate demo with default name
    /// oxidizepdf demo
    ///
    /// # Generate demo with custom name
    /// oxidizepdf demo -o showcase.pdf
    /// ```
    Demo {
        /// Output file path for the demo PDF
        #[arg(short, long, default_value = "demo.pdf")]
        output: PathBuf,
    },

    /// Merge multiple PDFs into one
    Merge {
        /// Input PDF files
        files: Vec<PathBuf>,

        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Page ranges for each file (e.g., "1-5,all,2,4-6")
        #[arg(short, long)]
        pages: Option<Vec<String>>,
    },

    /// Split a PDF into multiple files
    Split {
        /// Input PDF file
        input: PathBuf,

        /// Output pattern (use {} for page number)
        #[arg(short = 'p', long, default_value = "page_{}.pdf")]
        pattern: String,

        /// Split mode: pages, ranges, or at specific pages
        #[arg(short, long, default_value = "pages")]
        mode: String,

        /// Page specification (depends on mode)
        #[arg(long)]
        spec: Option<String>,
    },

    /// Rotate pages in a PDF
    ///
    /// Rotates specified pages by the given angle (90, 180, or 270 degrees).
    /// Supports various page selection methods including individual pages,
    /// ranges, and combinations.
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Rotate all pages 90 degrees
    /// oxidizepdf rotate input.pdf -o output.pdf
    ///
    /// # Rotate specific pages 180 degrees
    /// oxidizepdf rotate input.pdf -o output.pdf -a 180 -p "1,3,5"
    ///
    /// # Rotate page range 270 degrees
    /// oxidizepdf rotate input.pdf -o output.pdf -a 270 -p "2-6"
    ///
    /// # Rotate mixed selection
    /// oxidizepdf rotate input.pdf -o output.pdf -p "1,3-5,8"
    /// ```
    Rotate {
        /// Path to the input PDF file
        input: PathBuf,

        /// Path for the output PDF with rotated pages
        #[arg(short, long)]
        output: PathBuf,

        /// Rotation angle in degrees (must be 90, 180, or 270)
        #[arg(short, long, default_value = "90")]
        angle: i32,

        /// Pages to rotate: "all", single page "1", range "2-6", or list "1,3,5"
        #[arg(short = 'p', long, default_value = "all")]
        pages: String,
    },

    /// Get information about a PDF file
    ///
    /// Displays metadata and structural information about a PDF document.
    /// Basic mode shows version, page count, and metadata.
    /// Detailed mode adds page dimensions, catalog info, and more.
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Basic information
    /// oxidizepdf info document.pdf
    ///
    /// # Detailed information including page dimensions
    /// oxidizepdf info document.pdf --detailed
    /// ```
    Info {
        /// Path to the PDF file to analyze
        input: PathBuf,

        /// Show detailed information including page dimensions and catalog entries
        #[arg(short, long)]
        detailed: bool,
    },

    /// Extract text from a PDF file
    ///
    /// Extracts text content from PDF documents with support for:
    /// - Single page or full document extraction
    /// - Layout preservation (maintains positioning)
    /// - Output to file or stdout
    ///
    /// # Examples
    ///
    /// ```bash
    /// # Extract all text to stdout
    /// oxidizepdf extract-text document.pdf
    ///
    /// # Extract to a file
    /// oxidizepdf extract-text document.pdf -o extracted.txt
    ///
    /// # Extract specific page (0-based)
    /// oxidizepdf extract-text document.pdf -p 0
    ///
    /// # Extract with layout preservation
    /// oxidizepdf extract-text document.pdf --preserve-layout
    /// ```
    ExtractText {
        /// Path to the PDF file to extract text from
        input: PathBuf,

        /// Output text file (defaults to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Page number to extract (0-based index, extracts all pages if not specified)
        #[arg(short = 'p', long)]
        page: Option<u32>,

        /// Preserve the original layout and positioning of text
        #[arg(short = 'l', long)]
        preserve_layout: bool,
    },
}

/// Main entry point for the oxidize-pdf CLI application.
///
/// Parses command-line arguments and dispatches to the appropriate command handler.
/// All commands return a Result, with errors being displayed to stderr before
/// exiting with code 1.
///
/// # Exit Codes
///
/// - 0: Success
/// - 1: Error occurred (details printed to stderr)
///
/// # Error Handling
///
/// The CLI provides user-friendly error messages for common issues:
/// - File not found
/// - Invalid PDF format
/// - Unsupported operations
/// - Invalid arguments
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Create { output, text } => {
            let mut doc = Document::new();
            let mut page = Page::a4();

            page.text()
                .set_font(Font::Helvetica, 24.0)
                .at(50.0, 750.0)
                .write(&text)?;

            doc.add_page(page);
            doc.save(output)?;

            println!("PDF created successfully!");
        }

        Commands::Demo { output } => {
            let mut doc = Document::new();
            let mut page = Page::a4();

            // Add graphics
            page.graphics()
                .set_stroke_color(Color::red())
                .set_line_width(2.0)
                .rect(50.0, 50.0, 200.0, 100.0)
                .stroke()
                .set_fill_color(Color::blue())
                .circle(300.0, 400.0, 50.0)
                .fill();

            // Add text
            page.text()
                .set_font(Font::HelveticaBold, 36.0)
                .at(100.0, 700.0)
                .write("oxidizePdf Demo")?
                .set_font(Font::Helvetica, 16.0)
                .at(100.0, 650.0)
                .write("Native Rust PDF Generation")?;

            doc.add_page(page);
            doc.set_title("oxidizePdf Demo");
            doc.set_author("oxidizePdf CLI");
            doc.save(output)?;

            println!("Demo PDF created successfully!");
        }

        Commands::Merge { .. } => {
            eprintln!("PDF merge functionality coming in Q2 2025");
            eprintln!("This will require implementing the native PDF parser first");
        }

        Commands::Split { .. } => {
            eprintln!("PDF split functionality coming in Q2 2025");
            eprintln!("This will require implementing the native PDF parser first");
        }

        Commands::Info { input, detailed } => {
            eprintln!("Opening PDF file: {}", input.display());
            match PdfReader::open(&input) {
                Ok(mut reader) => {
                    println!("PDF Information for: {}", input.display());
                    println!("==========================================");

                    // Basic info
                    let version = reader.version();
                    println!("PDF Version: {version}");

                    // Try to get metadata
                    match reader.metadata() {
                        Ok(metadata) => {
                            if let Some(title) = &metadata.title {
                                println!("Title: {title}");
                            }
                            if let Some(author) = &metadata.author {
                                println!("Author: {author}");
                            }
                            if let Some(subject) = &metadata.subject {
                                println!("Subject: {subject}");
                            }
                            if let Some(creator) = &metadata.creator {
                                println!("Creator: {creator}");
                            }
                            if let Some(producer) = &metadata.producer {
                                println!("Producer: {producer}");
                            }
                            if let Some(page_count) = metadata.page_count {
                                println!("Pages: {page_count}");
                            }
                        }
                        Err(e) => {
                            eprintln!("Warning: Could not read metadata: {e}");
                        }
                    }

                    if detailed {
                        println!("\nDetailed Information:");
                        println!("--------------------");

                        // Try to get catalog info
                        match reader.catalog() {
                            Ok(catalog) => {
                                if let Some(catalog_type) = catalog.get_type() {
                                    println!("Catalog Type: {catalog_type}");
                                }

                                // Check for common catalog entries
                                if catalog.contains_key("ViewerPreferences") {
                                    println!("Has Viewer Preferences: Yes");
                                }
                                if catalog.contains_key("Names") {
                                    println!("Has Names Dictionary: Yes");
                                }
                                if catalog.contains_key("Outlines") {
                                    println!("Has Outlines (Bookmarks): Yes");
                                }
                            }
                            Err(e) => {
                                eprintln!("Warning: Could not read catalog: {e}");
                            }
                        }

                        // Show page information
                        match reader.page_count() {
                            Ok(count) if count > 0 => {
                                println!("\nPage Information:");
                                println!("-----------------");

                                // Show first few pages
                                let pages_to_show = std::cmp::min(3, count);
                                for i in 0..pages_to_show {
                                    match reader.get_page(i) {
                                        Ok(page) => {
                                            println!(
                                                "Page {}: {:.0}x{:.0} pts",
                                                i + 1,
                                                page.width(),
                                                page.height()
                                            );
                                        }
                                        Err(_) => {
                                            println!("Page {}: [Could not read]", i + 1);
                                        }
                                    }
                                }

                                if count > pages_to_show {
                                    println!("... and {} more pages", count - pages_to_show);
                                }
                            }
                            _ => {}
                        }
                    }

                    println!("\n✓ PDF parsed successfully!");
                }
                Err(e) => {
                    eprintln!("Error: Failed to parse PDF: {e}");
                    eprintln!("\nNote: The PDF parser is currently in early development.");
                    eprintln!("Some PDF features may not be supported yet.");
                    std::process::exit(1);
                }
            }
        }

        Commands::Rotate {
            input,
            output,
            angle,
            pages,
        } => {
            // Parse rotation angle
            let rotation = RotationAngle::from_degrees(angle).unwrap_or_else(|e| {
                eprintln!("Error: {e}. Valid angles are 90, 180, 270");
                std::process::exit(1);
            });

            // Parse page range
            let page_range = PageRange::parse(&pages).unwrap_or_else(|e| {
                eprintln!("Error parsing page range '{pages}': {e}");
                std::process::exit(1);
            });

            let options = RotateOptions {
                pages: page_range,
                angle: rotation,
                preserve_page_size: false,
            };

            match rotate_pdf_pages(&input, &output, options) {
                Ok(_) => {
                    println!(
                        "✓ Successfully rotated pages {} degrees in {}",
                        angle,
                        output.display()
                    );
                }
                Err(e) => {
                    eprintln!("Error rotating PDF: {e}");
                    std::process::exit(1);
                }
            }
        }

        Commands::ExtractText {
            input,
            output,
            page,
            preserve_layout,
        } => {
            let document = PdfReader::open_document(&input)
                .map_err(|e| anyhow::anyhow!("Failed to open PDF: {}", e))?;

            let options = ExtractionOptions {
                preserve_layout,
                ..Default::default()
            };

            let extractor = TextExtractor::with_options(options);

            // Extract text from specific page or all pages
            let extracted_text = if let Some(page_num) = page {
                vec![extractor
                    .extract_from_page(&document, page_num)
                    .map_err(|e| {
                        anyhow::anyhow!("Failed to extract text from page {}: {}", page_num, e)
                    })?]
            } else {
                extractor
                    .extract_from_document(&document)
                    .map_err(|e| anyhow::anyhow!("Failed to extract text: {}", e))?
            };

            // Combine all extracted text
            let full_text = extracted_text
                .iter()
                .map(|et| et.text.as_str())
                .collect::<Vec<_>>()
                .join("\n\n");

            // Write to output file or stdout
            if let Some(output_path) = output {
                std::fs::write(&output_path, &full_text)
                    .map_err(|e| anyhow::anyhow!("Failed to write output file: {}", e))?;
                println!("✓ Text extracted to: {}", output_path.display());
            } else {
                println!("{full_text}");
            }
        }
    }

    Ok(())
}
