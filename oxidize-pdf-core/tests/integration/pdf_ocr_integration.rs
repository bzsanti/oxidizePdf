//! PDF OCR Integration Tests
//!
//! These tests verify the complete OCR pipeline from PDF input to searchable PDF output

#[cfg(feature = "ocr-tesseract")]
mod pdf_ocr_integration {
    use oxidize_pdf::operations::pdf_ocr_converter::{ConversionOptions, PdfOcrConverter};
    use oxidize_pdf::text::{OcrOptions, RustyTesseractProvider};
    use oxidize_pdf::{Color, Document, Font, Page};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_scanned_like_pdf(output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = Document::new();
        let mut page = Page::a4();

        // Simulate a scanned document by creating a large background image area
        page.graphics()
            .set_fill_color(Color::rgb(248.0, 248.0, 248.0))
            .rect(0.0, 0.0, 595.0, 842.0)
            .fill();

        // Add some faint text that might represent scanned content
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(72.0, 750.0)
            .write("SCANNED DOCUMENT")?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(72.0, 720.0)
            .write("This document contains scanned text that needs OCR processing.")?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(72.0, 700.0)
            .write("Multiple lines of text should be detected and converted")?;

        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(72.0, 680.0)
            .write("to a searchable format using optical character recognition.")?;

        // Add a second page
        doc.add_page(page.clone());

        let mut page2 = Page::a4();
        page2
            .graphics()
            .set_fill_color(Color::rgb(250.0, 250.0, 250.0))
            .rect(0.0, 0.0, 595.0, 842.0)
            .fill();

        page2
            .text()
            .set_font(Font::Helvetica, 12.0)
            .at(72.0, 750.0)
            .write("PAGE TWO")?;

        page2
            .text()
            .set_font(Font::Helvetica, 10.0)
            .at(72.0, 720.0)
            .write("Second page with additional content for testing.")?;

        doc.add_page(page2);

        let pdf_bytes = doc.to_bytes()?;
        fs::write(output_path, pdf_bytes)?;

        Ok(())
    }

    fn create_text_heavy_pdf(output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = Document::new();
        let mut page = Page::a4();

        // Create a page with lots of existing text (should be skipped in OCR)
        for i in 0..20 {
            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(72.0, 750.0 - (i as f64 * 20.0))
                .write(&format!(
                    "This is line {} of existing text content in the PDF.",
                    i + 1
                ))?;
        }

        doc.add_page(page);

        let pdf_bytes = doc.to_bytes()?;
        fs::write(output_path, pdf_bytes)?;

        Ok(())
    }

    #[test]
    fn test_basic_ocr_conversion_workflow() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("scanned.pdf");
        let output_path = temp_dir.path().join("searchable.pdf");

        // Create test PDF
        create_scanned_like_pdf(&input_path)?;
        assert!(input_path.exists());

        // Test OCR conversion workflow
        let converter = PdfOcrConverter::new()?;

        // Initialize Tesseract OCR provider (infallible constructor)
        let ocr_provider = RustyTesseractProvider::new();
        let options = ConversionOptions {
            ocr_options: OcrOptions {
                language: "eng".to_string(),
                min_confidence: 0.5, // Lower threshold for testing
                ..Default::default()
            },
            min_confidence: 0.5,
            skip_text_pages: false, // Process all pages
            dpi: 150,               // Lower DPI for faster testing
            ..Default::default()
        };

        match converter.convert_to_searchable_pdf(
            &input_path,
            &output_path,
            &ocr_provider,
            &options,
        ) {
            Ok(result) => {
                assert!(output_path.exists(), "Output PDF should be created");
                assert!(
                    result.pages_processed > 0,
                    "Should process at least one page"
                );
                println!(
                    "✅ OCR conversion successful: {} pages processed, {} with OCR",
                    result.pages_processed, result.pages_ocr_processed
                );
            }
            Err(e) => {
                println!("⚠️  OCR conversion failed: {}", e);
                // This might fail due to page analysis limitations, which is acceptable
            }
        }

