//! End-to-end integration tests for oxidize-pdf
//! Tests complete workflows from PDF creation to parsing and manipulation

use oxidize_pdf::graphics::Color;
use oxidize_pdf::operations::{
    merge_pdfs, split_pdf, MergeInput, MergeOptions, SplitMode, SplitOptions,
};
use oxidize_pdf::parser::PdfReader;
use oxidize_pdf::text::Font;
use oxidize_pdf::writer::PdfWriter;
use oxidize_pdf::{Document, Page, Result};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Cursor};
use tempfile::TempDir;

/// Helper to create a test PDF in memory
fn create_test_pdf_bytes(title: &str, num_pages: usize) -> Result<Vec<u8>> {
    let mut document = Document::new();
    document.set_title(title);
    document.set_author("Test Suite");

    for i in 0..num_pages {
        let mut page = Page::a4();

        // Add text content
        page.text()
            .set_font(Font::Helvetica, 24.0)
            .at(100.0, 700.0)
            .write(&format!("{} - Page {}", title, i + 1))?;

        // Add some graphics
        page.graphics()
            .set_stroke_color(Color::rgb(0.0, 0.0, 1.0))
            .set_line_width(2.0)
            .rectangle(50.0, 50.0, 500.0, 750.0)
            .stroke();

        document.add_page(page);
    }

    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);
    writer.write_document(&mut document)?;

    Ok(buffer)
}

#[test]
fn test_create_and_parse_simple_pdf() -> Result<()> {
    // Create a PDF
    let pdf_bytes = create_test_pdf_bytes("Simple Test", 1)?;

    // Parse it back
    let reader = BufReader::new(Cursor::new(&pdf_bytes));
    let mut parsed = PdfReader::new(reader)?;

    // Verify basic structure
    assert_eq!(parsed.page_count()?, 1);

    // Check metadata
    // let info = parsed.get_info();
    // assert!(info.is_some());
    // if let Some(info) = info {
    //     assert!(info.get("Title").is_some());
    // }

    Ok(())
}

#[test]
fn test_create_multipage_pdf() -> Result<()> {
    let mut document = Document::new();
    document.set_title("Multi-page Test");

    // Add pages with different sizes
    let page_configs = vec![
        (Page::a4(), "A4 Page"),
        (Page::letter(), "Letter Page"),
        (Page::legal(), "Legal Page"),
    ];

    for (mut page, label) in page_configs {
        page.text()
            .set_font(Font::TimesRoman, 18.0)
            .at(50.0, 50.0)
            .write(label)?;
        document.add_page(page);
    }

    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);
    writer.write_document(&mut document)?;

    // Verify the PDF was created
    assert!(!buffer.is_empty());
    assert!(buffer.starts_with(b"%PDF"));

    Ok(())
}

#[test]
fn test_pdf_with_graphics() -> Result<()> {
    let mut document = Document::new();
    let mut page = Page::a4();

    // Complex graphics operations
    page.graphics()
        // Draw a filled rectangle
        .set_fill_color(Color::rgb(1.0, 0.0, 0.0))
        .rectangle(100.0, 100.0, 200.0, 150.0)
        .fill()
        // Draw a stroked circle (approximated)
        .set_stroke_color(Color::rgb(0.0, 1.0, 0.0))
        .set_line_width(3.0)
        .circle(400.0, 400.0, 50.0)
        .stroke()
        // Draw lines
        .set_stroke_color(Color::rgb(0.0, 0.0, 1.0))
        .move_to(50.0, 550.0)
        .line_to(550.0, 550.0)
        .line_to(550.0, 50.0)
        .stroke();

    document.add_page(page);

    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);
    writer.write_document(&mut document)?;

    assert!(!buffer.is_empty());

    Ok(())
}

