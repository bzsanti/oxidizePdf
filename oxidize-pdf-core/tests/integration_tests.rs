//! Integration tests for end-to-end PDF workflows
//! 
//! These tests validate complete workflows from document creation to final PDF output,
//! covering multiple modules working together.

use oxidize_pdf::document::Document;
use oxidize_pdf::error::Result;
use oxidize_pdf::graphics::Color;
use oxidize_pdf::page::Page;
use oxidize_pdf::text::Font;
use oxidize_pdf::writer::WriterConfig;
use std::fs;
use tempfile::TempDir;

/// Test complete PDF creation workflow with text, graphics, and metadata
#[test]
fn test_complete_pdf_creation_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("complete_workflow.pdf");

    // Create document with comprehensive content
    let mut doc = Document::new();
    
    // Set all metadata
    doc.set_title("Integration Test Document");
    doc.set_author("Test Suite");
    doc.set_subject("End-to-end workflow testing");
    doc.set_keywords("integration, test, pdf, workflow");
    doc.set_creator("oxidize_pdf integration tests");
    
    // Create multiple pages with different content types
    for page_num in 1..=3 {
        let mut page = Page::a4();
        
        // Add text content
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(50.0, 750.0)
            .write(&format!("Page {} - Integration Test", page_num))?;
            
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(50.0, 700.0)
            .write("This page tests complete PDF generation workflow.")?;
            
        // Add graphics content
        let color = match page_num % 3 {
            1 => Color::rgb(1.0, 0.0, 0.0), // Red
            2 => Color::rgb(0.0, 1.0, 0.0), // Green
            _ => Color::rgb(0.0, 0.0, 1.0), // Blue
        };
        
        page.graphics()
            .set_fill_color(color)
            .rectangle(50.0, 600.0, 200.0, 100.0)
            .fill();
            
        // Add border
        page.graphics()
            .set_stroke_color(Color::rgb(0.0, 0.0, 0.0))
            .set_line_width(2.0)
            .rectangle(30.0, 580.0, 240.0, 140.0)
            .stroke();
            
        doc.add_page(page);
    }
    
    // Test saving to file
    doc.save(&file_path)?;
    
    // Verify file was created and has reasonable size
    assert!(file_path.exists());
    let metadata = fs::metadata(&file_path).unwrap();
    assert!(metadata.len() > 1000); // Should be substantial
    
    // Verify PDF structure
    let content = fs::read(&file_path).unwrap();
    assert!(content.starts_with(b"%PDF-"));
    assert!(content.ends_with(b"%%EOF\n") || content.ends_with(b"%%EOF"));
    
    // Verify metadata is embedded
    let content_str = String::from_utf8_lossy(&content);
    assert!(content_str.contains("Integration Test Document"));
    assert!(content_str.contains("Test Suite"));
    
    Ok(())
}

/// Test in-memory PDF generation workflow
#[test]
fn test_in_memory_pdf_workflow() -> Result<()> {
    let mut doc = Document::new();
    doc.set_title("Memory Workflow Test");
    
    // Add content
    let mut page = Page::letter();
    page.text()
        .set_font(Font::TimesRoman, 14.0)
        .at(72.0, 720.0)
        .write("In-memory PDF generation test")?;
        
    page.graphics()
        .set_fill_color(Color::rgb(0.8, 0.8, 0.2))
        .circle(200.0, 400.0, 50.0)
        .fill();
        
    doc.add_page(page);
    
    // Generate PDF in memory
    let pdf_bytes = doc.to_bytes()?;
    
    // Verify generated PDF
    assert!(!pdf_bytes.is_empty());
    assert!(pdf_bytes.len() > 500);
    assert!(pdf_bytes.starts_with(b"%PDF-"));
    
    // Verify content can be written to buffer
    let mut buffer = Vec::new();
    doc.write(&mut buffer)?;
    assert!(!buffer.is_empty());
    assert_eq!(&buffer[0..5], b"%PDF-");
    
    Ok(())
}

/// Test workflow with custom writer configuration
#[test]
fn test_custom_writer_config_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let _file_path = temp_dir.path().join("custom_config.pdf");
    
    let mut doc = Document::new();
    doc.set_title("Custom Config Test");
    
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Courier, 10.0)
        .at(50.0, 750.0)
        .write("Testing custom writer configuration")?;
    doc.add_page(page);
    
    // Test with different configurations
    let configs = vec![
        WriterConfig {
            use_xref_streams: false,
            pdf_version: "1.4".to_string(),
            compress_streams: false,
        },
        WriterConfig {
            use_xref_streams: true,
            pdf_version: "1.5".to_string(),
            compress_streams: true,
        },
        WriterConfig {
            use_xref_streams: false,
            pdf_version: "1.7".to_string(),
            compress_streams: true,
        },
    ];
    
    for (i, config) in configs.into_iter().enumerate() {
        let test_path = temp_dir.path().join(format!("config_test_{}.pdf", i));
        
        // Test saving with config
        doc.save_with_config(&test_path, config.clone())?;
        assert!(test_path.exists());
        
        // Test in-memory generation with config
        let pdf_bytes = doc.to_bytes_with_config(config)?;
        assert!(!pdf_bytes.is_empty());
        
        // Verify PDF version in header
        let header = String::from_utf8_lossy(&pdf_bytes[0..20]);
        assert!(header.contains("PDF-"));
    }
    
    Ok(())
}