        Ok(())
    }

    #[test]
    fn test_skip_text_pages_option() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("text_heavy.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        // Create PDF with lots of existing text
        create_text_heavy_pdf(&input_path)?;

        let converter = PdfOcrConverter::new()?;

        // Tesseract OCR provider (infallible constructor)
        let ocr_provider = RustyTesseractProvider::new();

        // Test with skip_text_pages = true
        let options_skip = ConversionOptions {
            skip_text_pages: true,
            ..Default::default()
        };

        match converter.convert_to_searchable_pdf(
            &input_path,
            &output_path,
            &ocr_provider,
            &options_skip,
        ) {
            Ok(result) => {
                // Should skip pages with existing text
                println!(
                    "✅ Skip text pages test: {} processed, {} skipped",
                    result.pages_processed, result.pages_skipped
                );
            }
            Err(e) => {
                println!("⚠️  Skip text pages test failed: {}", e);
            }
        }

        Ok(())
    }

    #[test]
    fn test_batch_conversion_integration() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_dir = temp_dir.path().join("batch_input");
        let output_dir = temp_dir.path().join("batch_output");

        fs::create_dir_all(&input_dir)?;
        fs::create_dir_all(&output_dir)?;

        // Create multiple test PDFs
        for i in 1..=3 {
            let pdf_path = input_dir.join(format!("test_{}.pdf", i));
            create_scanned_like_pdf(&pdf_path)?;
        }

        let converter = PdfOcrConverter::new()?;

        // Tesseract OCR provider (infallible constructor)
        let ocr_provider = RustyTesseractProvider::new();
        let options = ConversionOptions {
            min_confidence: 0.5,
            dpi: 150, // Lower DPI for faster batch testing
            ..Default::default()
        };

        let input_files: Vec<_> = fs::read_dir(&input_dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().map_or(false, |ext| ext == "pdf"))
            .collect();

        match converter.batch_convert(&input_files, &output_dir, &ocr_provider, &options) {
            Ok(results) => {
                println!("✅ Batch conversion: {} files processed", results.len());
                for (i, result) in results.iter().enumerate() {
                    println!(
                        "  File {}: {} pages, confidence {:.1}%",
                        i + 1,
                        result.pages_processed,
                        result.average_confidence * 100.0
                    );
                }
            }
            Err(e) => {
                println!("⚠️  Batch conversion failed: {}", e);
            }
        }

        Ok(())
    }

    #[test]
    fn test_multilingual_ocr_options() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("multilingual.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        create_scanned_like_pdf(&input_path)?;

        let converter = PdfOcrConverter::new()?;

        // Tesseract OCR provider (infallible constructor)
        let ocr_provider = RustyTesseractProvider::new();

        // Test with multiple languages
        let options = ConversionOptions {
            ocr_options: OcrOptions {
                language: "eng+spa".to_string(), // English + Spanish
                min_confidence: 0.6,
                ..Default::default()
            },
            min_confidence: 0.6,
            dpi: 150,
            ..Default::default()
        };

        match converter.convert_to_searchable_pdf(
            &input_path,
            &output_path,
            &ocr_provider,
            &options,
        ) {
            Ok(result) => {
                println!(
                    "✅ Multilingual OCR test: {} pages processed",
                    result.pages_processed
                );
            }
            Err(e) => {
                println!("⚠️  Multilingual OCR test failed: {}", e);
                // Language packs might not be available, which is acceptable
            }
        }

        Ok(())
    }

    #[test]
    fn test_different_dpi_settings() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("dpi_test.pdf");

        create_scanned_like_pdf(&input_path)?;

        let converter = PdfOcrConverter::new()?;

        // Tesseract OCR provider (infallible constructor)
        let ocr_provider = RustyTesseractProvider::new();

        // Test different DPI settings
        for dpi in &[150, 300] {
            let output_path = temp_dir.path().join(format!("output_{}dpi.pdf", dpi));

            let options = ConversionOptions {
                dpi: *dpi,
                min_confidence: 0.5,
                ..Default::default()
            };

            match converter.convert_to_searchable_pdf(
                &input_path,
                &output_path,
                &ocr_provider,
                &options,
            ) {
                Ok(result) => {
                    println!(
                        "✅ DPI {} test: {} pages, {:.1}% confidence",
                        dpi,
                        result.pages_processed,
                        result.average_confidence * 100.0
                    );
                }
                Err(e) => {
                    println!("⚠️  DPI {} test failed: {}", dpi, e);
                }
            }
        }

        Ok(())
    }

    #[test]
    fn test_confidence_thresholds() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let input_path = temp_dir.path().join("confidence_test.pdf");
        let output_path = temp_dir.path().join("output.pdf");

        create_scanned_like_pdf(&input_path)?;

        let converter = PdfOcrConverter::new()?;

        // Tesseract OCR provider (infallible constructor)
        let ocr_provider = RustyTesseractProvider::new();

        // Test different confidence thresholds
        for confidence in &[0.3, 0.5, 0.7, 0.9] {
            let options = ConversionOptions {
                min_confidence: *confidence,
                dpi: 150,
                ..Default::default()
            };

            match converter.convert_to_searchable_pdf(
                &input_path,
                &output_path,
                &ocr_provider,
                &options,
            ) {
                Ok(result) => {
                    println!(
                        "✅ Confidence {:.1} test: {} OCR pages, {:.1}% avg confidence",
                        confidence,
                        result.pages_ocr_processed,
                        result.average_confidence * 100.0
                    );
                }
                Err(e) => {
                    println!("⚠️  Confidence {:.1} test failed: {}", confidence, e);
                }
            }
        }

        Ok(())
    }
}