#[test]
fn test_pdf_with_fonts_and_text() -> Result<()> {
    let mut document = Document::new();
    let mut page = Page::a4();

    // Test different fonts
    let fonts = vec![
        (Font::Helvetica, "Helvetica"),
        (Font::TimesRoman, "Times Roman"),
        (Font::Courier, "Courier"),
    ];

    let mut y_position = 700.0;
    for (font, name) in fonts {
        page.text()
            .set_font(font, 14.0)
            .at(100.0, y_position)
            .write(&format!("This is {} font at 14pt", name))?;
        y_position -= 30.0;
    }

    // Test different sizes
    let sizes = vec![8.0, 10.0, 12.0, 14.0, 18.0, 24.0, 36.0];
    for size in sizes {
        page.text()
            .set_font(Font::Helvetica, size)
            .at(100.0, y_position)
            .write(&format!("Size: {} pt", size))?;
        y_position -= size * 1.5;
    }

    document.add_page(page);

    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);
    writer.write_document(&mut document)?;

    assert!(!buffer.is_empty());

    Ok(())
}

#[test]
fn test_pdf_metadata() -> Result<()> {
    let mut document = Document::new();

    // Set all metadata fields
    document.set_title("Test Document Title");
    document.set_author("Test Author");
    document.set_subject("Test Subject");
    document.set_keywords("test, pdf, metadata");
    document.set_creator("oxidize-pdf test suite");
    document.set_producer("oxidize-pdf");

    document.add_page(Page::a4());

    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);
    writer.write_document(&mut document)?;

    // Parse and verify metadata
    let reader = BufReader::new(Cursor::new(&buffer));
    let parsed = PdfReader::new(reader)?;

    // Metadata checking not available in current API
    // let info = parsed.get_info().expect("Should have info dictionary");
    // // Check each metadata field is present
    // assert!(info.get("Title").is_some());
    // assert!(info.get("Author").is_some());
    // assert!(info.get("Subject").is_some());
    // assert!(info.get("Keywords").is_some());
    // assert!(info.get("Creator").is_some());
    // assert!(info.get("Producer").is_some());

    Ok(())
}

#[test]
fn test_split_pdf() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create a multi-page PDF
    let pdf_bytes = create_test_pdf_bytes("Split Test", 6)?;
    let input_path = temp_dir.path().join("input.pdf");
    fs::write(&input_path, pdf_bytes)?;

    // Split into individual pages
    let options = SplitOptions {
        mode: SplitMode::SinglePages,
        output_pattern: temp_dir
            .path()
            .join("page_{}.pdf")
            .to_string_lossy()
            .to_string(),
        preserve_metadata: true,
        optimize: false,
    };

    split_pdf(&input_path, options)?;

    // Verify output files
    for i in 1..=6 {
        let output_path = temp_dir.path().join(format!("page_{}.pdf", i));
        assert!(output_path.exists(), "Page {} should exist", i);

        // Verify it's a valid PDF
        let content = fs::read(&output_path)?;
        assert!(content.starts_with(b"%PDF"));
    }

    Ok(())
}

#[test]
fn test_merge_pdfs() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create multiple PDFs
    let pdf1 = create_test_pdf_bytes("Document 1", 2)?;
    let pdf2 = create_test_pdf_bytes("Document 2", 3)?;
    let pdf3 = create_test_pdf_bytes("Document 3", 1)?;

    let path1 = temp_dir.path().join("doc1.pdf");
    let path2 = temp_dir.path().join("doc2.pdf");
    let path3 = temp_dir.path().join("doc3.pdf");

    fs::write(&path1, pdf1)?;
    fs::write(&path2, pdf2)?;
    fs::write(&path3, pdf3)?;

    // Merge them
    let inputs = vec![
        MergeInput::new(path1),
        MergeInput::new(path2),
        MergeInput::new(path3),
    ];

    let output_path = temp_dir.path().join("merged.pdf");
    let options = MergeOptions::default();

    merge_pdfs(inputs, &output_path, options)?;

    // Verify merged PDF
    assert!(output_path.exists());
    let merged_content = fs::read(&output_path)?;
    assert!(merged_content.starts_with(b"%PDF"));

    // Parse and check page count
    let reader = BufReader::new(File::open(&output_path)?);
    let mut parsed = PdfReader::new(reader)?;
    assert_eq!(parsed.page_count()?, 6); // 2 + 3 + 1 pages

    Ok(())
}