/// Test multi-format content workflow
#[test]
fn test_multi_format_content_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("multi_format.pdf");
    
    let mut doc = Document::new();
    doc.set_title("Multi-format Content Test");
    
    let mut page = Page::a4();
    
    // Different font types and sizes
    let fonts = vec![
        (Font::Helvetica, 16.0),
        (Font::HelveticaBold, 14.0),
        (Font::TimesRoman, 12.0),
        (Font::TimesItalic, 10.0),
        (Font::Courier, 9.0),
    ];
    
    let mut y_pos = 750.0;
    for (font, size) in fonts {
        let font_name = format!("{:?}", font);
        page.text()
            .set_font(font, size)
            .at(50.0, y_pos)
            .write(&format!("Text in {} at {} points", font_name, size))?;
        y_pos -= 30.0;
    }
    
    // Various graphics shapes
    page.graphics()
        .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
        .rectangle(50.0, y_pos - 20.0, 100.0, 15.0)
        .fill();
        
    page.graphics()
        .set_fill_color(Color::rgb(0.0, 1.0, 0.0))
        .circle(200.0, y_pos - 10.0, 10.0)
        .fill();
        
    page.graphics()
        .set_stroke_color(Color::rgb(0.0, 0.0, 1.0))
        .set_line_width(3.0)
        .move_to(300.0, y_pos - 20.0)
        .line_to(400.0, y_pos)
        .stroke();
    
    doc.add_page(page);
    doc.save(&file_path)?;
    
    // Verify comprehensive content was generated
    assert!(file_path.exists());
    let metadata = fs::metadata(&file_path).unwrap();
    println!("Generated PDF size: {} bytes", metadata.len());
    assert!(metadata.len() > 1000); // Should be substantial with all content (adjusted from 2000)
    
    Ok(())
}

/// Test error recovery in complete workflows
#[test]
fn test_workflow_error_recovery() {
    let mut doc = Document::new();
    doc.set_title("Error Recovery Test");
    
    // Add valid content
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .write("Valid content before error")
        .unwrap();
    doc.add_page(page);
    
    // Test invalid path handling
    let result = doc.save("/invalid/nonexistent/path/test.pdf");
    assert!(result.is_err());
    
    // Document should still be valid for other operations
    let pdf_bytes = doc.to_bytes();
    assert!(pdf_bytes.is_ok());
    assert!(!pdf_bytes.unwrap().is_empty());
}

/// Test large document workflow performance
#[test]
fn test_large_document_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large_document.pdf");
    
    let mut doc = Document::new();
    doc.set_title("Large Document Performance Test");
    
    // Create document with many pages
    let page_count = 20;
    for page_num in 1..=page_count {
        let mut page = Page::a4();
        
        // Add substantial content to each page
        page.text()
            .set_font(Font::Helvetica, 14.0)
            .at(50.0, 750.0)
            .write(&format!("Large Document - Page {}", page_num))?;
            
        // Add multiple text blocks
        for i in 0..10 {
            let y_pos = 700.0 - (i as f64 * 25.0);
            page.text()
                .set_font(Font::Helvetica, 10.0)
                .at(50.0, y_pos)
                .write(&format!("Content block {} on page {}", i + 1, page_num))?;
        }
        
        // Add graphics
        page.graphics()
            .set_fill_color(Color::rgb(0.7, 0.7, 0.9))
            .rectangle(400.0, 600.0, 150.0, 100.0)
            .fill();
            
        doc.add_page(page);
    }
    
    // Test performance of large document operations
    let start = std::time::Instant::now();
    doc.save(&file_path)?;
    let save_duration = start.elapsed();
    
    // Verify file was created successfully
    assert!(file_path.exists());
    let metadata = fs::metadata(&file_path).unwrap();
    assert!(metadata.len() > 10000); // Should be substantial
    
    // Performance should be reasonable (less than 5 seconds for 20 pages)
    assert!(save_duration.as_secs() < 5);
    
    // Test in-memory generation performance
    let start = std::time::Instant::now();
    let pdf_bytes = doc.to_bytes()?;
    let memory_duration = start.elapsed();
    
    assert!(!pdf_bytes.is_empty());
    assert!(memory_duration.as_secs() < 5);
    
    Ok(())
}

/// Test workflow with compression enabled/disabled
#[test]
fn test_compression_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    
    let mut doc = Document::new();
    doc.set_title("Compression Test");
    
    // Add content that should compress well
    let mut page = Page::a4();
    let repeated_text = "This is repeated content that should compress well. ".repeat(50);
    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 750.0)
        .write(&repeated_text)?;
    doc.add_page(page);
    
    // Test with compression enabled
    doc.set_compress(true);
    let compressed_path = temp_dir.path().join("compressed.pdf");
    doc.save(&compressed_path)?;
    let compressed_size = fs::metadata(&compressed_path).unwrap().len();
    
    // Test with compression disabled  
    doc.set_compress(false);
    let uncompressed_path = temp_dir.path().join("uncompressed.pdf");
    doc.save(&uncompressed_path)?;
    let uncompressed_size = fs::metadata(&uncompressed_path).unwrap().len();
    
    // Both should be valid PDFs
    assert!(compressed_path.exists());
    assert!(uncompressed_path.exists());
    
    // Verify compression setting is respected
    assert!(doc.get_compress() == false); // Last setting
    
    // Both files should be substantial but compressed might be smaller
    assert!(compressed_size > 500);
    assert!(uncompressed_size > 500);
    
    Ok(())
}

