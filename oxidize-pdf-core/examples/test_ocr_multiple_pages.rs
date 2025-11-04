use oxidize_pdf::operations::page_analysis::PageContentAnalyzer;
use oxidize_pdf::parser::document::PdfDocument;
use oxidize_pdf::parser::pdf_reader::PdfReader;
use oxidize_pdf::text::ocr::OcrProvider;
use std::fs::File;
use std::path::Path;

struct MockOcrProvider;

impl OcrProvider for MockOcrProvider {
    fn extract_text(&self, _image_data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        // Mock OCR that just returns a placeholder
        Ok("Sample OCR text".to_string())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing OCR extraction from multiple pages...");

    let test_pdf_path = "tests/fixtures/scanned_document.pdf";
    if !Path::new(test_pdf_path).exists() {
        println!("❌ Test document not found at: {}", test_pdf_path);
        return Ok(());
    }

    let file = File::open(test_pdf_path)?;
    let reader = PdfReader::new(file)?;
    let document = PdfDocument::new(reader);
    let analyzer = PageContentAnalyzer::new(document);

    println!(
        "✅ PDF opened successfully. Pages: {}",
        analyzer.document.get_page_count()?
    );

    // Test specific pages
    let test_pages = [0, 30, 65];
    let mock_ocr = MockOcrProvider;

    for &page_num in &test_pages {
        if page_num >= analyzer.document.get_page_count()? as usize {
            continue;
        }

        println!("\n📄 Testing OCR on page {}...", page_num);

        match analyzer.extract_text_from_scanned_page(page_num, &mock_ocr) {
            Ok(result) => {
                println!("   ✅ Page {} OCR successful", page_num);
                println!("   📊 Confidence: {:.2}%", result.confidence);
                println!("   📝 Text length: {} characters", result.text.len());
            }
            Err(e) => {
                println!("   ❌ Page {} OCR failed: {}", page_num, e);
            }
        }
    }

    println!("\n✅ OCR test complete! Check debug output above for different Object numbers.");

    Ok(())
}