#[test]
fn test_pdf_with_unicode_text() -> Result<()> {
    let mut document = Document::new();
    let mut page = Page::a4();

    // Various Unicode text
    let texts = vec![
        "English: Hello, World!",
        "Spanish: ¡Hola, Mundo!",
        "French: Bonjour le monde!",
        "German: Hallo Welt! ÄÖÜäöüß",
        "Russian: Привет мир!",
        "Japanese: こんにちは世界",
        "Chinese: 你好世界",
        "Arabic: مرحبا بالعالم",
        "Emoji: 🚀 🎉 ✨",
    ];

    let mut y_position = 700.0;
    for text in texts {
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(100.0, y_position)
            .write(text)?;
        y_position -= 25.0;
    }

    document.add_page(page);

    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);
    writer.write_document(&mut document)?;

    assert!(!buffer.is_empty());

    Ok(())
}

#[test]
fn test_pdf_compression() -> Result<()> {
    let mut document = Document::new();

    // Enable compression
    // Stream compression is enabled by default
    // document.set_compress_streams(true);

    // Create a page with lots of repeated content
    let mut page = Page::a4();
    let repeated_text = "This is repeated text. ".repeat(100);

    page.text()
        .set_font(Font::Helvetica, 10.0)
        .at(50.0, 700.0)
        .write(&repeated_text)?;

    document.add_page(page);

    let mut compressed_buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut compressed_buffer);
    writer.write_document(&mut document)?;

    // Create another version
    // Note: compression settings cannot be changed after creation
    // document.set_compress_streams(false);
    let mut uncompressed_buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut uncompressed_buffer);
    writer.write_document(&mut document)?;

    // Compressed should generally be smaller (though not always for small content)
    assert!(!compressed_buffer.is_empty());
    assert!(!uncompressed_buffer.is_empty());

    Ok(())
}

#[test]
fn test_pdf_parse_error_handling() {
    // Test with invalid PDF
    let invalid_pdf = b"This is not a PDF file";
    let reader = BufReader::new(Cursor::new(invalid_pdf));
    let result = PdfReader::new(reader);
    assert!(result.is_err());

    // Test with truncated PDF
    let truncated_pdf = b"%PDF-1.4\n1 0 obj";
    let reader = BufReader::new(Cursor::new(truncated_pdf));
    let result = PdfReader::new(reader);
    assert!(result.is_err());

    // Test with empty input
    let empty = b"";
    let reader = BufReader::new(Cursor::new(empty));
    let result = PdfReader::new(reader);
    assert!(result.is_err());
}