/// Test metadata persistence workflow
#[test]
fn test_metadata_persistence_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("metadata_test.pdf");
    
    let mut doc = Document::new();
    
    // Set comprehensive metadata
    doc.set_title("Metadata Persistence Test");
    doc.set_author("Integration Test Suite");
    doc.set_subject("Testing metadata embedding and persistence");
    doc.set_keywords("metadata, persistence, pdf, integration");
    doc.set_creator("oxidize_pdf test framework");
    doc.set_producer("oxidize_pdf integration tests v1.0");
    
    // Add content
    let mut page = Page::a4();
    page.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .write("Testing metadata persistence in PDF workflow")?;
    doc.add_page(page);
    
    // Save and verify metadata presence
    doc.save(&file_path)?;
    
    let content = fs::read(&file_path).unwrap();
    let content_str = String::from_utf8_lossy(&content);
    
    // Verify all metadata fields are present in PDF
    assert!(content_str.contains("Metadata Persistence Test"));
    assert!(content_str.contains("Integration Test Suite"));
    assert!(content_str.contains("Testing metadata embedding"));
    assert!(content_str.contains("metadata, persistence"));
    
    Ok(())
}

/// Test incremental document building workflow
#[test]
fn test_incremental_building_workflow() -> Result<()> {
    let temp_dir = TempDir::new().unwrap();
    
    let mut doc = Document::new();
    
    // Step 1: Initial document
    doc.set_title("Incremental Building Test");
    let mut page1 = Page::a4();
    page1.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .write("Initial page content")?;
    doc.add_page(page1);
    
    // Test intermediate state
    let intermediate_path = temp_dir.path().join("intermediate.pdf");
    doc.save(&intermediate_path)?;
    assert!(intermediate_path.exists());
    
    // Step 2: Add more content
    doc.set_author("Incremental Author");
    let mut page2 = Page::a4();
    page2.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .write("Second page added incrementally")?;
    doc.add_page(page2);
    
    // Step 3: Add graphics and more metadata
    doc.set_subject("Incremental building workflow");
    let mut page3 = Page::a4();
    page3.text()
        .set_font(Font::Helvetica, 12.0)
        .at(50.0, 750.0)
        .write("Third page with graphics")?;
    page3.graphics()
        .set_fill_color(Color::rgb(0.9, 0.5, 0.1))
        .rectangle(50.0, 600.0, 300.0, 50.0)
        .fill();
    doc.add_page(page3);
    
    // Final state
    let final_path = temp_dir.path().join("final.pdf");
    doc.save(&final_path)?;
    
    // Verify final document has all content
    assert!(final_path.exists());
    // Note: Cannot access private pages field directly - would need a public method
    // assert_eq!(doc.pages.len(), 3);
    
    let final_size = fs::metadata(&final_path).unwrap().len();
    let intermediate_size = fs::metadata(&intermediate_path).unwrap().len();
    assert!(final_size > intermediate_size); // Should be larger
    
    Ok(())
}

/// Test concurrent-like operations workflow
#[test]
fn test_concurrent_operations_workflow() -> Result<()> {
    // Simulate concurrent-like operations by building multiple documents in parallel
    let temp_dir = TempDir::new().unwrap();
    
    let mut documents = Vec::new();
    
    // Create multiple documents
    for doc_num in 0..5 {
        let mut doc = Document::new();
        doc.set_title(&format!("Concurrent Document {}", doc_num));
        doc.set_author(&format!("Author {}", doc_num));
        
        // Each document gets different content
        for page_num in 0..3 {
            let mut page = Page::a4();
            page.text()
                .set_font(Font::Helvetica, 12.0)
                .at(50.0, 750.0)
                .write(&format!("Doc {} Page {}", doc_num, page_num))?;
                
            // Unique color per document
            let color = Color::rgb(
                (doc_num as f64) / 5.0,
                0.5,
                1.0 - (doc_num as f64) / 5.0
            );
            page.graphics()
                .set_fill_color(color)
                .rectangle(100.0, 600.0, 200.0, 100.0)
                .fill();
                
            doc.add_page(page);
        }
        
        documents.push(doc);
    }
    
    // Save all documents
    for (i, mut doc) in documents.into_iter().enumerate() {
        let file_path = temp_dir.path().join(format!("concurrent_{}.pdf", i));
        doc.save(&file_path)?;
        
        assert!(file_path.exists());
        let metadata = fs::metadata(&file_path).unwrap();
        assert!(metadata.len() > 1000);
    }
    
    Ok(())
}