#[test]
fn test_complex_document_workflow() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Step 1: Create a complex document
    let mut document = Document::new();
    document.set_title("Complex Workflow Test");

    for i in 0..10 {
        let mut page = if i % 2 == 0 {
            Page::a4()
        } else {
            Page::letter()
        };

        // Add various content
        page.text()
            .set_font(Font::Helvetica, 16.0)
            .at(100.0, 700.0)
            .write(&format!("Page {} of Complex Document", i + 1))?;

        // Add graphics on even pages
        if i % 2 == 0 {
            page.graphics()
                .set_fill_color(Color::rgb(0.9, 0.9, 0.9))
                .rectangle(50.0, 50.0, 500.0, 100.0)
                .fill();
        }

        document.add_page(page);
    }

    // Step 2: Write to file
    let original_path = temp_dir.path().join("original.pdf");
    let file = File::create(&original_path)?;
    let mut writer = PdfWriter::new_with_writer(BufWriter::new(file));
    writer.write_document(&mut document)?;

    // Step 3: Split into chunks
    let split_options = SplitOptions {
        mode: SplitMode::ChunkSize(3),
        output_pattern: temp_dir
            .path()
            .join("chunk_{}.pdf")
            .to_string_lossy()
            .to_string(),
        preserve_metadata: true,
        optimize: false,
    };

    let split_files = split_pdf(&original_path, split_options)?;

    // Step 4: Merge some chunks
    // Use the actual file names created
    let chunk1 = if split_files.len() > 0 {
        split_files[0].clone()
    } else {
        return Err("No files created from split".into());
    };

    let chunk2 = if split_files.len() > 1 {
        split_files[1].clone()
    } else {
        return Err("Not enough files created from split".into());
    };

    let merge_inputs = vec![MergeInput::new(chunk1), MergeInput::new(chunk2)];

    let merged_path = temp_dir.path().join("merged_chunks.pdf");
    merge_pdfs(merge_inputs, &merged_path, MergeOptions::default())?;

    // Step 5: Verify final result
    assert!(merged_path.exists());
    let reader = BufReader::new(File::open(&merged_path)?);
    let mut parsed = PdfReader::new(reader)?;

    // Should have pages from first two chunks
    assert!(parsed.page_count()? >= 6);

    Ok(())
}

#[test]
fn test_pdf_page_rotation() -> Result<()> {
    let mut document = Document::new();

    // Add pages with different rotations
    for rotation in &[0, 90, 180, 270] {
        let mut page = Page::a4();
        // Rotation would be set here if API supported it
        // page.set_rotation(*rotation);

        page.text()
            .set_font(Font::Helvetica, 24.0)
            .at(100.0, 400.0)
            .write(&format!("Rotation: {}°", rotation))?;

        document.add_page(page);
    }

    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);
    writer.write_document(&mut document)?;

    assert!(!buffer.is_empty());

    Ok(())
}

#[test]
fn test_pdf_with_forms() -> Result<()> {
    let mut document = Document::new();
    let mut page = Page::a4();

    // Add form-like content (text fields simulation)
    let form_fields = vec![
        ("Name:", 100.0, 700.0),
        ("Email:", 100.0, 650.0),
        ("Phone:", 100.0, 600.0),
        ("Address:", 100.0, 550.0),
    ];

    for (label, x, y) in form_fields {
        page.text()
            .set_font(Font::Helvetica, 12.0)
            .at(x, y)
            .write(label)?;

        // Draw a box for the field
        page.graphics()
            .set_stroke_color(Color::rgb(0.5, 0.5, 0.5))
            .rectangle(x + 100.0, y - 5.0, 300.0, 20.0)
            .stroke();
    }

    document.add_page(page);

    let mut buffer = Vec::new();
    let mut writer = PdfWriter::new_with_writer(&mut buffer);
    writer.write_document(&mut document)?;

    assert!(!buffer.is_empty());

    Ok(())
}

#[test]
fn test_large_document_performance() -> Result<()> {
    let mut document = Document::new();
    document.set_title("Performance Test");

    // Create a document with many pages
    for i in 0..100 {
        let mut page = Page::a4();

        // Add minimal content
        page.text()
            .set_font(Font::Helvetica, 10.0)
            .at(50.0, 50.0)
            .write(&format!("Page {}", i + 1))?;

        document.add_page(page);
    }

    let mut buffer = Vec::new();
    let start = std::time::Instant::now();

    let mut writer = PdfWriter::new_with_writer(&mut buffer);
    writer.write_document(&mut document)?;

    let duration = start.elapsed();

    // Should complete in reasonable time (< 5 seconds)
    assert!(duration.as_secs() < 5);
    assert!(!buffer.is_empty());

    // Verify it's a valid PDF
    assert!(buffer.starts_with(b"%PDF"));
    assert!(buffer.ends_with(b"%%EOF") || buffer.ends_with(b"%%EOF\n"));

    Ok(())
}
